//! Camera, weather, overlays, captions and the sound mix.
//! The body and the weather carry the information a HUD normally would.

use crate::controls::view_area;
use crate::hero::{HState, Hero};
use crate::hazards::Hazards;
use crate::story::{Stage, Story};
use crate::{VIEW_H, VIEW_W};
const WORLD_TOP: f32 = 1200.0;
use bevy::audio::{AudioSinkPlayback, Volume};
use bevy::prelude::*;
use bevy::render::camera::{Projection, ScalingMode};
use bevy::text::TextBounds;
use rand::Rng;

#[derive(Resource)]
pub struct Sounds {
    pub whumpf: Handle<AudioSource>,
    pub rumble: Handle<AudioSource>,
    pub crunch: Handle<AudioSource>,
    pub chink: Handle<AudioSource>,
    pub gust: Handle<AudioSource>,
    pub chime: Handle<AudioSource>,
    /// "Ashchhe Jamai Digombor" (The Parvathy Baul Project, SVF Music), used with permission —
    /// the opening, muffled and distant, as if carried through the mountain
    pub song: Handle<AudioSource>,
    pub steps_snow: Vec<Handle<AudioSource>>,
    pub steps_rock: Vec<Handle<AudioSource>>,
    pub steps_ice: Vec<Handle<AudioSource>>,
}

/// Play with a little random pitch so repeated sounds (footsteps) don't machine-gun.
pub fn play_varied(commands: &mut Commands, h: &Handle<AudioSource>, vol: f32) {
    let speed = rand::thread_rng().gen_range(0.88..1.12);
    commands.spawn((AudioPlayer::new(h.clone()), PlaybackSettings::DESPAWN.with_volume(Volume::Linear(vol)).with_speed(speed)));
}

/// The world camera (top part of the screen).
#[derive(Component)]
pub struct GameCam;
/// The control-strip camera (whole screen, draws only the controls layer).
#[derive(Component)]
pub struct UiCam;
pub const UI_LAYER: usize = 1;

/// Screen split: the game is drawn above a control strip so thumbs never cover the mountain.
#[derive(Resource, Default)]
pub struct Layout {
    /// control-space height (width is always 360)
    pub h: f32,
    /// control strip height, in control-space units
    pub strip: f32,
}

pub fn layout_system(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut layout: ResMut<Layout>,
    mut cam: Query<&mut Camera, With<GameCam>>,
) {
    let (Ok(win), Ok(mut cam)) = (windows.single(), cam.single_mut()) else { return };
    let (pw, ph) = (win.physical_width().max(1), win.physical_height().max(1));
    let h = 360.0 * ph as f32 / pw as f32;
    let strip = (h * 0.27).clamp(190.0, 300.0);
    let game_px = (((h - strip) / h) * ph as f32).round().max(1.0) as u32;
    layout.h = h;
    layout.strip = strip;
    let want = bevy::render::camera::Viewport { physical_position: UVec2::ZERO, physical_size: UVec2::new(pw, game_px), ..default() };
    if cam.viewport.as_ref().map(|v| v.physical_size) != Some(want.physical_size) {
        cam.viewport = Some(want);
    }
}

pub fn play(commands: &mut Commands, h: &Handle<AudioSource>, vol: f32) {
    commands.spawn((AudioPlayer::new(h.clone()), PlaybackSettings::DESPAWN.with_volume(Volume::Linear(vol))));
}

/// Continuous layers, mixed every frame.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum Chan {
    Wind,
    Breath,
    Heart,
    // music layers
    Drone,
    Bowls,
    Tension,
    Night,
    Lament,
    // natural ambiences
    Flags,
    Creak,
}
#[derive(Component)]
pub struct ChanVol(f32);

