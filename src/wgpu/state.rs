use std::sync::Arc;
use winit::window::Window;

pub struct State {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub window: Arc<Window>,
}

impl State {
    pub(crate) async fn new(window: Arc<Window>) -> State {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });

        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            color_space: Default::default(),
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);
        State { surface, device, queue, config, size, window }
    }

    pub(crate) fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        // Guard against a zero-sized surface (e.g. when minimised), which is
        // invalid and would make the GPU error.
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config); // re-apply at the new size
        }
    }

    // Draws one frame. Returns nothing — acquiring the surface texture gives
    // back an enum we match on, handling each outcome inline.
    pub(crate) fn render(&mut self) {
        // get_current_texture() returns a CurrentSurfaceTexture enum. We pull
        // the texture out of the usable variants and bail early on the rest.
        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            // Transient conditions (minimised, timed out) — skip this frame.
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded => return,
            // Surface needs reconfiguring — re-apply the current config, then skip.
            wgpu::CurrentSurfaceTexture::Outdated
            | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => return,
        };
        // A view is the handle render passes use to access the texture's memory.
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // The encoder records GPU commands on the CPU side before they're
        // submitted. `mut` because recording into it mutates it.
        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") }
        );

        // Inner scope: the render pass borrows `encoder`. Dropping it at the
        // end of this block releases that borrow so we can call
        // encoder.finish() afterwards.
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,          // write the results into our surface texture
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Clear the whole texture to this colour at the start
                        // of the pass (RGBA on a 0–1 scale)…
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0 }),
                        store: wgpu::StoreOp::Store, // …and keep the result in the texture
                    },
                    depth_slice: None, // 2D target, so no depth slice
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
        } // _render_pass dropped here, releasing its borrow of `encoder`

        // Finish recording and submit the command list to the GPU queue.
        // `std::iter::once` wraps our single command buffer in an iterator.
        self.queue.submit(std::iter::once(encoder.finish()));

        self.window.pre_present_notify();
        self.queue.present(output);
    }
}