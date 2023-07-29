use bevy::prelude::Component;
use idol_api::Face;

#[derive(Component)]
pub struct Faces {
    pub faces: Vec<Face>,
}
