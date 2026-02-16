use crate::{
    LevelStuff, PlayerCamera, TextBox,
    fonts::SERIF_FONT_PATH,
    get_scalar_boosted_rand_sfx_speed,
    text_parse::parse_random_text,
    ui::{GameState, TEXT_COLOR},
};
use avian3d::prelude::*;
use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    input::common_conditions::input_just_pressed,
    prelude::*,
    scene::SceneInstance,
};
use bevy_seedling::prelude::*;
use bevy_trenchbroom::prelude::*;

// not visible to player in game, used for marking player start loc in level
#[point_class(
    model({ path: "sprites/start.png", scale: 0.5 }),
)]
pub struct PlayerStart;

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
    pub name: String,
    pub voice_line: String,
}
impl Default for NPCSprite {
    fn default() -> Self {
        NPCSprite {
            selectable: true,
            text: None,
            name: "".to_string(),
            voice_line: "voice1_whiny".to_string(),
        }
    }
}

#[derive(Component, Clone, PartialEq)]
pub struct FocusDetails {
    pub name: String,
    pub focus_type: FocusType,
    pub selectable: bool,
    pub text: Option<String>,
    pub sound_on_action: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FocusType {
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
        let name = npc_sprite.name.clone();
        let voice_line = npc_sprite.voice_line.clone();
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
                    name,
                    selectable,
                    text,
                    sound_on_action: Some(voice_line),
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
            emissive_texture: Some(asset_server.load("sprites/hole.png")),
            emissive: Color::WHITE.into(),
            perceptual_roughness: 1.0,
            alpha_mode: AlphaMode::Mask(1.0),
            cull_mode: None,
            ..default()
        });
        let hover_material = asset_server.add(StandardMaterial {
            base_color_texture: Some(asset_server.load("sprites/hole.png")),
            emissive_texture: Some(asset_server.load("sprites/hole.png")),
            emissive: Color::WHITE.into(),
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
                    name: "hole".to_string(),
                    selectable: true,
                    text: Some(hole_target),
                    sound_on_action: Some("warp".to_string()),
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

#[point_class(
    model({ path: "sprites/plant.png", scale: 2. }),
)]
#[component(on_add = Self::on_add)]
struct PlantSprite {
    pub name: String,
}
impl Default for PlantSprite {
    fn default() -> Self {
        PlantSprite {
            name: String::new(),
        }
    }
}

impl PlantSprite {
    pub fn on_add(mut world: DeferredWorld, ctx: HookContext) {
        let Some(asset_server) = world.get_resource::<AssetServer>() else {
            return;
        };

        let plant_sprite = world.get::<PlantSprite>(ctx.entity).unwrap();
        let plant_name = plant_sprite.name.clone();

        let rect_mesh = asset_server.add(Mesh::from(Rectangle::new(1.414, 1.414)));
        let material = asset_server.add(StandardMaterial {
            base_color_texture: Some(asset_server.load("sprites/plant.png")),
            emissive_texture: Some(asset_server.load("sprites/plant.png")),
            emissive: Color::WHITE.into(),
            perceptual_roughness: 1.0,
            alpha_mode: AlphaMode::Mask(1.0),
            cull_mode: None,
            ..default()
        });
        world.commands().entity(ctx.entity).insert((
            Mesh3d(rect_mesh),
            MeshMaterial3d(material),
            RigidBody::Static,
            Sensor,
            Collider::from(Cuboid::default()),
            FocusDetails {
                name: plant_name,
                selectable: false,
                text: None,
                sound_on_action: None,
                focus_type: FocusType::Hole,
            },
            LevelStuff,
        ));
    }
}

#[point_class(
    model({ path: "sprites/face.png", scale: 1. }),
)]
#[component(on_add = Self::on_add)]
struct FaceSprite {
    pub name: String,
}
impl Default for FaceSprite {
    fn default() -> Self {
        FaceSprite {
            name: String::new(),
        }
    }
}

