use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use anyhow::anyhow;
use bevy::prelude::Resource;
use bevy::render::render_resource::{Buffer, BufferDescriptor, BufferUsages, MapMode};
use bevy::render::renderer::RenderDevice;
use parking_lot::{Condvar, Mutex};
use tokio::sync::oneshot;
use v4l::{Format, FourCC, Fraction};
use v4l::io::mmap;
use v4l::io::traits::OutputStream;
use v4l::video::Output;

use crate::output::OutputTexture;

#[derive(Default)]
struct CameraState {
    quit: AtomicBool,
    lock: Mutex<()>,
    cond: Condvar,
}

#[derive(Resource)]
pub struct VirtualCamera {
    output_buffer: Buffer,
    interval: Duration,
    deadline: Instant,
    state: Arc<CameraState>,
    quit_rx: oneshot::Receiver<()>,
}

impl VirtualCamera {
    fn update_thread(
        state: Arc<CameraState>, quit: oneshot::Sender<()>, device: v4l::Device, buffer: Buffer,
    ) -> anyhow::Result<()> {
        let mut stream = mmap::Stream::with_buffers(&device, v4l::buffer::Type::VideoOutput, 4)?;

        while !quit.is_closed() {
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

        Ok(())
    }

    pub fn new(
        output: &OutputTexture,
        render_device: &RenderDevice,
        index: usize,
        fps: Option<u32>,
    ) -> anyhow::Result<VirtualCamera> {
        let fourcc = FourCC::new(b"RGB4");
        let device = v4l::device::Device::new(index)?;
        for format in device.enum_formats()? {
            log::info!("supported format {:?} {}", &format, &format.fourcc);
        }
        let format = device.set_format(&Format::new(output.width, output.height, fourcc))?;
        if format.fourcc != fourcc || format.width != output.width || format.height != output.height {
            return Err(anyhow!("Camera doesn't support {}x{} {}", output.width, output.height, fourcc));
        }

        let mut params = device.params()?;
        if let Some(target_fps) = fps {
            params.interval = Fraction::new(1, target_fps);
        }
        let params = device.set_params(&params)?;
        let fps = (params.interval.denominator as f32) / (params.interval.numerator as f32);
        let interval = Duration::from_secs_f32(1.0 / fps);

        log::info!("Virtual Camera w={} h={} fps={} format={}", format.width, format.height, fps, format.fourcc);

        let state = Arc::new(CameraState::default());
        let state_clone = state.clone();

        let output_buffer_size = 4 * output.width * output.height;
        let output_buffer = render_device.create_buffer(&BufferDescriptor {
            label: None,
            size: output_buffer_size as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let buffer_clone = output_buffer.clone();

        let (quit_tx, quit_rx) = oneshot::channel();
        std::thread::spawn(move || {
            if let Err(err) = Self::update_thread(state_clone, quit_tx, device, buffer_clone) {
                log::error!("virtual camera error: {}", err);
            }
        });

        Ok(VirtualCamera {
            output_buffer,
            interval,
            deadline: Instant::now(),
            state,
            quit_rx,
        })
    }
}

pub fn update_virtual_cameras() {

}

