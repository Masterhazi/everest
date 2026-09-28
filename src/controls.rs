//! Touch controls: left-thumb joystick, right-thumb contextual equipment.
//! Also: mouse emulates a single touch, keyboard for desktop, and an optional
//! autopilot (EVEREST_AUTOPLAY=1) used for automated play-testing.

use crate::hero::{HState, Hero};
use crate::level::*;
use crate::story::{Stage, Story};
use crate::VIEW_H;
use bevy::prelude::*;
use bevy::render::camera::Projection;
use bevy::window::PrimaryWindow;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Axe,
    Rope,
    Dig,
    Rest,
}
pub const ALL_TOOLS: [Tool; 4] = [Tool::Axe, Tool::Rope, Tool::Dig, Tool::Rest];

/// What the player is asking for this frame.
#[derive(Resource, Default)]
pub struct Controls {
    pub stick: Vec2,
    pub taps: Vec<Tool>,
    pub any_tap: bool,
}

/// Which tools make sense right now (only these are shown).
#[derive(Resource, Default)]
pub struct ToolAvail {
    pub list: Vec<Tool>,
    pub pulse: Option<Tool>,
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
    cd: f32,
}

#[derive(Component)]
pub struct JoyBase;
#[derive(Component)]
pub struct JoyKnob;
#[derive(Component)]
pub struct ToolButton(pub Tool);

pub const JOY_R: f32 = 44.0;
const BTN: f32 = 58.0;

pub struct ControlsPlugin;
impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut App) {
        let env = |k: &str| std::env::var(k).map(|v| v != "0").unwrap_or(false);
        app.init_resource::<Controls>()
            .init_resource::<ToolAvail>()
            .init_resource::<JoyTouch>()
            .insert_resource(Autopilot {
                on: env("EVEREST_AUTOPLAY"),
                shelter: env("EVEREST_AUTOPLAY_SHELTER"),
                use_rope: !env("EVEREST_AUTOPLAY_NOROPE"),
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
            Tool::Rest => "sprites/ui_rest.png",
        };
        commands.spawn((
            ToolButton(t),
            Sprite { image: assets.load(img), custom_size: Some(Vec2::splat(BTN)), color: Color::srgba(1., 1., 1., 0.85), ..default() },
            Transform::from_xyz(0., 0., 950.),
            Visibility::Hidden,
        ));
    }
}

/// View-space layout (origin at camera centre, y up).
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
    Vec2::new(a.max.x - 48.0, a.min.y + 80.0 + slot as f32 * 70.0)
}

#[allow(clippy::too_many_arguments)]
pub fn read_input(
    mut controls: ResMut<Controls>,
    mut joy: ResMut<JoyTouch>,
    avail: Res<ToolAvail>,
    touches: Res<Touches>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    window: Query<&Window, With<PrimaryWindow>>,
    cam: Query<(&Camera, &GlobalTransform, &Projection)>,
    hero: Query<&Hero>,
    story: Res<Story>,
    mut auto: ResMut<Autopilot>,
    time: Res<Time>,
) {
    controls.taps.clear();
    controls.any_tap = false;
    controls.stick = Vec2::ZERO;
    let Ok((camera, cam_gt, proj)) = cam.single() else { return };
    let area = view_area(proj);
    let cam_pos = cam_gt.translation().truncate();
    let to_view = |p: Vec2| camera.viewport_to_world_2d(cam_gt, p).ok().map(|w| w - cam_pos);

    // gather pointer events: (id, view pos, just_pressed)
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
            // equipment buttons
            let mut hit = false;
            for (slot, t) in avail.list.iter().enumerate() {
                if v.distance(button_pos(area, slot)) < BTN * 0.62 {
                    controls.taps.push(*t);
                    hit = true;
                }
            }
            // anything in the lower-left becomes the joystick
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

    // keyboard
    let mut k = Vec2::ZERO;
    if keys.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]) { k.x -= 1.0; }
    if keys.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) { k.x += 1.0; }
    if keys.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) { k.y += 1.0; }
    if keys.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) { k.y -= 1.0; }
    if k != Vec2::ZERO {
        controls.stick = k.normalize();
    }
    for (key, tool) in [(KeyCode::Space, Tool::Axe), (KeyCode::KeyR, Tool::Rope), (KeyCode::KeyE, Tool::Dig), (KeyCode::KeyQ, Tool::Rest)] {
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
            autopilot(&mut controls, &mut auto, h, &story, time.delta_secs());
        }
    }
}

