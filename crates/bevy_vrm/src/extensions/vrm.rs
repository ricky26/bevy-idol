use bevy::ecs::entity::{EntityMapper, MapEntities};
use bevy::ecs::reflect::ReflectMapEntities;
use bevy::math::{vec2, vec4};
use bevy::prelude::*;
use bevy::reflect::Reflect;
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
pub enum LookAtModeJson {
    Bone,
    Expression,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LookAtJson {
    #[serde(rename = "type")]
    pub mode: LookAtModeJson,
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

#[derive(Debug, Clone, Default, Reflect, Component)]
#[reflect(Debug, Component, MapEntities)]
pub struct Humanoid {
    pub bones: HashMap<HumanoidBone, Entity>,
}

impl MapEntities for Humanoid {
    fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
        for entity in self.bones.values_mut() {
            *entity = entity_mapper.map_entity(*entity);
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Reflect, Component)]
#[reflect(Debug, Component)]
pub struct Eye;

#[derive(Debug, Clone, Copy, Reflect, Component)]
#[reflect(Debug, Component)]
pub struct LookAtRangeMap {
    pub input_scale: Vec4,
    pub output_scale: Vec4,
}

impl Default for LookAtRangeMap {
    fn default() -> Self {
        LookAtRangeMap {
            input_scale: Vec4::splat(1.),
            output_scale: Vec4::splat(1.),
        }
    }
}

impl LookAtRangeMap {
    pub fn flipped(&self) -> LookAtRangeMap {
        LookAtRangeMap {
            input_scale: self.input_scale.yxzw(),
            output_scale: self.output_scale.yxzw(),
        }
    }

    pub fn evaluate(&self, target: Vec3) -> Vec2 {
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

    pub fn evaluate_both(&self, target: Vec3) -> (Vec2, Vec2) {
        let left = self.evaluate(target);
        let right = self.evaluate(target * Vec3::new(-1., 1., 1.))
            * Vec2::new(-1., 1.);
        (left, right)
    }
}

impl From<&LookAtJson> for LookAtRangeMap {
    fn from(json: &LookAtJson) -> Self {
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
            input_scale,
            output_scale,
        }
    }
}

#[derive(Debug, Clone, Reflect, Component)]
#[reflect(Debug, Component, MapEntities)]
pub struct LookAtTarget(pub Entity);

impl FromWorld for LookAtTarget {
    fn from_world(_world: &mut World) -> Self {
        LookAtTarget(Entity::PLACEHOLDER)
    }
}

impl MapEntities for LookAtTarget {
    fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
        self.0 = entity_mapper.map_entity(self.0);
    }
}

#[derive(Debug, Clone, Reflect, Component)]
#[reflect(Debug, Component)]
pub struct TransformLookAt {
    pub offset: Quat,
}

#[derive(Debug, Clone, Reflect, Component)]
#[reflect(Debug, Component)]
pub struct MorphTargetLookAt {
    pub up_morph: Option<usize>,
    pub down_morph: Option<usize>,
    pub left_morph: Option<usize>,
    pub right_morph: Option<usize>,
}

pub fn apply_transform_look_at(
    mut set: ParamSet<(
        (
            Query<(
                &LookAtTarget,
                Option<&Parent>,
            )>,
            Query<&GlobalTransform>,
        ),
        Query<(
            &TransformLookAt,
            &mut Transform,
            &mut GlobalTransform,
            &LookAtRangeMap,
        )>,
    )>,
    mut scratch_targets: Local<Vec<Option<(GlobalTransform, Vec3)>>>,
) {
    let (query, global_transforms) = set.p0();
    for (target, parent) in &query {
        let parent_transform = if let Some(parent) = parent {
            if let Ok(transform) = global_transforms.get(parent.get()) {
                Some(transform.clone())
            } else {
                None
            }
        } else {
            Some(GlobalTransform::default())
        };
        let target = if let Ok(transform) = global_transforms.get(target.0) {
            Some(transform.translation())
        } else {
            None
        };
        let state = if let (Some(transform), Some(target)) = (parent_transform, target) {
            Some((transform, target))
        } else {
            None
        };
        scratch_targets.push(state)
    }

    for ((
        look_at,
        mut local_transform,
        mut global_transform,
        range_map,
    ), state) in set.p1().iter_mut().zip(scratch_targets.drain(..)) {
        let Some((parent_transform, target_pos)) = state else {
            continue;
        };

        let local_target = global_transform.affine().inverse().transform_point3(target_pos);
        let rotation2 = range_map.evaluate(local_target);
        let rotation = Quat::from_rotation_y(rotation2.x)
            * Quat::from_rotation_x(rotation2.y)
            * look_at.offset;
        local_transform.rotation = rotation;
        *global_transform = parent_transform * *local_transform;
    }
}
