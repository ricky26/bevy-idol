use bevy::ecs::entity::{EntityMapper, MapEntities};
use bevy::ecs::reflect::ReflectMapEntities;
use bevy::math::{Quat, Vec2, vec2, Vec3, Vec4, vec4};
use bevy::prelude::{Component, Entity, FromWorld, Query, ReflectComponent, Transform, With, Without, World};
use bevy::reflect::{Reflect, TypeUuid};
use bevy::utils::HashMap;
use serde::{Deserialize, Serialize};

pub static REQUIRED_BONES: &'static [HumanoidBone] = &[
    HumanoidBone::Hips,
    HumanoidBone::Spine,
    HumanoidBone::Head,
    HumanoidBone::LeftUpperLeg,
    HumanoidBone::LeftLowerLeg,
    HumanoidBone::LeftFoot,
    HumanoidBone::RightUpperLeg,
    HumanoidBone::RightLowerLeg,
    HumanoidBone::RightFoot,
    HumanoidBone::LeftUpperArm,
    HumanoidBone::LeftLowerArm,
    HumanoidBone::LeftHand,
    HumanoidBone::RightUpperArm,
    HumanoidBone::RightLowerArm,
    HumanoidBone::RightHand,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Reflect, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HumanoidBone {
    Hips,
    Spine,
    Chest,
    UpperChest,
    Neck,
    Head,
    LeftEye,
    RightEye,
    Jaw,
    LeftUpperLeg,
    LeftLowerLeg,
    LeftFoot,
    LeftToes,
    RightUpperLeg,
    RightLowerLeg,
    RightFoot,
    RightToes,
    LeftShoulder,
    LeftUpperArm,
    LeftLowerArm,
    LeftHand,
    RightShoulder,
    RightUpperArm,
    RightLowerArm,
    RightHand,
    LeftThumbMetacarpal,
    LeftThumbProximal,
    LeftThumbDistal,
    LeftIndexProximal,
    LeftIndexIntermediate,
    LeftIndexDistal,
    LeftMiddleProximal,
    LeftMiddleIntermediate,
    LeftMiddleDistal,
    LeftRingProximal,
    LeftRingIntermediate,
    LeftRingDistal,
    LeftLittleProximal,
    LeftLittleIntermediate,
    LeftLittleDistal,
    RightThumbMetacarpal,
    RightThumbProximal,
    RightThumbDistal,
    RightIndexProximal,
    RightIndexIntermediate,
    RightIndexDistal,
    RightMiddleProximal,
    RightMiddleIntermediate,
    RightMiddleDistal,
    RightRingProximal,
    RightRingIntermediate,
    RightRingDistal,
    RightLittleProximal,
    RightLittleIntermediate,
    RightLittleDistal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HumanBoneJson {
    pub node: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HumanoidJson {
    pub human_bones: HashMap<HumanoidBone, HumanBoneJson>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RangeMapJson {
    pub input_max_value: f32,
    pub output_scale: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LookAtMode {
    Bone,
    Expression,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LookAtJson {
    #[serde(rename = "type")]
    pub mode: LookAtMode,
    pub offset_from_head_bone: Vec3,
    pub range_map_horizontal_inner: RangeMapJson,
    pub range_map_horizontal_outer: RangeMapJson,
    pub range_map_vertical_down: RangeMapJson,
    pub range_map_vertical_up: RangeMapJson,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VrmExtensionJson {
    pub spec_version: String,
    pub humanoid: HumanoidJson,
    pub look_at: LookAtJson,
}

#[derive(Debug, Clone, Default, Reflect, Component, TypeUuid)]
#[reflect(Debug, Component, MapEntities)]
#[uuid = "04dbd854-742e-4309-91f9-b6b963e4c438"]
pub struct Humanoid {
    pub bones: HashMap<HumanoidBone, Entity>,
}

impl MapEntities for Humanoid {
    fn map_entities(&mut self, entity_mapper: &mut EntityMapper) {
        for entity in self.bones.values_mut() {
            *entity = entity_mapper.get_or_reserve(*entity);
        }
    }
}

#[derive(Debug, Clone, Default, Reflect, Component, TypeUuid)]
#[reflect(Debug, Component)]
#[uuid = "43d4202d-692f-4413-8051-30ecb3f9fac2"]
pub struct Eye;

#[derive(Debug, Clone, Default, Reflect, Component, TypeUuid)]
#[reflect(Debug, Component)]
#[uuid = "c8cd7bd2-7843-41cd-8774-164137508100"]
pub enum LookAt {
    Entity(Entity),
    Fixed { left: Vec2, right: Vec2 },
}

#[derive(Debug, Clone, Reflect, Component, TypeUuid)]
#[reflect(Debug, Component, MapEntities)]
#[uuid = "9891590c-8b98-4434-91b7-a9a6fc2e848f"]
pub struct Eyes {
    pub mode: LookAtMode,
    pub input_scale: Vec4,
    pub output_scale: Vec4,
    pub left_base_rotation: Quat,
    pub right_base_rotation: Quat,
}

impl Eyes {
    pub fn from_json(
        json: &LookAtJson, target: Entity,
        left_base_rotation: Quat,
        right_base_rotation: Quat,
    ) -> Self {
        let input_scale = vec4(
            json.range_map_horizontal_inner.input_max_value,
            json.range_map_horizontal_outer.input_max_value,
            json.range_map_vertical_down.input_max_value,
            json.range_map_vertical_up.input_max_value,
        );
        let output_scale = vec4(
            json.range_map_horizontal_inner.output_scale,
            json.range_map_horizontal_outer.output_scale,
            json.range_map_vertical_down.output_scale,
            json.range_map_vertical_up.output_scale,
        ) / input_scale;

        Self {
            mode: json.mode,
            target,
            input_scale,
            output_scale,
            left_base_rotation,
            right_base_rotation,
        }
    }

    pub fn evaluate_one(&self, target: Vec3) -> Vec2 {
        let (outer, inner) = if target.x < 0. {
            ((-target.x / target.z).atan(), 0.)
        } else {
            (0., (target.x / target.z).atan())
        };

        let (down, up) = if target.y < 0. {
            ((-target.y / target.z).atan(), 0.)
        } else {
            (0., (target.y / target.z).atan())
        };

        let result = Vec4::new(outer, inner, down, up).min(self.input_scale)
            * self.output_scale;
        vec2(result.x - result.y, result.z - result.w)
    }

    pub fn evaluate(&self, target: Vec3) -> (Vec2, Vec2) {
        let left = self.evaluate_one(target);
        let right = self.evaluate_one(target * Vec3::new(-1., 1., 1.))
            * Vec2::new(-1., 1.);
        (left, right)
    }
}

impl MapEntities for Eyes {
    fn map_entities(&mut self, entity_mapper: &mut EntityMapper) {
        self.target = entity_mapper.get_or_reserve(self.target);
    }
}

impl FromWorld for Eyes {
    fn from_world(_world: &mut World) -> Self {
        Eyes {
            mode: LookAtMode::Bone,
            input_scale: Default::default(),
            output_scale: Default::default(),
            left_base_rotation: Default::default(),
            right_base_rotation: Default::default(),
        }
    }
}

pub fn apply_look_at(
    humanoids: Query<(&Eyes, &Humanoid)>,
    targets: Query<&Transform, (With<Eyes>, Without<Eye>)>,
    mut eyes: Query<&mut Transform, (With<Eye>, Without<Eyes>)>,
) {
    for (look_at, humanoid) in &humanoids {
        let Some(target) = targets.get(look_at.target)
            .ok()
            .map(|t| t.translation) else {
            continue;
        };
        let (left, right) = look_at.evaluate(target);

        let mut update_eye = |bone, rotation: Vec2, base: Quat|
            if let Some(mut transform) = humanoid.bones
                .get(&bone)
                .and_then(|b| eyes.get_mut(*b).ok()) {
                transform.rotation = Quat::from_rotation_y(rotation.x)
                    * Quat::from_rotation_x(rotation.y)
                    * base;
            };

        match look_at.mode {
            LookAtMode::Bone => {
                update_eye(HumanoidBone::LeftEye, left, look_at.left_base_rotation);
                update_eye(HumanoidBone::RightEye, right, look_at.right_base_rotation);
            }
            LookAtMode::Expression => {}
        }
    }
}