impl FaceSprite {
    pub fn on_add(mut world: DeferredWorld, ctx: HookContext) {
        let Some(asset_server) = world.get_resource::<AssetServer>() else {
            return;
        };

        let face_sprite = world.get::<FaceSprite>(ctx.entity).unwrap();
        let face_name = face_sprite.name.clone();

        let rect_mesh = asset_server.add(Mesh::from(Rectangle::new(1.414, 1.414)));
        let material = asset_server.add(StandardMaterial {
            base_color_texture: Some(asset_server.load("sprites/face.png")),
            emissive_texture: Some(asset_server.load("sprites/face.png")),
            emissive: Color::WHITE.into(),
            perceptual_roughness: 1.0,
            alpha_mode: AlphaMode::Mask(1.0),
            cull_mode: None,
            ..default()
        });
        world.commands().entity(ctx.entity).insert((
            Mesh3d(rect_mesh),
            MeshMaterial3d(material),
            RigidBody::Static,
            Sensor,
            Collider::from(Cuboid::default()),
            FocusDetails {
                name: face_name,
                selectable: false,
                text: None,
                sound_on_action: None,
                focus_type: FocusType::Hole,
            },
            LevelStuff,
        ));
    }
}

#[derive(Component)]
struct SpeedCoin {
    respawn_timer: Option<Timer>,
    respawn_duration: f32,
}

const DEFAULT_COIN_RESPAWN_S: f32 = 1.0;

impl Default for SpeedCoin {
    fn default() -> Self {
        SpeedCoin {
            respawn_timer: None,
            respawn_duration: DEFAULT_COIN_RESPAWN_S,
        }
    }
}

#[point_class(
    model({ path: "sprites/coin.png", scale: .2 }),
)]
#[component(on_add = Self::on_add)]
struct CoinSprite;

impl CoinSprite {
    pub fn on_add(mut world: DeferredWorld, ctx: HookContext) {
        let Some(asset_server) = world.get_resource::<AssetServer>() else {
            return;
        };

        let rect_mesh = asset_server.add(Mesh::from(Rectangle::new(0.1, 0.1)));
        let material = asset_server.add(StandardMaterial {
            base_color_texture: Some(asset_server.load("sprites/coin.png")),
            emissive_texture: Some(asset_server.load("sprites/coin.png")),
            emissive: Color::WHITE.into(),
            perceptual_roughness: 1.0,
            alpha_mode: AlphaMode::Mask(1.0),
            cull_mode: None,
            ..default()
        });
        world.commands().entity(ctx.entity).insert((
            Mesh3d(rect_mesh),
            MeshMaterial3d(material),
            RigidBody::Static,
            Sensor,
            Collider::from(Cuboid::default()),
            FocusDetails {
                name: "coin".to_string(),
                selectable: false,
                text: None,
                sound_on_action: None,
                focus_type: FocusType::Hole,
            },
            // Enable collision events for this entity.
            CollisionEventsEnabled,
            // Read entities colliding with this entity.
            CollidingEntities::default(),
            LevelStuff,
            SpeedCoin::default(),
        ));
    }
}

#[solid_class]
pub struct CoolSolid;

#[derive(Resource)]
pub struct PlayerFocus(pub Option<FocusDetails>);

const MAX_DIST_FOR_FOCUS: f32 = 2.0;

fn update_material_on<E: EntityEvent>(
    new_material: Handle<StandardMaterial>,
    selection_mode: Selection,
) -> impl Fn(
    On<E>,
    Query<(
        &mut MeshMaterial3d<StandardMaterial>,
        &FocusDetails,
        &Transform,
    )>,
    ResMut<PlayerFocus>,
    Single<&Transform, With<PlayerCamera>>,
) {
    move |trigger, mut query, mut highlighted, player_tf| {
        if let Ok((mut material, sprite_deets, sprite_tf)) = query.get_mut(trigger.event_target()) {
            // Only make sprite 'focused' if it's selectable and close enough
            let dist = (sprite_tf.translation - player_tf.translation).length();
            if sprite_deets.selectable {
                material.0 = new_material.clone();
                match selection_mode {
                    Selection::Off => highlighted.0 = None,
                    Selection::On => {
                        // only actually select if they're close enough
                        if dist <= MAX_DIST_FOR_FOCUS {
                            if let Some(existing) = highlighted.0.clone() {
                                if existing.name == sprite_deets.name {
                                    return;
                                }
                            }
                            highlighted.0 = Some(sprite_deets.clone());
                        }
                    }
                }
            }
        }
    }
}

#[derive(Component)]
struct HoleSFX;

#[derive(Component)]
struct RatVoice;

