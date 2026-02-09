use bevy::prelude::*;

pub struct MenuPlugin;
impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.insert_state(MenuState::InGame)
            .add_systems(Update, toggle_menu)
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

#[derive(Component)]
struct Menu;

fn spawn_menu(mut commands: Commands) {
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

    commands.spawn((
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
        children![(
            Node { ..default() },
            children![(Text::new("menu"), TextColor(Color::WHITE))],
        )],
    ));
}

fn kill_menu(menu_entity: Query<Entity, With<Menu>>, mut commands: Commands) {
    for ent in menu_entity {
        let mut menu_ent = commands.entity(ent);
        menu_ent.despawn();
    }
}
