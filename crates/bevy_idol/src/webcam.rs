use bevy::pbr::StandardMaterial;
use bevy::prelude::{Handle, Image, Resource};

#[derive(Resource)]
pub struct WebcamTexture {
    pub image: Handle<Image>,
    pub material: Handle<StandardMaterial>,
}