fn handle_focus_click(
    highlighted: Res<PlayerFocus>,
    text_box_query: Query<Entity, With<TextBox>>,
    voice_query: Query<Entity, With<RatVoice>>,
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

    // similar thing for rat voice
    for voice_line_ent in &voice_query {
        commands.entity(voice_line_ent).despawn();
        // return early
        return;
    }

    if let Some(sprite_deets) = &highlighted.0 {
        // play noise if we got one
        if let Some(sound_name) = sprite_deets.sound_on_action.clone() {
            let sound_path = format!("sounds/{sound_name}.wav");
            match sprite_deets.focus_type {
                FocusType::Hole => {
                    commands.spawn((SamplePlayer::new(server.load(sound_path)), HoleSFX));
                }
                FocusType::NPC => {
                    commands.spawn((SamplePlayer::new(server.load(sound_path)), RatVoice));
                }
            };
        }

        match sprite_deets.focus_type {
            FocusType::Hole => {
                // Load new level
                if let Some(next_level) = &sprite_deets.text {
                    if next_level.len() > 0 {
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
                            Text::new(parse_random_text(sprite_text)),
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

// Plugin for keeping billboard sprites facing the camera
pub struct BillboardSpritePlugin;
impl Plugin for BillboardSpritePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource::<PlayerFocus>(PlayerFocus(None))
            .add_systems(
                Update,
                (
                    update_billboards::<NPCSprite>.run_if(in_state(GameState::InGame)),
                    update_billboards::<HoleSprite>.run_if(in_state(GameState::InGame)),
                    update_billboards::<PlantSprite>.run_if(in_state(GameState::InGame)),
                    update_billboards::<CoinSprite>.run_if(in_state(GameState::InGame)),
                    update_billboards::<FaceSprite>.run_if(in_state(GameState::InGame)),
                    check_for_coin_intersections.run_if(in_state(GameState::InGame)),
                    update_coin_respawn.run_if(in_state(GameState::InGame)),
                    // This last one should be last in the chain because it can despawn levels
                    handle_focus_click
                        .run_if(in_state(GameState::InGame))
                        .run_if(input_just_pressed(MouseButton::Left)),
                ),
            );
    }
}

const COIN_BOOST: f32 = 100.0;

fn check_for_coin_intersections(
    mut coin_query: Query<(&mut SpeedCoin, &CollidingEntities, &mut Visibility)>,
    mut player: Single<(&Transform, &mut LinearVelocity), With<PlayerCamera>>,
    mut commands: Commands,
    server: Res<AssetServer>,
) {
    for (mut speed_coin, colldiing, mut visibility) in &mut coin_query {
        if speed_coin.respawn_timer.is_none() && !colldiing.0.is_empty() {
            // hide coin
            *visibility = Visibility::Hidden;
            // start respawn timer
            speed_coin.respawn_timer = Some(Timer::from_seconds(
                speed_coin.respawn_duration,
                TimerMode::Once,
            ));
            // play noise
            commands.spawn((
                SamplePlayer::new(server.load("sounds/boost.wav")),
                bevy_seedling::sample::PlaybackSettings {
                    speed: get_scalar_boosted_rand_sfx_speed(1.5) * 2.0,
                    ..default()
                },
            ));
            // make player go!
            let mut boost_dir = player.0.local_z().as_vec3().normalize_or_zero() * -1.;
            boost_dir *= COIN_BOOST;
            player.1.0 += boost_dir;
            player.1.0 = player.1.0.clamp_length_max(COIN_BOOST);
        }
    }
}

fn update_coin_respawn(mut coin_query: Query<(&mut SpeedCoin, &mut Visibility)>, time: Res<Time>) {
    for (mut speed_coin, mut visibility) in &mut coin_query {
        if let Some(ref mut timer) = speed_coin.respawn_timer {
            if timer.tick(time.delta()).is_finished() {
                // Un-hide the coin
                *visibility = Visibility::Visible;
                speed_coin.respawn_timer = None;
            }
        }
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
        let target_tf = cam_tf.clone();
        // target_tf.look_at(cam_tf.translation, Vec3::Y);
        let target_tf_rotation = target_tf.rotation;
        let diff = target_tf_rotation.angle_between(current_sprite_rotation);
        if diff > SPRITE_ROTATE_THRESHOLD {
            sprite_tf.rotation = target_tf.rotation;
        }
    }
}
