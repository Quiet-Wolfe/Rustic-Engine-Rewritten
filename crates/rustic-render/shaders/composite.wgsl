// Reference -> native composite: full-screen triangle that samples the
// 1280x720 reference texture and presents it to the swapchain. See
// PLAN.md Section 7 (reference output mode).

@group(0) @binding(0) var t_ref: texture_2d<f32>;
@group(0) @binding(1) var s_ref: sampler;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VsOut {
    // Full-screen triangle covering the whole NDC quad.
    var p = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 3.0,  1.0),
    );
    var u = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 2.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(2.0, 0.0),
    );
    var out: VsOut;
    out.position = vec4<f32>(p[idx], 0.0, 1.0);
    out.uv = u[idx];
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(t_ref, s_ref, in.uv);
}
