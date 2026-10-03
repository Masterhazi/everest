//! The climber. Moves along the mountain path; climbs faces; slides when he falls and
//! keeps sliding until friction (or his axe, or his rope) stops him.
//! All five tools are always available — what they do depends on where he is.

use crate::controls::{Controls, Tool};
use crate::fx::{play, Sounds};
use crate::hazards::Hazards;
use crate::skill::Skill;
use crate::story::{Stage, Story};
use crate::terrain::{Surface, Terrain, Zone};
use bevy::prelude::*;
use bevy::sprite::Anchor;
use rand::Rng;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Act {
    Probe,
    Scoop,
    Clank,
    Fumble,
    Lift,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum HState {
    Move,
    Slide { v: f32, peak: f32 },
    Down { t: f32, dur: f32 },
    Rise { t: f32 },
    Rest,
    Pause { t: f32, act: Act },
    /// fell through a snow bridge; axe your way out
    Crevasse { left: i32 },
    Buried,
    // owned by the story
    Kneel,
    Sit,
    Crouch,
}

#[derive(Component)]
pub struct Hero {
    pub s: f32,
    pub pos: Vec2,
    pub state: HState,
    /// 0..100, never shown — breathing, vignette and pace carry it
    pub stamina: f32,
    pub facing: f32,
    pub planted: bool,
    pub pull_left: f32,
    pub axe_cd: f32,
    pub anim_t: f32,
    pub moving: bool,
    pub falls: u32,
    pub just_fell: bool,
    pub carrying_coat: bool,
    /// sideways shift while on a face (dodging rocks)
    pub lean: f32,
    pub arrest: bool,
    /// clipped to the anchor at this arc-length
    pub rope: Option<f32>,
    pub lamp: bool,
    pub battery: f32,
    pub dig_left: i32,
    pub max_s: f32,
    /// set when a tool hits the wrong surface: fx draws sparks / chips
    pub sparks: f32,
}

impl Default for Hero {
    fn default() -> Self {
        Hero {
            s: 30.0,
            pos: Vec2::ZERO,
            state: HState::Move,
            stamina: 100.0,
            facing: 1.0,
            planted: false,
            pull_left: 0.0,
            axe_cd: 0.0,
            anim_t: 0.0,
            moving: false,
            falls: 0,
            just_fell: false,
            carrying_coat: false,
            lean: 0.0,
            arrest: false,
            rope: None,
            lamp: false,
            battery: 100.0,
            dig_left: 0,
            max_s: 0.0,
            sparks: 0.0,
        }
    }
}

impl Hero {
    pub fn on_ground(&self) -> bool {
        self.state == HState::Move
    }
    pub fn start_slide(&mut self, v: f32) {
        self.planted = false;
        self.arrest = false;
        self.state = HState::Slide { v, peak: v.abs() };
    }
}

#[derive(Component)]
pub struct CoatRoll;

pub const HERO_SCALE: f32 = 0.95;
const PULL: f32 = 34.0;
const G: f32 = 520.0;

// atlas rows (see tools/slice.py)
const IDLE: usize = 0;
const WALK: usize = 1;
const CLIMB: usize = 2;
const AXE: usize = 3;
const ROPE: usize = 4;
const DIG: usize = 5;
const EXH: usize = 6;
const FALL: usize = 7;
const INJ: usize = 8;
fn cell(row: usize, col: usize) -> usize {
    row * 5 + col.min(4)
}

pub struct HeroPlugin;
impl Plugin for HeroPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_hero);
    }
}

fn spawn_hero(mut commands: Commands, assets: Res<AssetServer>, mut layouts: ResMut<Assets<TextureAtlasLayout>>) {
    let layout = layouts.add(TextureAtlasLayout::from_grid(UVec2::new(96, 88), 5, 9, None, None));
    commands.spawn((
        Hero::default(),
        Sprite { image: assets.load("sprites/hero.png"), texture_atlas: Some(TextureAtlas { layout, index: 0 }), anchor: Anchor::BottomCenter, ..default() },
        Transform::from_xyz(0.0, 0.0, 10.0).with_scale(Vec3::splat(HERO_SCALE)),
    ));
    commands.spawn((CoatRoll, Sprite::from_color(Color::srgb(0.86, 0.68, 0.2), Vec2::new(17.0, 8.0)), Transform::from_xyz(0., 0., 10.5), Visibility::Hidden));
}

