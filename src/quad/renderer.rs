use crate::camera::Camera;
use wgpu::util::DeviceExt;

impl QuadRenderer {
    pub fn new(device: &wgpu::Device, cam: &Camera, surface_fmt: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("quad_shader.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[cam.get_bind_group_layout()],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
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
                module: &shader,
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
        Self {
            render_pipeline: pipeline,
            vertices: vec![],
            indices: vec![],
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
            has_data: false,
        }
    }
    pub fn push(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 3]) {
        self.has_data = true;
        let start = self.vertices.len() as u16;

        self.vertices.extend_from_slice(&[
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

        self.indices
            .extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
    }
    pub fn flush(
        &mut self,
        render_pass: &mut wgpu::RenderPass,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cam: &Camera,
    ) {
        if self.has_data {
            self.upload_data(device, queue);
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, cam.get_bind_group(), &[]);
            render_pass.set_vertex_buffer(0, self.vbo.slice(..));
            render_pass.set_index_buffer(self.ibo.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
        }
    }

    pub fn clear(&mut self) {
        self.indices.clear();
        self.vertices.clear();
        self.has_data = false;
    }

    pub fn empty(&self) -> bool {
        self.vertices.is_empty()
    }

    pub fn upload_data(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.vertices.is_empty() {
            return;
        }
        if (self.vbo.size() as usize) < self.vertices.len() * std::mem::size_of::<Vertex>() {
            self.vbo.destroy();
            let vbo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&self.vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
            self.vbo = vbo;
        } else {
            queue.write_buffer(&self.vbo, 0, bytemuck::cast_slice(&self.vertices));
        }

        if (self.ibo.size() as usize) < self.indices.len() * std::mem::size_of::<u16>() {
            self.ibo.destroy();
            let ibo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&self.indices),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });
            self.ibo = ibo;
        } else {
            queue.write_buffer(&self.ibo, 0, bytemuck::cast_slice(&self.indices));
        }
    }
}


pub struct QuadRenderer {
    render_pipeline: wgpu::RenderPipeline,
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
    vbo: wgpu::Buffer,
    ibo: wgpu::Buffer,
    has_data: bool,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
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
                },
            ],
        }
    }
}