#[derive(Resource, Default)]
pub struct Mood {
    pub wind: f32,
    pub breath: f32,
    pub heart: f32,
    pub memory: f32,
    pub dark: f32,
    pub sunset: f32,
    pub wind_x: f32,
    pub shake: f32,
    pub flash: f32,
    pub vignette: f32,
    pub night: f32,
    pub dizzy: f32,
    // music + ambience targets
    pub drone: f32,
    pub bowls: f32,
    pub tension: f32,
    pub lament: f32,
    pub flags: f32,
    pub creak: f32,
}

// ------------------------------------------------------------------ captions
pub struct Caption {
    pub text: String,
    pub color: Color,
    pub t: f32,
    pub dur: f32,
}
#[derive(Resource, Default)]
pub struct Captions {
    pub day: Option<Caption>,
    pub sub: Option<Caption>,
    pub hint: Option<Caption>,
}
impl Captions {
    pub fn day(&mut self, s: &str) {
        self.day = Some(Caption { text: s.into(), color: Color::srgb(0.92, 0.94, 1.0), t: 0.0, dur: 4.5 });
    }
    pub fn sub(&mut self, c: Color, s: &str, dur: f32) {
        self.sub = Some(Caption { text: s.into(), color: c, t: 0.0, dur });
    }
    pub fn hint(&mut self, s: &str) {
        self.hint = Some(Caption { text: s.into(), color: Color::srgb(0.6, 0.66, 0.75), t: 0.0, dur: 1e9 });
    }
}
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum CapSlot {
    Day,
    Sub,
    Hint,
}

// ------------------------------------------------------------------ camera-locked things
#[derive(Component)]
pub struct Overlay {
    kind: OverlayKind,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum OverlayKind {
    BgDay,
    BgSunset,
    Vignette,
    Dark,
    Flash,
}

#[derive(Component)]
pub struct Flake {
    v: Vec2,
    size: f32,
}

#[derive(Resource, Default)]
pub struct CamState {
    p: Vec2,
    init: bool,
}

pub struct FxPlugin;
impl Plugin for FxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Mood>()
            .init_resource::<Captions>()
            .init_resource::<CamState>()
            .add_systems(Startup, setup_fx);
    }
}

