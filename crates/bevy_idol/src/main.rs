use std::fmt::Write;
use std::path::PathBuf;

use bevy::color::palettes::css::{BEIGE, BLUE, MAROON, RED};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::render::camera::{CameraOutputMode, RenderTarget};
use bevy::render::mesh::morph::MeshMorphWeights;
use bevy::render::mesh::VertexAttributeValues;
use bevy::render::render_resource::Face;
use bevy::render::view::RenderLayers;
use bevy::window::{WindowRef, WindowResolution};
use bevy_inspector_egui::inspector_egui_impls::InspectorEguiImpl;
use clap::Parser;

use bevy_vrm::extensions::vrm::LookAtTarget;
use bevy_vrm::VrmBundle;

use crate::add_blend_shapes::{AddBlendShapes, apply_blend_shapes, BlendShapeLibrary};
use crate::cameras::{OutputCamera, PreviewCamera};
use crate::tracking::Faces;
use crate::webcam::WebcamTexture;

mod api;
mod tracking;
mod webcam;
mod cameras;
mod debug_mesh;
mod add_blend_shapes;

#[derive(Parser, Resource)]
struct Options {
    #[arg(long, default_value = "127.0.0.1:8888")]
    pub api_bind: String,
    #[arg(long, short = 'c')]
    pub virtual_camera_index: Option<usize>,
    #[arg(long, short = 'f')]
    pub output_fps: Option<u32>,
    #[arg(long, short = 'W', default_value = "1920")]
    pub output_width: u32,
    #[arg(long, short = 'H', default_value = "1080")]
    pub output_height: u32,
    #[arg(long)]
    pub extra_blend_shapes: Option<PathBuf>,
    #[arg(long, default_value = "150")]
    pub hot_reload_delay: u64,
    #[arg(long, default_value = "avatars/demo.vrm")]
    pub avatar: String,
}

struct InspectorExtrasPlugin;

impl Plugin for InspectorExtrasPlugin {
    fn build(&self, app: &mut App) {
        let registry = app.world_mut().resource_mut::<AppTypeRegistry>();
        let mut registry = registry.write();
        registry.register_type_data::<Handle<Image>, InspectorEguiImpl>();
    }
}

fn main() -> anyhow::Result<()> {
    let options = Options::parse();
    let mut app = App::new();
    app
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Bevy Idol [Control]".into(),
                        ..default()
                    }),
                    ..default()
                }),
            bevy_egui::EguiPlugin,
            bevy_inspector_egui::quick::WorldInspectorPlugin::default(),
            InspectorExtrasPlugin,
            bevy_obj::ObjPlugin,
            bevy_vrm::VrmPlugin,
        ))
        .init_asset_loader::<debug_mesh::DebugMeshLoader>()
        .init_resource::<Faces>()
        .insert_resource(Msaa::Sample2)
        .add_systems(Update, (
            api::update_api,
            update_face_mesh,
            update_face_transforms,
            update_free_look,
            toggle_visibility,
            update_debug_text,
            update_camera_plane,
            apply_blend_shapes,
            update_morph_targets,
            dump_state,
        ))
        .add_systems(Startup, init);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    if let Some(path) = options.extra_blend_shapes.as_ref() {
        let contents = std::fs::read(path)?;
        let library = BlendShapeLibrary::from_slice(&contents)?;
        info!("loaded {} extra blend shapes", library.blend_shapes.len());
        app.insert_resource(ExtraBlendShapesLibrary {
            library,
        });
    }

    let api_addr = options.api_bind.parse()?;
    let (api_state, api_resource) = api::ApiState::new();
    runtime.spawn(async move {
        if let Err(err) = axum_server::Server::bind(api_addr)
            .serve(api::new_api().with_state(api_state).into_make_service()).await {
            error!("failed to serve API: {}", err);
        }
    });

    app
        .insert_resource(api_resource)
        .insert_resource(options)
        .run();
    Ok(())
}

#[derive(Component)]
struct NeedToCopyMesh;

#[derive(Component)]
struct FaceMesh;

#[derive(Component)]
struct FaceTransform;

#[derive(Component)]
struct FaceBlendShapes;

#[derive(Component)]
struct FreeLook {
    pub move_speed: f32,
    pub look_speed: f32,
}

#[derive(Component)]
struct CameraPlane;

#[derive(Component)]
struct ToggleVisibilityKey(KeyCode);

