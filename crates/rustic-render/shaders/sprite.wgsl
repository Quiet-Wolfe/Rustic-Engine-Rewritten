// Instanced sprite shader. See PLAN.md Section 7.
//
// Vertex inputs:
//   slot 0: a_quad_uv      vec2<f32>     unit-quad corner (0..1)
// Instance inputs (slot 1):
//   a_world_pos    vec2<f32>
//   a_size         vec2<f32>
//   a_pivot        vec2<f32>     // pivot in unit-quad space
//   a_scale        vec2<f32>
//   a_rotation     f32           // radians
//   a_affine_x     vec2<f32>     // [a, b]
//   a_affine_y     vec2<f32>     // [c, d]
//   a_affine_t     vec2<f32>     // [tx, ty]
//   a_uv_min       vec2<f32>
//   a_uv_max       vec2<f32>
//   a_uv_rotated   f32           // Sparrow rotated="true"
//   a_color        vec4<f32>
//
// Camera uniform binds the view-projection matrix (group=0 binding=0).
// Atlas texture + sampler bind at group=1.

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> u_camera: CameraUniform;

@group(1) @binding(0) var t_atlas: texture_2d<f32>;
@group(1) @binding(1) var s_atlas: sampler;

struct VsIn {
    @location(0) quad_uv: vec2<f32>,
    @location(1) world_pos: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) pivot: vec2<f32>,
    @location(4) scale: vec2<f32>,
    @location(5) rotation: f32,
    @location(6) affine_x: vec2<f32>,
    @location(7) affine_y: vec2<f32>,
    @location(8) affine_t: vec2<f32>,
    @location(9) uv_min: vec2<f32>,
    @location(10) uv_max: vec2<f32>,
    @location(11) uv_rotated: f32,
    @location(12) color: vec4<f32>,
};

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(input: VsIn) -> VsOut {
    // Local-space corner in pixels relative to the sprite's pivot.
    let local_px = (input.quad_uv - input.pivot) * input.size * input.scale;
    let c = cos(input.rotation);
    let s = sin(input.rotation);
    let rotated = vec2<f32>(
        local_px.x * c - local_px.y * s,
        local_px.x * s + local_px.y * c
    );
    let affine = vec2<f32>(
        rotated.x * input.affine_x.x + rotated.y * input.affine_y.x + input.affine_t.x,
        rotated.x * input.affine_x.y + rotated.y * input.affine_y.y + input.affine_t.y
    );
    let world = input.world_pos + affine;

    var atlas_uv = input.quad_uv;
    if (input.uv_rotated > 0.5) {
        // Funkin's Sparrow exporter marks these as angle -90.
        atlas_uv = vec2<f32>(input.quad_uv.y, 1.0 - input.quad_uv.x);
    }
    let uv = mix(input.uv_min, input.uv_max, atlas_uv);

    var out: VsOut;
    out.position = u_camera.view_proj * vec4<f32>(world, 0.0, 1.0);
    out.uv = uv;
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let tex = textureSample(t_atlas, s_atlas, in.uv);
    return tex * in.color;
}
