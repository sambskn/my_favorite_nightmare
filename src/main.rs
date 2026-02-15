use crate::{
    fonts::SANS_FONT_PATH,
    sprites::{BillboardSpritePlugin, FocusType, PlayerFocus, PlayerStart},
    ui::{GameState, LoadingPlugin, MenuPlugin, TEXT_COLOR},
};
use avian3d::{math::*, prelude::*};
use bevy::{
    input::{common_conditions::input_just_pressed, mouse::AccumulatedMouseMotion},
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};
use bevy_seedling::prelude::*;
use bevy_trenchbroom::prelude::*;
use bevy_trenchbroom_avian::AvianPhysicsBackend;
use rand::seq::IndexedRandom;

mod fonts;
mod sprites;
mod text_parse;
mod ui;

const GRAVITY_MULT: f32 = 160.0;

// marker component for stuff to destroy between levels
#[derive(Component)]
struct LevelStuff;

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.463, 0.722, 0.643)));
    app.add_plugins((
        DefaultPlugins
            .set(ImagePlugin::default_nearest()) // show pixels
            .set(WindowPlugin {
                // setup window
                primary_window: Window {
                    fit_canvas_to_parent: true, // make it fill on web
                    ..default()
                }
                .into(),
                ..default()
            }),
        SeedlingPlugin::default(),
    ))
    .add_plugins(MenuPlugin)
    .add_plugins(LoadingPlugin)
    .add_plugins((
        PhysicsPlugins::default(),
        PhysicsPickingPlugin,
        TrenchBroomPhysicsPlugin::new(AvianPhysicsBackend),
    ))
    .insert_resource(Gravity(Vec3::NEG_Y * GRAVITY_MULT))
    .add_plugins(
        TrenchBroomPlugins(
            TrenchBroomConfig::new("my_favorite_nightmare").default_solid_scene_hooks(|| {
                SceneHooks::new()
                    .smooth_by_default_angle()
                    .convex_collider()
            }),
        )
        .build(),
    )
    .add_plugins((
        CameraPlugin,
        AudioPlugin,
        TrenchLoaderPlugin,
        BillboardSpritePlugin,
    ));

    app.run();
}

struct AudioPlugin;
impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnTransition {
                exited: GameState::Loading,
                entered: GameState::InGame,
            },
            play_level_intro_stinger,
        )
        .add_systems(FixedUpdate, play_walking_noises)
        .add_observer(on_stinger_finished);
    }
}

#[derive(Component)]
struct OnIntroStingerFinished;

fn play_level_intro_stinger(mut commands: Commands, server: Res<AssetServer>) {
    commands.spawn((
        SamplePlayer::new(server.load("sounds/intro1.wav")),
        OnIntroStingerFinished,
    ));
}

fn on_stinger_finished(
    _: On<Remove, OnIntroStingerFinished>,
    server: Res<AssetServer>,
    mut commands: Commands,
) {
    // Start level bg, looping
    // TODO: Do lookup for level?
    commands.spawn((
        SamplePlayer::new(server.load("sounds/bgm1.wav"))
            .with_volume(Volume::from_percent(50.))
            .looping(),
        LevelStuff,
    ));
}

#[derive(Component)]
struct WalkingSFX;

const WALKING_NOISE_MIN_VEL: f32 = 2.5;

fn play_walking_noises(
    player_vels: Query<&LinearVelocity, With<PlayerCamera>>,
    playing_walking_samples: Query<&bevy_seedling::sample::PlaybackSettings, With<WalkingSFX>>,
    mut commands: Commands,
    server: Res<AssetServer>,
) {
    for vel in player_vels {
        if vel.length() > WALKING_NOISE_MIN_VEL {
            // only fire if none are playing
            if playing_walking_samples.is_empty() {
                let sfx_path = get_random_walking_sound_path();
                commands.spawn((SamplePlayer::new(server.load(sfx_path)), WalkingSFX));
            }
        }
    }
}

fn get_random_walking_sound_path() -> String {
    let mut rng = rand::rng();
    let noises = vec![
        "sounds/squelch1.wav",
        "sounds/squelch2.wav",
        "sounds/squelch3.wav",
        "sounds/squelch4.wav",
        "sounds/squelch5.wav",
        "sounds/squelch6.wav",
        "sounds/squelch7.wav",
    ];
    noises.choose(&mut rng).unwrap().to_string()
}

const DEFAULT_PLAYER_START_LOC: Vec3 = Vec3::new(1.375, 0.9, 0.6);