fn setup_fx(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((
        Camera2d,
        GameCam,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin { min_width: VIEW_W, min_height: VIEW_H * 0.82 },
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(VIEW_W / 2.0, VIEW_H / 2.0, 999.0),
    ));
    commands.spawn((
        Camera2d,
        UiCam,
        Camera { order: 1, clear_color: ClearColorConfig::None, ..default() },
        bevy::render::view::RenderLayers::layer(UI_LAYER),
        Projection::Orthographic(OrthographicProjection { scaling_mode: ScalingMode::FixedHorizontal { viewport_width: 360.0 }, ..OrthographicProjection::default_2d() }),
        Transform::from_xyz(0.0, 0.0, 999.0),
    ));
    commands.insert_resource(Sounds {
        whumpf: assets.load("audio/whumpf.ogg"),
        rumble: assets.load("audio/rumble.ogg"),
        crunch: assets.load("audio/crunch.ogg"),
        chink: assets.load("audio/chink.ogg"),
        gust: assets.load("audio/gust.ogg"),
        chime: assets.load("audio/chime.ogg"),
        song: assets.load("audio/song_distant.ogg"),
        steps_snow: (0..3).map(|k| assets.load(format!("audio/step_snow{k}.ogg"))).collect(),
        steps_rock: (0..3).map(|k| assets.load(format!("audio/step_rock{k}.ogg"))).collect(),
        steps_ice: (0..3).map(|k| assets.load(format!("audio/step_ice{k}.ogg"))).collect(),
    });
    for (c, file) in [
        (Chan::Wind, "audio/wind.ogg"),
        (Chan::Breath, "audio/breath.ogg"),
        (Chan::Heart, "audio/heart.ogg"),
        (Chan::Drone, "audio/m_drone.ogg"),
        (Chan::Bowls, "audio/m_bowls.ogg"),
        (Chan::Tension, "audio/m_tension.ogg"),
        (Chan::Night, "audio/m_night.ogg"),
        (Chan::Lament, "audio/m_lament.ogg"),
        (Chan::Flags, "audio/flags.ogg"),
        (Chan::Creak, "audio/creak.ogg"),
    ] {
        commands.spawn((c, ChanVol(0.0), AudioPlayer::new(assets.load(file)), PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0))));
    }

    // sky layers (behind the level) and screen overlays (in front)
    let ov = |kind, sprite: Sprite, z: f32| (Overlay { kind }, sprite, Transform::from_xyz(0., 0., z));
    commands.spawn(ov(OverlayKind::BgDay, Sprite::from_image(assets.load("sprites/bg_day.png")), 0.2));
    commands.spawn(ov(OverlayKind::BgSunset, Sprite { image: assets.load("sprites/bg_sunset.png"), color: Color::srgba(1., 1., 1., 0.), ..default() }, 0.3));
    commands.spawn(ov(OverlayKind::Vignette, Sprite::from_image(assets.load("sprites/vignette.png")), 900.0));
    commands.spawn(ov(OverlayKind::Flash, Sprite::from_color(Color::srgba(0.9, 0.93, 1.0, 0.0), Vec2::ONE), 905.0));
    commands.spawn(ov(OverlayKind::Dark, Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.0), Vec2::ONE), 910.0));

    let text = |slot, size: f32, width: f32| {
        (
            slot,
            Text2d::new(""),
            TextFont { font_size: size, ..default() },
            TextColor(Color::NONE),
            TextLayout::new_with_justify(JustifyText::Center),
            TextBounds::new_horizontal(width),
            Transform::from_xyz(0., 0., 940.0),
        )
    };
    commands.spawn(text(CapSlot::Day, 30.0, 340.0));
    commands.spawn(text(CapSlot::Sub, 15.0, 340.0));
    commands.spawn(text(CapSlot::Hint, 13.0, 300.0));

    let mut rng = rand::thread_rng();
    for _ in 0..160 {
        let size = rng.gen_range(1.0..2.6_f32).round();
        commands.spawn((
            Flake { v: Vec2::new(0.0, -rng.gen_range(25.0..60.0)), size },
            Sprite::from_color(Color::srgba(0.92, 0.95, 1.0, rng.gen_range(0.35..0.9)), Vec2::splat(size)),
            Transform::from_xyz(rng.gen_range(-40.0..400.0), rng.gen_range(0.0..WORLD_TOP), 20.0),
        ));
    }
}

