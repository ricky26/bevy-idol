use std::fmt::Write;

use bevy::core_pipeline::clear_color::ClearColorConfig;
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::mesh::VertexAttributeValues;
use bevy::render::view::RenderLayers;
use bevy::window::{WindowRef, WindowResolution};
use clap::Parser;

use crate::cameras::{OutputCamera, PreviewCamera};
use crate::tracking::Faces;
use crate::webcam::WebcamTexture;

mod api;
mod tracking;
mod webcam;
mod output;
mod cameras;
mod debug_mesh;

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
}

fn main() -> anyhow::Result<()> {
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
            bevy_inspector_egui::DefaultInspectorConfigPlugin,
            bevy_obj::ObjPlugin,
            bevy_vrm::VrmPlugin,
        ))
        .init_asset_loader::<debug_mesh::DebugMeshLoader>()
        .init_resource::<Faces>()
        .add_systems(Update, (
            api::update_api,
            update_face_mesh,
            update_face_transforms,
            update_free_look,
            toggle_visibility,
            update_debug_text,
        ))
        .add_systems(Startup, init);
    let options = Options::parse();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let api_addr = options.api_bind.parse()?;
    let (api_state, api_resource) = api::ApiState::new();
    runtime.spawn(async move {
        if let Err(err) = axum::Server::bind(&api_addr)
            .serve(api::new_api().with_state(api_state).into_make_service()).await {
            log::error!("failed to serve API: {}", err);
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
struct ToggleVisibilityKey(KeyCode);

#[derive(Component)]
struct DebugText;

fn init(
    assets: Res<AssetServer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    options: Res<Options>,
) {
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_xyz(10., 100., 0.)
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0., 0., -10.)
                .looking_at(Vec3::ZERO, Vec3::Y),
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
        PbrBundle {
            mesh: assets.load("meshes/canonical_face_model.dobj"),
            material: materials.add(StandardMaterial {
                double_sided: true,
                cull_mode: None,
                ..default()
            }),
            transform: Transform::default()
                .with_scale(Vec3::ONE * 10.),
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
        .spawn(Window {
            title: "Bevy Idol [Output]".into(),
            transparent: true,
            resizable: false,
            resolution: WindowResolution::new(options.output_width as f32, options.output_height as f32)
                .with_scale_factor_override(1.),
            ..default()
        })
        .id();
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0., 0., -10.)
                .looking_at(Vec3::ZERO, Vec3::Y),
            camera: Camera {
                target: RenderTarget::Window(WindowRef::Entity(output_window)),
                ..default()
            },
            camera_3d: Camera3d {
                clear_color: ClearColorConfig::Custom(Color::NONE),
                ..default()
            },
            ..default()
        },
        UiCameraConfig {
            show_ui: false,
        },
        RenderLayers::from_layers(&[0, 2]),
        OutputCamera,
    ));

    // Test Sphere
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::try_from(shape::Icosphere {
                radius: 1.0,
                subdivisions: 16,
            }).unwrap()),
            material: materials.add(StandardMaterial {
                base_color: Color::rgb(1.0, 0.0, 1.0),
                ..default()
            }),
            ..default()
        },
        FaceTransform,
    ));

    // Camera plane
    let mut camera_image = Image::default();
    camera_image.data.copy_from_slice(&[0xff, 0, 0, 0xff]);
    let camera_image = images.add(camera_image);
    let camera_material = materials.add(StandardMaterial {
        base_color_texture: Some(camera_image.clone()),
        perceptual_roughness: 1.,
        unlit: true,
        ..default()
    });
    commands.insert_resource(WebcamTexture {
        image: camera_image,
        material: camera_material.clone(),
    });
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane {
                size: 5.,
                subdivisions: 1,
            })),
            material: camera_material,
            transform: Transform::from_xyz(0., 1., 0.)
                .with_rotation(Quat::from_rotation_x(std::f32::consts::PI * -0.5)),
            visibility: Visibility::Hidden,
            ..default()
        },
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
    commands.spawn(SceneBundle {
        scene: assets.load("avatars/demo.vrm"),
        ..default()
    });
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

        if !meshes.contains(&mesh) {
            continue;
        }

        if need_to_copy.is_some() {
            let new_mesh = meshes.get(&mesh).unwrap().clone();
            *mesh = meshes.add(new_mesh);
            commands.entity(entity).remove::<NeedToCopyMesh>();
        }

        let mesh = meshes.get_mut(&mesh).unwrap();
        let face = &faces.faces[0];
        let Some(positions) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) else {
            continue;
        };
        let positions = match positions {
            VertexAttributeValues::Float32x3(positions) => positions,
            _ => panic!("expected vertices to be f32x3"),
        };

        // Workaround the debug mesh missing the eyes.
        let num_landmarks = positions.len().min(face.landmarks.len());
        for (idx, landmark) in face.landmarks[..num_landmarks].iter().enumerate() {
            positions[idx] = landmark.position.to_array();
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

        *transform = face.transform;
    }
}

fn update_free_look(
    time: Res<Time>,
    keys: Res<Input<KeyCode>>,
    mouse_buttons: Res<Input<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut entities: Query<(&mut Transform, &FreeLook)>,
) {
    let mut translate = Vec3::ZERO;
    if keys.pressed(KeyCode::W) {
        translate -= Vec3::Z;
    }
    if keys.pressed(KeyCode::S) {
        translate += Vec3::Z;
    }
    if keys.pressed(KeyCode::A) {
        translate -= Vec3::X;
    }
    if keys.pressed(KeyCode::D) {
        translate += Vec3::X;
    }
    if keys.pressed(KeyCode::Q) {
        translate -= Vec3::Y;
    }
    if keys.pressed(KeyCode::E) {
        translate += Vec3::Y;
    }
    translate *= time.delta_seconds();

    let mut rotate = Vec2::ZERO;

    if mouse_buttons.pressed(MouseButton::Left) {
        for motion in &mut mouse_motion {
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
    keys: Res<Input<KeyCode>>,
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

            text.sections[0].value = format!(
                "face p={}\n  f={}\n  u={}\n  p1={}\n  p50={}\n  p150={}\n",
                transform.translation, transform.forward(), transform.up(),
                face.landmarks[0].position,
                face.landmarks[49].position,
                face.landmarks[149].position,
            );
        } else {
            text.sections[0].value = "No face\n".into();
        }

        if let Some(transform) = preview_camera.iter().next() {
            text.sections[1].value = format!(
                "camera p={}\n  f={}\n  u={}\n",
                transform.translation, transform.forward(), transform.up());
        }
    }
}
