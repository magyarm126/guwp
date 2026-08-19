use std::{num::NonZeroU64, str::FromStr, sync::Arc};

use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle},
    window::{Window, WindowId},
};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

struct State {
    instance: wgpu::Instance,
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    render_pipeline: wgpu::RenderPipeline,
    arguments: Vec<f32>,
    result: Option<Vec<f32>>,
    cursor_position: (f64, f64),
    button_pressed: bool,
}

impl State {
    async fn new(
        display: OwnedDisplayHandle,
        window: Arc<Window>,
        arguments: Vec<f32>,
    ) -> State {
        let instance = wgpu::Instance::new(
            wgpu::InstanceDescriptor::new_with_display_handle(Box::new(display)),
        );

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("Failed to create adapter");

        println!("Running on Adapter: {:#?}", adapter.get_info());

        let downlevel_capabilities = adapter.get_downlevel_capabilities();

        if !downlevel_capabilities
            .flags
            .contains(wgpu::DownlevelFlags::COMPUTE_SHADERS)
        {
            panic!("Adapter does not support compute shaders");
        }

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("Failed to create device");

        let size = window.inner_size();

        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create surface");

        let capabilities = surface.get_capabilities(&adapter);

        let surface_format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .unwrap_or(capabilities.formats[0]);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Canvas Shader"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
) -> VertexOutput {
    var output: VertexOutput;

    output.position = vec4<f32>(
        position.x,
        position.y,
        0.0,
        1.0
    );

    output.color = color;

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
"#
                    .into(),
            ),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Canvas Pipeline Layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });

        let render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Canvas Pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(Vertex::desc())],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                cache: None,
                multiview_mask: None,
            });

        let state = State {
            instance,
            window,
            device,
            queue,
            size,
            surface,
            surface_format,
            render_pipeline,
            arguments,
            result: None,
            cursor_position: (0.0, 0.0),
            button_pressed: false,
        };

        state.configure_surface();

        state
    }

    fn configure_surface(&self) {
        if self.size.width == 0 || self.size.height == 0 {
            return;
        }

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };

        self.surface.configure(&self.device, &surface_config);
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }

        self.size = new_size;
        self.configure_surface();
    }

    fn button_rect(&self) -> (f64, f64, f64, f64) {
        let width = 180.0;
        let height = 60.0;
        let x = (self.size.width as f64 - width) / 2.0;
        let y = self.size.height as f64 - 100.0;

        (x, y, width, height)
    }

    fn cursor_in_button(&self) -> bool {
        let (x, y, width, height) = self.button_rect();

        self.cursor_position.0 >= x
            && self.cursor_position.0 <= x + width
            && self.cursor_position.1 >= y
            && self.cursor_position.1 <= y + height
    }

    fn run_compute(&mut self) {
        let module = self
            .device
            .create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let input_data_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Input Buffer"),
                    contents: bytemuck::cast_slice(&self.arguments),
                    usage: wgpu::BufferUsages::STORAGE,
                });

        let output_data_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: input_data_buffer.size(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let download_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Download Buffer"),
            size: input_data_buffer.size(),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Compute Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                min_binding_size: Some(
                                    NonZeroU64::new(4).unwrap(),
                                ),
                                has_dynamic_offset: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                min_binding_size: Some(
                                    NonZeroU64::new(4).unwrap(),
                                ),
                                has_dynamic_offset: false,
                            },
                            count: None,
                        },
                    ],
                });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_data_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_data_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Compute Pipeline Layout"),
                    bind_group_layouts: &[Some(&bind_group_layout)],
                    immediate_size: 0,
                });

        let pipeline =
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Compute Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &module,
                    entry_point: Some("doubleMe"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    cache: None,
                });

        let mut encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Compute Encoder"),
                });

        {
            let mut compute_pass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Compute Pass"),
                    timestamp_writes: None,
                });

            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            let workgroup_count = self.arguments.len().div_ceil(64);

            compute_pass.dispatch_workgroups(workgroup_count as u32, 1, 1);
        }

        encoder.copy_buffer_to_buffer(
            &output_data_buffer,
            0,
            &download_buffer,
            0,
            output_data_buffer.size(),
        );

        self.queue.submit([encoder.finish()]);
        
        let buffer_slice = download_buffer.slice(..);

        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});

        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("Failed while waiting for GPU");

        let result: Vec<f32> = {
            let data = buffer_slice
                .get_mapped_range()
                .expect("Failed to get mapped buffer");

            bytemuck::allocation::pod_collect_to_vec(&data)
        };

        download_buffer.unmap();

        println!("Result: {result:?}");

        self.result = Some(result);

    }

    fn make_rect(
        &self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
    ) -> [Vertex; 6] {
        let window_width = self.size.width as f32;
        let window_height = self.size.height as f32;

        let left = x / window_width * 2.0 - 1.0;
        let right = (x + width) / window_width * 2.0 - 1.0;

        let top = 1.0 - y / window_height * 2.0;
        let bottom = 1.0 - (y + height) / window_height * 2.0;

        [
            Vertex {
                position: [left, top],
                color,
            },
            Vertex {
                position: [right, top],
                color,
            },
            Vertex {
                position: [right, bottom],
                color,
            },
            Vertex {
                position: [left, top],
                color,
            },
            Vertex {
                position: [right, bottom],
                color,
            },
            Vertex {
                position: [left, bottom],
                color,
            },
        ]
    }

    fn digit_segments(digit: u8) -> [bool; 7] {
        match digit {
            0 => [true, true, true, true, true, true, false],
            1 => [false, true, true, false, false, false, false],
            2 => [true, true, false, true, true, false, true],
            3 => [true, true, true, true, false, false, true],
            4 => [false, true, true, false, false, true, true],
            5 => [true, false, true, true, false, true, true],
            6 => [true, false, true, true, true, true, true],
            7 => [true, true, true, false, false, false, false],
            8 => [true, true, true, true, true, true, true],
            9 => [true, true, true, true, false, true, true],
            _ => [false; 7],
        }
    }

    fn draw_number(
        &self,
        vertices: &mut Vec<Vertex>,
        value: f32,
        start_x: f32,
        start_y: f32,
    ) {
        let text = format!("{value:.2}");
        let mut x = start_x;

        for character in text.chars() {
            if character == '.' {
                vertices.extend_from_slice(&self.make_rect(
                    x,
                    start_y + 72.0,
                    10.0,
                    10.0,
                    [0.2, 0.9, 0.45, 1.0],
                ));

                x += 25.0;
                continue;
            }

            if character == '-' {
                vertices.extend_from_slice(&self.make_rect(
                    x,
                    start_y + 35.0,
                    25.0,
                    8.0,
                    [0.2, 0.9, 0.45, 1.0],
                ));

                x += 40.0;
                continue;
            }

            let Some(digit) = character.to_digit(10) else {
                continue;
            };

            let segments = Self::digit_segments(digit as u8);
            let color = [0.2, 0.9, 0.45, 1.0];

            if segments[0] {
                vertices.extend_from_slice(&self.make_rect(
                    x + 5.0,
                    start_y,
                    30.0,
                    8.0,
                    color,
                ));
            }

            if segments[1] {
                vertices.extend_from_slice(&self.make_rect(
                    x + 32.0,
                    start_y + 5.0,
                    8.0,
                    30.0,
                    color,
                ));
            }

            if segments[2] {
                vertices.extend_from_slice(&self.make_rect(
                    x + 32.0,
                    start_y + 43.0,
                    8.0,
                    30.0,
                    color,
                ));
            }

            if segments[3] {
                vertices.extend_from_slice(&self.make_rect(
                    x + 5.0,
                    start_y + 70.0,
                    30.0,
                    8.0,
                    color,
                ));
            }

            if segments[4] {
                vertices.extend_from_slice(&self.make_rect(
                    x,
                    start_y + 43.0,
                    8.0,
                    30.0,
                    color,
                ));
            }

            if segments[5] {
                vertices.extend_from_slice(&self.make_rect(
                    x,
                    start_y + 5.0,
                    8.0,
                    30.0,
                    color,
                ));
            }

            if segments[6] {
                vertices.extend_from_slice(&self.make_rect(
                    x + 5.0,
                    start_y + 35.0,
                    30.0,
                    8.0,
                    color,
                ));
            }

            x += 50.0;
        }
    }

    fn render(&mut self) {
        if self.size.width == 0 || self.size.height == 0 {
            return;
        }

        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,

            wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Timeout => {
                return;
            }

            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                drop(texture);
                self.configure_surface();
                return;
            }

            wgpu::CurrentSurfaceTexture::Outdated => {
                self.configure_surface();
                return;
            }

            wgpu::CurrentSurfaceTexture::Validation => {
                unreachable!(
                    "No error scope registered, so validation errors will panic"
                );
            }

            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self
                    .instance
                    .create_surface(self.window.clone())
                    .expect("Failed to recreate surface");

                self.configure_surface();
                return;
            }
        };

        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        let mut vertices = Vec::new();

        let (button_x, button_y, button_width, button_height) =
            self.button_rect();

        let button_color = if self.button_pressed {
            [0.15, 0.45, 0.9, 1.0]
        } else if self.cursor_in_button() {
            [0.2, 0.55, 1.0, 1.0]
        } else {
            [0.12, 0.35, 0.75, 1.0]
        };

        vertices.extend_from_slice(&self.make_rect(
            button_x as f32,
            button_y as f32,
            button_width as f32,
            button_height as f32,
            button_color,
        ));

        if let Some(result) = &self.result {
            if let Some(&value) = result.first() {
                self.draw_number(
                    &mut vertices,
                    value,
                    (self.size.width as f32 - 250.0) / 2.0,
                    100.0,
                );
            }
        }

        let vertex_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Canvas Vertex Buffer"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

        let mut encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        {
            let mut render_pass =
                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Canvas Render Pass"),
                    color_attachments: &[Some(
                        wgpu::RenderPassColorAttachment {
                            view: &texture_view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(
                                    wgpu::Color {
                                        r: 0.025,
                                        g: 0.035,
                                        b: 0.055,
                                        a: 1.0,
                                    },
                                ),
                                store: wgpu::StoreOp::Store,
                            },
                        },
                    )],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.draw(0..vertices.len() as u32, 0..1);
        }

        self.queue.submit([encoder.finish()]);

        self.window.pre_present_notify();
        self.queue.present(surface_texture);

        self.button_pressed = false;
    }
}