pub fn mood_system(
    mut mood: ResMut<Mood>,
    story: Res<Story>,
    hz: Res<Hazards>,
    terrain: Res<crate::terrain::Terrain>,
    controls: Res<crate::controls::Controls>,
    mut hq: Query<&mut Hero>,
    time: Res<Time>,
) {
    let Ok(mut h) = hq.single_mut() else { return };
    let dt = time.delta_secs().min(0.05);
    let fatigue = 1.0 - h.stamina / 100.0;
    let climbing = h.state == HState::Move && h.moving && h.pos.y > 0.0;

    let mut wind = 0.35 + (h.pos.y / 1200.0).min(0.3);
    let mut wind_x = -22.0 - h.pos.y * 0.04;
    let mut breath = (fatigue * 0.9 + if climbing { 0.1 } else { 0.0 } + hz.dizzy * 0.5).min(1.0);
    let mut heart: f32 = 0.0;
    let mut memory: f32 = 0.0;
    let mut dark = 0.0;
    // ---- music: a drone while he climbs, bowls when he stops, tension when the mountain warns,
    // silence when it strikes, the lament for memory and for the moments he might give up
    let resting = matches!(h.state, HState::Rest | HState::Sit | HState::Kneel);
    let danger = hz.rock_warn > 0.0 || hz.rock_danger > 0.0 || hz.ice_warn > 0.0 || hz.avalanche_running() || matches!(h.state, HState::Slide { .. } | HState::Crevasse { .. });
    let mut drone = 0.55 * (1.0 - 0.5 * hz.night);
    let mut bowls = if resting || terrain.part(h.s) == 0 { 0.6 } else { 0.12 };
    let mut tension = if danger { 0.75 } else { 0.0 };
    let near_quit = (controls.idle / 14.0 - 1.0).clamp(0.0, 1.0); // stopped for a long while
    let mut lament = (0.55 * near_quit).max(0.12 * hz.night);
    if hz.avalanche_warning() || h.state == HState::Buried {
        drone = 0.0;
        bowls = 0.0;
        tension = 0.0;
        lament = 0.0;
    }
    let flag_s = [205.0, terrain.part_s[16] + 90.0];
    let flags = flag_s.iter().map(|&f| (1.0 - (h.s - f).abs() / 220.0).max(0.0)).fold(0.0, f32::max);
    let (c0, c1) = terrain.part_range(12);
    let creak = if h.s > c0 - 80.0 && h.s < c1 + 40.0 { 0.6 } else { 0.0 };
    if hz.avalanche_warning() {
        wind = 0.04; // the silence before
        heart = 0.5;
    } else if hz.avalanche_running() {
        wind = 0.3;
        heart = 0.6;
        wind_x = -140.0;
    }
    if hz.ice_warn > 0.0 || hz.rock_warn > 0.0 {
        heart = heart.max(0.35);
    }
    match h.state {
        HState::Buried => {
            wind = 0.03;
            breath = 0.95;
            heart = 0.85;
            let total = if story.coat_pending() { 8.0 } else { 6.0 };
            dark = 0.94 - (1.0 - h.dig_left as f32 / total).clamp(0.0, 1.0) * 0.45;
        }
        HState::Crevasse { left } => {
            wind = 0.08;
            heart = 0.6;
            breath = breath.max(0.6);
            dark = 0.35 + left as f32 * 0.04;
        }
        HState::Down { .. } | HState::Rise { .. } => breath = 1.0,
        _ => {}
    }
    match story.stage {
        Stage::CoatHold | Stage::Voice => {
            wind = 0.1;
            breath = 0.3;
            memory = 0.65;
            wind_x = -12.0;
            drone = 0.0;
            tension = 0.0;
            lament = 0.85;
        }
        Stage::Checkpoint => {
            wind = 0.16;
            breath = 0.25;
            memory = 0.5;
            wind_x = -15.0;
            drone = 0.2;
            bowls = 0.6;
            lament = 0.6;
        }
        Stage::End => {
            wind = 0.14; // the world keeps going after the picture ends
            breath = 0.0;
            memory = 0.45;
            dark = (story.t / 3.0).min(1.0);
            drone = 0.0;
            bowls = 0.0;
            lament = 0.0;
        }
        _ => {}
    }
    if h.just_fell {
        h.just_fell = false;
        mood.flash = 0.45;
        mood.shake = mood.shake.max(0.35);
    }
    let _ = memory;
    mood.drone = drone;
    mood.bowls = bowls;
    mood.tension = tension;
    mood.lament = lament;
    mood.flags = flags;
    mood.creak = creak;
    mood.wind = wind;
    mood.breath = breath;
    mood.heart = heart;
    mood.memory = memory;
    mood.dark = dark;
    mood.sunset = hz.dawn;
    mood.night = hz.night;
    mood.dizzy = hz.dizzy;
    mood.wind_x = mood.wind_x + (wind_x - mood.wind_x) * (dt * 3.0).min(1.0);
    mood.flash = (mood.flash - dt * 0.9).max(0.0);
    mood.shake = (mood.shake - dt * 0.5).max(0.0);
    let hurt = matches!(h.state, HState::Down { .. } | HState::Rise { .. }) as i32 as f32;
    mood.vignette = (0.18 + fatigue.powf(1.5) * 0.7 + hurt * 0.35 + hz.dizzy * 0.35).min(1.0);
}