#[derive(Component)]
struct DebugText;

#[derive(Resource)]
struct ExtraBlendShapesLibrary {
    library: BlendShapeLibrary,
}

fn init(
    assets: Res<AssetServer>,
    extra_blend_shapes: Option<Res<ExtraBlendShapesLibrary>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    options: Res<Options>,
) {
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_xyz(1., 10., 10.)
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Preview Camera
    commands.spawn((
        Name::from("Preview Camera"),
        Camera3dBundle {
            transform: Transform::from_xyz(0., 1., 5.)
                .looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
            tonemapping: Tonemapping::None,
            ..default()
        },
        RenderLayers::from_layers(&[0, 1]),
        PreviewCamera,
        FreeLook {
            move_speed: 10.,
            look_speed: 0.001,
        },
    ));

    // Debug Face
    commands.spawn((
        Name::from("Debug Face"),
        PbrBundle {
            mesh: assets.load("meshes/canonical_face_model.dobj"),
            material: materials.add(StandardMaterial {
                double_sided: true,
                cull_mode: None,
                ..default()
            }),
            transform: Transform::from_xyz(0., 1., -4.8)
                .with_scale(Vec3::ONE * 5.),
            visibility: Visibility::Hidden,
            ..default()
        },
        NeedToCopyMesh,
        FaceMesh,
        RenderLayers::layer(1),
        ToggleVisibilityKey(KeyCode::F8),
    ));

    // Output window
    let output_window = commands
        .spawn((
            Name::from("Output Window"),
            Window {
                title: "Bevy Idol [Output]".into(),
                transparent: true,
                resizable: false,
                resolution: WindowResolution::new(options.output_width as f32, options.output_height as f32)
                    .with_scale_factor_override(1.),
                ..default()
            },
        ))
        .id();
    commands.spawn((
        Name::from("Output Camera"),
        Camera3dBundle {
            transform: Transform::from_xyz(0., 1.5, 1.)
                .looking_at(Vec3::new(0., 1.5, 0.), Vec3::Y),
            camera: Camera {
                target: RenderTarget::Window(WindowRef::Entity(output_window)),
                output_mode: CameraOutputMode::Write {
                    blend_state: None,
                    clear_color: Color::NONE.into(),
                },
                ..default()
            },
            tonemapping: Tonemapping::None,
            ..default()
        },
        RenderLayers::from_layers(&[0, 2]),
        OutputCamera,
    ));

    // Debug Marker
    commands
        .spawn((
            Name::from("Debug Marker"),
            PbrBundle {
                mesh: assets.load("meshes/canonical_face_model.dobj"),
                material: materials.add(StandardMaterial {
                    base_color: Color::linear_rgb(1.0, 0.0, 1.0),
                    double_sided: true,
                    cull_mode: None,
                    ..default()
                }),
                transform: Transform::default()
                    .with_scale(Vec3::ONE * 0.1),
                visibility: Visibility::Hidden,
                ..default()
            },
            FaceTransform,
            ToggleVisibilityKey(KeyCode::F9),
        ));

    // Camera plane
    let mut camera_image = Image::default();
    camera_image.data.copy_from_slice(&[0xff, 0, 0, 0xff]);
    let camera_image = images.add(camera_image);
    let camera_material = materials.add(StandardMaterial {
        base_color_texture: Some(camera_image.clone()),
        perceptual_roughness: 1.,
        unlit: true,
        cull_mode: Some(Face::Front),
        ..default()
    });
    commands.insert_resource(WebcamTexture {
        image: camera_image,
        material: camera_material.clone(),
    });
    commands.spawn((
        Name::from("Camera Plane"),
        PbrBundle {
            mesh: meshes.add(Mesh::from(Plane3d {
                normal: Dir3::NEG_Y,
                half_size: Vec2::splat(5.),
            })),
            material: camera_material,
            transform: Transform::from_xyz(0., 1., -5.)
                .with_rotation(Quat::from_rotation_z(std::f32::consts::PI)
                    * Quat::from_rotation_x(std::f32::consts::PI * 0.5)),
            visibility: Visibility::Hidden,
            ..default()
        },
        CameraPlane,
        RenderLayers::layer(1),
        ToggleVisibilityKey(KeyCode::F7),
    ));

    // Debug Text
    let debug_text_style = TextStyle {
        font: assets.load("fonts/Chewy-Regular.ttf"),
        font_size: 24.,
        color: Color::WHITE,
    };
    commands.spawn((
        TextBundle {
            text: Text::from_sections([
                TextSection::new("", debug_text_style.clone()),
                TextSection::new("", debug_text_style.clone()),
                TextSection::new("", debug_text_style),
            ]),
            visibility: Visibility::Hidden,
            ..default()
        },
        RenderLayers::layer(1),
        ToggleVisibilityKey(KeyCode::F3),
        DebugText,
    ));

    // Avatar
    let mut avatar = commands.spawn((
        Name::from("Avatar"),
        VrmBundle {
            vrm: assets.load(&options.avatar),
            ..default()
        },
    ));

    if let Some(extra_blend_shapes) = extra_blend_shapes.as_ref() {
        avatar
            .insert(AddBlendShapes {
                blend_shapes: extra_blend_shapes.library.blend_shapes.clone(),
            });
    }
}

