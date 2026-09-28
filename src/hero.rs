//! The climber: movement, climbing, stamina (never shown as a bar), falling and getting up.

use crate::controls::{Controls, Tool};
use crate::fx::{play, Sounds};
use crate::level::*;
use crate::story::{Stage, Story};
use bevy::prelude::*;
use bevy::sprite::Anchor;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum HState {
    Ground,
    Climb { wall: usize },
    Fall { vy: f32, to_y: f32 },
    Down { t: f32 },
    Rise { t: f32 },
    Rest,
    Knocked { t: f32, dir: f32 },
    Pause { t: f32 },
    // owned by the story
    Swept,
    Buried,
    Crouch,
    Kneel,
    Sit,
}

#[derive(Component)]
pub struct Hero {
    pub pos: Vec2,
    pub ledge: usize,
    pub state: HState,
    /// 0..100. Hidden: the body tells you (breathing, vignette, pace).
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
}

impl Default for Hero {
    fn default() -> Self {
        Hero {
            pos: Vec2::new(40.0, LEDGES[0].y),
            ledge: 0,
            state: HState::Ground,
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
        }
    }
}

#[derive(Component)]
pub struct CoatRoll;

pub const HERO_SCALE: f32 = 0.95;
const PULL: f32 = 34.0;

// atlas rows (see tools/slice.py)
const IDLE: usize = 0;
const WALK: usize = 1;
const CLIMB: usize = 2;
const AXE: usize = 3;
const DIG: usize = 5;
const EXH: usize = 6;
const FALL: usize = 7;
const INJ: usize = 8;
fn cell(row: usize, col: usize) -> usize {
    row * 5 + col
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
        Sprite {
            image: assets.load("sprites/hero.png"),
            texture_atlas: Some(TextureAtlas { layout, index: 0 }),
            anchor: Anchor::BottomCenter,
            ..default()
        },
        Transform::from_xyz(40.0, LEDGES[0].y, 10.0).with_scale(Vec3::splat(HERO_SCALE)),
    ));
    // the friend's coat, rolled on top of his pack once he carries it
    commands.spawn((
        CoatRoll,
        Sprite::from_color(Color::srgb(0.86, 0.68, 0.2), Vec2::new(17.0, 8.0)),
        Transform::from_xyz(0., 0., 10.5),
        Visibility::Hidden,
    ));
}

