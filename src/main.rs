mod camera;
mod quad;
mod font;
use camera::Camera;
use std::sync::Arc;

use image::EncodableLayout;

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
        renderer
            .quad_renderer
            .push(0.0, 0.0, 100.0, 100.0, [0.0, 1.0, 0.0]);
        // renderer.draw_quad(100.0, 100.0, 100.0, 100.0, [1.0, 1.0, 1.0]);
        // renderer.draw_quad(200.0, 200.0, 100.0, 100.0, [1.0, 1.0, 1.0]);
        // renderer.draw_quad(300.0, 300.0, 100.0, 100.0, [1.0, 1.0, 1.0]);
        renderer.font_renderer.push(50.0, 50.0, [1.0, 1.0, 1.0], 'A', &renderer.font_atlas);
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

struct Renderer {
    window: Arc<winit::window::Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_fmt: wgpu::TextureFormat,

    camera: Camera,

    quad_renderer: quad::QuadRenderer,

    font_atlas: MonoGlyphAtlas,
    font_renderer: font::FontRenderer
}


pub struct MonoGlyphAtlas {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
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
        label: None,
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
    let bind_group_layout =
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
        label: None,
    });

    MonoGlyphAtlas {
        texture,
        view,
        sampler,
        glyph_map,
        cell_size: (cell_w, cell_h),
        bind_group,
        bind_group_layout
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

        let cam = Camera::new_from_size(&device, size);

        // font setup
        let font = include_bytes!("iosevka-regular.ttf");
        let atlas = create_monospace_atlas(&device, &queue, font, 128.0);

        let renderer = Self {
            window,
            quad_renderer: quad::QuadRenderer::new(&device, &cam, surface_fmt),
            font_renderer: font::FontRenderer::new(&device, &cam, &atlas, surface_fmt),
            device,
            queue,
            size,
            surface,
            surface_fmt,
            camera: cam,
            font_atlas: atlas,

        };

        renderer.configure_surface();

        renderer
    }

    pub fn begin_frame(&mut self) {
        self.quad_renderer.clear();
        self.font_renderer.clear();
    }

    pub fn end_frame(&mut self) {
        if self.quad_renderer.empty() || self.font_renderer.empty() {
            return;
        }

        self.quad_renderer.upload_data(&self.device, &self.queue);
        self.font_renderer.upload_data(&self.device, &self.queue);
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
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        self.quad_renderer
            .flush(&mut renderpass, &self.device, &self.queue, &self.camera);

        self.font_renderer
            .flush(&mut renderpass, &self.device, &self.queue, &self.camera, &self.font_atlas);

        drop(renderpass);

        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.camera.resize(new_size, &self.queue);
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
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_cfg);
    }
}