fn update_face_mesh(
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
    mut entities: Query<(Entity, &mut Handle<Mesh>, Option<&NeedToCopyMesh>), With<FaceMesh>>,
    faces: Res<Faces>,
) {
    for (entity, mut mesh, need_to_copy) in &mut entities {
        if faces.faces.len() == 0 {
            continue;
        }

        if !meshes.contains(&*mesh) {
            continue;
        }

        if need_to_copy.is_some() {
            let new_mesh = meshes.get(&*mesh).unwrap().clone();
            *mesh = meshes.add(new_mesh);
            commands.entity(entity).remove::<NeedToCopyMesh>();
        }

        let mesh = meshes.get_mut(&*mesh).unwrap();
        let face = &faces.faces[0];
        let Some(positions) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) else {
            continue;
        };
        let positions = match positions {
            VertexAttributeValues::Float32x3(positions) => positions,
            _ => panic!("expected vertices to be f32x3"),
        };

        let mut center = face.transform.translation / 8.;
        center.z = 0.;

        let rotation = face.transform.rotation.inverse();

        // Workaround the debug mesh missing the eyes.
        let num_landmarks = positions.len().min(face.landmarks.len());
        for (idx, landmark) in face.landmarks[..num_landmarks].iter().enumerate() {
            let p = landmark.position;
            let p = rotation * (p - center);
            positions[idx] = p.to_array();
        }
    }
}

fn update_face_transforms(
    faces: Res<Faces>,
    mut entities: Query<&mut Transform, With<FaceTransform>>,
) {
    for mut transform in &mut entities {
        let Some(face) = faces.faces.get(0) else {
            continue;
        };

        transform.translation = face.transform.translation + Vec3::Y;
        transform.rotation = face.transform.rotation
            * Quat::from_rotation_y(std::f32::consts::PI);
    }
}

fn update_free_look(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut entities: Query<(&mut Transform, &FreeLook)>,
) {
    let mut translate = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        translate -= Vec3::Z;
    }
    if keys.pressed(KeyCode::KeyS) {
        translate += Vec3::Z;
    }
    if keys.pressed(KeyCode::KeyA) {
        translate -= Vec3::X;
    }
    if keys.pressed(KeyCode::KeyD) {
        translate += Vec3::X;
    }
    if keys.pressed(KeyCode::KeyQ) {
        translate -= Vec3::Y;
    }
    if keys.pressed(KeyCode::KeyE) {
        translate += Vec3::Y;
    }
    translate *= time.delta_seconds();

    let mut rotate = Vec2::ZERO;

    if mouse_buttons.pressed(MouseButton::Left) {
        for motion in mouse_motion.read() {
            rotate += motion.delta;
        }
    }

    for (mut transform, look) in &mut entities {
        transform.rotate_local_y(rotate.x * look.look_speed);
        transform.rotate_local_x(rotate.y * look.look_speed);
        let delta_translation = transform.rotation * translate * look.move_speed;
        transform.translation += delta_translation;
    }
}

fn toggle_visibility(
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Visibility, &ToggleVisibilityKey)>,
) {
    for (mut visibility, toggle) in &mut query {
        if keys.just_pressed(toggle.0) {
            *visibility = match *visibility {
                Visibility::Inherited | Visibility::Visible => Visibility::Hidden,
                Visibility::Hidden => Visibility::Visible,
            }
        }
    }
}