#[derive(Default)]
struct App {
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("wgpu Compute Demo")
                        .with_inner_size(
                            winit::dpi::PhysicalSize::new(800, 600),
                        ),
                )
                .unwrap(),
        );

        let mut arguments: Vec<f32> = std::env::args()
            .skip(1)
            .map(|s| {
                f32::from_str(&s).unwrap_or_else(|_| {
                    panic!("Cannot parse argument {s:?} as a float.")
                })
            })
            .collect();

        if arguments.is_empty() {
            arguments.push(1.0);
        }

        arguments.push(1.0);

        println!("Parsed {} arguments", arguments.len());

        let state = pollster::block_on(State::new(
            event_loop.owned_display_handle(),
            window.clone(),
            arguments,
        ));

        self.state = Some(state);

        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::CursorMoved { position, .. } => {
                state.cursor_position = (position.x, position.y);
                state.window.request_redraw();
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if state.cursor_in_button() {
                    state.button_pressed = true;
                    state.run_compute();
                    state.window.request_redraw();
                }
            }

            WindowEvent::Resized(size) => {
                state.resize(size);
                state.window.request_redraw();
            }

            WindowEvent::RedrawRequested => {
                state.render();
            }

            _ => {}
        }
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::default();

    event_loop.run_app(&mut app).unwrap();
}
