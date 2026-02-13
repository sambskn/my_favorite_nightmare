use crate::{
    fonts::{SANS_FONT_PATH, SERIF_FONT_PATH},
    ui::{GameState, MenuPlugin, TEXT_COLOR},
};
use avian3d::{math::*, prelude::*};
use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    input::{common_conditions::input_just_pressed, mouse::AccumulatedMouseMotion},
    prelude::*,
    scene::SceneInstance,
    window::{CursorGrabMode, CursorOptions},
};
use bevy_seedling::prelude::*;
use bevy_trenchbroom::prelude::*;
use bevy_trenchbroom_avian::AvianPhysicsBackend;
use rand::seq::IndexedRandom;

mod fonts;
mod ui;

// not visible to player in game, used for marking player start loc in level
#[point_class(
    model({ path: "sprites/start.png", scale: 0.5 }),
)]
struct PlayerStart;

// point_class marks it for bevy_trenchbroom
// - adding a model path is for display in trenchbroom, not pulled for bevy side atm
// component is the bevy macro used to set up the hook for spawning our billboarded sprite
// (calling the on_add fn below)
#[point_class(
    model({ path: "sprites/rat.png", scale: 0.5 }),
)]
#[component(on_add = Self::on_add)]
struct NPCSprite {
    pub selectable: bool,
    pub text: Option<String>,
}
impl Default for NPCSprite {
    fn default() -> Self {
        NPCSprite {
            selectable: true,
            text: None,
        }
    }
}

#[derive(Component, Clone)]
struct FocusDetails {
    pub focus_type: FocusType,
    pub selectable: bool,
    pub text: Option<String>,
}

#[derive(Clone, Copy)]
enum FocusType {
    NPC,
    Hole,
}

enum Selection {
    On,
    Off,
}

impl NPCSprite {
    pub fn on_add(mut world: DeferredWorld, ctx: HookContext) {
        let Some(asset_server) = world.get_resource::<AssetServer>() else {
            return;
        };
        // Get the selectable value from the NPCSprite
        // (not spawning out bundles as children of the NPCSprite,
        // so we need to get any values we need now, put em in SpriteDetails)
        let npc_sprite = world.get::<NPCSprite>(ctx.entity).unwrap();
        let selectable = npc_sprite.selectable;
        let text = if !selectable {
            None
        } else {
            Some(
                npc_sprite
                    .text
                    .clone()
                    .unwrap_or("THERE SHOULD BE REAL TEXT HERE LOL".to_string()),
            )
        };

        let rect_mesh = asset_server.add(Mesh::from(Rectangle::new(0.42, 0.42)));
        let material = asset_server.add(StandardMaterial {
            base_color_texture: Some(asset_server.load("sprites/rat.png")),
            emissive: Color::WHITE.into(),
            emissive_texture: Some(asset_server.load("sprites/rat.png")),
            perceptual_roughness: 1.0,
            alpha_mode: AlphaMode::Mask(1.0),
            cull_mode: None,
            ..default()
        });
        let hover_material = asset_server.add(StandardMaterial {
            base_color_texture: Some(asset_server.load("sprites/rat2.png")),
            emissive: Color::WHITE.into(),
            emissive_texture: Some(asset_server.load("sprites/rat2.png")),
            perceptual_roughness: 1.0,
            alpha_mode: AlphaMode::Mask(1.0),
            cull_mode: None,
            ..default()
        });
        world
            .commands()
            .entity(ctx.entity)
            .insert((
                Mesh3d(rect_mesh),
                MeshMaterial3d(material.clone()),
                PhysicsPickable,
                RigidBody::Static,
                Sensor,
                Collider::from(Cuboid::default()),
                FocusDetails {
                    selectable,
                    text,
                    focus_type: FocusType::NPC,
                },
                LevelStuff,
            ))
            .observe(update_material_on::<Pointer<Over>>(
                hover_material.clone(),
                Selection::On,
            ))
            .observe(update_material_on::<Pointer<Out>>(
                material.clone(),
                Selection::Off,
            ));
    }
}

