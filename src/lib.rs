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
use bevy_asset_loader::asset_collection::AssetCollection;
use bevy_asset_loader::loading_state::config::ConfigureLoadingState;
use bevy_asset_loader::loading_state::{LoadingState, LoadingStateAppExt};
use bevy_kira_audio::prelude::AudioSource;
use bevy_kira_audio::AudioPlugin;
use bevy_kira_audio::{Audio, AudioControl};
use bevy_vector_shapes::shapes::DiscPainter;
use bevy_vector_shapes::Shape2dPlugin;
use bevy_vector_shapes::{painter::ShapePainter, shapes::Cap};
pub mod sampling;
use iyes_progress::{ProgressCounter, ProgressPlugin};
#[cfg(feature = "hot_reload")]
use ridiculous_bevy_hot_reloading::{hot_reloading_macros::make_hot, HotReloadPlugin};
use sampling::hash_noise;

const DRAW_DEBUG_RINGS: bool = false;
const DRAW_DEBUG_ARC: bool = false;
const DRAW_DEBUG_PLAYER: bool = false;
const GAME_SPEED: f32 = 0.08;
const STARTING_LEVEL: u32 = 10;
const STEP_ANIM_SPEED: f32 = 16.0;
const COOLDOWN_ANIM_SPEED: f32 = 1.0;

#[cfg(feature = "hot_reload")]
#[no_mangle] // Needed so libloading can find this entry point
fn main() {
    app();
}

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
enum GameLoading {
    #[default]
    AssetLoading,
    Loaded,
}

pub fn app() {
    App::new()
        .insert_resource(Msaa::Off)
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
                        title: String::from("BevyJam5"),
                        present_mode: PresentMode::AutoVsync,
                        fit_canvas_to_parent: true,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .init_state::<GameLoading>()
        .add_plugins(ProgressPlugin::new(GameLoading::AssetLoading))
        .add_loading_state(
            LoadingState::new(GameLoading::AssetLoading)
                .continue_to_state(GameLoading::Loaded)
                .load_collection::<AudioAssets>(),
        )
        .add_plugins((
            Material2dPlugin::<DataMaterial>::default(),
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin,
            AudioPlugin,
            //bevy_framepace::debug::DiagnosticsPlugin, // Crashes
            Shape2dPlugin::default(),
            #[cfg(feature = "hot_reload")]
            HotReloadPlugin {
                auto_watch: true,
                bevy_dylib: true,
                ..default()
            },
            bevy_framepace::FramepacePlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(OnEnter(GameLoading::Loaded), start_music)
        .add_systems(Update, draw.run_if(in_state(GameLoading::Loaded)))
        .add_systems(
            Update,
            loading_ui.run_if(in_state(GameLoading::AssetLoading)),
        )
        //.add_systems(Update, update_cursor)
        .run();
}

fn loading_ui(
    progress: Option<Res<ProgressCounter>>,
    mut last_done: Local<u32>,
    mut text: Query<&mut Text, With<GameText>>,
) {
    let mut text = text.single_mut();
    if let Some(progress) = progress.map(|counter| counter.progress()) {
        if progress.done > *last_done {
            *last_done = progress.done;
        }
        text.sections[0].value = "LOADING ".to_string();
        text.sections[1].value = format!("{}/{}", *last_done, progress.total);
    }
}

fn start_music(asset_server: Res<AssetServer>, audio: Res<Audio>) {
    audio
        .play(asset_server.load("audio/theme1.flac"))
        .looped()
        .with_volume(0.5);
}

#[derive(AssetCollection, Resource)]
pub struct AudioAssets {
    #[asset(path = "audio/enemyexplode1.flac")]
    pub enemyexplode1: Handle<AudioSource>,
    #[asset(path = "audio/enemygun1.flac")]
    pub enemygun1: Handle<AudioSource>,
    #[asset(path = "audio/playergun1.flac")]
    pub playergun1: Handle<AudioSource>,
    #[asset(path = "audio/playerhit1.flac")]
    pub playerhit1: Handle<AudioSource>,
    #[asset(path = "audio/theme1.flac")]
    pub theme1: Handle<AudioSource>,
}

pub fn update_cursor(windows: Query<&Window>, mut gizmos: Gizmos) {
    // Latency test
    if let Some(pos) = windows.single().cursor_position() {
        let pos = Vec2::new(
            pos.x - windows.single().width() / 2.0,
            windows.single().height() / 2.0 - pos.y,
        );
        gizmos.circle_2d(pos, 10.0, bevy::color::palettes::basic::GREEN);
    }
}

#[derive(Component)]
struct GameText;

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

    let style = TextStyle {
        font_size: 40.0,
        color: Color::WHITE,
        ..default()
    };

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                TextBundle::from_sections(vec![
                    TextSection {
                        value: String::from(""),
                        style: style.clone(),
                    };
                    3
                ]),
                GameText,
            ));
        });
}

