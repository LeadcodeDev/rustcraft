mod events;
mod interaction;
mod player;
mod render;
mod ui;
mod world;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Rustcraft".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(events::EventsPlugin)
        .add_plugins(world::WorldPlugin)
        .add_plugins(render::RenderPlugin)
        .add_plugins(player::PlayerPlugin)
        .add_plugins(interaction::InteractionPlugin)
        .add_plugins(ui::UiPlugin)
        .add_systems(Startup, setup_lighting)
        .run();
}

fn setup_lighting(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 15000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, 0.3, 0.0)),
    ));

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 200.0,
    });
}
