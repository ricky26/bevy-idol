use bevy::core_pipeline::clear_color::ClearColorConfig;
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::view::RenderLayers;
use bevy::window::{WindowRef, WindowResolution};
use clap::Parser;

use crate::cameras::{OutputCamera, PreviewCamera};

mod api;
mod tracking;
mod webcam;
mod output;
mod cameras;

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
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Idol [Control]".into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Update, (
            api::update_api,
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

fn init(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
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
    ));

    let output_window = commands
        .spawn(Window {
            title: "Bevy Idol [Output]".into(),
            transparent: true,
            resizable: false,
            resolution: WindowResolution::new(options.output_width as f32, options.output_height as f32),
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
                clear_color: ClearColorConfig::None,
                ..default()
            },
            ..default()
        },
        RenderLayers::from_layers(&[0, 2]),
        OutputCamera,
    ));

    // Test Sphere
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::try_from(shape::Icosphere {
            radius: 1.0,
            subdivisions: 16,
        }).unwrap()),
        material: materials.add(StandardMaterial {
            base_color: Color::rgb(1.0, 0.0, 1.0),
            ..default()
        }),
        ..default()
    });
}
