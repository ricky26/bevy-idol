use bevy::pbr::StandardMaterial;
use bevy::prelude::{Component, Handle, Image};

#[derive(Component)]
pub struct WebcamTexture {
    pub image: Handle<Image>,
    pub material: Handle<StandardMaterial>,
}
