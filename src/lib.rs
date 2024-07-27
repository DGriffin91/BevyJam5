#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use std::f32::consts::*;

use bevy::asset::AssetMetaCheck;
use bevy::core_pipeline::fxaa::{Fxaa, Sensitivity};

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::math::*;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use bevy::sprite::{Material2d, Material2dPlugin, MaterialMesh2dBundle};
use bevy::window::PresentMode;
use bevy::winit::{UpdateMode, WinitSettings};
use bevy_vector_shapes::shapes::DiscPainter;
use bevy_vector_shapes::Shape2dPlugin;
use bevy_vector_shapes::{painter::ShapePainter, shapes::Cap};
pub mod palette;
pub mod sampling;

use ridiculous_bevy_hot_reloading::{hot_reloading_macros::make_hot, HotReloadPlugin};
use sampling::hash_noise;

#[no_mangle] // Needed so libloading can find this entry point
fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.05)))
        .insert_resource(WinitSettings {
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
        })
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    // Wasm builds will check for meta files (that don't exist) if this isn't set.
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        present_mode: PresentMode::Immediate,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins((
            Material2dPlugin::<DataMaterial>::default(),
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin,
            Shape2dPlugin::default(),
            HotReloadPlugin {
                auto_watch: true,
                bevy_dylib: true,
                ..default()
            },
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, draw)
        .run();
}

fn setup(
    mut commands: Commands,
    _asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<DataMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.spawn(Camera2dBundle::default()).insert(Fxaa {
        enabled: true,
        edge_threshold: Sensitivity::Ultra,
        edge_threshold_min: Sensitivity::Ultra,
    });

    // quad
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(Triangle2d::default()).into(),
        transform: Transform::from_translation(vec3(0.0, 0.0, -100.0)),
        material: materials.add(DataMaterial::default()),
        ..default()
    });
}

