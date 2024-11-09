//! Demonstrates setting custom cursors that respond to interactions on UI elements.

use bevy::prelude::*;
use bevy::window::WindowTheme;
use bevy_cobweb_ui::prelude::*;
use bevy_cobweb_ui::sickle_ext::ui_builder::*;

//-------------------------------------------------------------------------------------------------------------------

fn build_ui(mut c: Commands, mut s: ResMut<SceneLoader>)
{
    let scene = SceneRef::new("main.caf", "scene");
    c.ui_root().load_scene(&mut s, scene);
}

//-------------------------------------------------------------------------------------------------------------------

fn main()
{
    App::new()
        .add_plugins(bevy::DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { window_theme: Some(WindowTheme::Dark), ..default() }),
            ..default()
        }))
        .add_plugins(CobwebUiPlugin)
        .load("main.caf")
        .add_systems(PreStartup, |mut c: Commands| {
            c.spawn(Camera2d);
        })
        .add_systems(OnEnter(LoadState::Done), build_ui)
        .run();
}

//-------------------------------------------------------------------------------------------------------------------