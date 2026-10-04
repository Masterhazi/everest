//! Nature as the antagonist: altitude spells, a hidden crevasse, rockfall, icefall,
//! night, and avalanches that come from conditions, not a script.
//! Every danger announces itself — the player's job is to learn to read it.

use crate::fx::{play, Mood, Sounds};
use crate::hero::{HState, Hero};
use crate::hints::{Cause, HintLog};
use crate::fx::GameCam;
use crate::skill::Skill;
use crate::story::Story;
use crate::terrain::{Terrain, Zone};
use crate::LevelEntity;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use rand::Rng;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Avalanche {
    pub part: usize,
    pub t: f32,
    pub warn: f32,
    pub front: f32,
    pub story: bool,
    pub hit: bool,
    pub sheltered: bool,
}

#[derive(Resource)]
pub struct Hazards {
    /// 0..1 strength of the current altitude spell
    pub dizzy: f32,
    dizzy_t: f32,
    bands: Vec<(f32, bool)>,
    /// hidden snow bridge (arc-length range)
    pub bridge: (f32, f32),
    pub bridge_probed: bool,
    pub bridge_broken: bool,
    on_bridge: bool,
    rock_timer: f32,
    pub rock_warn: f32,
    pub rock_lane: f32,
    /// a stone is in the air (seconds left)
    pub rock_danger: f32,
    ice_timer: f32,
    pub ice_warn: f32,
    pub ice_target: f32,
    ice_impact: f32,
    pub av: Option<Avalanche>,
    av_cool: f32,
    pub bury_pending: bool,
    /// 0 day .. 1 full night
    pub night: f32,
    pub dawn: f32,
    dark_t: f32,
}

impl Hazards {
    pub fn new(t: &Terrain) -> Self {
        let (b0, _) = t.part_range(8);
        let bands = [5, 11, 15].iter().map(|&p| (t.point(t.part_s[p]).y, false)).collect();
        Hazards {
            dizzy: 0.0,
            dizzy_t: 0.0,
            bands,
            bridge: (b0 + 55.0, b0 + 105.0),
            bridge_probed: false,
            bridge_broken: false,
            on_bridge: false,
            rock_timer: 1.5,
            rock_warn: 0.0,
            rock_lane: 0.0,
            rock_danger: 0.0,
            ice_timer: 2.5,
            ice_warn: 0.0,
            ice_target: 0.0,
            ice_impact: 0.0,
            av: None,
            av_cool: 0.0,
            bury_pending: false,
            night: 0.0,
            dawn: 0.0,
            dark_t: 0.0,
        }
    }
    pub fn avalanche_warning(&self) -> bool {
        self.av.is_some_and(|a| a.t < a.warn)
    }
    pub fn avalanche_running(&self) -> bool {
        self.av.is_some_and(|a| a.t >= a.warn)
    }
    pub fn start_avalanche(&mut self, t: &Terrain, part: usize, story: bool, warn: f32) {
        let (_, p1) = t.part_range(part);
        self.av = Some(Avalanche { part, t: 0.0, warn, front: p1 + 30.0, story, hit: false, sheltered: false });
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum FxKind {
    Pebble,
    Rock { dodged: bool },
    Ice,
    Spark,
    Spray,
}
#[derive(Component)]
pub struct Particle {
    pub vel: Vec2,
    pub life: f32,
    pub kind: FxKind,
}

#[derive(Component)]
pub struct Serac {
    s: f32,
    base: Vec2,
}
#[derive(Component)]
pub struct BridgeMark(u8); // 0 = sag, 1 = probed cracks, 2 = open hole
#[derive(Component)]
pub struct RandomMass;
#[derive(Component)]
pub struct AvCrack;
#[derive(Component)]
pub struct Boulder;
#[derive(Component)]
pub struct LampMask;

pub const BOULDER_OFFSET: f32 = 130.0; // within part 10

pub struct HazardsPlugin;
impl Plugin for HazardsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_hazards_startup);
    }
}

