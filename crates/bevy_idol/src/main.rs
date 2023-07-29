use bevy::prelude::*;
use bevy::render::camera::{ManualTextureView, ManualTextureViewHandle, ManualTextureViews, RenderTarget};
use bevy::render::render_resource::{TextureFormat, TextureViewDescriptor};
use bevy::render::RenderApp;
use bevy::render::renderer::{RenderDevice};
use bevy::render::view::RenderLayers;
use clap::Parser;
use wgpu::TextureViewDimension;

use crate::cameras::{OutputCamera, PreviewCamera};
use crate::output::OutputTexture;
use crate::virtual_camera::{update_virtual_cameras, VirtualCamera};

mod api;
mod tracking;
mod webcam;
mod output;
mod virtual_camera;
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
    tracing_subscriber::fmt().init();
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


    let mut app = App::new();
    app
        .add_plugins(DefaultPlugins)
        .insert_resource(api_resource)
        .add_systems(Update, (
            api::update_api,
            update_virtual_cameras,
        ))
        .add_systems(Startup, init);

    // Finish app setup now, so that we can access the render device
    while !app.ready() {
        bevy::tasks::tick_global_task_pools_on_main_thread();
    }
    app.finish();

    // Create output
    let render_app = app.sub_app_mut(RenderApp);
    let render_device = render_app.world.resource::<RenderDevice>();
    let output_width = options.output_width;
    let output_height = options.output_height;
    let output_texture = OutputTexture::new(
        &render_device, output_width, output_height);
    if let Some(virtual_camera_index) = options.virtual_camera_index {
        let virtual_camera = VirtualCamera::new(&output_texture, &render_device, virtual_camera_index, options.output_fps)?;
        app.insert_resource(virtual_camera);
    }

    app.insert_resource(output_texture);
    app.run();
    Ok(())
}

fn init(
    mut commands: Commands,
    mut texture_views: ResMut<ManualTextureViews>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    output_texture: Res<OutputTexture>,
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

    let output_view_handle = ManualTextureViewHandle(0);
    let output_view = output_texture.texture().create_view(&TextureViewDescriptor {
        label: Some("Output View"),
        format: Some(TextureFormat::Bgra8UnormSrgb),
        dimension: Some(TextureViewDimension::D2),
        ..default()
    });
    let output_view = ManualTextureView {
        texture_view: output_view,
        size: UVec2::new(output_texture.width(), output_texture.height()),
        format: TextureFormat::Bgra8UnormSrgb,
    };
    texture_views.insert(output_view_handle.clone(), output_view);
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0., 0., -10.)
                .looking_at(Vec3::ZERO, Vec3::Y),
            camera: Camera {
                target: RenderTarget::TextureView(output_view_handle),
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
