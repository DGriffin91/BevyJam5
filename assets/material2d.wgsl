
#import "shaders/custom_material_import.wgsl"::COLOR_MULTIPLIER

const TAU: f32 = 6.28318530717958647692528676655900577;
const PI: f32 = 3.14159265358979323846264338327950288;

struct State {
    position: vec4<f32>,
    resolution: vec4<f32>,
    scale_factor: f32,
    ring_thick: f32,
    frame: f32,
    time: f32,
}

@group(2) @binding(0) var<uniform> state: State;

struct FullscreenVertexOutput {
    @builtin(position)
    position: vec4<f32>,
    @location(0)
    uv: vec2<f32>,
};

@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32) -> FullscreenVertexOutput {
    let uv = vec2<f32>(f32(vertex_index >> 1u), f32(vertex_index & 1u)) * 2.0;
    let clip_position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    return FullscreenVertexOutput(clip_position, uv);
}

@fragment
fn fragment(input: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let pos = state.position.xy;
    let uv = input.position.xy / state.resolution.xy;
    let screen_mid = state.resolution.xy / 2.0;
    let p = input.position.xy - screen_mid - pos * state.scale_factor;
    let ring = length(p) / (state.ring_thick * state.scale_factor);
    let theta = (atan2(p.y, p.x) + PI) / TAU;

    let m = ring % 2.0;
    var color = vec3(0.0);
    if floor(m) == 0.0 {
        color = vec3(0.2,0.6,0.2);
    }
    if floor(m) == 1.0 {
        color = vec3(0.1,0.1,0.5);
    }


    //color = vec3(theta);
    let r = fract(theta + state.time * 0.1);
    if r > 0.25 && r < 0.5 && floor(ring) == 5.0 {
        color = vec3(0.5, 0.1, 0.05);
    }

    return vec4(color, 1.0);
}