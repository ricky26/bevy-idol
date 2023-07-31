use std::collections::HashMap;

use bevy::prelude::{Resource, Transform};

use idol_api::FaceLandmark;

#[derive(Debug)]
pub struct Face {
    pub landmarks: Vec<FaceLandmark>,
    pub blend_shapes: HashMap<String, f32>,
    pub transform: Transform,
}

#[derive(Debug, Default, Resource)]
pub struct Faces {
    pub faces: Vec<Face>,
}
