
#import "shaders/custom_material_import.wgsl"::COLOR_MULTIPLIER

const TAU: f32 = 6.28318530717958647692528676655900577;
const PI: f32 = 3.14159265358979323846264338327950288;

fn uhash(a: u32, b: u32) -> u32 { 
    var x = ((a * 1597334673u) ^ (b * 3812015801u));
    // from https://nullprogram.com/blog/2018/07/31/
    x = x ^ (x >> 16u);
    x = x * 0x7feb352du;
    x = x ^ (x >> 15u);
    x = x * 0x846ca68bu;
    x = x ^ (x >> 16u);
    return x;
}

fn unormf(n: u32) -> f32 { 
    return f32(n) * (1.0 / f32(0xffffffffu)); 
}

fn hash_noise(x: u32, y: u32, z: u32) -> f32 {
    let urnd = uhash(x, (y << 11) + z);
    return unormf(urnd);
}

fn get_arc_size(ring: u32, level: u32, seed: u32) -> f32 {
    return (select(0.9,0.3,ring % 2==0)) / (f32(ring + 1));
}

fn get_ring_speed(ring: u32, level: u32, seed: u32) -> f32 {
    return ((hash_noise(ring, level, seed) * 0.2 + 0.1) / (f32(ring + 1)))
        * (1.0 + f32(ring) * 0.4);
}

fn get_ring_color(ring: u32, level: u32, seed: u32) -> u32 {
    return u32(floor(hash_noise(ring + 2048, level, seed) * 1.0 - 0.0000001));
}

struct State {
    position: vec4<f32>,
    resolution: vec4<f32>,
    scale_factor: f32,
    ring_thick: f32,
    frame: f32,
    time: f32,
    local_player_pos: u32,
    player_offset: f32,
    t: f32,
    spare3: u32,
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

fn render(coord: vec2<f32>) -> vec3<f32> {
    var color = vec3(0.0);
    let pos = state.position.xy;
    let uv = coord / state.resolution.xy;
    let screen_mid = state.resolution.xy / 2.0;
    let p = coord - screen_mid - pos * state.scale_factor;
    let fring = length(p) / (state.ring_thick * state.scale_factor) + 1.0;
    let ring = u32(floor(fring));
    let theta = (atan2(p.y, p.x) + PI * 0.5) / TAU;

    {
        // Draw rings
        let m = ring % 2;
        if m == 0 {
            color = vec3(0.2,0.6,0.2);
        }
        if m == 1 {
            color = vec3(0.1,0.1,0.5);
        }
    }


    {
        // Draw arcs
        let ring_speed = get_ring_speed(ring, 0u, 0u);
        var arc_size: f32;
        if ring == state.local_player_pos {
            arc_size = 0.13 / (1.0 + f32(ring) * 0.03);
        } else {
            arc_size = get_arc_size(ring, 0u, 0u);
        }

        var ring_start = state.player_offset;
        if ring != state.local_player_pos {
            ring_start = fract(state.t * (ring_speed * f32(ring + 1)));
        }

        let start = fract(theta - ring_start);
        if start < arc_size {
            color = vec3(0.1, 0.02, 0.2);
        }
    }

    return color;
}

@fragment
fn fragment(input: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let coord = input.position.xy;
    var color = vec3(0.0);
    color += render(coord);
    // MSAA RGSS
    // color += render(coord + vec2( 0.125,  0.375));
    // color += render(coord + vec2(-0.375,  0.125));
    // color += render(coord + vec2( 0.375, -0.125));
    // color += render(coord + vec2(-0.125, -0.375));
    // color *= 0.25;
    return vec4(color, 1.0);
}