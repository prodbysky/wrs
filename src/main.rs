use std::sync::Arc;

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
    renderer: Option<Renderer>
}

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Arc::new(event_loop.create_window(winit::window::Window::default_attributes()).unwrap());

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
                }
            ] 
        }
    }
}



const VERTICES: &[Vertex] = &[
    Vertex { pos: [-0.25, -0.25, 0.0], color: [1.0, 0.0, 0.0] }, // bottom left
    Vertex { pos: [0.25, 0.25, 0.0], color: [1.0, 0.0, 0.0] },   // top right
    Vertex { pos: [-0.25, 0.25, 0.0], color: [1.0, 0.0, 0.0] },  // top left
    Vertex { pos: [0.25, -0.25, 0.0], color: [1.0, 0.0, 0.0] },  // bottom right
    Vertex { pos: [0.25, 0.25, 0.0], color: [1.0, 0.0, 0.0] },  // top right
    Vertex { pos: [-0.25, -0.25, 0.0], color: [1.0, 0.0, 0.0] },  // bottom left
];

struct Renderer {
    window: Arc<winit::window::Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_fmt: wgpu::TextureFormat,
    render_pipeline: wgpu::RenderPipeline,
    vbo: wgpu::Buffer
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

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let vbo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor { 
            label: None, 
            layout: Some(&render_pipeline_layout), 
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default()
            }, 
            primitive: wgpu::PrimitiveState { 
                topology: wgpu::PrimitiveTopology::TriangleList, 
                strip_index_format: None, 
                front_face: wgpu::FrontFace::Ccw, 
                cull_mode: Some(wgpu::Face::Back), 
                polygon_mode: wgpu::PolygonMode::Fill, 
                unclipped_depth: false, 
                conservative: false 
            }, 
            depth_stencil: None, 
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false
            }, 
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(
                    wgpu::ColorTargetState {
                        format: surface_fmt,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL
                    }
                )],
                compilation_options: wgpu::PipelineCompilationOptions::default()
            }), 
            multiview: None, 
            cache: None 
        });

        let renderer = Self {
            window,
            device,
            queue,
            size,
            surface,
            surface_fmt,
            render_pipeline,
            vbo
        };

        renderer.configure_surface();

        renderer
    }

    pub fn render(&mut self) {
        let surface_texture = self.surface.get_current_texture().unwrap();
        let texture_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.surface_fmt.add_srgb_suffix()),
            ..Default::default()
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());

        let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[
                Some(
                    wgpu::RenderPassColorAttachment {
                        view: &texture_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 5 as f64 / 255.0,
                            g: 5 as f64 / 255.0,
                            b: 5 as f64 / 255.0,
                            a: 1.0,
                        }), store: wgpu::StoreOp::Store },
                    }
                )
            ],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        renderpass.set_pipeline(&self.render_pipeline);
        renderpass.set_vertex_buffer(0, self.vbo.slice(..));
        renderpass.draw(0..6, 0..1);

        drop(renderpass);

        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
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
            present_mode: wgpu::PresentMode::AutoVsync
        };
        self.surface.configure(&self.device, &surface_cfg);
    }



}
