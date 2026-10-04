//! Touch controls: left-thumb joystick, right-thumb equipment column.
//! Every tool is always there — no hints about which one fits. The mountain teaches that.
//! Also: mouse emulates a single touch, keyboard for desktop, and an optional autopilot
//! (EVEREST_AUTOPLAY=1) used for automated play-testing.

use crate::hazards::{Hazards, BOULDER_OFFSET};
use crate::hints::HintLog;
use crate::hero::{HState, Hero};
use crate::story::{coat_s, Stage, Story};
use crate::terrain::{Surface, Terrain, Zone};
use crate::VIEW_H;
use bevy::prelude::*;
use bevy::render::camera::Projection;
use bevy::window::PrimaryWindow;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Axe,
    Rope,
    Dig,
    Lamp,
    Rest,
}
/// bottom → top on the right edge
pub const ALL_TOOLS: [Tool; 5] = [Tool::Axe, Tool::Rope, Tool::Dig, Tool::Lamp, Tool::Rest];

#[derive(Resource, Default)]
pub struct Controls {
    pub stick: Vec2,
    pub taps: Vec<Tool>,
    pub any_tap: bool,
}

#[derive(Resource, Default)]
pub struct JoyTouch {
    id: Option<u64>,
}

#[derive(Resource)]
pub struct Autopilot {
    pub on: bool,
    pub shelter: bool,
    pub use_rope: bool,
    pub probe: bool,
    pub arrest: bool,
    pub runback: bool,
    cd: f32,
}

#[derive(Component)]
pub struct JoyBase;
#[derive(Component)]
pub struct JoyKnob;
#[derive(Component)]
pub struct ToolButton(pub Tool);

pub const JOY_R: f32 = 44.0;
const BTN: f32 = 52.0;

pub struct ControlsPlugin;
impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut App) {
        let env = |k: &str| std::env::var(k).map(|v| v != "0").unwrap_or(false);
        app.init_resource::<Controls>()
            .init_resource::<JoyTouch>()
            .insert_resource(Autopilot {
                on: env("EVEREST_AUTOPLAY"),
                shelter: env("EVEREST_AUTOPLAY_SHELTER"),
                use_rope: !env("EVEREST_AUTOPLAY_NOROPE"),
                probe: !env("EVEREST_AUTOPLAY_NOPROBE"),
                arrest: !env("EVEREST_AUTOPLAY_NOARREST"),
                runback: env("EVEREST_AUTOPLAY_RUNBACK"),
                cd: 0.0,
            })
            .add_systems(Startup, spawn_controls);
    }
}

fn spawn_controls(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((
        JoyBase,
        Sprite { image: assets.load("sprites/joy_base.png"), custom_size: Some(Vec2::splat(JOY_R * 2.3)), color: Color::srgba(1., 1., 1., 0.6), ..default() },
        Transform::from_xyz(0., 0., 950.),
    ));
    commands.spawn((
        JoyKnob,
        Sprite { image: assets.load("sprites/joy_knob.png"), custom_size: Some(Vec2::splat(34.0)), color: Color::srgba(1., 1., 1., 0.8), ..default() },
        Transform::from_xyz(0., 0., 951.),
    ));
    for t in ALL_TOOLS {
        let img = match t {
            Tool::Axe => "sprites/ui_axe.png",
            Tool::Rope => "sprites/ui_rope.png",
            Tool::Dig => "sprites/ui_dig.png",
            Tool::Lamp => "sprites/ui_headlamp.png",
            Tool::Rest => "sprites/ui_rest.png",
        };
        commands.spawn((ToolButton(t), Sprite { image: assets.load(img), custom_size: Some(Vec2::splat(BTN)), color: Color::srgba(1., 1., 1., 0.85), ..default() }, Transform::from_xyz(0., 0., 950.)));
    }
}

pub fn view_area(proj: &Projection) -> Rect {
    match proj {
        Projection::Orthographic(o) => o.area,
        _ => Rect::from_center_size(Vec2::ZERO, Vec2::new(crate::VIEW_W, VIEW_H)),
    }
}
fn joy_center(a: Rect) -> Vec2 {
    Vec2::new(a.min.x + 80.0, a.min.y + 112.0)
}
fn button_pos(a: Rect, slot: usize) -> Vec2 {
    Vec2::new(a.max.x - 42.0, a.min.y + 66.0 + slot as f32 * 60.0)
}