#[make_hot]
fn draw(
    time: Res<Time>,
    mut painter: ShapePainter,
    mut player_ring: Local<u32>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_offset: Local<f32>,
    mut player_color_idx: Local<u32>,
    mut materials: ResMut<Assets<DataMaterial>>,
    window: Query<(Entity, &mut Window)>,
) {
    let (_, gpu) = materials.iter_mut().next().unwrap();
    let (_, window) = window.iter().next().unwrap();

    gpu.state.resolution = window
        .physical_size()
        .as_vec2()
        .extend(window.width())
        .extend(window.height());
    gpu.state.scale_factor = window.scale_factor();
    gpu.state.time = time.elapsed_seconds();

    let game_speed = 0.09;
    painter.hollow = true;
    painter.thickness = 5.0;
    painter.cap = Cap::None;
    let t = time.elapsed_seconds() * game_speed;
    gpu.state.t = t;
    let mut pressed_up = false;
    //if *player_direction == 0.0 {
    //    *player_direction = 1.0;
    //}
    if keyboard_input.just_pressed(KeyCode::ArrowUp) || keyboard_input.just_pressed(KeyCode::KeyW) {
        pressed_up = true;
        //*player_direction *= -1.0;
    }
    if keyboard_input.just_pressed(KeyCode::ArrowDown) {
        *player_ring = player_ring.saturating_sub(1);
    }

    for (i, k) in [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
    ]
    .iter()
    .enumerate()
    {
        if keyboard_input.just_pressed(*k) {
            *player_color_idx = i as u32;
        };
    }

    let local_player_pos = 10 + *player_ring;
    gpu.state.local_player_pos = local_player_pos;

    let ring_thick = (25.0 - (*player_ring as f32) * 0.25).max(7.0);
    gpu.state.ring_thick = ring_thick;

    {
        let ring_speed = get_ring_speed(local_player_pos, 0, 0);
        let ring_start = (t * (ring_speed * (local_player_pos + 1) as f32) + *player_offset)
            .rem_euclid(1.0)
            * TAU;
        let norm_pos = vec3(-ring_start.sin(), -ring_start.cos(), 0.0);
        let position =
            norm_pos * local_player_pos as f32 * ring_thick - norm_pos * ring_thick * 0.5;

        gpu.state.position = vec4(position.x, -position.y, 0.0, 0.0);

        painter.set_translation(position);
    }
    let draw_debug_rings = false;
    let draw_debug_arc = false;

    if draw_debug_rings {
        for i in 0..115u32 + local_player_pos {
            if i % 2 == 0 {
                painter.set_color(Color::srgb(0.0, 1.0, 1.0));
                arc(&mut painter, 0.0, 1.0, i, ring_thick);
            }
        }
    }

    let mut missed = false;
    let start = local_player_pos;
    let mut end = local_player_pos + 2;
    if draw_debug_arc {
        end = local_player_pos + 60; // Only eval further out rings if draw debug
    }
    for ring in start..end {
        let ring_speed = get_ring_speed(ring, 0, 0);
        let arc_size = get_arc_size(ring, 0, 0);

        let ring_start = (t * (ring_speed * (ring + 1) as f32)).rem_euclid(1.0);

        if ring == local_player_pos && pressed_up {
            let next_speed = get_ring_speed(ring + 1, 0, 0);
            let this_p = (ring_start + *player_offset).rem_euclid(1.0);

            let next_size = get_arc_size(ring + 1, 0, 0);
            let next_p = (t * (next_speed * (ring + 2) as f32)).rem_euclid(1.0);

            let within = (this_p - next_p).rem_euclid(1.0);
            if within < next_size {
                *player_offset = within;
                *player_ring = player_ring.saturating_add(1);
            } else {
                missed = true;
            }
        }

        if missed {
            painter.hollow = false;
            painter.set_color(Color::srgb(1.0, 0.0, 0.0));
            painter.circle(5000.0);
        }

        if draw_debug_arc {
            painter.set_color(Color::srgb(1.0, 0.0, 1.0));
            arc(&mut painter, ring_start, arc_size, ring, ring_thick);
        }

        {
            // Edge Lines
            if ring == local_player_pos {
                painter.hollow = true;
                painter.thickness = 0.5;
                painter.cap = Cap::None;
                let inset = 0.0;
                let p = ring_start.fract() + inset;
                painter.set_color(Color::WHITE);
                painter.arc(
                    ring_thick * ((ring + 1) as f32) - ring_thick,
                    TAU * p,
                    TAU * (p + arc_size - inset * 2.0),
                );
            }
            if ring == local_player_pos + 1 {
                painter.hollow = true;
                painter.thickness = 0.5;
                painter.cap = Cap::None;
                let inset = 0.0;
                let p = ring_start.fract() + inset;
                painter.set_color(Color::WHITE);
                painter.arc(
                    ring_thick * ((ring + 1) as f32) - (ring_thick - 1.0) - ring_thick,
                    TAU * p,
                    TAU * (p + arc_size - inset * 2.0),
                );
            }
        }
        if ring == local_player_pos {
            let temp = painter.transform;
            painter.hollow = false;
            painter.set_translation(Vec3::ZERO);
            painter.set_color(Color::srgb(1.0, 1.0, 1.0));
            painter.circle((ring_thick * 0.5) * 0.8);
            painter.set_translation(temp.translation);
        }
    }
    gpu.state.player_offset = *player_offset;
}

fn arc(painter: &mut ShapePainter, start: f32, size: f32, ring: u32, ring_thick: f32) {
    painter.hollow = true;
    let debug_thick_scale = 0.5;
    painter.thickness = ring_thick * debug_thick_scale;
    painter.cap = Cap::None;
    let start = start.fract();
    painter.arc(
        ring_thick * ((ring + 1) as f32) - ring_thick,
        TAU * start,
        TAU * (start + size),
    );
}

fn get_arc_size(ring: u32, level: u32, seed: u32) -> f32 {
    (hash_noise(ring, level, seed) * 1.0 + 0.8) / (((ring + 1) as f32) * 0.5)
}

fn get_ring_speed(ring: u32, level: u32, seed: u32) -> f32 {
    ((hash_noise(ring, level, seed) * 1.0 + 0.8) / ((ring + 1) as f32))
        * (1.0 + ring as f32 * 0.0)
        * if ring % 2 == 0 { -1.0 } else { 1.0 }
}

#[derive(Clone, ShaderType, Default, Debug)]
struct GpuState {
    position: Vec4,
    resolution: Vec4,
    scale_factor: f32,
    ring_thick: f32,
    frame: f32,
    time: f32,
    local_player_pos: u32,
    player_offset: f32,
    t: f32,
    spare3: u32,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
struct DataMaterial {
    #[uniform(0)]
    state: GpuState,
}

impl Material2d for DataMaterial {
    fn fragment_shader() -> ShaderRef {
        "material2d.wgsl".into()
    }
    fn vertex_shader() -> ShaderRef {
        "material2d.wgsl".into()
    }
}
