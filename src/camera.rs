use wgpu::util::DeviceExt;

#[derive(Debug)]
pub struct Camera {
    size: winit::dpi::PhysicalSize<u32>,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
    view_proj: [[f32; 4]; 4],
}

impl Camera {
    pub fn new_from_size(device: &wgpu::Device, size: winit::dpi::PhysicalSize<u32>) -> Self {
        let proj = Self::build_proj(&size);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[proj]),
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
        Self {
            size,
            uniform_buffer: camera_buffer,
            bind_group: camera_bind_group,
            bind_group_layout: camera_bind_group_layout,
            view_proj: proj,
        }
    }
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>, queue: &wgpu::Queue) {
        self.size = new_size;
        self.view_proj = Self::build_proj(&new_size);
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.view_proj]),
        );
    }

    pub fn get_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn get_bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    fn build_proj(size: &winit::dpi::PhysicalSize<u32>) -> [[f32; 4]; 4] {
        let m = OPENGL_TO_WGPU_MATRIX
            * cgmath::ortho(0.0, size.width as f32, size.height as f32, 0.0, 0.0, 2.0);
        m.into()
    }
}

#[rustfmt::skip]
const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::from_cols(
    cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 1.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 1.0),
);