#[allow(clippy::too_many_arguments)]
pub fn read_input(
    mut controls: ResMut<Controls>,
    mut joy: ResMut<JoyTouch>,
    touches: Res<Touches>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    window: Query<&Window, With<PrimaryWindow>>,
    cam: Query<(&Camera, &GlobalTransform, &Projection)>,
    hero: Query<&Hero>,
    story: Res<Story>,
    terrain: Res<Terrain>,
    hz: Res<Hazards>,
    mut auto: ResMut<Autopilot>,
    time: Res<Time>,
) {
    controls.taps.clear();
    controls.any_tap = false;
    controls.stick = Vec2::ZERO;
    let Ok((camera, cam_gt, proj)) = cam.single() else { return };
    let area = view_area(proj);
    let cam_pos = cam_gt.translation().truncate();
    // camera may be tilted (altitude); undo it so touch maps to the screen, not the world
    let inv = cam_gt.compute_transform().rotation.inverse();
    let to_view = |p: Vec2| camera.viewport_to_world_2d(cam_gt, p).ok().map(|w| (inv * (w - cam_pos).extend(0.0)).truncate());
    let mut pointers: Vec<(u64, Vec2, bool)> = vec![];
    for t in touches.iter() {
        if let Some(v) = to_view(t.position()) {
            pointers.push((t.id(), v, touches.just_pressed(t.id())));
        }
    }
    if let Ok(win) = window.single() {
        if mouse.pressed(MouseButton::Left) {
            if let Some(v) = win.cursor_position().and_then(|p| to_view(p)) {
                pointers.push((u64::MAX, v, mouse.just_pressed(MouseButton::Left)));
            }
        }
    }
    let jc = joy_center(area);
    let mut joy_alive = false;
    for (id, v, just) in pointers.iter().copied() {
        if just {
            controls.any_tap = true;
            let mut hit = false;
            for (slot, t) in ALL_TOOLS.iter().enumerate() {
                if v.distance(button_pos(area, slot)) < BTN * 0.62 {
                    controls.taps.push(*t);
                    hit = true;
                }
            }
            if !hit && joy.id.is_none() && v.x < area.center().x && v.y < area.min.y + area.height() * 0.45 {
                joy.id = Some(id);
            }
        }
        if joy.id == Some(id) {
            joy_alive = true;
            let d = (v - jc) / JOY_R;
            controls.stick = if d.length() > 1.0 { d.normalize() } else { d };
        }
    }
    if !joy_alive {
        joy.id = None;
    }
    let mut k = Vec2::ZERO;
    if keys.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]) { k.x -= 1.0; }
    if keys.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) { k.x += 1.0; }
    if keys.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) { k.y += 1.0; }
    if keys.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) { k.y -= 1.0; }
    if k != Vec2::ZERO {
        controls.stick = k.normalize();
    }
    for (key, tool) in [(KeyCode::Space, Tool::Axe), (KeyCode::KeyR, Tool::Rope), (KeyCode::KeyE, Tool::Dig), (KeyCode::KeyL, Tool::Lamp), (KeyCode::KeyQ, Tool::Rest)] {
        if keys.just_pressed(key) {
            controls.taps.push(tool);
            controls.any_tap = true;
        }
    }
    if keys.just_pressed(KeyCode::Enter) {
        controls.any_tap = true;
    }
    if auto.on {
        if let Ok(h) = hero.single() {
            autopilot(&mut controls, &mut auto, h, &story, &terrain, &hz, time.delta_secs());
        }
    }
}