// point_class marks it for bevy_trenchbroom
// - adding a model path is for display in trenchbroom, not pulled for bevy side atm
// component is the bevy macro used to set up the hook for spawning our billboarded sprite
// (calling the on_add fn below)
#[point_class(
    model({ path: "sprites/hole.png", scale: 0.5 }),
)]
#[component(on_add = Self::on_add)]
struct HoleSprite {
    pub hole_target: String,
}
impl Default for HoleSprite {
    fn default() -> Self {
        HoleSprite {
            hole_target: String::new(),
        }
    }
}

impl HoleSprite {
    pub fn on_add(mut world: DeferredWorld, ctx: HookContext) {
        let Some(asset_server) = world.get_resource::<AssetServer>() else {
            return;
        };

        let hole_sprite = world.get::<HoleSprite>(ctx.entity).unwrap();
        let hole_target = hole_sprite.hole_target.clone();

        let rect_mesh = asset_server.add(Mesh::from(Rectangle::new(0.42, 0.42)));
        let material = asset_server.add(StandardMaterial {
            base_color_texture: Some(asset_server.load("sprites/hole.png")),
            perceptual_roughness: 1.0,
            alpha_mode: AlphaMode::Mask(1.0),
            cull_mode: None,
            ..default()
        });
        let hover_material = asset_server.add(StandardMaterial {
            base_color_texture: Some(asset_server.load("sprites/hole.png")),
            emissive_texture: Some(asset_server.load("sprites/hole_emissive.png")),
            perceptual_roughness: 1.0,
            alpha_mode: AlphaMode::Mask(1.0),
            cull_mode: None,
            ..default()
        });
        world
            .commands()
            .entity(ctx.entity)
            .insert((
                Mesh3d(rect_mesh),
                MeshMaterial3d(material.clone()),
                PhysicsPickable,
                RigidBody::Static,
                Sensor,
                Collider::from(Cuboid::default()),
                FocusDetails {
                    selectable: true,
                    text: Some(hole_target),
                    focus_type: FocusType::Hole,
                },
                LevelStuff,
            ))
            .observe(update_material_on::<Pointer<Over>>(
                hover_material.clone(),
                Selection::On,
            ))
            .observe(update_material_on::<Pointer<Out>>(
                material.clone(),
                Selection::Off,
            ));
    }
}

const GRAVITY_MULT: f32 = 160.0;

// marker component for stuff to destroy between levels
#[derive(Component)]
struct LevelStuff;

fn main() {
    let mut app = App::new();
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
        MenuPlugin,
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

const WALKING_NOISE_MIN_VEL: f32 = 9.5;

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

        // Print info message with control info to console
        info!("\n\n\nControls:\n\tWASD => move, Mouse => look, Space => 'jump'\n\n");
    }
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
        // Log current player transform
        if input.pressed(KeyCode::KeyL) {
            info!("Player Loc: {:?}", player_tf);
        }
    }
}

fn update_player_start_location(
    new_player_start: Query<(&PlayerStart, &Transform), Added<Transform>>,
    mut level_start: ResMut<LevelStartLocation>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (_new_start, start_transform) in &new_player_start {
        info!("Saving level start loc: {:?}", start_transform);
        level_start.0 = start_transform.translation;
        // Also set state to loaded (is this the right place to do this lol?)
        next_state.set(GameState::InGame);
    }
}

fn update_camera_transform(
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    mut camera: Query<&mut Transform, With<PlayerCamera>>,
) {
    let Ok(mut transform) = camera.single_mut() else {
        return;
    };

    let delta = accumulated_mouse_motion.delta;
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
        app.add_systems(Startup, spawn_test_map);
    }
}

fn spawn_test_map(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(SceneRoot(asset_server.load("maps/test.map#Scene")));
}

