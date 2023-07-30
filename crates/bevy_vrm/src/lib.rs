use bevy::app::{App, Plugin};
use bevy::asset::AddAsset;

use crate::loader::VrmLoader;

mod loader;

pub struct VrmPlugin;

impl Plugin for VrmPlugin {
    fn name(&self) -> &str {
        "VRM"
    }

    fn build(&self, app: &mut App) {
        app
            .init_asset_loader::<VrmLoader>();
    }
}