/// A simple scripted player, used only for automated play-throughs.
fn autopilot(c: &mut Controls, auto: &mut Autopilot, h: &Hero, story: &Story, dt: f32) {
    auto.cd -= dt;
    let mut tap = |c: &mut Controls, t: Tool, every: f32| {
        if auto.cd <= 0.0 {
            c.taps.push(t);
            c.any_tap = true;
            auto.cd = every;
        }
    };
    let walk_to = |c: &mut Controls, x: f32, target: f32| {
        if (target - x).abs() > 3.0 {
            c.stick.x = (target - x).signum();
        }
    };
    if story.stage == Stage::End {
        if story.t > 5.5 {
            c.any_tap = true;
        }
        return;
    }
    match h.state {
        HState::Buried => tap(c, Tool::Dig, 0.3),
        HState::Climb { wall } => {
            if WALLS[wall].kind == WallKind::Ice && !h.planted {
                tap(c, Tool::Axe, 0.25);
            }
            c.stick.y = 1.0;
        }
        HState::Rest => {
            if h.stamina > 92.0 {
                c.stick.x = 0.8;
            }
        }
        HState::Ground => {
            if h.stamina < 22.0 && story.lock <= 0.0 && story.stage != Stage::Slide && story.stage != Stage::Whumpf {
                tap(c, Tool::Rest, 0.4);
                return;
            }
            let x = h.pos.x;
            let ledge = h.ledge;
            // default: the wall that leaves this ledge
            let mut target = WALLS.iter().find(|w| (w.y0 - LEDGES[ledge].y).abs() < 0.5).map(|w| w.climb_x()).unwrap_or(FLAGS_X + 10.0);
            if ledge == 2 && auto.use_rope && !story.roped {
                target = ANCHOR_X + 6.0;
                if (x - target).abs() < 6.0 {
                    tap(c, Tool::Rope, 0.5);
                    return;
                }
            }
            if ledge == 2 && story.gust_active && !story.roped {
                return; // brace, wait it out
            }
            if ledge == 3 {
                match story.stage {
                    Stage::Whumpf | Stage::Slide if auto.shelter => target = (SHELTER.0 + SHELTER.1) / 2.0,
                    Stage::Aftermath | Stage::CoatDig => {
                        target = COAT_X + 20.0;
                        if (x - target).abs() < 8.0 {
                            tap(c, Tool::Dig, 0.35);
                            return;
                        }
                    }
                    _ => {}
                }
            }
            walk_to(c, x, target);
            if (x - target).abs() <= 3.0 {
                c.stick.y = 1.0;
            }
        }
        _ => {}
    }
}

pub fn update_tool_avail(mut avail: ResMut<ToolAvail>, hero: Query<&Hero>, story: Res<Story>) {
    avail.list.clear();
    avail.pulse = None;
    let Ok(h) = hero.single() else { return };
    let near = |x: f32, r: f32| (h.pos.x - x).abs() < r;
    match h.state {
        HState::Climb { wall } if WALLS[wall].kind == WallKind::Ice => avail.list.push(Tool::Axe),
        HState::Buried => {
            avail.list.push(Tool::Dig);
            avail.pulse = Some(Tool::Dig);
        }
        HState::Ground if story.lock <= 0.0 => {
            if WALLS.iter().any(|w| w.kind == WallKind::Ice && (w.y0 - h.pos.y).abs() < 0.5 && near(w.climb_x(), 30.0)) {
                avail.list.push(Tool::Axe);
            }
            if h.ledge == 2 && !story.roped && near(ANCHOR_X, 32.0) {
                avail.list.push(Tool::Rope);
            }
            if h.ledge == 3 && matches!(story.stage, Stage::Aftermath | Stage::CoatDig) && near(COAT_X + 20.0, 26.0) {
                avail.list.push(Tool::Dig);
                avail.pulse = Some(Tool::Dig);
            }
            if h.stamina < 70.0 && !matches!(story.stage, Stage::Whumpf | Stage::Slide) {
                avail.list.push(Tool::Rest);
            }
        }
        _ => {}
    }
}

pub fn draw_controls(
    controls: Res<Controls>,
    avail: Res<ToolAvail>,
    story: Res<Story>,
    time: Res<Time>,
    cam: Query<(&Transform, &Projection), (With<Camera2d>, Without<JoyBase>, Without<JoyKnob>, Without<ToolButton>)>,
    mut base: Query<(&mut Transform, &mut Sprite), (With<JoyBase>, Without<JoyKnob>, Without<ToolButton>)>,
    mut knob: Query<(&mut Transform, &mut Sprite), (With<JoyKnob>, Without<JoyBase>, Without<ToolButton>)>,
    mut buttons: Query<(&ToolButton, &mut Transform, &mut Visibility, &mut Sprite), (Without<JoyBase>, Without<JoyKnob>)>,
) {
    let Ok((ct, proj)) = cam.single() else { return };
    let area = view_area(proj);
    let cp = ct.translation.truncate();
    // controls fade away during the quiet story moments
    let ui_alpha = if story.hide_ui { 0.0 } else { 1.0 };
    let jc = cp + joy_center(area);
    if let Ok((mut t, mut s)) = base.single_mut() {
        t.translation = jc.extend(950.0);
        s.color = Color::srgba(1., 1., 1., 0.55 * ui_alpha);
    }
    if let Ok((mut t, mut s)) = knob.single_mut() {
        t.translation = (jc + controls.stick * JOY_R).extend(951.0);
        s.color = Color::srgba(1., 1., 1., 0.85 * ui_alpha);
    }
    for (b, mut t, mut vis, mut s) in buttons.iter_mut() {
        if let Some(slot) = avail.list.iter().position(|x| *x == b.0) {
            *vis = Visibility::Visible;
            t.translation = (cp + button_pos(area, slot)).extend(950.0);
            let pulse = if avail.pulse == Some(b.0) { 1.0 + 0.08 * (time.elapsed_secs() * 6.0).sin() } else { 1.0 };
            t.scale = Vec3::splat(pulse);
            s.color = Color::srgba(1., 1., 1., 0.9 * ui_alpha);
        } else {
            *vis = Visibility::Hidden;
        }
    }
}