pub fn camera_follow(
    mut cam: Query<(&mut Transform, &Projection), With<GameCam>>,
    hq: Query<&Hero>,
    mood: Res<Mood>,
    mut st: ResMut<CamState>,
    time: Res<Time>,
) {
    let (Ok((mut t, proj)), Ok(h)) = (cam.single_mut(), hq.single()) else { return };
    let area = view_area(proj);
    let dt = time.delta_secs().min(0.05);
    // the route climbs up and to the right: keep him lower-left, the mountain ahead in view
    let target = Vec2::new((h.pos.x + 60.0).max(area.width() / 2.0 - 20.0), (h.pos.y + area.height() * 0.18).max(area.height() / 2.0 - 30.0));
    if !st.init {
        st.p = target;
        st.init = true;
    }
    let p = st.p;
    st.p = p + (target - p) * (dt * 2.2).min(1.0);
    let mut rng = rand::thread_rng();
    let s = mood.shake * 6.0;
    let shake = if s > 0.05 { Vec2::new(rng.gen_range(-s..s), rng.gen_range(-s..s)) } else { Vec2::ZERO };
    t.translation.x = st.p.x + shake.x;
    t.translation.y = st.p.y + shake.y;
    // altitude: the world sways a little until his body adjusts
    let sway = (time.elapsed_secs() * 1.7).sin() * 0.04 * mood.dizzy;
    t.rotation = Quat::from_rotation_z(sway);
}

pub fn camera_locked(
    cam: Query<(&Transform, &Projection), (With<GameCam>, Without<Overlay>)>,
    mut q: Query<(&Overlay, &mut Transform, &mut Sprite)>,
    mood: Res<Mood>,
) {
    let Ok((ct, proj)) = cam.single() else { return };
    let area = view_area(proj);
    let c = ct.translation.truncate();
    for (o, mut t, mut s) in q.iter_mut() {
        let z = t.translation.z;
        match o.kind {
            OverlayKind::BgDay | OverlayKind::BgSunset => {
                // slow parallax: distant peaks barely move while he climbs
                let scale = (area.width() / 360.0).max(area.height() / 780.0).max(1.0);
                let off = (120.0 - (c.y - 320.0) * 0.08).clamp(-120.0, 120.0);
                t.translation = Vec3::new(c.x, c.y + off * scale, z);
                t.scale = Vec3::splat(scale * 1.15);
                t.rotation = ct.rotation;
                let night = 1.0 - 0.75 * mood.night;
                if o.kind == OverlayKind::BgSunset {
                    s.color = Color::srgba(1., 1., 1., mood.sunset);
                } else {
                    s.color = Color::srgb(night, night, night * 1.05);
                }
            }
            _ => {
                t.translation = c.extend(z);
                t.rotation = ct.rotation;
                s.custom_size = Some(area.size() + Vec2::splat(60.0));
                let a = match o.kind {
                    OverlayKind::Vignette => mood.vignette,
                    OverlayKind::Dark => mood.dark,
                    OverlayKind::Flash => mood.flash,
                    _ => 0.0,
                };
                let base = match o.kind {
                    OverlayKind::Flash => (0.9, 0.93, 1.0),
                    _ => (0.0, 0.0, 0.0),
                };
                s.color = Color::srgba(base.0, base.1, base.2, a);
            }
        }
    }
}

pub fn snow_system(
    cam: Query<(&Transform, &Projection), (With<GameCam>, Without<Flake>)>,
    mut q: Query<(&mut Flake, &mut Transform)>,
    mood: Res<Mood>,
    time: Res<Time>,
) {
    let Ok((ct, proj)) = cam.single() else { return };
    let r = view_area(proj);
    let c = ct.translation.truncate();
    let (x0, x1, y0, y1) = (c.x + r.min.x - 20.0, c.x + r.max.x + 20.0, c.y + r.min.y - 20.0, c.y + r.max.y + 20.0);
    let dt = time.delta_secs().min(0.05);
    let mut rng = rand::thread_rng();
    for (f, mut t) in q.iter_mut() {
        let depth = f.size / 2.6; // bigger flakes are closer and faster
        t.translation.x += (mood.wind_x * (0.6 + depth) + (time.elapsed_secs() * 1.3 + t.translation.y * 0.05).sin() * 8.0) * dt;
        t.translation.y += f.v.y * (0.6 + depth) * dt;
        if t.translation.y < y0 || t.translation.x < x0 || t.translation.x > x1 || t.translation.y > y1 + 40.0 {
            // re-enter from the top or the windward edge
            if rng.gen_bool(0.5) || mood.wind_x > -60.0 {
                t.translation.x = rng.gen_range(x0..x1);
                t.translation.y = y1;
            } else {
                t.translation.x = x1;
                t.translation.y = rng.gen_range(y0..y1);
            }
        }
    }
}