// Plugin that spawns the camera.
struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource::<LevelStartLocation>(LevelStartLocation(
            DEFAULT_PLAYER_START_LOC.clone(),
        ))
        .add_systems(
            OnTransition {
                exited: GameState::Loading,
                entered: GameState::InGame,
            },
            spawn_camera,
        )
        .add_systems(
            OnTransition {
                exited: GameState::InGame,
                entered: GameState::Loading,
            },
            reset_focus,
        )
        .add_systems(
            FixedUpdate,
            (
                (player_camera_movement, debug_commands_and_oob_reset)
                    .run_if(in_state(GameState::InGame)),
                update_player_start_location.run_if(in_state(GameState::Loading)),
            ),
        )
        .add_systems(
            Update,
            (
                update_camera_transform.run_if(in_state(GameState::InGame)),
                capture_cursor
                    .run_if(input_just_pressed(MouseButton::Left))
                    .run_if(in_state(GameState::InGame)),
                release_cursor
                    .run_if(input_just_pressed(KeyCode::Escape))
                    .run_if(in_state(GameState::InGame)),
                update_action_text.run_if(in_state(GameState::InGame)),
            ),
        );
    }
}

#[derive(Component)]
struct TextBox;

#[derive(Component)]
struct ActionText;

fn update_action_text(
    existing_action_text: Query<Entity, With<ActionText>>,
    focus: Res<PlayerFocus>,
    mut commands: Commands,
    server: Res<AssetServer>,
) {
    let current_focus = focus.0.clone();

    // Check if action text exists
    let action_text_exists = !existing_action_text.is_empty();

    match (action_text_exists, current_focus.is_some()) {
        // Exists but shouldn't - despawn it
        (true, false) => {
            for ent in &existing_action_text {
                commands.entity(ent).despawn();
            }
        }
        // Doesn't exist but should - spawn it
        (false, true) => {
            let text = match current_focus.unwrap().focus_type {
                FocusType::Hole => get_action_str_hole(),
                FocusType::NPC => get_action_str_npc(),
            };
            commands.spawn((
                Node {
                    top: vh(50),
                    left: vw(50),
                    position_type: PositionType::Absolute,
                    display: Display::Flex,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(px(20), px(10)),
                    margin: UiRect {
                        left: px(-36),
                        right: px(0),
                        top: px(0),
                        bottom: px(0),
                    },
                    ..default()
                },
                BackgroundColor {
                    0: Color::Oklcha(Oklcha::new(0.1788, 0.0099, 288.85, 0.9)),
                },
                ActionText,
                LevelStuff,
                children![(
                    Text::new(text),
                    TextColor(TEXT_COLOR),
                    TextFont {
                        font: server.load(SANS_FONT_PATH),
                        font_size: 34.0,
                        ..default()
                    },
                )],
            ));
        }
        _ => {}
    }
}

fn get_action_str_npc() -> String {
    let mut rng = rand::rng();
    let words = vec![
        "chat up this rodent!",
        "kiss this rat! with language!",
        "rat talk?",
        "TALK",
        "TALK",
        "TALK",
        "rat chat",
        "TALK",
        "TALK",
        "TALK",
        "TALK",
        "TALK",
        "TALK",
        "TALK",
        "TALK",
        "t a l k  ? ?",
        "TALK",
        "TALK",
        "TALK",
        "TALK",
        "TALK",
        "TALK",
        "TALK",
        "TALK",
        "TALK",
        "TALK",
        "TALK",
        "TALK",
        "TALK",
        "TALK",
        "TALK",
        "TALK",
        "TaLK",
        "TLaK!",
        "TALK to POOPY rat",
    ];
    words.choose(&mut rng).unwrap().to_string()
}

fn get_action_str_hole() -> String {
    let mut rng = rand::rng();
    let words = vec![
        "you could go in this hole!",
        "HOLE",
        "HOLE?",
        "HOLE",
        "HOLE",
        "HOLE",
        "HOLE",
        "HOLE?",
        "HOLE",
        "HOLE",
        "HOLE",
        "HOLE",
        "HOLE?",
        "HOLE",
        "HOLE",
        "HOLE",
        "HOLE",
        "HOLE?",
        "HOLE",
        "HOLE",
        "HOLE",
        "HOLE",
        "HOLE?",
        "HOLE",
        "HOLE?",
        "hole....",
        "back in the hole dont get too excited",
    ];
    words.choose(&mut rng).unwrap().to_string()
}

fn reset_focus(mut focus: ResMut<PlayerFocus>) {
    focus.0 = None;
}

#[derive(Component)]
struct PlayerCamera;

#[derive(Resource, Clone, Debug)]
struct LevelStartLocation(Vec3);

