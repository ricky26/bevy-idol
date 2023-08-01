use serde::{Deserialize, Serialize};

pub mod vrm;
pub mod mtoon;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TextureInfo {
    pub index: u32,
    #[serde(default)]
    pub tex_coord: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RootExtensions {
    #[serde(rename = "VRMC_vrm")]
    pub vrm: vrm::VrmExtensionJson,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaterialExtensions {
    #[serde(rename = "VRMC_materials_mtoon")]
    pub mtoon: Option<mtoon::MToonExtensionJson>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtendedMaterial {
    pub extensions: MaterialExtensions,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtendedRoot {
    pub extensions: RootExtensions,
    pub materials: Vec<ExtendedMaterial>,
}
