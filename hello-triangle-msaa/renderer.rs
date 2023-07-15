use wgpu::{InstanceDescriptor, Instance, include_wgsl, TextureDescriptor, TextureFormat, TextureDimension, TextureUsages, Texture, TextureViewDescriptor, TextureView};
use winit::window::Window;

const SAMPLE_COUNT: u32 = 4;
pub struct Renderer {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: wgpu::RenderPipeline,
    texture_view_for_multisampling: TextureView,
}

impl Renderer {
    pub async fn new(window: &Window) -> Self {
        let instance = Instance::new(InstanceDescriptor::default());
        
        let surface = unsafe { instance.create_surface(window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference:
                    wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter.request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                },
                None, // Trace path
            ).await.unwrap();

        let size =  window.inner_size();

        let surface_config = surface.get_default_config(&adapter, size.width, size.height).unwrap();

        let texture_view_for_multisampling = device.create_texture(&TextureDescriptor { 
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: SAMPLE_COUNT,
            dimension: TextureDimension::D2,
            format: surface_config.format,
            usage: TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &surface_config.view_formats,
        }).create_view(&TextureViewDescriptor::default()); // That's not how to API, my fellow nerds 0_0

        let vertex_shader = device.create_shader_module(
            include_wgsl!("shaders/triangle.vert.wgsl"),
        );

        let fragment_shader = device.create_shader_module(
            include_wgsl!("shaders/red.frag.wgsl"),
        );

        let render_pipeline_layout = device
            .create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[],
                    push_constant_ranges: &[],
                },
            );

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_shader,
                entry_point: "main", // 1.
                buffers: &[], // 2.
            },
            fragment: Some(wgpu::FragmentState { // 3.
                module: &fragment_shader,
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState { // 4.
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // 2.
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },    
            depth_stencil: None, // 1.
            multisample: wgpu::MultisampleState {
                count: SAMPLE_COUNT, // 2.
                mask: !0, // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            multiview: None, // 5.
        });

        surface.configure(&device, &surface_config);

        Self {
            surface,
            device,
            queue,
            render_pipeline,
            texture_view_for_multisampling
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(
            &wgpu::TextureViewDescriptor::default(),
        );
        let mut encoder =
            self.device.create_command_encoder(
                &wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                },
            );
        {
            let mut render_pass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(
                        wgpu::RenderPassColorAttachment {
                            view: &self.texture_view_for_multisampling,
                            resolve_target: Some(&view),
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(
                                    wgpu::Color::BLACK,
                                ),
                                store: true,
                            },
                        },
                    )],
                    depth_stencil_attachment: None,
                },
            );

            render_pass.set_pipeline(&self.render_pipeline); // 2.
            render_pass.draw(0..3, 0..1); // 3.
        }
    
        // submit will accept anything that implements IntoIter
        self.queue
            .submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}