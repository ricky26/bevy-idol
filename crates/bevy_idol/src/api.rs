use std::sync::Arc;

use axum::{Json, Router};
use axum::extract::{State, TypedHeader};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{post, put};
use bevy::prelude::{Assets, Image, Query, Res, ResMut, Resource};
use bevy::render::render_resource::Extent3d;
use bytes::Bytes;
use headers::ContentLength;
use tokio::sync::{mpsc, oneshot};

use idol_api::{ApiError, SetCameraRequest, SetFacesRequest, TextureResponse};
use crate::webcam::WebcamTexture;
use crate::output::OutputTexture;
use crate::tracking::Faces;

pub enum Command {
    SetFaces(SetFacesRequest),
    SetCamera(SetCameraRequest),
    RequestTexture(oneshot::Sender<TextureResponse>),
}

pub struct ApiState {
    tx: mpsc::UnboundedSender<Command>,
}

impl ApiState {
    pub fn new() -> (Arc<Self>, ApiResource) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Arc::new(Self {
            tx,
        }), ApiResource {
            rx
        })
    }
}

async fn put_camera(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    TypedHeader(ContentLength(content_length)): TypedHeader<ContentLength>,
    payload: Bytes,
) -> Result<StatusCode, ApiError> {
    let Some(width) = headers.get("width")
        .and_then(|w| w.to_str().ok())
        .and_then(|s| s.parse::<u32>().ok()) else {
        return Err(ApiError::unknown("missing width"));
    };

    let Some(height) = headers.get("height")
        .and_then(|w| w.to_str().ok())
        .and_then(|s| s.parse::<u32>().ok()) else {
        return Err(ApiError::unknown("missing height"));
    };

    let payload_size = width * height * 4;
    if content_length != payload_size as u64 {
        return Err(ApiError::unknown("invalid payload size"));
    }

    state.tx.send(Command::SetCamera(SetCameraRequest {
        width,
        height,
        payload,
    })).ok();
    Ok(StatusCode::OK)
}

async fn put_faces(State(state): State<Arc<ApiState>>, Json(faces): Json<SetFacesRequest>) {
    state.tx.send(Command::SetFaces(faces)).ok();
}

async fn share_texture(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<TextureResponse>, ApiError> {
    let (tx, rx) = oneshot::channel();
    state.tx.send(Command::RequestTexture(tx)).ok();

    rx.await
        .map(Json)
        .map_err(|_| ApiError::unavailable())
}

pub fn new_api() -> Router<Arc<ApiState>> {
    Router::new()
        .route("/v1/camera", put(put_camera))
        .route("/v1/faces", put(put_faces))
        .route("/v1/share-texture", post(share_texture))
}

#[derive(Resource)]
pub struct ApiResource {
    rx: mpsc::UnboundedReceiver<Command>,
}

pub fn update_api(
    mut api: ResMut<ApiResource>,
    mut faces: Query<&mut Faces>,
    mut cameras: Query<&WebcamTexture>,
    output_texture: Res<OutputTexture>,
    mut images: ResMut<Assets<Image>>,
) {
    while let Ok(command) = api.rx.try_recv() {
        match command {
            Command::SetFaces(request) => {
                for mut component in &mut faces {
                    component.faces.clear();
                    component.faces.extend(request.faces.iter().cloned());
                }
            }
            Command::SetCamera(request) => {
                for component in &mut cameras {
                    if let Some(image) = images.get_mut(&component.image) {
                        image.resize(Extent3d {
                            width: request.width,
                            height: request.height,
                            depth_or_array_layers: 1,
                        });
                        image.data.copy_from_slice(&request.payload);
                    }
                }
            }
            Command::RequestTexture(request) => {
                if let Some(response) = output_texture.export() {
                    request.send(response).ok();
                }
            }
        }
    }
}