// This mess is here to workaround using hot reloading with a quick way to still use wasm
#[cfg(feature = "hot_reload")]
#[make_hot]
fn draw(
    time: Res<Time>,
    painter: ShapePainter,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    materials: ResMut<Assets<DataMaterial>>,
    window: Query<(Entity, &mut Window)>,
    text: Query<&mut Text, With<GameText>>,
    audio: Res<bevy_kira_audio::Audio>,
    audio_assets: Res<AudioAssets>,
) {
    draw_fn(
        time,
        painter,
        keyboard_input,
        materials,
        window,
        text,
        audio,
        audio_assets,
    );
}

#[cfg(not(feature = "hot_reload"))]
fn draw(
    time: Res<Time>,
    painter: ShapePainter,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    materials: ResMut<Assets<DataMaterial>>,
    window: Query<(Entity, &mut Window)>,
    text: Query<&mut Text, With<GameText>>,
    audio: Res<bevy_kira_audio::Audio>,
    audio_assets: Res<AudioAssets>,
) {
    draw_fn(
        time,
        painter,
        keyboard_input,
        materials,
        window,
        text,
        audio,
        audio_assets,
    );
}

fn draw_fn(
    time: Res<Time>,
    mut painter: ShapePainter,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut materials: ResMut<Assets<DataMaterial>>,
    window: Query<(Entity, &mut Window)>,
    mut text: Query<&mut Text, With<GameText>>,
    audio: Res<bevy_kira_audio::Audio>,
    audio_assets: Res<AudioAssets>,
) {
    let (_, gpu) = materials.iter_mut().next().unwrap();
    let (_, window) = window.iter().next().unwrap();
    let mut text = text.single_mut();

    let state = &mut gpu.state;
    if state.player_ring == 0 {
        state.player_ring = STARTING_LEVEL;
    }

    state.resolution = window
        .physical_size()
        .as_vec2()
        .extend(window.width())
        .extend(window.height());
    state.scale_factor = window.scale_factor();
    state.time += time.delta_seconds();
    if state.paused == 0 {
        if state.t * 7.0 > (state.player_ring + 1) as f32 {
            state.t += time.delta_seconds() * GAME_SPEED * 0.3;

            state.player_dead = 1;
        } else {
            state.t += time.delta_seconds() * GAME_SPEED;
        }
    }

    text.sections[0].value = String::new();
    text.sections[1].value = String::new();
    text.sections[2].value = String::new();
    if state.player_dead != 0 || state.paused != 0 {
        if keyboard_input.just_pressed(KeyCode::Enter) {
            *state = Default::default();
            return;
        }
        text.sections[0].value = format!(
            "LEVEL        {:>9}\nMISSED JUMPS {:>9}",
            state.player_miss,
            state.player_ring as i32 - STARTING_LEVEL as i32
        );
        text.sections[1].value = "\n\nPRESS ENTER TO RESTART".to_string();
        text.sections[1].style.color = Color::srgba(
            1.0,
            1.0,
            1.0,
            ((state.time * 5.0).sin() * 0.5 + 0.5) * 0.85 + 0.15,
        );
        if state.paused != 0 {
            text.sections[2].value = "\n\nPRESS P OR TAB TO RESUME".to_string();
            text.sections[2].style.color = Color::srgba(
                1.0,
                1.0,
                1.0,
                ((state.time * 5.0).cos() * 0.5 + 0.5) * 0.85 + 0.15,
            );
        }
    }

    painter.hollow = true;
    painter.thickness = 5.0;
    painter.cap = Cap::None;
    let mut pressed_up = false;
    //if *player_direction == 0.0 {
    //    *player_direction = 1.0;
    //}
    if keyboard_input.just_pressed(KeyCode::ArrowUp)
        || keyboard_input.just_pressed(KeyCode::KeyW)
        || keyboard_input.just_pressed(KeyCode::Space)
    {
        pressed_up = true;
        //*player_direction *= -1.0;
    }

    if keyboard_input.just_pressed(KeyCode::KeyP)
        || keyboard_input.just_pressed(KeyCode::Escape)
        || keyboard_input.just_pressed(KeyCode::Tab)
    {
        state.paused = !state.paused;
    }

    if keyboard_input.just_pressed(KeyCode::ArrowDown) {
        state.player_ring = state.player_ring.saturating_sub(1);
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
            state.player_color_idx = i as u32;
        };
    }

    let ring_thick = (25.0 - (state.player_ring as f32) * 0.2).max(6.0);
    state.ring_thick = ring_thick;

    {
        let ring_speed = get_ring_speed(state.player_ring, state.player_sub_ring, 0);
        let ring_start = (state.t * (ring_speed * (state.player_ring + 1) as f32)
            + state.player_offset)
            .rem_euclid(1.0)
            * TAU;
        let norm_pos = vec3(-ring_start.sin(), -ring_start.cos(), 0.0);
        let ring_center_offset = norm_pos * ring_thick * 0.5;
        let step_anim_offset = norm_pos * ring_thick * -(1.0 - state.step_anim);
        let position = norm_pos * state.player_ring as f32 * ring_thick - ring_center_offset
            + step_anim_offset;

        state.position = vec4(position.x, -position.y, 0.0, 0.0);

        painter.set_translation(position);
    }

    if DRAW_DEBUG_RINGS {
        for i in 0..115u32 + state.player_ring {
            if i % 2 == 0 {
                painter.set_color(Color::srgb(0.0, 1.0, 1.0));
                arc(&mut painter, 0.0, 1.0, i, ring_thick);
            }
        }
    }

    let start = state.player_ring;
    let mut end = state.player_ring + 2;
    if DRAW_DEBUG_ARC {
        end = state.player_ring + 60; // Only eval further out rings if draw debug
    }

    for ring in start..end {
        let max_arcs = get_max_arcs(ring);
        for sub_ring in 0..max_arcs {
            let ring_speed = get_ring_speed(ring, sub_ring, 0);
            let arc_size = get_arc_size(ring, sub_ring, 0);
            let ring_start = (state.t * (ring_speed * (ring + 1) as f32)).rem_euclid(1.0);

            if DRAW_DEBUG_ARC {
                painter.set_color(Color::srgb(1.0, 0.0, 1.0));
                arc(&mut painter, ring_start, arc_size, ring, ring_thick);
            }
        }
    }

    if pressed_up && state.move_cooldown == 1.0 {
        let mut missed_all = true;
        let max_arcs = get_max_arcs(state.player_ring);
        for sub_ring in 0..max_arcs {
            let ring = state.player_ring;
            let ring_speed = get_ring_speed(ring, state.player_sub_ring, 0);
            let ring_start = (state.t * (ring_speed * (ring + 1) as f32)).rem_euclid(1.0);
            let this_p = (ring_start + state.player_offset).rem_euclid(1.0);

            let next_speed = get_ring_speed(ring + 1, sub_ring, 0);
            let next_size = get_arc_size(ring + 1, sub_ring, 0);
            let next_p = (state.t * (next_speed * (ring + 2) as f32)).rem_euclid(1.0);

            let within = (this_p - next_p).rem_euclid(1.0);
            if within < next_size {
                state.player_offset = within;
                state.player_ring = state.player_ring.saturating_add(1);
                state.step_anim = 0.0;
                state.player_sub_ring = sub_ring;
                missed_all = false;
                break;
            }
        }
        if missed_all {
            state.move_cooldown = 0.0;
            state.player_miss += 1;
            audio.play(audio_assets.playerhit1.clone());
        } else {
            audio.play(audio_assets.playergun1.clone());
        }
    }

    if DRAW_DEBUG_PLAYER {
        if state.player_dead == 0 {
            // Draw player
            if state.move_cooldown < 1.0 {
                let v = ((state.t * 200.0).sin() * 0.7 + 0.5) * 0.6 + 0.1;
                painter.set_color(Color::srgba(1.0, 1.0, 1.0, v));
            } else {
                painter.set_color(Color::srgb(1.0, 1.0, 1.0));
            }
            let temp = painter.transform;
            painter.hollow = false;
            painter.set_translation(Vec3::ZERO);
            painter.circle((ring_thick * 0.5) * 0.8);
            painter.set_translation(temp.translation);
        }
    }

    state.step_anim = (state.step_anim + time.delta_seconds() * STEP_ANIM_SPEED).min(1.0);

    state.move_cooldown =
        (state.move_cooldown + time.delta_seconds() * COOLDOWN_ANIM_SPEED).min(1.0);
}

fn get_max_arcs(ring: u32) -> u32 {
    ((ring as i32 - 16).max(0) as u32 / 4).clamp(2, 6)
}

fn get_arc_size(ring: u32, level: u32, seed: u32) -> f32 {
    (hash_noise(ring, level, seed) * 0.2 + 0.2) / (((ring + 1) as f32) * 0.13 + 2.0)
}

fn get_ring_speed(ring: u32, level: u32, seed: u32) -> f32 {
    ((hash_noise(ring, level, seed) * 1.0 + 0.8) / ((ring + 1) as f32))
        * (1.0 + ring as f32 * 0.0)
        * if ring % 2 == 0 { -1.0 } else { 1.0 }
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

#[derive(Clone, ShaderType, Default, Debug)]
struct GpuState {
    position: Vec4,
    resolution: Vec4,

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
    paused: u32,
    spare1: u32,
    spare2: u32,
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
