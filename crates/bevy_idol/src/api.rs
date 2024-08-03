use std::sync::Arc;

use axum::{Json, Router};
use axum::extract::{DefaultBodyLimit, State, TypedHeader};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::put;
use bevy::math::Vec3;
use bevy::prelude::{Assets, Image, Res, ResMut, Resource, StandardMaterial, Transform};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bytes::Bytes;
use headers::ContentLength;
use tokio::sync::mpsc;

use idol_api::{ApiError, SetCameraRequest, SetFacesRequest};

use crate::tracking::Faces;
use crate::webcam::WebcamTexture;

pub enum Command {
    SetFaces(SetFacesRequest),
    SetCamera(SetCameraRequest),
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

pub fn new_api() -> Router<Arc<ApiState>> {
    Router::new()
        .route("/v1/camera", put(put_camera))
        .route("/v1/faces", put(put_faces))
        .layer(DefaultBodyLimit::disable())
}

#[derive(Resource)]
pub struct ApiResource {
    rx: mpsc::UnboundedReceiver<Command>,
}

pub fn update_api(
    mut api: ResMut<ApiResource>,
    mut faces: ResMut<Faces>,
    webcam: Res<WebcamTexture>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    while let Ok(command) = api.rx.try_recv() {
        match command {
            Command::SetFaces(request) => {
                faces.faces.clear();
                faces.faces.extend(request.faces.into_iter().map(|f| {
                    let transform = Transform::from_matrix(f.transform);
                    crate::tracking::Face {
                        landmarks: f.landmarks,
                        blend_shapes: f.blend_shapes,
                        transform,
                    }
                }));
            }
            Command::SetCamera(request) => {
                // Convert to RGBA
                let size = Extent3d {
                    width: request.width,
                    height: request.height,
                    depth_or_array_layers: 1,
                };
                let image = Image::new(size, TextureDimension::D2, request.payload.to_vec(), TextureFormat::Rgba8UnormSrgb);
                let _ = images.set(&webcam.image, image.clone());
                materials.get_mut(&webcam.material);
            }
        }
    }
}

