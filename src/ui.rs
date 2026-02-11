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
use bevy_seedling::prelude::*;

const SLIDER_TRACK: Color = Color::srgb(0.05, 0.05, 0.05);
const SLIDER_THUMB: Color = Color::srgb(0.35, 0.75, 0.35);

#[derive(Component)]
struct UISlider;

#[derive(Component)]
struct UISliderThumb;

#[derive(Component)]
struct ValueLabel(Entity);

pub struct MenuPlugin;
impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.insert_state(MenuState::InGame)
            .add_plugins((UiWidgetsPlugins, InputDispatchPlugin, TabNavigationPlugin))
            .add_systems(
                Update,
                (
                    toggle_menu,
                    update_slider_visuals,
                    update_value_labels,
                    update_volume,
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

const TEXT_COLOR: Color = Color::Oklcha(Oklcha::new(0.8994, 0.0715, 331.2, 0.98));

#[derive(Component)]
struct Menu;

fn spawn_menu(mut commands: Commands, sound_settings: Single<&VolumeNode, With<SoundEffectsBus>>) {
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
            Node { ..default() },
            children![(
                Text::new("menu"),
                TextColor(TEXT_COLOR),
                TextFont {
                    // TODO: lets put in custom font?
                    font_size: 32.0,
                    ..default()
                },
            )],
        ));
        parent.spawn((
            Node { ..default() },
            children![(
                Text::new("sound"),
                TextColor(TEXT_COLOR),
                TextFont {
                    // TODO: lets put in custom font?
                    font_size: 24.0,
                    ..default()
                },
            )],
        ));
        parent
            .spawn((Node {
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
                        // TODO: lets put in custom font?
                        font_size: 16.0,
                        ..default()
                    },
                ),));

                let volume_label = subparent
                    .spawn(((
                        Text::new(format!("{current_sound_vol}%")),
                        TextColor(TEXT_COLOR),
                        TextFont {
                            // TODO: lets put in custom font?
                            font_size: 16.0,
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
