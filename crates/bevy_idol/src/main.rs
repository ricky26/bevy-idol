use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use anyhow::anyhow;
use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::render::render_resource::{Buffer, BufferDescriptor, BufferUsages, Extent3d, MapMode, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};
use bevy::render::renderer::RenderDevice;
use clap::Parser;
use parking_lot::{Condvar, Mutex};
use v4l::{Format, FourCC, Fraction};
use v4l::io::{mmap, traits::OutputStream};
use v4l::video::Output;

#[derive(Parser)]
struct Options {
    #[arg(long, short = 'c')]
    pub virtual_camera_index: Option<usize>,
    #[arg(long, short = 'f')]
    pub output_fps: Option<usize>,
    #[arg(long, short = 'w', default_value = "1920")]
    pub output_width: u32,
    #[arg(long, short = 'h', default_value = "1080")]
    pub output_height: u32,
}

#[derive(Resource)]
struct VirtualCamera {
    output_buffer: Buffer,
    interval: Duration,
    deadline: Instant,
    state: Arc<CameraState>,
}

#[derive(Default)]
struct CameraState {
    quit: AtomicBool,
    lock: Mutex<()>,
    cond: Condvar,
}

fn virtual_camera_thread(state: Arc<CameraState>, device: v4l::Device, buffer: Buffer) -> anyhow::Result<()> {
    let mut stream = mmap::Stream::with_buffers(&device, v4l::buffer::Type::VideoOutput, 4)?;

    loop {
        let (out_buffer, metadata) = OutputStream::next(&mut stream)?;

        // Wait for a new frame to be ready.
        {
            let mut m = state.lock.lock();
            state.cond.wait(&mut m);
        }

        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let buffer_slice = buffer.slice(..);
        buffer_slice.map_async(MapMode::Read, move |_| tx.send(()).expect("tx should succeed"));
        rx.recv().unwrap();

        let in_buffer = &*buffer_slice.get_mapped_range();
        out_buffer.copy_from_slice(in_buffer);
        buffer.unmap();

        metadata.field = 0;
        metadata.bytesused = in_buffer.len() as u32;
    }
}

fn update_virtual_camera() {}

async fn handle_api() -> anyhow::Result<()> {
    Ok(())
}

fn handle_fatal_errors(mut rx: std::sync::mpsc::Receiver<anyhow::Error>, mut events: EventWriter<AppExit>) {
    while let Ok(v) = rx.try_recv() {
        log::error!("{}", v);
        events.send(AppExit);
    }
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    let options = Options::parse();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let (fatal_error_tx, fatal_error_rx) = std::sync::mpsc::channel();
    let mut app = App::new();
    app
        .add_plugins(DefaultPlugins)
        .add_systems(Update, move |events| handle_fatal_errors(fatal_error_rx, events));

    let fourcc = FourCC::new(b"RGB4");

    let mut render_device = app.world.resource_mut::<RenderDevice>();
    let output_width = options.output_width;
    let output_height = options.output_height;
    let output_size = Extent3d {
        width: output_width,
        height: output_height,
        depth_or_array_layers: 1,
    };
    let output_texture = render_device.create_texture(&TextureDescriptor {
        label: None,
        size: output_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Bgra8UnormSrgb,
        usage: TextureUsages::empty(),
        view_formats: &[TextureFormat::Bgra8UnormSrgb],
    });

    if let Some(virtual_camera_index) = options.virtual_camera_index {
        let device = v4l::device::Device::new(virtual_camera_index)?;
        for format in device.enum_formats()? {
            log::info!("supported format {:?} {}", &format, &format.fourcc);
        }
        let format = device.set_format(&Format::new(output_width, output_height, fourcc))?;
        if format.fourcc != fourcc || format.width != output_width || format.height != output_height {
            return Err(anyhow!("Camera doesn't support {}x{} {}", output_width, output_height, fourcc));
        }

        let mut params = device.params()?;
        if let Some(target_fps) = options.output_fps {
            params.interval = Fraction::new(1, target_fps as u32);
        }
        let params = device.set_params(&params)?;
        let fps = (params.interval.denominator as f32) / (params.interval.numerator as f32);
        let interval = Duration::from_secs_f32(1.0 / fps);

        // let mut stream = mmap::Stream::with_buffers(&device, v4l::buffer::Type::VideoOutput, 4)?;
        // let epoch = Instant::now();
        // let mut deadline = epoch;

        log::info!("Virtual Camera w={} h={} fps={} format={}", format.width, format.height, fps, format.fourcc);

        let state = Arc::new(CameraState::default());
        let state_clone = state.clone();

        let output_buffer_size = 4 * output_width * output_height;
        let output_buffer = render_device.create_buffer(&BufferDescriptor {
            label: None,
            size: output_buffer_size as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let buffer_clone = output_buffer.clone();

        let error_tx = fatal_error_tx.clone();
        std::thread::spawn(move || {
            if let Err(err) = virtual_camera_thread(state_clone, device, buffer_clone) {
                error_tx.send(err).ok();
            }
        });
        app.insert_resource(VirtualCamera {
            output_buffer,
            interval,
            deadline: Instant::now(),
            state,
        });

        // let frame_size = 4 * format.width * format.height;
        // loop {
        //     let frame_start = Instant::now();
        //     let timestamp = frame_start - epoch;
        //     let (buffer, metadata) = OutputStream::next(&mut stream)?;
        //
        //     metadata.field = 0;
        //     metadata.bytesused = frame_size;
        //
        //     if deadline > frame_start {
        //         let to_sleep = deadline - frame_start;
        //         sleep(to_sleep);
        //     }
        //     deadline = frame_start + interval;
        //     buffer.fill(0xff);
        // }
    }

    app.run();
    Ok(())
}