fn spawn_camera(mut commands: Commands, level_start: Res<LevelStartLocation>) {
    commands.spawn((
        PlayerCamera,
        Camera3d::default(),
        Camera {
            order: 1,
            ..default()
        },
        Transform::from_xyz(level_start.0.x, level_start.0.y, level_start.0.z)
            .looking_at(Vec3::new(0., 1.414, 0.), Vec3::Y),
        RigidBody::Dynamic,
        Collider::cuboid(0.1, 0.5, 0.1),
        TransformInterpolation,
        CollidingEntities::default(),
        LockedAxes::ROTATION_LOCKED,
        LevelStuff,
        DistanceFog {
            color: Color::srgba(0.35, 0.48, 0.66, 1.0),
            directional_light_color: Color::srgba(1.0, 0.95, 0.85, 0.5),
            directional_light_exponent: 30.0,
            falloff: FogFalloff::from_visibility_colors(
                15.0, // distance in world units up to which objects retain visibility (>= 5% contrast)
                Color::srgb(0.35, 0.5, 0.66), // atmospheric extinction color (after light is lost due to absorption by atmospheric particles)
                Color::srgb(0.8, 0.844, 1.0), // atmospheric inscattering color (light gained due to scattering from the sun)
            ),
        },
    ));
}

const PLAYER_SPEED: f32 = 3.5;
const PLAYER_SPRINT_BOOST: f32 = 3.0;
const PLAYER_SLOWDOWN_MULT: f32 = 20.0;

fn player_camera_movement(
    mut query: Query<(&mut LinearVelocity, &Transform), With<PlayerCamera>>,
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
) {
    for (mut lin_vel, camera) in &mut query {
        // build movement vec from current inputs
        let mut movement_vel = Vec3::ZERO;
        if input.pressed(KeyCode::KeyW) {
            movement_vel += Vec3::NEG_Z
        }
        if input.pressed(KeyCode::KeyS) {
            movement_vel += Vec3::Z
        }
        if input.pressed(KeyCode::KeyA) {
            movement_vel += Vec3::NEG_X
        }
        if input.pressed(KeyCode::KeyD) {
            movement_vel += Vec3::X
        }
        if input.pressed(KeyCode::Space) || input.pressed(KeyCode::KeyE) {
            movement_vel += Vec3::Y
        }
        if input.pressed(KeyCode::ControlLeft) || input.pressed(KeyCode::KeyQ) {
            movement_vel += Vec3::NEG_Y
        }
        movement_vel = movement_vel.normalize_or_zero();
        movement_vel *= PLAYER_SPEED;
        if input.pressed(KeyCode::ShiftLeft) {
            movement_vel *= PLAYER_SPRINT_BOOST;
        }
        movement_vel = camera.rotation * movement_vel;

        // Add to current velocity
        lin_vel.0 += movement_vel.adjust_precision();

        let current_speed = lin_vel.length();
        if current_speed > 0.0 {
            // Apply friction
            lin_vel.0 = lin_vel.0 / current_speed
                * (current_speed
                    - current_speed * PLAYER_SLOWDOWN_MULT * time.delta_secs().adjust_precision())
                .max(0.0)
        }
    }
}

const MIN_Y: f32 = -5.;
fn debug_commands_and_oob_reset(
    mut player_tf_query: Query<&mut Transform, With<PlayerCamera>>,
    level_start: Res<LevelStartLocation>,
    input: Res<ButtonInput<KeyCode>>,
) {
    for mut player_tf in &mut player_tf_query {
        // reset player location to start transform
        // also do it if we're way oob )happens on wasm sometimes
        if input.pressed(KeyCode::KeyR) || player_tf.translation.y < MIN_Y {
            player_tf.translation = level_start.0.clone();
        }
    }
}

fn update_player_start_location(
    new_player_start: Query<(&PlayerStart, &Transform), Added<Transform>>,
    mut level_start: ResMut<LevelStartLocation>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (_new_start, start_transform) in &new_player_start {
        level_start.0 = start_transform.translation;
        // Also set state to loaded (is this the right place to do this lol?)
        next_state.set(GameState::InGame);
    }
}

fn update_camera_transform(
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    cursor_options: Single<&CursorOptions>,
    mut camera: Query<&mut Transform, With<PlayerCamera>>,
) {
    let Ok(mut transform) = camera.single_mut() else {
        return;
    };

    let delta = if cursor_options.grab_mode == CursorGrabMode::Locked {
        accumulated_mouse_motion.delta
    } else {
        Vec2::ZERO
    };
    let delta_yaw = -delta.x * 0.003;
    let delta_pitch = -delta.y * 0.003;

    let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);
    let yaw = yaw + delta_yaw;

    const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01;
    let pitch = (pitch + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);

    transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
}

fn capture_cursor(mut cursor: Single<&mut CursorOptions>) {
    cursor.visible = false;
    cursor.grab_mode = CursorGrabMode::Locked;
}

fn release_cursor(mut cursor: Single<&mut CursorOptions>) {
    cursor.visible = true;
    cursor.grab_mode = CursorGrabMode::None;
}

// Plugin that loads trenchbroom map
struct TrenchLoaderPlugin;
impl Plugin for TrenchLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_initial_map);
    }
}

const INITIAL_LEVEL: &'static str = "test.map";

fn spawn_initial_map(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(SceneRoot(
        asset_server.load(format!("maps/{INITIAL_LEVEL}#Scene")),
    ));
}
