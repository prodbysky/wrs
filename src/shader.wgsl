struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

struct VertexIn {
    @location(0) pos: vec3<f32>,
    @location(1) color: vec3<f32>,
}

@vertex
fn vs_main(
    model: VertexIn
) -> VertexOut {
    var out: VertexOut;
    out.color = model.color;
    out.clip_position = vec4<f32>(model.pos, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