/// A simple scripted player, used only for automated play-throughs. Deliberately imperfect.
fn autopilot(c: &mut Controls, auto: &mut Autopilot, h: &Hero, story: &Story, t: &Terrain, hz: &Hazards, dt: f32) {
    auto.cd -= dt;
    let (use_rope, probe, arrest, shelter, runback) = (auto.use_rope, auto.probe, auto.arrest, auto.shelter, auto.runback);
    let cd = &mut auto.cd;
    let mut tap = |c: &mut Controls, tool: Tool, every: f32| {
        if *cd <= 0.0 {
            c.taps.push(tool);
            c.any_tap = true;
            *cd = every;
        }
    };
    if story.stage == Stage::End {
        if story.t > 5.5 {
            c.any_tap = true;
        }
        return;
    }
    let g = *t.seg(h.s);
    match h.state {
        HState::Buried => tap(c, Tool::Dig, 0.3),
        HState::Crevasse { .. } => tap(c, Tool::Axe, 0.3),
        HState::Slide { v, .. } => {
            if arrest && !h.arrest && v.abs() > 30.0 {
                tap(c, Tool::Axe, 0.2);
            }
        }
        HState::Rest => {
            if h.stamina > 92.0 {
                c.stick = g.dir;
            }
        }
        HState::Move => {
            if story.lock > 0.0 && !matches!(story.stage, Stage::AvWhumpf) {
                return;
            }
            let night = hz.night > 0.3;
            if night != h.lamp && h.battery > 0.0 {
                tap(c, Tool::Lamp, 0.4);
                return;
            }
            let face_ahead = !g.wall && t.seg(h.s + 14.0).wall;
            if (h.stamina < 20.0 || (face_ahead && h.stamina < 85.0)) && !g.wall && hz.av.is_none() && hz.ice_warn <= 0.0 {
                tap(c, Tool::Rest, 0.4);
                return;
            }
            if use_rope {
                if let Some(&a) = t.anchors.iter().find(|&&a| (h.s - a).abs() < 10.0) {
                    if h.rope != Some(a) {
                        tap(c, Tool::Rope, 0.4);
                        return;
                    }
                }
            }
            if probe && g.zone == Zone::Crevasse && !hz.bridge_probed && h.s < hz.bridge.0 && hz.bridge.0 - h.s < 30.0 {
                tap(c, Tool::Axe, 0.5);
                return;
            }
            let cs = coat_s(t);
            if matches!(story.stage, Stage::Aftermath | Stage::CoatDig) && (h.s - (cs - 18.0)).abs() < 10.0 {
                tap(c, Tool::Dig, 0.35);
                return;
            }
            // where to go
            let mut dir = 1.0;
            if matches!(story.stage, Stage::AvWhumpf | Stage::AvSlide) && shelter {
                let target = t.part_s[10] + BOULDER_OFFSET - 18.0;
                dir = if (h.s - target).abs() < 4.0 { 0.0 } else { (target - h.s).signum() };
            }
            if matches!(story.stage, Stage::AvWhumpf | Stage::AvSlide) && runback {
                dir = -1.0; // panics and runs: the wrong call
            }
            if hz.ice_warn > 0.0 && (h.s - hz.ice_target).abs() < 45.0 {
                dir = if h.s > hz.ice_target { 1.0 } else { -1.0 };
            }
            if g.wall {
                if g.surf == Surface::Ice {
                    if !h.planted {
                        tap(c, Tool::Axe, 0.25);
                    }
                    c.stick = Vec2::new(0.0, 1.0);
                } else {
                    c.stick = Vec2::new(0.0, 1.0);
                    if hz.rock_warn > 0.0 || hz.rock_danger > 0.0 {
                        // lean out of the line of the falling stone
                        c.stick = Vec2::new(if hz.rock_lane >= h.pos.x { -1.0 } else { 1.0 }, 0.0);
                    }
                }
            } else {
                let ahead = t.seg(h.s + 10.0);
                if ahead.wall && ahead.surf == Surface::Ice {
                    tap(c, Tool::Axe, 0.3);
                }
                c.stick = g.dir * dir;
                if ahead.wall {
                    c.stick = (g.dir + Vec2::Y).normalize() * dir;
                }
            }
        }
        _ => {}
    }
}

pub fn draw_controls(
    controls: Res<Controls>,
    story: Res<Story>,
    hints: Res<HintLog>,
    time: Res<Time>,
    cam: Query<(&Transform, &Projection), (With<Camera2d>, Without<JoyBase>, Without<JoyKnob>, Without<ToolButton>)>,
    mut base: Query<(&mut Transform, &mut Sprite), (With<JoyBase>, Without<JoyKnob>, Without<ToolButton>)>,
    mut knob: Query<(&mut Transform, &mut Sprite), (With<JoyKnob>, Without<JoyBase>, Without<ToolButton>)>,
    mut buttons: Query<(&ToolButton, &mut Transform, &mut Sprite), (Without<JoyBase>, Without<JoyKnob>)>,
) {
    let Ok((ct, proj)) = cam.single() else { return };
    let area = view_area(proj);
    let cp = ct.translation.truncate();
    let rot = ct.rotation;
    let place = |v: Vec2| cp + (rot * v.extend(0.0)).truncate();
    let ui_alpha = if story.hide_ui { 0.0 } else { 1.0 };
    let jc = joy_center(area);
    if let Ok((mut t, mut s)) = base.single_mut() {
        t.translation = place(jc).extend(950.0);
        t.rotation = rot;
        s.color = Color::srgba(1., 1., 1., 0.55 * ui_alpha);
    }
    if let Ok((mut t, mut s)) = knob.single_mut() {
        t.translation = place(jc + controls.stick * JOY_R).extend(951.0);
        s.color = Color::srgba(1., 1., 1., 0.85 * ui_alpha);
    }
    for (b, mut t, mut s) in buttons.iter_mut() {
        let slot = ALL_TOOLS.iter().position(|x| *x == b.0).unwrap_or(0);
        t.translation = place(button_pos(area, slot)).extend(950.0);
        t.rotation = rot;
        let pressed = controls.taps.contains(&b.0);
        let glow = hints.glow.is_some_and(|(g, _)| g == b.0);
        let pulse = if glow { 1.0 + 0.1 * (time.elapsed_secs() * 6.0).sin().abs() } else { 1.0 };
        t.scale = Vec3::splat(if pressed { 0.9 } else { pulse });
        s.color = if glow { Color::srgba(1.3, 1.2, 0.7, ui_alpha) } else { Color::srgba(1., 1., 1., 0.85 * ui_alpha) };
    }
}
