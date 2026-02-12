use bevy::{
    input_focus::{
        InputDispatchPlugin,
        tab_navigation::{TabIndex, TabNavigationPlugin},
    },
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{
        CoreSliderDragState, Slider, SliderRange, SliderThumb, SliderValue, TrackClick,
        UiWidgetsPlugins, observe, slider_self_update,
    },
};
use bevy_persistent::prelude::*;
use bevy_seedling::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

const SLIDER_TRACK: Color = Color::oklcha(0.5912, 0.1184, 318.87, 0.8);
const SLIDER_THUMB: Color = Color::oklcha(0.6088, 0.2417, 356.26, 0.92);

#[derive(Component)]
struct UISlider;

#[derive(Component)]
struct UISliderThumb;

#[derive(Component)]
struct ValueLabel(Entity);

// Settings resource to persist
#[derive(Default, Resource, Serialize, Deserialize, Clone)]
struct GameSettings {
    sound_volume: f32,
}

pub struct MenuPlugin;
impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        // Setup persistent settings
        let settings_dir = dirs::config_dir()
            .map(|native_config_dir| native_config_dir.join(env!("CARGO_PKG_NAME")))
            .unwrap_or(Path::new("local").join("config"));
        app.insert_state(MenuState::InGame)
            .insert_resource(
                Persistent::<GameSettings>::builder()
                    .name("game settings")
                    .format(StorageFormat::Toml)
                    .path(settings_dir.join("settings.toml"))
                    .default(GameSettings { sound_volume: 50.0 })
                    .build()
                    .expect("failed to initialize game settings"),
            )
            .add_plugins((UiWidgetsPlugins, InputDispatchPlugin, TabNavigationPlugin))
            .add_systems(Startup, load_initial_settings)
            .add_systems(
                Update,
                (
                    toggle_menu,
                    update_slider_visuals,
                    update_value_labels,
                    update_volume,
                    save_settings_on_change,
                ),
            )
            .add_systems(OnEnter(MenuState::Menu), spawn_menu)
            .add_systems(OnEnter(MenuState::InGame), kill_menu);
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum MenuState {
    #[default]
    Menu,
    InGame,
}

fn toggle_menu(
    input: Res<ButtonInput<KeyCode>>,
    state: Res<State<MenuState>>,
    mut next_state: ResMut<NextState<MenuState>>,
) {
    if input.just_released(KeyCode::Escape) {
        *next_state = match state.get() {
            MenuState::InGame => NextState::Pending(MenuState::Menu),
            MenuState::Menu => NextState::Pending(MenuState::InGame),
        }
    }
}

pub const TEXT_COLOR: Color = Color::Oklcha(Oklcha::new(0.8994, 0.0715, 331.2, 0.98));

#[derive(Component)]
struct Menu;

fn spawn_menu(
    mut commands: Commands,
    sound_settings: Single<&VolumeNode, With<SoundEffectsBus>>,
    server: Res<AssetServer>,
) {
    let current_sound_vol = sound_settings.volume.percent();

    // Spawn the menu ui elements
    commands.spawn((
        Camera2d,
        Camera {
            order: 3,
            ..default()
        },
        IsDefaultUiCamera,
        Menu,
    ));

    commands.spawn(menu()).with_children(|parent| {
        parent.spawn((
            Node {
                padding: UiRect::all(px(30)),
                ..default()
            },
            children![(
                Text::new("MENU"),
                TextColor(TEXT_COLOR),
                TextFont {
                    font: server.load("fonts/OTNeueMontreal-BoldItalicSqueezed.ttf"),
                    font_size: 36.0,
                    ..default()
                },
            )],
        ));
        parent.spawn((
            Node {
                padding: UiRect::axes(px(40), px(10)),
                ..default()
            },
            children![(
                Text::new("sound"),
                TextColor(TEXT_COLOR),
                TextFont {
                    font: server.load("fonts/OTNeueMontreal-BoldItalicSqueezed.ttf"),
                    font_size: 32.0,
                    ..default()
                },
            )],
        ));
        parent
            .spawn((Node {
                padding: UiRect::axes(px(50), px(10)),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                width: percent(100),
                ..default()
            },))
            .with_children(|subparent| {
                subparent.spawn(((
                    Text::new("volume"),
                    TextColor(TEXT_COLOR),
                    TextFont {
                        font: server.load("fonts/OTNeueMontreal-BoldItalicSqueezed.ttf"),
                        font_size: 20.0,
                        ..default()
                    },
                ),));

                let volume_label = subparent
                    .spawn(((
                        Text::new(format!("{current_sound_vol}%")),
                        TextColor(TEXT_COLOR),
                        TextFont {
                            font: server.load("fonts/OTNeueMontreal-BoldItalicSqueezed.ttf"),
                            font_size: 28.0,
                            ..default()
                        },
                    ),))
                    .id();

                subparent.spawn((
                    horizontal_slider(current_sound_vol),
                    ValueLabel(volume_label),
                    observe(slider_self_update),
                ));
            });
    });
}