fn spawn_hazards_startup(mut commands: Commands, terrain: Res<Terrain>, assets: Res<AssetServer>) {
    commands.insert_resource(Hazards::new(&terrain));
    spawn_hazard_props(&mut commands, &terrain, &assets);
}

pub fn spawn_hazard_props(commands: &mut Commands, t: &Terrain, assets: &AssetServer) {
    // ice towers leaning over the route
    let (s0, s1) = t.part_range(12);
    let mut s = s0 + 30.0;
    let mut rng = rand::thread_rng();
    while s < s1 - 20.0 {
        let p = t.point(s);
        let h = rng.gen_range(95.0..150.0);
        let base = p + Vec2::new(rng.gen_range(8.0..22.0), -6.0); // standing on the slope, looming over the route
        commands.spawn((
            LevelEntity,
            Serac { s, base },
            Sprite { image: assets.load(format!("sprites/serac{}.png", rng.gen_range(0..3))), custom_size: Some(Vec2::new(h * 0.38, h)), anchor: Anchor::BottomCenter, ..default() },
            Transform::from_translation(base.extend(3.5)).with_rotation(Quat::from_rotation_z(rng.gen_range(0.04..0.16))),
        ));
        s += rng.gen_range(45.0..70.0);
    }
    // the snow bridge: a faint sag in the snow. Probing shows cracks; falling through leaves a hole.
    let hz = Hazards::new(t);
    let mid = t.point((hz.bridge.0 + hz.bridge.1) / 2.0);
    let len = hz.bridge.1 - hz.bridge.0;
    let ang = t.seg(hz.bridge.0 + 1.0).dir;
    let rot = Quat::from_rotation_z(ang.y.atan2(ang.x));
    for (k, color, h) in [(0u8, Color::srgba(0.75, 0.82, 0.95, 0.55), 5.0), (1, Color::srgba(0.12, 0.16, 0.28, 0.0), 3.0), (2, Color::srgba(0.04, 0.06, 0.12, 0.0), 48.0)] {
        commands.spawn((
            LevelEntity,
            BridgeMark(k),
            Sprite::from_color(color, Vec2::new(len, h)),
            Transform::from_translation((mid - Vec2::Y * (h / 2.0 - 1.0)).extend(3.6 + k as f32 * 0.01)).with_rotation(rot),
        ));
    }
    // the boulder on the avalanche slope (its downhill side is the only shelter)
    let (a0, _) = t.part_range(10);
    commands.spawn((
        LevelEntity,
        Boulder,
        Sprite { image: assets.load("sprites/boulder.png"), anchor: Anchor::BottomCenter, ..default() },
        Transform::from_translation((t.point(a0 + BOULDER_OFFSET) - Vec2::new(0.0, 8.0)).extend(7.0)),
    ));
    commands.spawn((
        LevelEntity,
        RandomMass,
        Sprite { image: assets.load("sprites/avalanche.png"), custom_size: Some(Vec2::new(230.0, 184.0)), ..default() },
        Transform::from_xyz(0., 0., 12.0),
        Visibility::Hidden,
    ));
    commands.spawn((LevelEntity, AvCrack, Sprite::from_image(assets.load("sprites/crack.png")), Transform::from_xyz(0., 0., 3.7), Visibility::Hidden));
}

fn spawn_particle(commands: &mut Commands, p: Vec2, vel: Vec2, life: f32, kind: FxKind) {
    let (size, color) = match kind {
        FxKind::Pebble => (Vec2::splat(2.0), Color::srgb(0.45, 0.45, 0.5)),
        FxKind::Rock { .. } => (Vec2::new(11.0, 9.0), Color::srgb(0.28, 0.27, 0.3)),
        FxKind::Ice => (Vec2::new(10.0, 9.0), Color::srgb(0.75, 0.88, 1.0)),
        FxKind::Spark => (Vec2::splat(2.0), Color::srgb(1.0, 0.85, 0.4)),
        FxKind::Spray => (Vec2::splat(2.0), Color::srgb(0.95, 0.97, 1.0)),
    };
    commands.spawn((LevelEntity, Particle { vel, life, kind }, Sprite::from_color(color, size), Transform::from_translation(p.extend(13.0))));
}

