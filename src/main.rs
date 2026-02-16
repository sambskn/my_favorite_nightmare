use crate::{
    fonts::SANS_FONT_PATH,
    sprites::{BillboardSpritePlugin, FocusType, PlayerFocus, PlayerStart},
    text_parse::parse_random_text,
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
use rand::{Rng, seq::IndexedRandom};

mod fonts;
mod sprites;
mod text_parse;
mod ui;

const GRAVITY_MULT: f32 = 16.8;

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
    level_start: Res<LevelStartLocation>,
    mut commands: Commands,
) {
    // Start level bg, looping
    commands.spawn((
        SamplePlayer::new(server.load(format!("sounds/{}.wav", level_start.bgm_name)))
            .with_volume(Volume::from_percent(level_start.bgm_vol))
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
        if get_xz_len(&vel) > WALKING_NOISE_MIN_VEL {
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
        app.insert_resource::<LevelStartLocation>(LevelStartLocation {
            spawn: DEFAULT_PLAYER_START_LOC.clone(),
            bgm_name: "bgm1".to_string(),
            bgm_vol: 50.,
            bg_color: Color::srgba(0.35, 0.48, 0.66, 1.0),
        })
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
                player_camera_movement.run_if(in_state(GameState::InGame)),
                update_player_start_location.run_if(in_state(GameState::Loading)),
                update_grounded.run_if(in_state(GameState::InGame)),
                debug_commands_and_oob_reset.run_if(in_state(GameState::InGame)),
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
    parse_random_text(
        "<TALK:80|TALK!!:10|TALK...|TaLK|rat chat?:5|talk to POOPY rat|chat up this rodent playa?|kiss this rat with language|RAT>",
    )
}

fn get_action_str_hole() -> String {
    parse_random_text(
        "<HOLE:80|HOLE?:10|HOLE?:5|back in the hole don't get too excited|you could go in this hole>",
    )
}

fn reset_focus(mut focus: ResMut<PlayerFocus>) {
    focus.0 = None;
}

#[derive(Component)]
struct Grounded;

#[derive(Component)]
struct PlayerCamera;

#[derive(Resource, Clone, Debug)]
struct LevelStartLocation {
    pub spawn: Vec3,
    pub bg_color: Color,
    pub bgm_name: String,
    pub bgm_vol: f32,
}

fn spawn_camera(mut commands: Commands, level_start: Res<LevelStartLocation>) {
    let player_collider = Collider::cuboid(0.5, 0.5, 0.5);
    let mut caster_shape = player_collider.clone();
    caster_shape.set_scale(Vector::ONE * 0.99, 10);

    commands.spawn((
        PlayerCamera,
        Camera3d::default(),
        Camera {
            order: 1,
            ..default()
        },
        Transform::from_xyz(
            level_start.spawn.x,
            level_start.spawn.y,
            level_start.spawn.z,
        )
        .looking_at(Vec3::new(0., level_start.spawn.y, 0.), Vec3::Y),
        RigidBody::Dynamic,
        ShapeCaster::new(
            caster_shape,
            Vector::ZERO,
            Quaternion::default(),
            Dir3::NEG_Y,
        )
        .with_max_distance(0.2),
        player_collider,
        TransformInterpolation,
        CollidingEntities::default(),
        LockedAxes::ROTATION_LOCKED,
        LevelStuff,
        DistanceFog {
            color: level_start.bg_color,
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

const MAX_SLOPE_ANGLE: f32 = 45.;

fn update_grounded(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut query: Query<(Entity, &ShapeHits, &Rotation, Has<Grounded>), With<PlayerCamera>>,
) {
    for (entity, hits, rotation, already_grounded) in &mut query {
        let is_grounded = hits
            .iter()
            .any(|hit| (rotation * -hit.normal2).angle_between(Vector::Y).abs() <= MAX_SLOPE_ANGLE);
        if is_grounded {
            commands.entity(entity).insert(Grounded);
            if !already_grounded {
                commands.spawn((
                    SamplePlayer::new(server.load(get_random_oof_sound_path())),
                    bevy_seedling::sample::PlaybackSettings {
                        speed: get_scalar_boosted_rand_sfx_speed(1.5),
                        ..default()
                    },
                ));
            }
        } else {
            commands.entity(entity).remove::<Grounded>();
        }
    }
}

fn get_random_oof_sound_path() -> String {
    let mut rng = rand::rng();
    let noises = vec![
        "sounds/huh1.wav",
        "sounds/huh2.wav",
        "sounds/huh3.wav",
        "sounds/huh4.wav",
        "sounds/huh5.wav",
    ];
    noises.choose(&mut rng).unwrap().to_string()
}

const PLAYER_SPEED: f32 = 3.5;
const PLAYER_JUMP_SPEED: f32 = 4.0;
const PLAYER_SPRINT_BOOST: f32 = 1.5;
const PLAYER_SLOWDOWN_MULT: f32 = 20.0;
const PLAYER_IN_AIR_SLOWDOWN_MULT: f32 = 15.;

fn player_camera_movement(
    mut query: Query<(&mut LinearVelocity, &Transform, Has<Grounded>), With<PlayerCamera>>,
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    server: Res<AssetServer>,
) {
    for (mut lin_vel, camera, is_grounded) in &mut query {
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
        movement_vel = movement_vel.normalize_or_zero();
        movement_vel *= PLAYER_SPEED;
        if input.pressed(KeyCode::ShiftLeft) {
            movement_vel *= PLAYER_SPRINT_BOOST;
        }
        movement_vel = camera.rotation * movement_vel;

        // Add to current velocity
        lin_vel.0.x += movement_vel.adjust_precision().x;
        lin_vel.0.z += movement_vel.adjust_precision().z;

        let current_speed = get_xz_len(&lin_vel);
        if current_speed > 0.0 {
            // Apply friction, though only in X and Z dirs
            let mult = if is_grounded {
                PLAYER_SLOWDOWN_MULT
            } else {
                PLAYER_IN_AIR_SLOWDOWN_MULT
            } * time.delta_secs().adjust_precision();
            lin_vel.0.x =
                lin_vel.0.x / current_speed * (current_speed - current_speed * mult).max(0.0);
            lin_vel.0.z =
                lin_vel.0.z / current_speed * (current_speed - current_speed * mult).max(0.0);
        }

        // handle vert component
        // only jump if on the ground
        if input.just_pressed(KeyCode::Space) && is_grounded {
            commands.spawn((
                SamplePlayer::new(server.load("sounds/boing.wav")),
                bevy_seedling::sample::PlaybackSettings {
                    speed: get_scalar_boosted_rand_sfx_speed(current_speed),
                    ..default()
                },
            ));
            lin_vel.0 += Vec3::Y * PLAYER_JUMP_SPEED;
            // Limit jump speed
            lin_vel.0.y = lin_vel.0.y.min(PLAYER_JUMP_SPEED);
        }
    }
}

pub fn get_scalar_boosted_rand_sfx_speed(scalar: f32) -> f64 {
    let mut rng = rand::rng();
    let adjusted_scalar = scalar * 0.15;
    let low_bound = 1.00 + adjusted_scalar / 3.0;
    let high_bound = adjusted_scalar.max(1.125);
    (if low_bound == high_bound {
        low_bound
    } else if low_bound < high_bound {
        rng.random_range(low_bound..high_bound)
    } else {
        rng.random_range(high_bound..low_bound)
    }) as f64
}

fn get_xz_len(input: &Vec3) -> f32 {
    (input.x * input.x + input.z * input.z).sqrt()
}

const MIN_Y: f32 = 0.0;
fn debug_commands_and_oob_reset(
    mut player_tf_query: Query<(&mut Transform, &mut LinearVelocity), With<PlayerCamera>>,
    level_start: Res<LevelStartLocation>,
    input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    server: Res<AssetServer>,
) {
    for (mut player_tf, mut lin_vel) in &mut player_tf_query {
        // reset player location to start transform
        // also do it if we're way oob )happens on wasm sometimes
        if input.pressed(KeyCode::KeyR) || player_tf.translation.y < MIN_Y {
            commands.spawn(SamplePlayer::new(server.load(get_random_dead_sound_path())));
            player_tf.translation = level_start.spawn.clone();
            // also set the velocity to 0 so we don't clip through stuff on respawn
            lin_vel.0 = Vec3::ZERO;
        }
    }
}

fn get_random_dead_sound_path() -> String {
    let mut rng = rand::rng();
    let noises = vec![
        "sounds/dead1.wav",
        "sounds/dead2.wav",
        "sounds/dead3.wav",
        "sounds/dead4.wav",
        "sounds/dead5.wav",
        "sounds/dead6.wav",
    ];
    noises.choose(&mut rng).unwrap().to_string()
}

fn update_player_start_location(
    new_player_start: Query<(&PlayerStart, &Transform), Added<Transform>>,
    mut level_start: ResMut<LevelStartLocation>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (new_start, start_transform) in &new_player_start {
        level_start.spawn = start_transform.translation;
        level_start.bgm_name = new_start.bgm_name.clone();
        level_start.bgm_vol = new_start.bgm_vol as f32;
        level_start.bg_color = new_start.level_atmosphere_color;

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

const INITIAL_LEVEL: &'static str = "start.map";

fn spawn_initial_map(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(SceneRoot(
        asset_server.load(format!("maps/{INITIAL_LEVEL}#Scene")),
    ));
}
