use wgpu::util::DeviceExt;

use std::{convert::TryInto, num::NonZeroU64};
use wgpu::{BufferAsyncError, Device, Queue, RequestDeviceError, ShaderModule};

async fn init_device() -> Result<(Device, Queue), RequestDeviceError> {
    let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: None,
        })
        .await
        .expect("Failed to find an appropriate adapter");

    adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::TIMESTAMP_QUERY
                    | wgpu::Features::SPIRV_SHADER_PASSTHROUGH,
                limits: wgpu::Limits::default(),
            },
            None,
        ).await
}

fn load_collatz_shader_module(device: &Device) -> ShaderModule {
    let shader_bytes: &[u8] = include_bytes!(env!("collatz.spv"));
    let spirv = std::borrow::Cow::Owned(wgpu::util::make_spirv_raw(shader_bytes).into_owned());
    let shader_binary = wgpu::ShaderModuleDescriptorSpirV {
        label: None,
        source: spirv,
    };

    // Load the shaders from disk
    unsafe { device.create_shader_module_spirv(&shader_binary) }
}

async fn run_collatz_shader(input: &[u8]) -> Result<Vec<u32>, BufferAsyncError> {
    let (device, queue) = init_device().await.expect("Failed to create device");
    let module = load_collatz_shader_module(&device);

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            // XXX - some graphics cards do not support empty bind layout groups, so
            // create a dummy entry.
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                count: None,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    has_dynamic_offset: false,
                    min_binding_size: Some(NonZeroU64::new(1).unwrap()),
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                },
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: "main_cs",
    });

    let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: input.len() as wgpu::BufferAddress,
        // Can be read to the CPU, and can be copied from the shader's storage buffer
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let storage_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Collatz Conjecture Input"),
        contents: input,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: storage_buffer.as_entire_binding(),
        }],
    });

    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.set_pipeline(&compute_pipeline);
        cpass.dispatch(input.len() as u32 / 64, 1, 1);
    }

    encoder.copy_buffer_to_buffer(
        &storage_buffer,
        0,
        &readback_buffer,
        0,
        input.len() as wgpu::BufferAddress,
    );

    queue.submit(Some(encoder.finish()));
    let buffer_slice = readback_buffer.slice(..);
    let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);
    device.poll(wgpu::Maintain::Wait);

    buffer_future.await.map(|_| {
        buffer_slice
            .get_mapped_range()
            .chunks_exact(4)
            .map(|b| u32::from_ne_bytes(b.try_into().unwrap()))
            .collect::<Vec<_>>()
    })
}

async fn collatz() {
    let top = 2u32.pow(20);
    let src_range = 1..top;

    let src = src_range
        .clone()
        .flat_map(u32::to_ne_bytes)
        .collect::<Vec<_>>();

    if let Ok(result) = run_collatz_shader(&src).await {
        let mut max = 0;
        for (src, out) in src_range.zip(result.iter().copied()) {
            if out == u32::MAX {
                println!("{}: overflowed", src);
                break;
            } else if out > max {
                max = out;
                // Should produce <https://oeis.org/A006877>
                println!("{}: {}", src, out);
            }
        }
    }
}

fn main() {
    futures::executor::block_on(collatz());
}