fn kill_menu(menu_entity: Query<Entity, With<Menu>>, mut commands: Commands) {
    for ent in menu_entity {
        let mut menu_ent = commands.entity(ent);
        menu_ent.despawn();
    }
}

fn menu() -> impl Bundle {
    (
        Menu,
        Node {
            position_type: PositionType::Absolute,
            padding: UiRect::axes(px(10), px(10)),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Baseline,
            min_width: vw(50),
            min_height: vh(80),
            top: px(40),
            left: vw(10),
            ..default()
        },
        BackgroundColor {
            0: Color::Oklcha(Oklcha::new(0.1788, 0.0099, 288.85, 1.0)),
        },
        children![],
    )
}

fn horizontal_slider(initial_val: f32) -> impl Bundle {
    (
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Stretch,
            column_gap: px(4),
            height: px(12),
            width: px(200),
            ..default()
        },
        UISlider,
        Hovered::default(),
        Slider {
            track_click: TrackClick::Snap,
        },
        SliderValue(initial_val),
        SliderRange::new(0.0, 100.0),
        TabIndex(0),
        Children::spawn((
            Spawn((
                Node {
                    height: px(6),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(SLIDER_TRACK),
            )),
            Spawn((
                Node {
                    display: Display::Flex,
                    position_type: PositionType::Absolute,
                    left: px(0),
                    right: px(12),
                    top: px(0),
                    bottom: px(0),
                    ..default()
                },
                children![(
                    UISliderThumb,
                    SliderThumb,
                    Node {
                        display: Display::Flex,
                        width: px(12),
                        height: px(12),
                        position_type: PositionType::Absolute,
                        left: percent(0),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(SLIDER_THUMB),
                )],
            )),
        )),
    )
}

fn update_slider_visuals(
    sliders: Query<
        (
            Entity,
            &SliderValue,
            &SliderRange,
            &Hovered,
            &CoreSliderDragState,
        ),
        (
            Or<(
                Changed<SliderValue>,
                Changed<Hovered>,
                Changed<CoreSliderDragState>,
            )>,
            With<UISlider>,
        ),
    >,
    children: Query<&Children>,
    mut thumbs: Query<(&mut Node, &mut BackgroundColor, Has<UISliderThumb>), Without<UISlider>>,
) {
    for (slider_ent, value, range, hovered, drag_state) in sliders.iter() {
        for child in children.iter_descendants(slider_ent) {
            if let Ok((mut thumb_node, mut thumb_bg, is_thumb)) = thumbs.get_mut(child)
                && is_thumb
            {
                let position = range.thumb_position(value.0) * 100.0;
                thumb_node.left = percent(position);

                let is_active = hovered.0 | drag_state.dragging;
                thumb_bg.0 = if is_active {
                    SLIDER_THUMB.lighter(0.3)
                } else {
                    SLIDER_THUMB
                };
            }
        }
    }
}

fn update_value_labels(
    sliders: Query<(&SliderValue, &ValueLabel), (Changed<SliderValue>, With<UISlider>)>,
    mut texts: Query<&mut Text>,
) {
    for (value, label) in sliders.iter() {
        if let Ok(mut text) = texts.get_mut(label.0) {
            **text = format!("{:.0}%", value.0);
        }
    }
}

fn update_volume(
    sliders: Query<&SliderValue, (Changed<SliderValue>, With<UISlider>)>,
    mut sound_settings: Single<&mut VolumeNode, With<SoundEffectsBus>>,
) {
    for value in sliders.iter() {
        sound_settings.as_mut().set_percent(value.0);
    }
}

// Load settings on startup and apply to audio
fn load_initial_settings(
    settings: Res<Persistent<GameSettings>>,
    mut sound_settings: Single<&mut VolumeNode, With<SoundEffectsBus>>,
) {
    // set sound volume
    sound_settings.as_mut().set_percent(settings.sound_volume);
}

fn save_settings_on_change(
    sliders: Query<&SliderValue, (Changed<SliderValue>, With<UISlider>)>,
    mut settings: ResMut<Persistent<GameSettings>>,
) {
    for value in sliders.iter() {
        settings.sound_volume = value.0;
        if let Err(e) = settings.persist() {
            error!("Failed to save settings: {}", e);
        }
    }
}
