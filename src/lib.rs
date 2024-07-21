#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use std::f32::consts::{PI, TAU};

use bevy::asset::AssetMetaCheck;
use bevy::math::{vec3, VectorSpace};
use bevy::prelude::*;
use bevy::winit::{UpdateMode, WinitSettings};
use bevy_vector_shapes::shapes::DiscPainter;
use bevy_vector_shapes::Shape2dPlugin;
use bevy_vector_shapes::{painter::ShapePainter, shapes::Cap};
pub mod palette;
pub mod sampling;

use palette::RGB_PALETTE;
use ridiculous_bevy_hot_reloading::{hot_reloading_macros::make_hot, HotReloadPlugin};
use sampling::hash_noise;

/// #[no_mangle] Needed so libloading can find this entry point
#[no_mangle]
fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.05)))
        .insert_resource(WinitSettings {
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
        })
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            // Wasm builds will check for meta files (that don't exist) if this isn't set.
            // This causes errors and even panics in web builds on itch.
            // See https://github.com/bevyengine/bevy_github_ci_template/issues/48.
            meta_check: AssetMetaCheck::Never,
            ..default()
        }))
        .add_plugins((
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

fn setup(mut commands: Commands, _asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
}

const RING_DEPTH: f32 = 30.0;

#[make_hot]
fn draw(
    time: Res<Time>,
    mut painter: ShapePainter,
    mut player_ring: Local<u32>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_offset: Local<f32>,
    mut player_color_idx: Local<u32>,
    mut radius_offset: Local<f32>,
) {
    let autoaim = 0.04;
    let game_speed = 2.0 + *player_ring as f32 * 0.01;
    painter.hollow = true;
    painter.thickness = 5.0;
    painter.cap = Cap::None;
    let t = time.elapsed_seconds() * game_speed;
    //*radius_offset += time.delta_seconds() * game_speed * 2.0;
    let mut pressed_up = false;
    if keyboard_input.just_pressed(KeyCode::ArrowUp) {
        //*player_offset = 0.0;
        //*player_ring = player_ring.saturating_add(1);
        pressed_up = true;
    }
    if keyboard_input.just_pressed(KeyCode::ArrowDown) {
        *player_ring = player_ring.saturating_sub(1);
    }
    //if keyboard_input.pressed(KeyCode::ArrowRight) {
    *player_offset += 0.3 * time.delta_seconds() * game_speed;
    //};
    //if keyboard_input.pressed(KeyCode::ArrowLeft) {
    //    *player_offset -= 0.005;
    //};

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

    let local_player_pos = 6;

    if false {
        // Dizzy mode
        let ring_speed_player =
            get_ring_speed(local_player_pos + *player_ring, 0, 0, local_player_pos);

        let game_ring = local_player_pos + *player_ring;
        let arc_size = get_arc_size(game_ring + 1024, 0, 0, local_player_pos);
        let p_player = t * (ring_speed_player * (local_player_pos + 1) as f32) + *player_offset;
        let t_player = (p_player + arc_size * 0.5) * TAU;
        painter.set_color(Color::WHITE);
        //painter.hollow = false;
        painter.arc(
            RING_DEPTH * ((local_player_pos + 1) as f32) - RING_DEPTH,
            TAU * p_player,
            TAU * (p_player + 0.1),
        );
        painter.set_translation(vec3(
            -t_player.sin()
                * local_player_pos as f32
                * RING_DEPTH
                * (1.0 + local_player_pos as f32 / 80.0),
            -t_player.cos()
                * local_player_pos as f32
                * RING_DEPTH
                * (1.0 + local_player_pos as f32 / 80.0),
            0.0,
        ));
        //painter.circle(10.0);
        //painter.set_translation(Vec3::ZERO);
    }

    for i in 0..100u32 {
        let game_ring = i + *player_ring;
        let mut color = if i % 2 == 0 {
            RGB_PALETTE[1][1]
        } else {
            RGB_PALETTE[1][2]
        };

        //if i == local_player_pos {
        //    color = RGB_PALETTE[0][1];
        //};
        painter.set_color(color);
        arc(&mut painter, 0.0, 1.0, i, *radius_offset);
    }
    let mut missed = false;
    for i in 0..100u32 {
        let game_ring = i + *player_ring;
        let color = if game_ring % 2 == 0 {
            RGB_PALETTE[1][1]
        } else {
            RGB_PALETTE[1][4]
        };
        let ring_speed = get_ring_speed(game_ring, 0, 0, i);
        let arc_size = get_arc_size(game_ring + 1024, 0, 0, i);
        let mut ring_color_idx = get_ring_color(game_ring, 0, 0);
        let mut offset = 0.0;
        if i == local_player_pos {
            offset = *player_offset;
            ring_color_idx = *player_color_idx;
        }
        let p = (t * (ring_speed * (i + 1) as f32) + offset).rem_euclid(1.0);
        let next_speed = get_ring_speed(game_ring + 1, 0, 0, i + 1);

        painter.set_color(idx_color(ring_color_idx));
        if i == local_player_pos && pressed_up {
            //
            let mut this_p = p;
            let mut this_size = arc_size;
            let mut next_size = get_arc_size(game_ring + 1 + 1024, 0, 0, i + 1);
            let mut next_p = (t * (next_speed * (i + 2) as f32)).rem_euclid(1.0);
            if arc_size > next_size {
                (this_p, next_p) = (next_p, this_p);
                (this_size, next_size) = (next_size, this_size);
            }
            this_size -= autoaim;
            this_p = (this_p + autoaim * 0.5).rem_euclid(1.0);
            let within = (this_p - next_p).rem_euclid(1.0);
            if within + this_size < next_size {
                *player_offset = 0.0;
                *player_ring = player_ring.saturating_add(1);
                //*radius_offset -= 1.0;
            } else {
                missed = true;
            }
        }

        if missed {
            painter.hollow = false;
            painter.set_color(Color::srgb(1.0, 0.0, 0.0));
            painter.circle(5000.0);
        }

        arc(&mut painter, p, arc_size, i, *radius_offset);
        if i == local_player_pos {
            painter.thickness = 0.5;
            painter.cap = Cap::None;
            let inset = 0.0;
            let p = p.fract() + inset;
            painter.set_color(Color::WHITE);
            painter.arc(
                RING_DEPTH * ((i + 1) as f32) - *radius_offset * RING_DEPTH,
                TAU * p,
                TAU * (p + arc_size - inset * 2.0),
            );
        }
        if i == local_player_pos + 1 {
            painter.thickness = 0.5;
            painter.cap = Cap::None;
            let inset = 0.0;
            let p = p.fract() + inset;
            painter.set_color(Color::WHITE);
            painter.arc(
                RING_DEPTH * ((i + 1) as f32) - (RING_DEPTH - 1.0) - *radius_offset * RING_DEPTH,
                TAU * p,
                TAU * (p + arc_size - inset * 2.0),
            );
            //painter.hollow = true;
            //painter.thickness = RING_DEPTH;
            //painter.cap = Cap::None;
            //let inset = ring_size * 0.49;
            //let p = p.fract() + inset;
            //painter.set_color(Color::WHITE);
            //painter.arc(
            //    RING_DEPTH * ((i + 1) as f32),
            //    TAU * p,
            //    TAU * (p + ring_size - inset * 2.0),
            //);
        }
    }
}

fn arc(painter: &mut ShapePainter, t: f32, size: f32, ring: u32, radius_offset: f32) {
    painter.hollow = true;
    painter.thickness = RING_DEPTH;
    painter.cap = Cap::None;
    let t = t.fract();
    painter.arc(
        RING_DEPTH * ((ring + 1) as f32) - radius_offset * RING_DEPTH,
        TAU * t,
        TAU * (t + size),
    );
}

fn get_arc_size(ring: u32, level: u32, attempt: u32, local_ring: u32) -> f32 {
    (if ring % 2 == 0 { 0.3 } else { 0.9 }) / ((local_ring + 1) as f32)
}

fn get_ring_speed(ring: u32, level: u32, attempt: u32, local_ring: u32) -> f32 {
    (hash_noise(ring, level, attempt) * 0.2 + 0.1) / ((local_ring + 1) as f32)
}

fn get_ring_color(ring: u32, level: u32, attempt: u32) -> u32 {
    (hash_noise(ring + 2048, level, attempt) * 1.0 - 0.0000001).floor() as u32
}

fn idx_color(i: u32) -> Color {
    match i {
        0 => RGB_PALETTE[0][1],
        1 => RGB_PALETTE[1][4],
        2 => RGB_PALETTE[2][4],
        _ => RGB_PALETTE[0][0],
    }
}
