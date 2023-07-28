use std::collections::HashMap;
use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Message {
    Faces(SetFacesRequest),
    Camera(SetCameraRequest),
    TextureRequest,
    TextureResponse(TextureResponse),
}

#[derive(Serialize, Deserialize)]
pub struct FaceLandmark {
    pub position: Vec3,
    pub presence: Option<f32>,
    pub visibility: Option<f32>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct Face {
    pub landmarks: Vec<FaceLandmark>,
    pub blend_shapes: HashMap<String, f32>,
    pub transform: Mat4,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetFacesRequest {
    pub faces: Vec<Face>,
}

#[derive(Serialize, Deserialize)]
pub struct SetCameraRequest {
    pub width: u32,
    pub height: u32,
    pub bytes: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct TextureResponse {
    pub fd: usize,
    pub width: u32,
    pub height: u32,
}
