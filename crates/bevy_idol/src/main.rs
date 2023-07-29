use bevy::prelude::*;
use bevy::render::renderer::RenderDevice;
use clap::Parser;

use crate::virtual_camera::{update_virtual_cameras, VirtualCamera};

mod api;
mod tracking;
mod webcam;
mod output;
mod virtual_camera;

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
    let render_device = app.world.resource::<RenderDevice>();
    let output_width = options.output_width;
    let output_height = options.output_height;
    let output_texture = output::OutputTexture::new(&render_device, output_width, output_height);
    if let Some(virtual_camera_index) = options.virtual_camera_index {
        app.insert_resource(VirtualCamera::new(&output_texture, &render_device, virtual_camera_index, options.output_fps)?);
    }

    app.insert_resource(output_texture);
    app.run();
    Ok(())
}

fn init() {}
