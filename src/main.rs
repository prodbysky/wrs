use std::sync::Arc;

use cgmath::SquareMatrix;
use image::EncodableLayout;
use wgpu::util::DeviceExt;

fn main() {
    env_logger::init();

    let event_loop = winit::event_loop::EventLoop::new().unwrap();

    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let mut app = App::default();

    event_loop.run_app(&mut app).unwrap();
}

#[derive(Default)]
struct App {
    renderer: Option<Renderer>,
}

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(winit::window::Window::default_attributes())
                .unwrap(),
        );

        let state = pollster::block_on(Renderer::new(window.clone()));
        self.renderer = Some(state);
        window.request_redraw();
    }
    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let renderer = self.renderer.as_mut().unwrap();
        renderer.begin_frame();
        renderer.draw_quad(0.0, 0.0, 100.0, 100.0, [1.0, 1.0, 1.0]);
        renderer.draw_quad(100.0, 100.0, 100.0, 100.0, [1.0, 1.0, 1.0]);
        renderer.draw_quad(200.0, 200.0, 100.0, 100.0, [1.0, 1.0, 1.0]);
        renderer.draw_quad(300.0, 300.0, 100.0, 100.0, [1.0, 1.0, 1.0]);
        renderer.end_frame();

        match event {
            winit::event::WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            winit::event::WindowEvent::RedrawRequested => {
                renderer.render();
                renderer.get_window().request_redraw();
            }
            winit::event::WindowEvent::Resized(size) => {
                renderer.resize(size);
            }
            _ => {
                // dbg!(e);
            }
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pos: [f32; 3],
    color: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FontVertex {
    pos: [f32; 3],
    color: [f32; 3],
    texture_coords: [f32; 2],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

impl FontVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<FontVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

struct Camera {
    w: f32,
    h: f32,
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::from_cols(
    cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 1.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 1.0),
);

impl Camera {
    pub fn build(&self) -> cgmath::Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * cgmath::ortho(0.0, self.w, self.h, 0.0, 0.0, 2.0)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }
    pub fn update(&mut self, cam: &Camera) {
        self.view_proj = cam.build().into();
    }
}

struct Renderer {
    window: Arc<winit::window::Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_fmt: wgpu::TextureFormat,
    quad_render_pipeline: wgpu::RenderPipeline,

    vbo: wgpu::Buffer,
    ibo: wgpu::Buffer,

    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    draw_vertices: Vec<Vertex>,
    draw_indices: Vec<u16>,

    font_atlas: MonoGlyphAtlas,
    font_bind_group: wgpu::BindGroup,

    font_draw_vertices: Vec<FontVertex>,
    font_draw_indices: Vec<u16>,
}

pub struct MonoGlyphAtlas {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub glyph_map: std::collections::HashMap<char, (f32, f32, f32, f32)>,
    pub cell_size: (u32, u32),
}

pub fn create_monospace_atlas(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    font_data: &[u8],
    scale: f32,
) -> MonoGlyphAtlas {
    use ab_glyph::Font;
    let font = ab_glyph::FontRef::try_from_slice(font_data).unwrap();
    let scale = ab_glyph::PxScale::from(scale);

    let chars: Vec<char> = (0x20u8..0x7Fu8).map(|c| c as char).collect();

    let test_glyph = font
        .outline_glyph(font.glyph_id('M').with_scale(scale))
        .unwrap();
    let bb = test_glyph.px_bounds();
    let cell_w = bb.width().ceil() as u32;
    let cell_h = bb.height().ceil() as u32;

    let cols = 16;
    let rows = ((chars.len() as f32) / cols as f32).ceil() as u32;
    let atlas_width = cols * cell_w;
    let atlas_height = rows * cell_h;

    let mut atlas = image::RgbaImage::new(atlas_width, atlas_height);
    let mut glyph_map = std::collections::HashMap::new();

    for (i, &ch) in chars.iter().enumerate() {
        let glyph = font.glyph_id(ch).with_scale(scale);
        if let Some(og) = font.outline_glyph(glyph) {
            let mut img = image::RgbaImage::new(cell_w, cell_h);
            let glyph_bb = og.px_bounds();

            let x_off = ((cell_w as f32 - glyph_bb.width()) / 2.0).floor() as i32;
            let y_off = ((cell_h as f32 - glyph_bb.height()) / 2.0).floor() as i32;

            og.draw(|x, y, v| {
                let px = (x as i32 + x_off).max(0) as u32;
                let py = (y as i32 + y_off).max(0) as u32;
                if px < cell_w && py < cell_h {
                    img.put_pixel(px, py, image::Rgba([255, 255, 255, (v * 255.0) as u8]));
                }
            });

            let x = (i as u32 % cols) * cell_w;
            let y = (i as u32 / cols) * cell_h;

            image::imageops::overlay(&mut atlas, &img, x.into(), y.into());

            let u0 = x as f32 / atlas_width as f32;
            let v0 = y as f32 / atlas_height as f32;
            let u1 = (x + cell_w) as f32 / atlas_width as f32;
            let v1 = (y + cell_h) as f32 / atlas_height as f32;
            glyph_map.insert(ch, (u0, v0, u1, v1));
        }
    }

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Monospace Glyph Atlas"),
        size: wgpu::Extent3d {
            width: atlas_width,
            height: atlas_height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        atlas.as_bytes(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * atlas_width),
            rows_per_image: Some(atlas_height),
        },
        wgpu::Extent3d {
            width: atlas_width,
            height: atlas_height,
            depth_or_array_layers: 1,
        },
    );

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("Glyph Sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    MonoGlyphAtlas {
        texture,
        view,
        sampler,
        glyph_map,
        cell_size: (cell_w, cell_h),
    }
}

