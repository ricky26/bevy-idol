use bevy::asset::{AssetLoader, BoxedFuture, Error, LoadContext};

#[derive(Default)]
pub struct VrmLoader;

impl AssetLoader for VrmLoader {
    fn load<'a>(&'a self, bytes: &'a [u8], load_context: &'a mut LoadContext) -> BoxedFuture<'a, anyhow::Result<(), Error>> {
        Box::pin(load(bytes, load_context))
    }

    fn extensions(&self) -> &[&str] {
        &["vrm"]
    }
}


async fn load(bytes: &[u8], load_context: &mut LoadContext<'_>) -> anyhow::Result<(), Error> {
    // let gltf = gltf::Gltf::from_slice(bytes)?;
    // load_context.asset_io().load_path()
    Ok(())
}