fn update_debug_text(
    faces: Res<Faces>,
    mut debug_text: Query<&mut Text, With<DebugText>>,
    preview_camera: Query<&Transform, With<PreviewCamera>>,
) {
    for mut text in &mut debug_text {
        if let Some(face) = faces.faces.get(0) {
            let transform = face.transform;
            let face_text = format!(
                "face p={}\n  s={}\n  f={:?}\n  u={:?}\n  p1={}\n  p50={}\n  p150={}\n",
                transform.translation, transform.scale, transform.forward(), transform.up(),
                face.landmarks[0].position,
                face.landmarks[49].position,
                face.landmarks[149].position,
            );

            text.sections[0].value = face_text;
        } else {
            text.sections[0].value = "No face\n".into();
        }

        if let Some(transform) = preview_camera.iter().next() {
            text.sections[1].value = format!(
                "camera p={}\n  f={:?}\n  u={:?}\n",
                transform.translation, transform.forward(), transform.up());
        }
    }
}

fn update_camera_plane(
    webcam: Res<WebcamTexture>,
    images: Res<Assets<Image>>,
    mut query: Query<&mut Transform, With<CameraPlane>>,
) {
    let Some(image) = images.get(&webcam.image) else {
        return;
    };

    let size = image.texture_descriptor.size;

    let (x, y) = if size.width > size.height {
        ((size.width as f32) / (size.height as f32), 1.)
    } else {
        (1., (size.height as f32) / (size.width as f32))
    };

    for mut transform in &mut query {
        transform.scale.x = x;
        transform.scale.z = -y;
    }
}

fn update_morph_targets(
    mut gizmos: Gizmos,
    faces: Res<Faces>,
    meshes: Res<Assets<Mesh>>,
    mut entities: Query<(&Handle<Mesh>, &mut MeshMorphWeights)>,
    // humanoids: Query<&Eyes>,
    mut look_targets: Query<&mut Transform, With<LookAtTarget>>,
) {
    let Some(face) = faces.faces.get(0) else {
        return;
    };
    let blend_shapes = &face.blend_shapes;

    let p = face.transform.translation + Vec3::Y;
    let d = face.transform.translation.normalize();
    let l = Vec3::Y + d * -5. / d.z;

    let u = p + face.transform.up() * 0.1;
    let f = p + face.transform.forward() * 0.1;
    gizmos.line(p, u, MAROON);
    gizmos.line(p, f, BEIGE);
    gizmos.line(l, Vec3::Y, BLUE);
    // gizmos.line(p, l, Color::CYAN);

    let mut color = RED;
    for landmarks in face.landmarks[468..].windows(2) {
        let a = &landmarks[0];
        let b = &landmarks[1];

        // let p0 = q.transform_point(a.position);
        // let p1 = q.transform_point(b.position);

        // log::info!("axax {} / {} - {} {}", a.position, b.position, p0, p1);

        // gizmos.line(p0, p1, color);
        // color.set_r(color.r() - 0.1);
    }

    for (mesh, mut weights) in &mut entities {
        let Some(mesh) = meshes.get(mesh) else {
            continue;
        };

        let Some(names) = mesh.morph_target_names() else {
            continue;
        };

        let weights = weights.weights_mut();
        for (name, weight) in names.iter().zip(weights.iter_mut()) {
            *weight = blend_shapes.get(name.as_str()).copied().unwrap_or(0.);
        }
    }

    // let look_target = Vec3::new(
    //     face.blend_shapes.get("")
    //     0.,
    //     0.,
    //     -10.,
    // );

    // for look_at in &humanoids {
    //     let Some(mut target) = look_targets.get_mut(look_at.target).ok() else {
    //         continue;
    //     };

    // target.translation = Vec3::new(0., 0., -10.);
    // }
}

fn dump_state(
    keys: Res<ButtonInput<KeyCode>>,
    faces: Res<Faces>,
) {
    if !keys.just_pressed(KeyCode::F11) {
        return;
    }

    let mut out = String::new();

    for face in &faces.faces {
        writeln!(&mut out, "face").unwrap();

        let mut shapes = face.blend_shapes.iter()
            .map(|(a, b)| (a.clone(), *b))
            .collect::<Vec<_>>();
        shapes.sort_by(|(a, _), (b, _)| a.cmp(b));

        for (shape, weight) in shapes {
            writeln!(&mut out, "  shape {shape} = {weight}").unwrap();
        }
    }

    std::fs::write("out.txt", out).unwrap();
}