impl Renderer {
    pub async fn new(window: Arc<winit::window::Window>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();

        let size = window.inner_size();

        let surface = instance.create_surface(window.clone()).unwrap();

        let capabilities = surface.get_capabilities(&adapter);

        let surface_fmt = capabilities.formats[0];

        // camera setup begin
        let camera = Camera {
            w: size.width as f32,
            h: size.height as f32,
        };

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update(&camera);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // this setups that we can use the orthographic projection in the vertex shader
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: None,
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: None,
        });
        // camera setup end

        // quad renderer pipeline
        let quad_shader = device.create_shader_module(wgpu::include_wgsl!("quad_shader.wgsl"));
        let quad_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let quad_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&quad_render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &quad_shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &quad_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_fmt,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview: None,
            cache: None,
        });

        // font setup
        let font = include_bytes!("iosevka-regular.ttf");
        let atlas = create_monospace_atlas(&device, &queue, font, 32.0);

        let font_shader = device.create_shader_module(wgpu::include_wgsl!("font_shader.wgsl"));
        let font_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let font_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &font_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(
                        camera_buffer.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&atlas.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&atlas.sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        let font_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&font_bind_group_layout],
                push_constant_ranges: &[],
            });

        let font_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&font_render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &font_shader,
                entry_point: Some("vs_main"),
                buffers: &[FontVertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &font_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_fmt,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview: None,
            cache: None,
        });

        let renderer = Self {
            window,
            vbo: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: &[],
                usage: wgpu::BufferUsages::VERTEX,
            }),
            ibo: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: &[],
                usage: wgpu::BufferUsages::INDEX,
            }),
            device,
            queue,
            size,
            surface,
            surface_fmt,

            quad_render_pipeline,

            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,

            draw_indices: vec![],
            draw_vertices: vec![],

            font_atlas: atlas,
            font_bind_group,
            font_draw_indices: vec![],
            font_draw_vertices: vec![],
        };

        renderer.configure_surface();

        renderer
    }

    pub fn begin_frame(&mut self) {
        self.draw_vertices.clear();
        self.draw_indices.clear();
    }

    pub fn draw_quad(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 3]) {
        let start = self.draw_vertices.len() as u16;

        self.draw_vertices.extend_from_slice(&[
            Vertex {
                pos: [x, y, 0.0],
                color,
            },
            Vertex {
                pos: [x + w, y, 0.0],
                color,
            },
            Vertex {
                pos: [x + w, y + h, 0.0],
                color,
            },
            Vertex {
                pos: [x, y + h, 0.0],
                color,
            },
        ]);

        self.draw_indices.extend_from_slice(&[
            start,
            start + 1,
            start + 2,
            start,
            start + 2,
            start + 3,
        ]);
    }

    pub fn end_frame(&mut self) {
        if self.draw_vertices.is_empty() {
            return;
        }

        if (self.vbo.size() as usize) < self.draw_vertices.len() * std::mem::size_of::<Vertex>() {
            self.vbo.destroy();
            let vbo = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&self.draw_vertices),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });
            self.vbo = vbo;
        } else {
            self.queue
                .write_buffer(&self.vbo, 0, bytemuck::cast_slice(&self.draw_vertices));
        }

        if (self.ibo.size() as usize) < self.draw_indices.len() * std::mem::size_of::<u16>() {
            self.ibo.destroy();
            let ibo = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&self.draw_indices),
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                });
            self.ibo = ibo;
        } else {
            self.queue
                .write_buffer(&self.ibo, 0, bytemuck::cast_slice(&self.draw_indices));
        }
    }

    pub fn render(&mut self) {
        let surface_texture = self.surface.get_current_texture().unwrap();
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.surface_fmt.add_srgb_suffix()),
                ..Default::default()
            });

        let mut encoder = self.device.create_command_encoder(&Default::default());

        let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        renderpass.set_pipeline(&self.quad_render_pipeline);
        renderpass.set_bind_group(0, &self.camera_bind_group, &[]);
        renderpass.set_vertex_buffer(0, self.vbo.slice(..));
        renderpass.set_index_buffer(self.ibo.slice(..), wgpu::IndexFormat::Uint16);
        renderpass.draw_indexed(0..self.draw_indices.len() as u32, 0, 0..1);

        drop(renderpass);

        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.camera.w = new_size.width as f32;
        self.camera.h = new_size.height as f32;
        self.camera_uniform.update(&self.camera);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
        self.configure_surface();
    }

    pub fn get_window(&self) -> &winit::window::Window {
        &self.window
    }

    fn configure_surface(&self) {
        let surface_cfg = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_fmt,
            view_formats: vec![self.surface_fmt.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::Immediate,
        };
        self.surface.configure(&self.device, &surface_cfg);
    }
}