pub fn audio_mix(mood: Res<Mood>, time: Res<Time>, mut q: Query<(&Chan, &mut ChanVol, &mut AudioSink)>) {
    let dt = time.delta_secs().min(0.05);
    for (c, mut v, mut sink) in q.iter_mut() {
        let target = match c {
            Chan::Wind => mood.wind,
            Chan::Breath => mood.breath * 0.8,
            Chan::Heart => mood.heart * 0.7,
            Chan::Drone => mood.drone * 0.55,
            Chan::Bowls => mood.bowls * 0.5,
            Chan::Tension => mood.tension * 0.6,
            Chan::Night => mood.night * 0.45,
            Chan::Lament => mood.lament * 0.6,
            Chan::Flags => mood.flags * 0.6,
            Chan::Creak => mood.creak * 0.55,
        };
        // music swells slowly; danger cuts in fast; silence falls fast
        let rate = match c {
            Chan::Wind => 1.6,
            Chan::Tension => if target > v.0 { 3.0 } else { 0.6 },
            Chan::Drone | Chan::Bowls | Chan::Lament | Chan::Night => if target < v.0 && target < 0.05 { 2.5 } else { 0.35 },
            _ => 1.0,
        };
        v.0 += (target - v.0) * (dt * rate).min(1.0);
        sink.set_volume(Volume::Linear(v.0.max(0.0)));
    }
}

pub fn captions_system(
    mut caps: ResMut<Captions>,
    time: Res<Time>,
    cam: Query<(&Transform, &Projection), (With<GameCam>, Without<CapSlot>)>,
    mut q: Query<(&CapSlot, &mut Text2d, &mut TextColor, &mut Transform)>,
    hq: Query<&Hero>,
) {
    let hero_y = hq.single().map(|h| h.pos.y).unwrap_or(0.0);
    let Ok((ct, proj)) = cam.single() else { return };
    let area = view_area(proj);
    let c = ct.translation.truncate();
    let dt = time.delta_secs().min(0.05);
    let caps = caps.as_mut();
    for cap in [&mut caps.day, &mut caps.sub, &mut caps.hint].into_iter().flatten() {
        cap.t += dt;
    }
    for (slot, mut text, mut color, mut t) in q.iter_mut() {
        let (cap, pos) = match slot {
            CapSlot::Day => (&caps.day, Vec2::new(0.0, area.max.y - 96.0)),
            CapSlot::Sub => (&caps.sub, Vec2::new(0.0, (hero_y - c.y + 150.0).min(area.max.y - 70.0))),
            CapSlot::Hint => (&caps.hint, Vec2::new(0.0, area.min.y + 120.0)),
        };
        t.translation = (c + pos).extend(940.0);
        match cap {
            Some(cp) => {
                let a = (cp.t / 1.2).min(1.0).min(((cp.dur - cp.t) / 1.2).max(0.0));
                let spaced = if *slot == CapSlot::Day { cp.text.chars().map(|ch| ch.to_string()).collect::<Vec<_>>().join(" ") } else { cp.text.clone() };
                if text.0 != spaced {
                    text.0 = spaced;
                }
                color.0 = cp.color.with_alpha(a * if *slot == CapSlot::Day { 0.8 } else { 0.95 });
            }
            None => color.0 = Color::NONE,
        }
    }
}

