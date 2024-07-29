#![allow(clippy::too_many_arguments, clippy::type_complexity)]
#![allow(non_snake_case)] // For game name

use std::f32::consts::*;
use std::time::Duration;

use bevy::asset::AssetMetaCheck;
use bevy::core_pipeline::fxaa::{Fxaa, Sensitivity};

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};

use bevy::math::*;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use bevy::sprite::{Material2d, Material2dPlugin, MaterialMesh2dBundle};
use bevy::window::{PresentMode, WindowMode};
use bevy::winit::{UpdateMode, WinitSettings};
use bevy_asset_loader::{
    asset_collection::AssetCollection,
    loading_state::{config::ConfigureLoadingState, LoadingState, LoadingStateAppExt},
};
use bevy_kira_audio::{prelude::AudioSource, Audio, AudioControl, AudioPlugin};
use bevy_kira_audio::{AudioInstance, AudioTween};
pub mod sampling;
use iyes_progress::{ProgressCounter, ProgressPlugin};
#[cfg(feature = "hot_reload")]
use ridiculous_bevy_hot_reloading::{hot_reloading_macros::make_hot, HotReloadPlugin};
use sampling::{gain_from_db, hash_noise, pfract};

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
                        title: String::from("Sol"),
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
        //.add_systems(Update, update_cursor_latency_test)
        .run();
}