pub fn hero_system(
    mut commands: Commands,
    controls: Res<Controls>,
    story: Res<Story>,
    sounds: Res<Sounds>,
    time: Res<Time>,
    mut q: Query<&mut Hero>,
) {
    let Ok(mut h) = q.single_mut() else { return };
    let dt = time.delta_secs().min(0.05);
    let locked = story.lock > 0.0;
    let stick = if locked { Vec2::ZERO } else { controls.stick };
    let tapped = |t: Tool| !locked && controls.taps.contains(&t);
    // altitude and fatigue both slow him down
    let perf = (0.45 + 0.55 * (h.stamina / 100.0)) * (1.0 - h.pos.y / 4000.0);
    h.moving = false;
    h.axe_cd -= dt;

    match h.state {
        HState::Ground => {
            let l = LEDGES[h.ledge];
            if tapped(Tool::Rest) {
                h.state = HState::Rest;
                return;
            }
            let mut speed = 64.0 * perf;
            if story.roped {
                speed *= 0.65; // slower, safer
            }
            if story.stage == Stage::Slide {
                speed *= 1.9; // panic
            }
            let exposed = h.ledge == 2 && story.gust_active;
            if exposed && stick.x.abs() > 0.2 {
                if story.roped {
                    speed *= 0.4; // the rope holds him
                } else {
                    // moving upright in a gust on an exposed ledge: it takes him
                    h.state = HState::Knocked { t: 0.0, dir: -stick.x.signum() };
                    play(&mut commands, &sounds.crunch, 0.6);
                    return;
                }
            }
            if stick.x.abs() > 0.15 {
                h.pos.x += stick.x * speed * dt;
                h.facing = stick.x.signum();
                h.moving = true;
                h.anim_t += dt * perf;
            }
            let mut min_x = l.x0 + 10.0;
            if h.ledge == 3 {
                min_x = min_x.max(story.min_x);
            }
            h.pos.x = h.pos.x.clamp(min_x, l.x1 - 10.0);
            let regen = if h.moving { 3.0 } else { 9.0 };
            h.stamina = (h.stamina + regen * dt).min(100.0);

            // start a climb from the bottom of a wall
            for (i, w) in WALLS.iter().enumerate() {
                if (w.y0 - l.y).abs() < 0.5 && (h.pos.x - w.climb_x()).abs() < 16.0 {
                    let axe = w.kind == WallKind::Ice && tapped(Tool::Axe);
                    if stick.y > 0.55 || axe {
                        h.state = HState::Climb { wall: i };
                        h.pos.x = w.climb_x();
                        h.pos.y += 1.0;
                        h.facing = w.side;
                        if axe {
                            plant(&mut h, &mut commands, &sounds);
                        }
                        return;
                    }
                }
                // or climb back down from the top
                if (w.y1 - l.y).abs() < 0.5 && (h.pos.x - w.climb_x()).abs() < 20.0 && stick.y < -0.6 {
                    h.state = HState::Climb { wall: i };
                    h.pos = Vec2::new(w.climb_x(), w.y1 - 2.0);
                    h.facing = w.side;
                    return;
                }
            }
        }
        HState::Climb { wall } => {
            let w = WALLS[wall];
            h.facing = w.side;
            let mut dy = 0.0;
            match w.kind {
                WallKind::Rock => {
                    if stick.y > 0.2 {
                        dy = stick.y * 34.0 * perf * dt;
                        h.stamina -= 8.0 * stick.y * dt;
                    } else {
                        h.stamina -= 1.5 * dt; // just hanging on
                    }
                }
                WallKind::Ice => {
                    if tapped(Tool::Axe) && !h.planted && h.axe_cd <= 0.0 {
                        plant(&mut h, &mut commands, &sounds);
                    }
                    if h.planted {
                        h.stamina -= 2.0 * dt;
                        if stick.y > 0.3 {
                            let step = (58.0 * perf * dt).min(h.pull_left);
                            dy = step;
                            h.pull_left -= step;
                            if h.pull_left <= 0.0 {
                                // plant → pull → recover
                                h.planted = false;
                                h.axe_cd = 0.22;
                                h.stamina -= 4.5;
                            }
                        }
                    } else {
                        // crampons alone won't hold on steep ice: he creeps down
                        dy = -10.0 * dt;
                        h.stamina -= 3.0 * dt;
                    }
                }
            }
            if stick.y < -0.5 && !h.planted {
                dy = -42.0 * dt;
            }
            h.pos.y += dy;
            if dy.abs() > 0.0001 && dy > 0.0 {
                h.moving = true;
                h.anim_t += dt;
            }
            if h.pos.y >= w.y1 {
                h.pos = Vec2::new(w.climb_x() - w.side * 18.0, w.y1);
                h.ledge = ledge_at(w.y1).unwrap_or(h.ledge);
                h.planted = false;
                h.state = HState::Ground;
                h.facing = -w.side;
            } else if h.pos.y <= w.y0 {
                h.pos.y = w.y0;
                h.ledge = ledge_at(w.y0).unwrap_or(h.ledge);
                h.planted = false;
                h.state = HState::Ground;
            } else if h.stamina <= 0.0 {
                h.planted = false;
                h.state = HState::Fall { vy: 30.0, to_y: w.y0 };
            }
        }
        HState::Fall { vy, to_y } => {
            let vy = vy - 620.0 * dt;
            h.pos.y += vy * dt;
            h.pos.x -= h.facing * 12.0 * dt; // pushed off the face
            if h.pos.y <= to_y {
                h.pos.y = to_y;
                h.ledge = ledge_at(to_y).unwrap_or(h.ledge);
                h.state = HState::Down { t: 0.0 };
                h.falls += 1;
                h.just_fell = true;
                h.stamina = 0.0;
                play(&mut commands, &sounds.whumpf, 0.5);
                play(&mut commands, &sounds.crunch, 0.8);
            } else {
                h.state = HState::Fall { vy, to_y };
            }
        }
        HState::Down { t } => {
            h.stamina += 6.0 * dt;
            h.state = if t > 2.4 { HState::Rise { t: 0.0 } } else { HState::Down { t: t + dt } };
        }
        HState::Rise { t } => {
            if t > 1.3 {
                h.state = HState::Ground;
                h.stamina = h.stamina.max(45.0);
            } else {
                h.state = HState::Rise { t: t + dt };
            }
        }
        HState::Rest => {
            h.stamina = (h.stamina + 22.0 * dt).min(100.0);
            if stick.length() > 0.4 {
                h.state = HState::Ground;
            }
        }
        HState::Knocked { t, dir } => {
            let l = LEDGES[h.ledge];
            h.pos.x = (h.pos.x + dir * 150.0 * (1.0 - t / 0.8).max(0.0) * dt).clamp(l.x0 + 10.0, l.x1 - 10.0);
            h.state = if t > 0.8 { HState::Down { t: 1.2 } } else { HState::Knocked { t: t + dt, dir } };
            if t > 0.8 {
                h.just_fell = true;
                h.falls += 1;
                h.stamina = (h.stamina - 25.0).max(0.0);
            }
        }
        HState::Pause { t } => {
            h.state = if t <= 0.0 { HState::Ground } else { HState::Pause { t: t - dt } };
        }
        HState::Swept | HState::Buried | HState::Crouch | HState::Kneel | HState::Sit => {
            h.stamina = (h.stamina + 4.0 * dt).min(100.0);
        }
    }
    h.stamina = h.stamina.clamp(0.0, 100.0);
}