fn friction(surf: Surface) -> f32 {
    match surf {
        Surface::Snow => 0.62,
        Surface::Ice => 0.12,
        Surface::Rock => 0.45,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn hero_system(
    mut commands: Commands,
    controls: Res<Controls>,
    story: Res<Story>,
    terrain: Res<Terrain>,
    sounds: Res<Sounds>,
    mut skill: ResMut<Skill>,
    mut hz: ResMut<Hazards>,
    time: Res<Time>,
    mut q: Query<&mut Hero>,
) {
    let Ok(mut h) = q.single_mut() else { return };
    let dt = time.delta_secs().min(0.05);
    let locked = story.lock > 0.0;
    let mut stick = if locked { Vec2::ZERO } else { controls.stick };
    // altitude: for a while his body lags behind his intentions
    if hz.dizzy > 0.0 {
        let wob = (time.elapsed_secs() * 2.3).sin() * 0.35 * hz.dizzy.min(1.0);
        stick = stick * (1.0 - 0.45 * hz.dizzy.min(1.0)) + Vec2::new(wob, 0.0) * stick.length();
    }
    let tapped = |t: Tool| !locked && controls.taps.contains(&t);
    let diff = skill.diff;
    let drain = 0.85 + 0.3 * diff;
    let perf = (0.45 + 0.55 * (h.stamina / 100.0)) * (1.0 - h.pos.y / 5000.0);
    h.moving = false;
    h.axe_cd -= dt;
    h.sparks = (h.sparks - dt).max(0.0);

    // ---- tools that work anywhere
    if tapped(Tool::Lamp) {
        if h.battery > 0.0 {
            h.lamp = !h.lamp;
        }
        play(&mut commands, &sounds.chink, 0.15);
    }
    if h.lamp {
        h.battery = (h.battery - 1.1 * dt).max(0.0);
        if h.battery <= 0.0 {
            h.lamp = false;
        }
    }
    if let Some(a) = h.rope {
        if h.s - a > 320.0 {
            h.rope = None; // rope runs out; he unclips
        }
    }

    let g = *terrain.seg(h.s);
    let ice_face = g.wall && g.surf == Surface::Ice;
    let rock_face = g.wall && g.surf == Surface::Rock;

    match h.state {
        HState::Move => {
            if tapped(Tool::Rope) {
                if let Some(&a) = terrain.anchors.iter().find(|&&a| (h.s - a).abs() < 26.0) {
                    h.rope = Some(a);
                    play(&mut commands, &sounds.chink, 0.5);
                } else {
                    h.state = HState::Pause { t: 0.6, act: Act::Fumble };
                    play(&mut commands, &sounds.crunch, 0.25);
                    return;
                }
            }
            if tapped(Tool::Rest) && !g.wall {
                h.state = HState::Rest;
                return;
            }
            if tapped(Tool::Dig) {
                if g.surf == Surface::Snow && !g.wall {
                    h.state = HState::Pause { t: 0.5, act: Act::Scoop };
                    play(&mut commands, &sounds.crunch, 0.5);
                } else {
                    h.state = HState::Pause { t: 0.35, act: Act::Clank };
                    h.sparks = 0.3;
                    play(&mut commands, &sounds.chink, 0.35);
                }
                return;
            }
            if tapped(Tool::Axe) && h.axe_cd <= 0.0 {
                // what the axe does depends entirely on what it hits
                let ahead = terrain.seg(h.s + 12.0);
                if ice_face || (!g.wall && ahead.wall && ahead.surf == Surface::Ice) {
                    if !g.wall {
                        h.s = ahead.s0 + 1.0;
                    }
                    if !h.planted {
                        h.planted = true;
                        h.pull_left = PULL;
                        play(&mut commands, &sounds.chink, 0.55);
                    }
                } else if rock_face {
                    // skids off rock: sparks, a lurch, lost grip
                    h.sparks = 0.5;
                    h.s -= 10.0;
                    h.stamina -= 5.0;
                    h.axe_cd = 0.4;
                    play(&mut commands, &sounds.chink, 0.7);
                } else {
                    // probing the snow in front of him
                    let (b0, _) = hz.bridge;
                    if g.zone == Zone::Crevasse && h.s < b0 && b0 - h.s < 45.0 && !hz.bridge_probed {
                        hz.bridge_probed = true;
                        skill.event(0.06);
                    }
                    h.state = HState::Pause { t: 0.45, act: Act::Probe };
                    h.axe_cd = 0.3;
                    play(&mut commands, &sounds.crunch, 0.3);
                    return;
                }
            }

            // ---- movement along the path
            let mut ds = 0.0;
            if g.wall {
                h.facing = 1.0;
                if rock_face {
                    h.lean += ((stick.x * 14.0) - h.lean) * (dt * 10.0).min(1.0);
                    if stick.y > 0.2 {
                        ds = stick.y * 34.0 * perf * dt;
                        h.stamina -= 8.0 * stick.y * drain * dt;
                    } else {
                        h.stamina -= 1.5 * drain * dt;
                    }
                } else {
                    h.lean *= 1.0 - (dt * 8.0).min(1.0);
                    if h.planted {
                        h.stamina -= 2.0 * drain * dt;
                        if stick.y > 0.3 {
                            let step = (58.0 * perf * dt).min(h.pull_left);
                            ds = step;
                            h.pull_left -= step;
                            if h.pull_left <= 0.0 {
                                h.planted = false; // plant → pull → recover
                                h.axe_cd = 0.22;
                                h.stamina -= 4.5 * drain;
                            }
                        }
                    } else if h.s > g.s0 + 1.0 || terrain.seg(h.s - 2.0).wall {
                        ds = -10.0 * dt; // crampons alone won't hold steep ice
                        h.stamina -= 3.0 * drain * dt;
                    }
                }
                if stick.y < -0.5 && !h.planted {
                    ds = -42.0 * dt;
                }
            } else {
                h.lean = 0.0;
                let input = if stick.length() > 0.2 { stick.dot(g.dir) / stick.length().max(0.001) * stick.length() } else { 0.0 };
                if input.abs() > 0.15 {
                    let slope = 1.0 - g.deg.max(0.0) / 110.0;
                    let mut speed = 64.0 * perf * slope;
                    if story.stage == Stage::AvSlide {
                        speed *= 1.9; // panic
                    }
                    if g.zone == Zone::Crevasse && hz.bridge_probed && h.s > hz.bridge.0 - 4.0 && h.s < hz.bridge.1 {
                        speed *= 0.5; // stepping on the solid edge
                    }
                    ds = input * speed * dt;
                    h.facing = input.signum();
                    h.stamina -= 3.0 * (g.deg.max(0.0) / 35.0) * drain * dt * (input > 0.0) as i32 as f32;
                    // tired legs on steep snow: a slip
                    if g.deg >= 25.0 && h.stamina < 25.0 {
                        let p = (0.15 + 0.5 * diff) * (1.0 - h.stamina / 25.0) * dt;
                        if rand::thread_rng().gen_bool(p.clamp(0.0, 1.0) as f64) {
                            h.start_slide(-60.0);
                            skill.event(-0.04);
                            return;
                        }
                    }
                } else {
                    h.stamina += 9.0 * dt;
                }
                if ds.abs() > 0.0 {
                    h.stamina += 3.0 * dt * (g.deg < 10.0) as i32 as f32;
                }
            }
            if ds.abs() > 0.0001 {
                h.moving = true;
                h.anim_t += dt * perf;
            }
            h.s = (h.s + ds).clamp(5.0, story.gate.min(terrain.total - 5.0));
            if g.wall && h.stamina <= 0.0 {
                h.start_slide(-20.0);
                skill.event(-0.07);
            }
        }
        HState::Slide { mut v, mut peak } => {
            if tapped(Tool::Axe) {
                h.arrest = true;
            }
            let th = g.deg.to_radians();
            let mut mu = friction(g.surf);
            if h.arrest {
                if g.surf == Surface::Snow && !g.wall {
                    mu += 1.6 * (1.0 - (v.abs() / 420.0).min(0.75)); // late = less bite
                } else {
                    mu += 0.08; // ice and rock don't take the pick
                    h.sparks = 0.2;
                }
            }
            v += -G * th.sin() * dt;
            let fr = mu * G * th.cos() * dt;
            if v < 0.0 {
                v = (v + fr).min(0.0);
            } else {
                v = (v - fr).max(0.0);
            }
            peak = peak.max(v.abs());
            h.s += v * dt;
            // the rope catches him
            if let Some(a) = h.rope {
                if h.s < a - 12.0 {
                    h.s = a - 12.0;
                    v = 0.0;
                    play(&mut commands, &sounds.chink, 0.7);
                }
            }
            if h.s <= 5.0 {
                h.s = 5.0;
                v = 0.0;
            }
            let holds = mu * th.cos() >= th.sin();
            if v.abs() < 6.0 && holds {
                let hard = peak > 70.0;
                // a self-arrest only counts if the slope wouldn't have stopped him on its own
                let base_holds = friction(g.surf) * th.cos() >= th.sin();
                if h.arrest && g.surf == Surface::Snow && !base_holds && peak > 40.0 {
                    skill.event(0.07);
                }
                h.arrest = false;
                if hard {
                    h.falls += 1;
                    h.just_fell = true;
                    h.stamina = (h.stamina - peak / 12.0).max(0.0);
                    let dur = (0.8 + peak / 300.0 * 2.0).min(3.5);
                    h.state = HState::Down { t: 0.0, dur };
                    play(&mut commands, &sounds.whumpf, 0.4);
                    play(&mut commands, &sounds.crunch, 0.8);
                } else {
                    h.state = HState::Rise { t: 0.6 };
                }
            } else {
                h.state = HState::Slide { v, peak };
                h.anim_t += dt;
            }
        }
        HState::Down { t, dur } => {
            h.stamina += 6.0 * dt;
            h.state = if t > dur { HState::Rise { t: 0.0 } } else { HState::Down { t: t + dt, dur } };
        }
        HState::Rise { t } => {
            if t > 1.3 {
                h.state = HState::Move;
                h.stamina = h.stamina.max(40.0);
            } else {
                h.state = HState::Rise { t: t + dt };
            }
        }
        HState::Rest => {
            h.stamina = (h.stamina + 22.0 * dt).min(100.0);
            if stick.length() > 0.4 {
                h.state = HState::Move;
            }
        }
        HState::Pause { t, act } => {
            h.state = if t <= 0.0 { HState::Move } else { HState::Pause { t: t - dt, act } };
        }
        HState::Crevasse { left } => {
            h.stamina = (h.stamina + 2.0 * dt).min(100.0);
            if tapped(Tool::Axe) && h.axe_cd <= 0.0 {
                h.axe_cd = 0.25;
                play(&mut commands, &sounds.chink, 0.5);
                if left <= 1 {
                    h.s = hz.bridge.1 + 6.0;
                    h.state = HState::Rise { t: 0.5 };
                    hz.bridge_probed = true; // the hole is open now; there's a lip to cross on
                } else {
                    h.state = HState::Crevasse { left: left - 1 };
                }
            } else if tapped(Tool::Dig) || tapped(Tool::Rope) {
                h.sparks = 0.3;
                play(&mut commands, &sounds.chink, 0.25);
            }
        }
        HState::Buried => {
            if tapped(Tool::Dig) {
                h.dig_left -= 1;
                play(&mut commands, &sounds.crunch, 0.9);
                if h.dig_left <= 0 {
                    h.state = HState::Rise { t: 0.0 };
                }
            }
        }
        HState::Kneel | HState::Sit | HState::Crouch => {
            h.stamina = (h.stamina + 4.0 * dt).min(100.0);
        }
    }
    h.stamina = h.stamina.clamp(0.0, 100.0);
    h.max_s = h.max_s.max(h.s);

    // where his feet are in the world
    let g = *terrain.seg(h.s);
    let mut p = terrain.point(h.s);
    if g.wall {
        p.x += -7.0 + h.lean;
    }
    if let HState::Crevasse { left } = h.state {
        p = terrain.point((hz.bridge.0 + hz.bridge.1) / 2.0) - Vec2::Y * (8.0 + 6.0 * left as f32);
    }
    h.pos = p;
}

pub fn animate_hero(
    time: Res<Time>,
    terrain: Res<Terrain>,
    mut q: Query<(&Hero, &mut Sprite, &mut Transform, &mut Visibility), Without<CoatRoll>>,
    mut roll: Query<(&mut Transform, &mut Visibility), With<CoatRoll>>,
) {
    let Ok((h, mut s, mut t, mut vis)) = q.single_mut() else { return };
    let f = |rate: f32, n: usize| ((h.anim_t * rate) as usize) % n;
    let g = terrain.seg(h.s);
    let idx = match h.state {
        HState::Move if g.wall => {
            if g.surf == Surface::Ice && h.planted {
                cell(AXE, (((PULL - h.pull_left) / PULL) * 4.0).round() as usize)
            } else {
                cell(CLIMB, f(6.0, 5))
            }
        }
        HState::Move if h.moving => cell(WALK, f(9.0, 5)),
        HState::Move if h.stamina < 30.0 => cell(EXH, 0),
        HState::Move => cell(IDLE, 0),
        HState::Slide { v, .. } => {
            if h.arrest {
                cell(INJ, 2)
            } else {
                cell(FALL, if v.abs() < 120.0 { 1 } else { 2 })
            }
        }
        HState::Down { .. } => cell(INJ, 3),
        HState::Rise { t } => cell(INJ, 3 - ((t / 1.3) * 3.0).min(3.0) as usize),
        HState::Rest => cell(EXH, 2),
        HState::Pause { act, .. } => match act {
            Act::Probe => cell(AXE, 4),
            Act::Scoop => cell(DIG, 1),
            Act::Clank => cell(DIG, 2),
            Act::Fumble => cell(ROPE, 2),
            Act::Lift => cell(EXH, 3),
        },
        HState::Crevasse { .. } => cell(CLIMB, ((time.elapsed_secs() * 3.0) as usize) % 5),
        HState::Buried => cell(FALL, 3),
        HState::Crouch => cell(DIG, 0),
        HState::Kneel => cell(EXH, 3),
        HState::Sit => cell(EXH, 2),
    };
    if let Some(a) = s.texture_atlas.as_mut() {
        a.index = idx;
    }
    s.flip_x = h.facing < 0.0;
    let still = matches!(h.state, HState::Move | HState::Rest | HState::Sit | HState::Kneel) && !h.moving;
    let breathe = if still {
        let rate = 1.2 + (1.0 - h.stamina / 100.0) * 2.5;
        1.0 + 0.012 * (time.elapsed_secs() * rate * std::f32::consts::TAU).sin()
    } else {
        1.0
    };
    t.translation = Vec3::new(h.pos.x, h.pos.y, 10.0);
    t.scale = Vec3::new(HERO_SCALE, HERO_SCALE * breathe, 1.0);
    *vis = if h.state == HState::Buried { Visibility::Hidden } else { Visibility::Visible };

    if let Ok((mut rt, mut rv)) = roll.single_mut() {
        let show = h.carrying_coat && !matches!(h.state, HState::Buried | HState::Down { .. } | HState::Slide { .. } | HState::Crevasse { .. });
        *rv = if show { Visibility::Visible } else { Visibility::Hidden };
        let up = if matches!(h.state, HState::Rest | HState::Sit | HState::Kneel | HState::Pause { .. }) { 38.0 } else { 56.0 };
        rt.translation = Vec3::new(h.pos.x - h.facing * 9.0, h.pos.y + up * HERO_SCALE * breathe, 10.5);
    }
}