#[allow(clippy::too_many_arguments)]
pub fn hazards_system(
    mut commands: Commands,
    mut hz: ResMut<Hazards>,
    terrain: Res<Terrain>,
    story: Res<Story>,
    sounds: Res<Sounds>,
    mut skill: ResMut<Skill>,
    mut hints: ResMut<HintLog>,
    mut mood: ResMut<Mood>,
    time: Res<Time>,
    cam: Query<&Transform, (With<GameCam>, Without<Particle>, Without<Serac>, Without<Hero>)>,
    mut hq: Query<&mut Hero>,
    mut parts: Query<(Entity, &mut Particle, &mut Transform), (Without<Serac>, Without<Hero>)>,
    mut seracs: Query<(&Serac, &mut Transform), (Without<Particle>, Without<Hero>)>,
) {
    let Ok(mut h) = hq.single_mut() else { return };
    let dt = time.delta_secs().min(0.05);
    let diff = skill.diff;
    let cam_y = cam.single().map(|c| c.translation.y).unwrap_or(h.pos.y);
    let mut rng = rand::thread_rng();
    let part = terrain.part(h.s);
    let quiet = story.lock > 0.0 && !story.allow_hazards();

    // ---------------------------------------------------------------- altitude
    let y = h.pos.y;
    let mut trigger = false;
    for b in hz.bands.iter_mut() {
        if !b.1 && y > b.0 + 4.0 {
            b.1 = true;
            trigger = true;
        }
    }
    if trigger {
        hz.dizzy_t = 7.0;
    }
    hz.dizzy_t = (hz.dizzy_t - dt).max(0.0);
    hz.dizzy = if hz.dizzy_t > 0.0 { ((7.0 - hz.dizzy_t) / 1.0).min(1.0).min(hz.dizzy_t / 2.5) } else { 0.0 };

    // ---------------------------------------------------------------- night
    let (n0, _) = terrain.part_range(12);
    let (n1, _) = terrain.part_range(13);
    let t = ((h.max_s - n0) / (n1 - n0)).clamp(0.0, 1.0);
    hz.night = (t * t * (3.0 - 2.0 * t)) * (1.0 - hz.dawn);
    if hz.night > 0.6 && !h.lamp && h.moving {
        hz.dark_t += dt;
        if hz.dark_t > 8.0 {
            hz.dark_t = -20.0;
            hints.note(Cause::Dark);
        }
    }

    // ---------------------------------------------------------------- crevasse
    let on_bridge = h.state == HState::Move && h.s > hz.bridge.0 && h.s < hz.bridge.1;
    if on_bridge && !hz.on_bridge && !hz.bridge_probed {
        let p = if hz.bridge_broken { 0.5 + 0.4 * diff } else { 1.0 };
        if rng.gen_bool(p.clamp(0.0, 1.0) as f64) {
            h.state = HState::Crevasse { left: 7 };
            h.planted = false;
            hz.bridge_broken = true;
            skill.event(-0.04);
            hints.note(Cause::Crevasse);
            mood.shake = 0.4;
            play(&mut commands, &sounds.whumpf, 0.9);
            play(&mut commands, &sounds.crunch, 1.0);
        }
    }
    hz.on_bridge = on_bridge;

    hz.rock_danger = (hz.rock_danger - dt).max(0.0);
    // ---------------------------------------------------------------- rockfall (rock face)
    let (r0, r1) = terrain.part_range(4);
    let under_face = h.s > r0 - 30.0 && h.s < r1 && matches!(h.state, HState::Move | HState::Rest);
    if under_face && !quiet {
        if hz.rock_warn > 0.0 {
            hz.rock_warn -= dt;
            if rng.gen_bool((dt * 14.0).min(1.0) as f64) {
                spawn_particle(&mut commands, Vec2::new(hz.rock_lane + rng.gen_range(-3.0..3.0), cam_y + 340.0), Vec2::new(0.0, -260.0), 2.5, FxKind::Pebble);
            }
            if hz.rock_warn <= 0.0 {
                spawn_particle(&mut commands, Vec2::new(hz.rock_lane, cam_y + 360.0), Vec2::new(0.0, -430.0), 3.0, FxKind::Rock { dodged: false });
                hz.rock_danger = 1.3;
                hz.rock_timer = rng.gen_range(2.5..5.0) / (0.6 + diff);
            }
        } else {
            hz.rock_timer -= dt;
            if hz.rock_timer <= 0.0 {
                hz.rock_warn = 1.6 - 0.9 * diff;
                hz.rock_lane = h.pos.x + rng.gen_range(-3.0..3.0);
                play(&mut commands, &sounds.crunch, 0.45);
            }
        }
    }

    // ---------------------------------------------------------------- icefall (seracs)
    let (i0, i1) = terrain.part_range(12);
    let under_seracs = h.s > i0 && h.s < i1;
    if under_seracs && !quiet {
        if hz.ice_warn > 0.0 {
            hz.ice_warn -= dt;
            if hz.ice_warn <= 0.0 {
                for k in 0..5 {
                    let s = hz.ice_target - 32.0 + k as f32 * 16.0 + rng.gen_range(-5.0..5.0);
                    let p = terrain.point(s) + Vec2::new(rng.gen_range(-4.0..4.0), 120.0 + rng.gen_range(0.0..25.0));
                    spawn_particle(&mut commands, p, Vec2::new(rng.gen_range(-15.0..15.0), -260.0), 1.2, FxKind::Ice);
                }
                hz.ice_impact = 0.45;
                play(&mut commands, &sounds.rumble, 0.35);
            }
        } else if hz.ice_impact <= 0.0 {
            hz.ice_timer -= dt;
            if hz.ice_timer <= 0.0 {
                hz.ice_warn = 1.9 - 0.9 * diff;
                hz.ice_target = h.s + rng.gen_range(5.0..70.0);
                hz.ice_timer = rng.gen_range(3.0..6.0) / (0.6 + diff);
                play(&mut commands, &sounds.chink, 0.35);
                play(&mut commands, &sounds.crunch, 0.5);
            }
        }
    }
    if hz.ice_impact > 0.0 {
        hz.ice_impact -= dt;
        if hz.ice_impact <= 0.0 {
            mood.shake = mood.shake.max(0.3);
            let d = (h.s - hz.ice_target).abs();
            if d < 38.0 && matches!(h.state, HState::Move | HState::Rest | HState::Pause { .. }) {
                h.stamina -= 12.0;
                h.start_slide(-90.0);
                skill.event(-0.06);
                hints.note(Cause::IceHit);
                hz.ice_timer = hz.ice_timer.max(5.0); // a moment to recover
            } else if d < 90.0 {
                skill.event(0.04);
            }
        }
    }
    // the serac that's about to go shivers
    for (sr, mut tr) in seracs.iter_mut() {
        let shaking = hz.ice_warn > 0.0 && (sr.s - hz.ice_target).abs() < 45.0;
        let j = if shaking { (time.elapsed_secs() * 60.0).sin() * 1.5 } else { 0.0 };
        tr.translation.x = sr.base.x + j;
    }

    // ---------------------------------------------------------------- random avalanches
    hz.av_cool = (hz.av_cool - dt).max(0.0);
    let (zp0, zp1) = terrain.part_range(part);
    if hz.av.is_none() && hz.av_cool <= 0.0 && terrain.seg(h.s).zone == Zone::Avalanche && h.state == HState::Move && !quiet {
        let early = if part == 2 { 0.35 } else { 1.0 };
        let r = (0.01 + 0.045 * diff) * if h.moving { 1.0 } else { 0.5 } * early * (1.0 + 0.3 * hz.night);
        if rng.gen_bool((r * dt).clamp(0.0, 1.0) as f64) {
            let warn = 3.6 - 1.6 * diff;
            hz.start_avalanche(&terrain, part, false, warn);
            play(&mut commands, &sounds.whumpf, 1.0);
            mood.shake = 0.4;
        }
    }
    let _ = (zp0, zp1);
    if let Some(mut av) = hz.av {
        let was_warning = av.t < av.warn;
        av.t += dt;
        let (p0, p1) = terrain.part_range(av.part);
        if av.t >= av.warn {
            if was_warning {
                play(&mut commands, &sounds.rumble, 1.0);
            }
            av.front -= 320.0 * dt;
            mood.shake = mood.shake.max(0.25);
            let on_slope = h.s > p0 - 45.0 && h.s < p1 + 4.0; // the runout reaches past the slope
            if !av.hit && on_slope && av.front <= h.s + 8.0 && matches!(h.state, HState::Move | HState::Rest | HState::Pause { .. } | HState::Crouch) {
                av.hit = true;
                let shelter = av.story && {
                    let bs = terrain.part_s[10] + BOULDER_OFFSET;
                    h.s > bs - 34.0 && h.s < bs - 4.0
                };
                if shelter {
                    av.sheltered = true;
                    h.state = HState::Crouch;
                    skill.event(0.08);
                } else {
                    h.start_slide(-260.0);
                    h.stamina -= 20.0;
                    hz.bury_pending = av.story || rng.gen_bool((0.5 + 0.3 * diff).clamp(0.0, 1.0) as f64);
                    skill.event(-0.06);
                    if !av.story {
                        hints.note(Cause::Swept); // the story avalanche is meant to catch you
                    }
                }
            }
            if av.front < p0 - 80.0 {
                if !av.hit {
                    skill.event(0.08); // read it and got out of the way
                }
                if av.sheltered && h.state == HState::Crouch {
                    h.state = HState::Move;
                }
                hz.av = None;
                hz.av_cool = 25.0;
            } else {
                hz.av = Some(av);
            }
        } else {
            hz.av = Some(av);
        }
    }
    if hz.bury_pending && matches!(h.state, HState::Down { .. }) {
        hz.bury_pending = false;
        h.state = HState::Buried;
        h.dig_left = if story.coat_pending() { 8 } else { 6 };
    }
    if hz.bury_pending && h.state == HState::Move {
        hz.bury_pending = false; // stopped softly; not buried
    }

    // ---------------------------------------------------------------- particles
    if h.sparks > 0.0 {
        for _ in 0..2 {
            spawn_particle(&mut commands, h.pos + Vec2::new(8.0 * h.facing, 30.0), Vec2::new(rng.gen_range(-60.0..60.0), rng.gen_range(10.0..90.0)), 0.35, FxKind::Spark);
        }
    }
    if let HState::Slide { v, .. } = h.state {
        if v.abs() > 40.0 && rng.gen_bool(0.5) {
            spawn_particle(&mut commands, h.pos + Vec2::new(0.0, 4.0), Vec2::new(rng.gen_range(-30.0..30.0), rng.gen_range(20.0..70.0)), 0.5, FxKind::Spray);
        }
    }
    for (e, mut p, mut tr) in parts.iter_mut() {
        p.life -= dt;
        let grav = match p.kind {
            FxKind::Spark | FxKind::Spray => 200.0,
            FxKind::Ice => 300.0,
            _ => 0.0,
        };
        p.vel.y -= grav * dt;
        tr.translation += (p.vel * dt).extend(0.0);
        let pos = tr.translation.truncate();
        if let FxKind::Rock { dodged } = p.kind {
            let hit_zone = pos.y < h.pos.y + 62.0 && pos.y > h.pos.y + 8.0 && (pos.x - h.pos.x).abs() < 10.0;
            if hit_zone && h.state == HState::Move {
                h.stamina -= 15.0;
                h.start_slide(-120.0);
                skill.event(-0.06);
                hints.note(Cause::RockHit);
                if h.rope.is_none() && terrain.anchors.iter().any(|&a| a < h.s && h.s - a < 120.0) {
                    hints.note(Cause::Unroped);
                }
                mood.flash = 0.4;
                play(&mut commands, &sounds.crunch, 1.0);
                commands.entity(e).despawn();
                continue;
            }
            if !dodged && pos.y < h.pos.y && (pos.x - h.pos.x).abs() < 40.0 && h.state == HState::Move && terrain.seg(h.s).wall {
                p.kind = FxKind::Rock { dodged: true };
                skill.event(0.04);
            }
        }
        if p.kind == FxKind::Ice && pos.y <= terrain.top_at_x(pos.x) + 2.0 {
            for _ in 0..4 {
                spawn_particle(&mut commands, pos, Vec2::new(rng.gen_range(-50.0..50.0), rng.gen_range(20.0..80.0)), 0.4, FxKind::Spray);
            }
            commands.entity(e).despawn();
            continue;
        }
        if p.life <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}

/// Visual state of hazard props: bridge marks, the avalanche mass, the crack, the night mask.
#[allow(clippy::type_complexity)]
pub fn hazard_visuals(
    hz: Res<Hazards>,
    terrain: Res<Terrain>,
    hq: Query<&Hero>,
    mut marks: Query<(&BridgeMark, &mut Sprite), (Without<RandomMass>, Without<AvCrack>, Without<LampMask>)>,
    mut mass: Query<(&mut Transform, &mut Visibility), (With<RandomMass>, Without<AvCrack>, Without<LampMask>)>,
    mut crack: Query<(&mut Transform, &mut Visibility), (With<AvCrack>, Without<RandomMass>, Without<LampMask>)>,
    mut lamp: Query<(&mut Transform, &mut Sprite, &mut Visibility), (With<LampMask>, Without<RandomMass>, Without<AvCrack>, Without<BridgeMark>)>,
    assets: Res<AssetServer>,
    time: Res<Time>,
) {
    let Ok(h) = hq.single() else { return };
    for (m, mut s) in marks.iter_mut() {
        let a = match m.0 {
            0 => if hz.bridge_broken { 0.0 } else { 0.55 },
            1 => if hz.bridge_probed && !hz.bridge_broken { 0.9 } else { 0.0 },
            _ => if hz.bridge_broken { 0.95 } else { 0.0 },
        };
        s.color.set_alpha(a);
    }
    if let Ok((mut t, mut v)) = mass.single_mut() {
        match hz.av {
            Some(av) if av.t >= av.warn => {
                let p = terrain.point(av.front.min(terrain.total));
                t.translation = (p + Vec2::new(70.0, 50.0) + Vec2::Y * (time.elapsed_secs() * 18.0).sin() * 3.0).extend(12.0);
                *v = Visibility::Visible;
            }
            _ => *v = Visibility::Hidden,
        }
    }
    if let Ok((mut t, mut v)) = crack.single_mut() {
        match hz.av {
            Some(av) if av.t < av.warn + 1.0 => {
                let (_, p1) = terrain.part_range(av.part);
                t.translation = (terrain.point(p1 - 30.0) + Vec2::new(30.0, 40.0)).extend(3.7);
                *v = Visibility::Visible;
            }
            _ => *v = Visibility::Hidden,
        }
    }
    // night: darkness everywhere except the headlamp's pool of light
    if let Ok((mut t, mut s, mut v)) = lamp.single_mut() {
        if hz.night > 0.01 {
            *v = Visibility::Visible;
            t.translation = (h.pos + Vec2::new(10.0 * h.facing, 34.0)).extend(895.0);
            let on = h.lamp && h.battery > 0.0;
            // a dying battery flickers
            let flicker = on && h.battery < 20.0 && (time.elapsed_secs() * 13.0).sin() > 0.6;
            let want = if on && !flicker { "sprites/lamp_on.png" } else { "sprites/lamp_off.png" };
            if s.image.path().map(|p| p.to_string()) != Some(want.to_string()) {
                s.image = assets.load(want);
            }
            s.color.set_alpha(0.93 * hz.night);
        } else {
            *v = Visibility::Hidden;
        }
    }
}

pub fn spawn_lamp_mask(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((
        LampMask,
        Sprite { image: assets.load("sprites/lamp_off.png"), custom_size: Some(Vec2::splat(1600.0)), ..default() },
        Transform::from_xyz(0., 0., 895.0),
        Visibility::Hidden,
    ));
}
