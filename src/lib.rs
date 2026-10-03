//! EVEREST — vertical slice
//! Base Camp → first ascent → avalanche → the yellow coat → friend's voice → first checkpoint.
//!
//! Portrait, touch-first. Left thumb: joystick. Right thumb: contextual equipment.
//! Desktop dev keys: arrows/WASD move, Space = ice axe, R = rope, E = dig, Q = rest.

pub mod controls;
pub mod fx;
pub mod hero;
pub mod level;
pub mod story;

use bevy::prelude::*;
use bevy::window::WindowResolution;

/// Logical play area. The camera always shows at least this much (portrait 9:16).
pub const VIEW_W: f32 = 360.0;
pub const VIEW_H: f32 = 640.0;

/// Everything that belongs to one run of the slice; despawned on restart.
#[derive(Component)]
pub struct LevelEntity;

/// Entry point. `bevy_main` also generates the Android entry point.
#[bevy_main]
pub fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.03, 0.05, 0.09)))
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Everest".into(),
                        resolution: WindowResolution::new(405.0, 720.0),
                        // web: fill the page, don't let the browser scroll/zoom on touch
                        canvas: Some("#bevy".into()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: true,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_systems(PreStartup, install_panic_logger)
        .add_plugins((
            level::LevelPlugin,
            controls::ControlsPlugin,
            story::StoryPlugin,
            hero::HeroPlugin,
            fx::FxPlugin,
        ))
        .add_systems(
            Update,
            (
                controls::read_input,
                story::story_system,
                hero::hero_system,
                hero::animate_hero,
                fx::mood_system,
                fx::camera_follow,
                fx::camera_locked,
                fx::snow_system,
                fx::audio_mix,
                fx::captions_system,
                controls::update_tool_avail,
                controls::draw_controls,
                story::restart_system,
            )
                .chain(),
        )
        .run();
}

/// On Android, a Rust panic goes to stderr, which nobody sees. Send it to the system log too.
fn install_panic_logger() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        error!("EVEREST PANIC: {info}");
        default(info);
    }));
    info!("everest: started");
}