pub fn update_cursor_latency_test(windows: Query<&Window>, mut gizmos: Gizmos) {
    let window: &Window = windows.single();
    if let Some(pos) = window.cursor_position() {
        let pos = Vec2::new(pos.x - window.width() / 2.0, window.height() / 2.0 - pos.y);
        gizmos.circle_2d(pos, 10.0, Color::WHITE);
    }
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

#[derive(AssetCollection, Resource)]
pub struct AudioAssets {
    #[asset(path = "audio/tone.flac")]
    pub tone: Handle<AudioSource>,
    #[asset(path = "audio/miss_tone.flac")]
    pub miss_tone: Handle<AudioSource>,
    #[asset(path = "audio/close.flac")]
    pub close: Handle<AudioSource>,
    #[asset(path = "audio/theme1.flac")]
    pub theme1: Handle<AudioSource>,
}
#[derive(Resource)]
pub struct OrbAudioHandle(pub Handle<AudioInstance>);

fn start_music(mut commands: Commands, asset_server: Res<AssetServer>, audio: Res<Audio>) {
    audio
        .play(asset_server.load("audio/theme1.flac"))
        .looped()
        .with_volume(gain_from_db(-8.0) as f64);
    commands.insert_resource(OrbAudioHandle(
        audio
            .play(asset_server.load("audio/close.flac"))
            .looped()
            .with_volume(0.0)
            .handle(),
    ));
}

#[derive(Component)]
struct GameText;

fn setup(
    mut commands: Commands,
    _asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<DataMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // FXAA is a bit silly here but with everything moving so much it doesn't really matter.
    // This allows for simpler math in the game shader while avoiding multi sampling in the shader for every fragment.
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

#[cfg_attr(feature = "hot_reload", make_hot)]
fn draw(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut materials: ResMut<Assets<DataMaterial>>,
    mut window: Query<(Entity, &mut Window)>,
    mut text: Query<&mut Text, With<GameText>>,
    audio: Res<bevy_kira_audio::Audio>,
    audio_assets: Res<AudioAssets>,
    close_audio: Option<Res<OrbAudioHandle>>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
    mut audio_muted: Local<bool>,
) {
    let (_, gpu) = materials.iter_mut().next().unwrap();
    let (_, mut window) = window.iter_mut().next().unwrap();
    let mut text = text.single_mut();
    if keyboard_input.just_pressed(KeyCode::F11) || keyboard_input.just_pressed(KeyCode::KeyF) {
        if window.mode != WindowMode::BorderlessFullscreen {
            window.mode = WindowMode::BorderlessFullscreen;
            window.cursor.visible = false;
        } else {
            window.mode = WindowMode::Windowed;
            window.cursor.visible = true;
        }
    }
    if keyboard_input.just_pressed(KeyCode::Escape) {
        window.mode = WindowMode::Windowed;
        window.cursor.visible = true;
    }
    if keyboard_input.just_pressed(KeyCode::KeyM) {
        *audio_muted = !*audio_muted;
        if *audio_muted {
            audio.pause();
        } else {
            audio.resume();
        }
    }

    let state = &mut gpu.state;
    if state.player_ring == 0 {
        state.player_ring = STARTING_LEVEL;
    }
    let rel_player_level = state.player_ring as i32 - STARTING_LEVEL as i32;

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
            rel_player_level, state.player_miss
        );
        text.sections[1].value = "\n\nPRESS ENTER TO RESTART".to_string();
        text.sections[1].style.color =
            Color::WHITE.with_alpha(((state.time * 5.0).sin() * 0.5 + 0.5) * 0.85 + 0.15);
        if state.paused != 0 {
            text.sections[2].value = "\n\nPRESS UP OR SPACE TO RESUME".to_string();
            text.sections[2].style.color =
                Color::WHITE.with_alpha(((state.time * 5.0).cos() * 0.5 + 0.5) * 0.85 + 0.15);
        }
    }

    let mut pressed_up = false;
    if keyboard_input.just_pressed(KeyCode::ArrowUp)
        || keyboard_input.just_pressed(KeyCode::KeyW)
        || keyboard_input.just_pressed(KeyCode::Space)
    {
        if state.paused == 0 {
            pressed_up = true;
        } else {
            state.paused = 0;
        }
    }

    if (keyboard_input.just_pressed(KeyCode::KeyP)
        || keyboard_input.just_pressed(KeyCode::Escape)
        || keyboard_input.just_pressed(KeyCode::Tab))
        && state.paused == 0
    {
        state.paused = u32::MAX;
    }

    let ring_thick = (25.0 - (state.player_ring as f32) * 0.2).max(6.0);
    state.ring_thick = ring_thick;

    {
        let ring_speed = get_ring_speed(state.player_ring, state.player_sub_ring, 0);
        let ring_start =
            pfract(state.t * (ring_speed * (state.player_ring + 1) as f32) + state.player_offset)
                * TAU;
        let norm_pos = vec3(-ring_start.sin(), -ring_start.cos(), 0.0);
        let ring_center_offset = norm_pos * ring_thick * 0.5;
        let step_anim_offset = norm_pos * ring_thick * -(1.0 - state.step_anim);
        let position = norm_pos * state.player_ring as f32 * ring_thick - ring_center_offset
            + step_anim_offset;

        state.position = vec4(position.x, -position.y, 0.0, 0.0);
    }

    if pressed_up && state.move_cooldown == 1.0 {
        let mut missed_all = true;
        let max_arcs = get_max_arcs(state.player_ring);
        for sub_ring in 0..max_arcs {
            let ring = state.player_ring;
            let ring_speed = get_ring_speed(ring, state.player_sub_ring, 0);
            let ring_start = pfract(state.t * (ring_speed * (ring + 1) as f32));
            let this_p = pfract(ring_start + state.player_offset);

            let next_speed = get_ring_speed(ring + 1, sub_ring, 0);
            let next_size = get_arc_size(ring + 1, sub_ring, 0);
            let next_p = pfract(state.t * (next_speed * (ring + 2) as f32));

            let within = pfract(this_p - next_p);
            if within < next_size {
                state.player_offset = within;
                state.player_ring += 1;
                state.step_anim = 0.0;
                state.player_sub_ring = sub_ring;
                missed_all = false;
                break;
            }
        }
        if missed_all {
            state.move_cooldown = 0.0;
            state.player_miss += 1;
            audio
                .play(audio_assets.miss_tone.clone())
                .with_playback_rate(0.9)
                .with_volume(0.6);
            audio
                .play(audio_assets.miss_tone.clone())
                .with_playback_rate(0.8)
                .with_volume(0.6);
        } else {
            let intervals = [0, 1, 3, 5, 7, 8, 11, 12];
            let intervals2 = [0, 1, 3, 5, 7, 8, 12];
            let intervals3 = [0, 3, 5, 7, 8, 12];
            let vol = gain_from_db(-8.0) as f64;

            if rel_player_level >= 8 {
                let interval = intervals2[rel_player_level as usize % intervals2.len()];
                audio
                    .play(audio_assets.tone.clone())
                    .with_playback_rate(1.059463f64.powi(interval) * 0.5)
                    .with_volume(vol);
                if rel_player_level >= 100 {
                    audio
                        .play(audio_assets.tone.clone())
                        .with_playback_rate(1.059463f64.powi(interval) * 0.5)
                        .with_volume(vol * 1.5)
                        .reverse();
                }
                let interval = intervals3[(rel_player_level + 4) as usize % intervals3.len()];
                audio
                    .play(audio_assets.tone.clone())
                    .with_playback_rate(1.059463f64.powi(interval))
                    .with_volume(vol * 0.6);
            } else {
                let interval = intervals[rel_player_level as usize % intervals.len()];
                audio
                    .play(audio_assets.tone.clone())
                    .with_playback_rate(1.059463f64.powi(interval) * 0.5)
                    .with_volume(vol);
            }
            if rel_player_level >= 16 {
                let interval = intervals2[rel_player_level as usize % intervals2.len()];
                audio
                    .play(audio_assets.tone.clone())
                    .with_playback_rate(1.059463f64.powi(interval) * 0.25)
                    .with_volume(vol * 0.6);
            }
            if rel_player_level >= 24 {
                let interval = intervals3[(rel_player_level + 6) as usize % intervals3.len()];
                audio
                    .play(audio_assets.tone.clone())
                    .with_playback_rate(1.059463f64.powi(interval) * 2.0)
                    .with_volume(vol * 0.11);
            }
        }
    }

    if let Some(close_audio) = close_audio {
        if let Some(close) = audio_instances.get_mut(&close_audio.0) {
            let v = (state.t * 7.0 - state.player_ring as f32 + 3.0).clamp(0.0, 3.0) / 3.0;
            close.set_volume(
                (v * 0.1) as f64,
                AudioTween::linear(Duration::from_secs_f32(0.1)),
            );
            if state.player_dead != 0 {
                close.set_volume(0.0, AudioTween::linear(Duration::from_secs_f32(0.01)));
            }
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
        "game_shader.wgsl".into()
    }
    fn vertex_shader() -> ShaderRef {
        "game_shader.wgsl".into()
    }
}
