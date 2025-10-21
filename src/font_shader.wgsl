struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) texture_coords: vec2<f32>,
};

struct VertexIn {
    @location(0) pos: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) texture_coords: vec2<f32>,
}

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(
    model: VertexIn
) -> VertexOut {
    var out: VertexOut;
    out.color = model.color;
    out.clip_position = camera.view_proj * vec4<f32>(model.pos, 1.0);
    out.texture_coords = model.texture_coords;
    return out;
}

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let tex = textureSample(t_diffuse, s_diffuse, in.texture_coords);
    if tex.a < 0.001 {
        discard;
    }
    return vec4<f32>(in.color * tex.rgb, tex.a);
}
