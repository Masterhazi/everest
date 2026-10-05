//! EVEREST — vertical slice
//! Base Camp → first ascent → avalanche → the yellow coat → friend's voice → first checkpoint.
//!
//! Portrait, touch-first. Left thumb: joystick. Right thumb: the five tools, always visible.
//! Desktop dev keys: arrows/WASD move, Space = ice axe, R = rope, E = dig, L = headlamp, Q = rest.

pub mod controls;
pub mod fx;
pub mod hazards;
pub mod hero;
pub mod hints;
pub mod skill;
pub mod story;
pub mod terrain;

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
                .set(render_plugin())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Everest".into(),
                        resolution: window_size(),
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
        .init_resource::<skill::Skill>()
        .init_resource::<hints::HintLog>()
        .init_resource::<fx::Layout>()
        .add_plugins((
            terrain::TerrainPlugin,
            hazards::HazardsPlugin,
            controls::ControlsPlugin,
            story::StoryPlugin,
            hero::HeroPlugin,
            fx::FxPlugin,
        ))
        .add_systems(Startup, hazards::spawn_lamp_mask)
        .add_systems(
            Update,
            (
                fx::layout_system,
                controls::read_input,
                story::story_system,
                hero::hero_system,
                hazards::hazards_system,
                skill::skill_system,
                hints::hints_system,
                hero::animate_hero,
                fx::mood_system,
                fx::camera_follow,
                hazards::hazard_visuals,
                fx::camera_locked,
                fx::snow_system,
                fx::audio_mix,
                fx::captions_system,
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

/// Graphics backend choice for Android.
fn render_plugin() -> bevy::render::RenderPlugin {
    #[allow(unused_mut)]
    let mut settings = bevy::render::settings::WgpuSettings::default();
    // Vulkan first (it's what the emulator test exercises, so what ships is what's tested);
    // OpenGL ES only on phones that have no Vulkan at all.
    #[cfg(target_os = "android")]
    {
        settings.backends = Some(bevy::render::settings::Backends::VULKAN | bevy::render::settings::Backends::GL);
    }
    bevy::render::RenderPlugin { render_creation: settings.into(), ..default() }
}

/// Desktop window size; EVEREST_WINDOW=WxH for testing other phone shapes (e.g. 360x800 = 20:9).
fn window_size() -> WindowResolution {
    let (w, h) = std::env::var("EVEREST_WINDOW")
        .ok()
        .and_then(|v| v.split_once('x').and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?))))
        .unwrap_or((405.0, 720.0));
    WindowResolution::new(w, h)
}
