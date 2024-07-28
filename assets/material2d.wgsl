
#import "shaders/custom_material_import.wgsl"::COLOR_MULTIPLIER

const TAU: f32 = 6.28318530717958647692528676655900577;
const PI: f32 = 3.14159265358979323846264338327950288;

fn rem_euclid(a: f32, b: f32) -> f32 {
    let r = a % b;
    return select(r + b, r, r >= 0.0);
}

fn pfract(a: f32) -> f32 {
    return rem_euclid(a, 1.0);
}

// ---------------------------------------
// ---------------------------------------
// ---------------------------------------


fn HUEtoRGB(hue: f32) -> vec3<f32> {
    // Hue [0..1] to RGB [0..1]
    // See http://www.chilliant.com/rgb2hsv.html
    let rgb = abs(hue * 6.0 - vec3<f32>(3.0, 2.0, 4.0)) * vec3<f32>(1.0, -1.0, -1.0) + vec3<f32>(-1.0, 2.0, 2.0);
    return clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn RGBtoHCV(rgb: vec3<f32>) -> vec3<f32> {
    // RGB [0..1] to Hue-Chroma-Value [0..1]
    // Based on work by Sam Hocevar and Emil Persson
    var p: vec4<f32>;
    if rgb.y < rgb.z { 
        p = vec4<f32>(rgb.z, rgb.y, -1.0, 2.0 / 3.0);
    } else { 
        p = vec4<f32>(rgb.y, rgb.z, 0.0, -1.0 / 3.0);
    }
    var q: vec4<f32>;
    if rgb.x < p.x { 
        q = vec4<f32>(p.x, p.y, p.z, rgb.x);
    } else { 
        q = vec4<f32>(rgb.x, p.y, p.z, p.x);
    }
    let c = q.x - min(q.w, q.y);
    let h = abs((q.w - q.y) / (6.0 * c + 0.00001) + q.z); // EPSILON replaced with 0.00001
    return vec3<f32>(h, c, q.x);
}

fn HSVtoRGB(hsv: vec3<f32>) -> vec3<f32> {
    // Hue-Saturation-Value [0..1] to RGB [0..1]
    let rgb = HUEtoRGB(hsv.x);
    return ((rgb - 1.0) * hsv.y + 1.0) * hsv.z;
}

fn HSLtoRGB(hsl: vec3<f32>) -> vec3<f32> {
    // Hue-Saturation-Lightness [0..1] to RGB [0..1]
    let rgb = HUEtoRGB(hsl.x);
    let c = (1.0 - abs(2.0 * hsl.z - 1.0)) * hsl.y;
    return (rgb - 0.5) * c + hsl.z;
}

fn RGBtoHSV(rgb: vec3<f32>) -> vec3<f32> {
    // RGB [0..1] to Hue-Saturation-Value [0..1]
    let hcv = RGBtoHCV(rgb);
    let s = hcv.y / (hcv.z + 0.00001); // EPSILON replaced with 0.00001
    return vec3<f32>(hcv.x, s, hcv.z);
}

fn RGBtoHSL(rgb: vec3<f32>) -> vec3<f32> {
    // RGB [0..1] to Hue-Saturation-Lightness [0..1]
    let hcv = RGBtoHCV(rgb);
    let z = hcv.z - hcv.y * 0.5;
    let s = hcv.y / (1.0 - abs(z * 2.0 - 1.0) + 0.00001); // EPSILON replaced with 0.00001
    return vec3<f32>(hcv.x, s, z);
}

// ---------------------------------------
// ---------------------------------------
// ---------------------------------------

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

// ---------------------------------------
// ---------------------------------------
// ---------------------------------------

fn get_arc_size(ring: u32, level: u32, seed: u32) -> f32 {
    return (hash_noise(ring, level, seed) * 0.2 + 0.2) / ((f32(ring + 1)) * 0.13 + 2.0);
}

fn get_ring_speed(ring: u32, level: u32, seed: u32) -> f32 {
    return 
          ((hash_noise(ring, level, seed) * 1.0 + 0.8) / (f32(ring + 1)))
        * (1.0 + f32(ring) * 0.0)
        * select(1.0,-1.0,ring % 2 == 0);
}

fn get_ring_color(ring: u32, level: u32, seed: u32) -> u32 {
    return u32(floor(hash_noise(ring + 2048, level, seed) * 1.0 - 0.0000001));
}

fn get_max_arcs(ring: u32) -> u32 {
    return clamp(u32(max(i32(ring) - 16, 0)) / 4u, 2u, 6u);
}

struct State {
    position: vec4<f32>,
    resolution: vec4<f32>,

    scale_factor: f32,
    ring_thick: f32,
    frame: f32,
    time: f32,

    t: f32,
    player_ring: u32,
    player_offset: f32,
    player_color_idx: u32,

    step_anim: f32,
    move_cooldown: f32,
    player_sub_ring: u32,
    player_dead: u32,

    player_miss: u32,
    spare0: u32,
    spare1: u32,
    spare2: u32,
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
    let ffring = floor(fring);
    let ring = u32(ffring);

    if fring < state.t * 7.0 {
        return vec3(1.0, 0.0, 0.0);
    }

    let theta = (atan2(p.y, p.x) + PI * 0.5) / TAU;

    {
        // Draw rings
        let m = ring % 2;
        if m == 0 {
            color = mix(vec3(0.03, 0.001, 0.0), vec3(0.01, 0.0, 0.01), sin(state.t * 40.0 + ffring * 0.2));
        }
        if m == 1 {
            //color = mix(vec3(0.02, 0.05, 0.05), vec3(0.0, 0.0, 0.05), cos(state.t * 20.0 + ffring * 0.2));
            let v = sin(state.t * 40.) * 0.5 + 0.5;
            let v2 = cos(state.t * 40.) * 0.5 + 0.5;
            color = vec3(0.05 * v, 0.0, 0.01 * v2);
        }
    }

    
    {
        // Draw arcs
        for (var sub_ring = 0u; sub_ring <  get_max_arcs(ring); sub_ring += 1u) {
            let ring_speed = get_ring_speed(ring, sub_ring, 0u);
            let arc_size = get_arc_size(ring, sub_ring, 0u);
            let ring_start = pfract(state.t * (ring_speed * f32(ring + 1)));
            let start = pfract(theta - ring_start);
            if start < arc_size {
                let v = (1.0 - pow(abs(f32(state.player_ring + 1) - f32(ffring)), 0.2) * 0.6);
                color = vec3(0.4 * v, 0.0, 0.3 * v + 0.03);
                if ring == state.player_ring + 1 {
                    color = vec3(1.0, 0.3, 0.0);
                }
                //else if ring == state.player_ring {
                //    color = vec3(0.1, 0.1, 0.1);
                //}
            }
        }
    }

    color = RGBtoHSV(color);
    color.x = fract(color.x + (max(state.t - 10.0, 0.0) * 0.05));
    color = HSVtoRGB(color);

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