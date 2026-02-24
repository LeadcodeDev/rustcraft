use bevy::prelude::*;

use crate::app_state::AppState;
use crate::player::camera::GameState;

#[derive(Component)]
pub struct PauseMenuRoot;

#[derive(Component)]
pub struct ResumeButton;

#[derive(Component)]
pub struct QuitToMenuButton;

#[derive(Component)]
pub struct QuitButton;

pub fn spawn_pause_menu(mut commands: Commands) {
    commands
        .spawn((
            PauseMenuRoot,
            StateScoped(AppState::InGame),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            Visibility::Hidden,
            GlobalZIndex(10),
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("Pause"),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(24.0)),
                    ..default()
                },
            ));

            // Resume button
            parent
                .spawn((
                    ResumeButton,
                    Button,
                    Node {
                        width: Val::Px(250.0),
                        height: Val::Px(50.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("Reprendre"),
                        TextFont {
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            // Quit to menu button
            parent
                .spawn((
                    QuitToMenuButton,
                    Button,
                    Node {
                        width: Val::Px(250.0),
                        height: Val::Px(50.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("Retour au menu"),
                        TextFont {
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            // Quit game button
            parent
                .spawn((
                    QuitButton,
                    Button,
                    Node {
                        width: Val::Px(250.0),
                        height: Val::Px(50.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.5, 0.15, 0.15)),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("Quitter le jeu"),
                        TextFont {
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

pub fn show_hide_pause_menu(
    game_state: Res<GameState>,
    mut query: Query<&mut Visibility, With<PauseMenuRoot>>,
) {
    if !game_state.is_changed() {
        return;
    }
    for mut vis in &mut query {
        *vis = match *game_state {
            GameState::Paused => Visibility::Visible,
            _ => Visibility::Hidden,
        };
    }
}

pub fn handle_resume_button(
    interaction: Query<&Interaction, (Changed<Interaction>, With<ResumeButton>)>,
    mut game_state: ResMut<GameState>,
    mut windows: Query<&mut Window>,
) {
    for &inter in &interaction {
        if inter == Interaction::Pressed {
            *game_state = GameState::Playing;
            if let Ok(mut window) = windows.get_single_mut() {
                window.cursor_options.grab_mode = bevy::window::CursorGrabMode::Locked;
                window.cursor_options.visible = false;
            }
        }
    }
}

pub fn handle_quit_to_menu_button(
    interaction: Query<&Interaction, (Changed<Interaction>, With<QuitToMenuButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for &inter in &interaction {
        if inter == Interaction::Pressed {
            next_state.set(AppState::MainMenu);
        }
    }
}

pub fn handle_quit_button(
    interaction: Query<&Interaction, (Changed<Interaction>, With<QuitButton>)>,
    mut app_exit: EventWriter<AppExit>,
) {
    for &inter in &interaction {
        if inter == Interaction::Pressed {
            app_exit.send(AppExit::Success);
        }
    }
}

pub fn button_hover(
    mut query: Query<
        (&Interaction, &mut BackgroundColor, Option<&QuitButton>),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut bg, is_quit) in &mut query {
        let base = if is_quit.is_some() {
            Color::srgb(0.5, 0.15, 0.15)
        } else {
            Color::srgb(0.3, 0.3, 0.3)
        };
        let hover = if is_quit.is_some() {
            Color::srgb(0.6, 0.2, 0.2)
        } else {
            Color::srgb(0.4, 0.4, 0.4)
        };

        *bg = match interaction {
            Interaction::Hovered => BackgroundColor(hover),
            _ => BackgroundColor(base),
        };
    }
}