fn plant(h: &mut Hero, commands: &mut Commands, sounds: &Sounds) {
    h.planted = true;
    h.pull_left = PULL;
    play(commands, &sounds.chink, 0.55);
}

pub fn animate_hero(
    time: Res<Time>,
    mut q: Query<(&Hero, &mut Sprite, &mut Transform, &mut Visibility), Without<CoatRoll>>,
    mut roll: Query<(&mut Transform, &mut Visibility), With<CoatRoll>>,
) {
    let Ok((h, mut s, mut t, mut vis)) = q.single_mut() else { return };
    let f = |rate: f32, n: usize| ((h.anim_t * rate) as usize) % n;
    let idx = match h.state {
        HState::Ground if h.moving => cell(WALK, f(9.0, 5)),
        HState::Ground if h.stamina < 30.0 => cell(EXH, 0),
        HState::Ground => cell(IDLE, 0),
        HState::Climb { wall } => {
            if WALLS[wall].kind == WallKind::Ice && h.planted {
                cell(AXE, (((PULL - h.pull_left) / PULL) * 4.0).round() as usize)
            } else {
                cell(CLIMB, f(6.0, 5))
            }
        }
        HState::Fall { vy, .. } => cell(FALL, if vy > -120.0 { 0 } else { 1 }),
        HState::Down { .. } => cell(INJ, 3),
        HState::Rise { t } => cell(INJ, 3 - ((t / 1.3) * 3.0).min(3.0) as usize),
        HState::Rest => cell(EXH, 2),
        HState::Knocked { t, .. } => cell(FALL, ((t / 0.8) * 3.0).min(3.0) as usize),
        HState::Pause { .. } => cell(EXH, 3),
        HState::Swept => cell(FALL, 1 + ((time.elapsed_secs() * 8.0) as usize % 2)),
        HState::Buried => cell(FALL, 3),
        HState::Crouch => cell(DIG, 0),
        HState::Kneel => cell(EXH, 3),
        HState::Sit => cell(EXH, 2),
    };
    if let Some(a) = s.texture_atlas.as_mut() {
        a.index = idx;
    }
    s.flip_x = h.facing < 0.0;
    // idle breathing: tiny, and heavier when he's spent
    let breathe = if matches!(h.state, HState::Ground | HState::Rest | HState::Sit | HState::Kneel) && !h.moving {
        let rate = 1.2 + (1.0 - h.stamina / 100.0) * 2.5;
        1.0 + 0.012 * (time.elapsed_secs() * rate * std::f32::consts::TAU).sin()
    } else {
        1.0
    };
    t.translation = Vec3::new(h.pos.x, h.pos.y, 10.0);
    t.scale = Vec3::new(HERO_SCALE, HERO_SCALE * breathe, 1.0);
    *vis = if h.state == HState::Buried { Visibility::Hidden } else { Visibility::Visible };

    if let Ok((mut rt, mut rv)) = roll.single_mut() {
        let show = h.carrying_coat && !matches!(h.state, HState::Buried | HState::Down { .. } | HState::Fall { .. } | HState::Knocked { .. });
        *rv = if show { Visibility::Visible } else { Visibility::Hidden };
        let up = if matches!(h.state, HState::Rest | HState::Sit | HState::Kneel | HState::Pause { .. }) { 38.0 } else { 56.0 };
        rt.translation = Vec3::new(h.pos.x - h.facing * 9.0, h.pos.y + up * HERO_SCALE * breathe, 10.5);
    }
}