// Plugin for keeping billboard sprites facing the camera
struct BillboardSpritePlugin;
impl Plugin for BillboardSpritePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource::<PlayerFocus>(PlayerFocus(None))
            .add_systems(
                Update,
                (
                    update_billboards::<NPCSprite>.run_if(in_state(GameState::InGame)),
                    update_billboards::<HoleSprite>.run_if(in_state(GameState::InGame)),
                    handle_focus_click
                        .run_if(in_state(GameState::InGame))
                        .run_if(input_just_pressed(MouseButton::Left)),
                ),
            );
    }
}

const SPRITE_ROTATE_THRESHOLD: f32 = 0.0001;

fn update_billboards<C: Component>(
    camera_query: Query<&Transform, (With<Camera3d>, Without<C>)>,
    mut sprite_query: Query<&mut Transform, (With<C>, Without<Camera3d>)>,
) {
    let Ok(cam_tf) = camera_query.single() else {
        return;
    };
    for mut sprite_tf in &mut sprite_query {
        // check diff between current sprite rotation and target
        let current_sprite_rotation = sprite_tf.rotation.clone();
        let mut target_tf = sprite_tf.clone();
        target_tf.look_at(cam_tf.translation, Vec3::Y);
        let target_tf_rotation = target_tf.rotation;
        let diff = target_tf_rotation.angle_between(current_sprite_rotation);
        if diff > SPRITE_ROTATE_THRESHOLD {
            sprite_tf.rotation = target_tf.rotation;
        }
    }
}

#[derive(Resource)]
struct PlayerFocus(Option<FocusDetails>);

fn update_material_on<E: EntityEvent>(
    new_material: Handle<StandardMaterial>,
    selection_mode: Selection,
) -> impl Fn(On<E>, Query<(&mut MeshMaterial3d<StandardMaterial>, &FocusDetails)>, ResMut<PlayerFocus>)
{
    move |trigger, mut query, mut highlighted| {
        if let Ok((mut material, sprite_deets)) = query.get_mut(trigger.event_target()) {
            // only swap material if sprite is selectable
            if sprite_deets.selectable {
                material.0 = new_material.clone();
                match selection_mode {
                    Selection::Off => highlighted.0 = None,
                    Selection::On => highlighted.0 = Some(sprite_deets.clone()),
                }
            }
        }
    }
}

fn handle_focus_click(
    highlighted: Res<PlayerFocus>,
    text_box_query: Query<Entity, With<TextBox>>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    scene: Single<Entity, With<SceneInstance>>,
    level_stuff: Query<Entity, With<LevelStuff>>,
    server: Res<AssetServer>,
) {
    // if textbox exists, make it go away (dirty)
    for text_box_ent in &text_box_query {
        commands.entity(text_box_ent).despawn();
        // return early
        return;
    }

    if let Some(sprite_deets) = &highlighted.0 {
        match sprite_deets.focus_type {
            FocusType::Hole => {
                // Load new level
                if let Some(next_level) = &sprite_deets.text {
                    if next_level.len() > 0 {
                        info!("Going into this hole {:?}", next_level);
                        // despawn old level
                        for stuff_ent in &level_stuff {
                            commands.entity(stuff_ent).despawn();
                        }
                        commands.entity(*scene).despawn();
                        // set game state to loading
                        next_state.set(GameState::Loading);
                        // kick off load of new level
                        let new_level_asset = format!("maps/{next_level}#Scene");
                        commands.spawn(SceneRoot(server.load(new_level_asset)));
                    }
                }
            }
            FocusType::NPC => {
                if let Some(sprite_text) = &sprite_deets.text {
                    commands.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            bottom: vh(10),
                            left: vw(15),
                            right: vw(15),
                            padding: UiRect::all(px(20)),
                            ..default()
                        },
                        TextBox,
                        BackgroundColor {
                            0: Color::Oklcha(Oklcha::new(0.1788, 0.0099, 288.85, 1.0)),
                        },
                        children![(
                            Text::new(sprite_text),
                            TextColor(TEXT_COLOR),
                            TextFont {
                                font: server.load(SERIF_FONT_PATH),
                                font_size: 18.0,
                                ..default()
                            },
                        )],
                    ));
                }
            }
        }
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
        "aaaaaah rat",
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
        "KALT?",
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
