//! The beats. No objectives, no text boxes — only the mountain, a few words of memory,
//! and what the player chooses to do.

use crate::controls::{Controls, Tool};
use crate::fx::{play, Captions, Mood, Sounds};
use crate::hazards::{spawn_hazard_props, Hazards, BOULDER_OFFSET};
use crate::hero::{HState, Hero};
use crate::skill::Skill;
use crate::terrain::{spawn_terrain, Terrain, CROSS_S};
use crate::LevelEntity;
use bevy::prelude::*;
use bevy::sprite::Anchor;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    Intro,
    Climb,
    AvWhumpf,
    AvSlide,
    Aftermath,
    CoatDig,
    CoatHold,
    Voice,
    Climb2,
    Checkpoint,
    End,
}

#[derive(Resource)]
pub struct Story {
    pub stage: Stage,
    pub t: f32,
    /// seconds of ignored input (a beat that must play out)
    pub lock: f32,
    pub hide_ui: bool,
    pub cross_taken: bool,
    pub cross_t: f32,
    pub coat_taps: i32,
    /// furthest arc-length he may walk (he stops at the coat)
    pub gate: f32,
    falls_seen: u32,
    pub restart: bool,
}

impl Default for Story {
    fn default() -> Self {
        Story { stage: Stage::Intro, t: 0.0, lock: 0.0, hide_ui: false, cross_taken: false, cross_t: 0.0, coat_taps: 0, gate: f32::MAX, falls_seen: 0, restart: false }
    }
}

impl Story {
    fn go(&mut self, s: Stage) {
        info!("stage -> {:?}", s);
        self.stage = s;
        self.t = 0.0;
    }
    /// hazards keep running during short input locks (e.g. the avalanche freeze)
    pub fn allow_hazards(&self) -> bool {
        matches!(self.stage, Stage::AvWhumpf | Stage::AvSlide | Stage::Climb | Stage::Climb2)
    }
    pub fn coat_pending(&self) -> bool {
        matches!(self.stage, Stage::AvWhumpf | Stage::AvSlide)
    }
}

pub const FRIEND: Color = Color::srgb(0.98, 0.86, 0.6);
pub const SELF: Color = Color::srgb(0.85, 0.9, 0.97);

#[derive(Component)]
pub struct CrossProp;
#[derive(Component)]
pub struct CoatProp;
#[derive(Component)]
pub struct CoatMound;
#[derive(Component)]
pub struct RopeLine;

pub fn coat_s(t: &Terrain) -> f32 {
    t.part_s[10] + BOULDER_OFFSET + 42.0
}

pub struct StoryPlugin;
impl Plugin for StoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Story>().add_systems(Startup, spawn_story_props_startup).add_systems(Update, dev_capture);
    }
}

fn spawn_story_props_startup(mut commands: Commands, terrain: Res<Terrain>, assets: Res<AssetServer>) {
    spawn_story_props(&mut commands, &terrain, &assets);
}

pub fn spawn_story_props(commands: &mut Commands, t: &Terrain, assets: &AssetServer) {
    let c = t.point(coat_s(t));
    commands.spawn((
        LevelEntity,
        CoatProp,
        Sprite { image: assets.load("sprites/coat_friend.png"), custom_size: Some(Vec2::new(48.0, 39.0)), anchor: Anchor::BottomCenter, ..default() },
        Transform::from_translation((c - Vec2::Y * 4.0).extend(6.2)).with_rotation(Quat::from_rotation_z(0.5)),
        Visibility::Hidden,
    ));
    commands.spawn((
        LevelEntity,
        CoatMound,
        Sprite { image: assets.load("sprites/mound.png"), custom_size: Some(Vec2::new(80.0, 44.0)), anchor: Anchor::BottomCenter, ..default() },
        Transform::from_translation((c + Vec2::new(6.0, -8.0)).extend(6.6)),
        Visibility::Hidden,
    ));
    commands.spawn((LevelEntity, RopeLine, Sprite::from_color(Color::srgb(0.72, 0.42, 0.18), Vec2::new(1.0, 2.0)), Transform::from_xyz(0., 0., 9.0), Visibility::Hidden));
}

type PropQ<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, &'static mut Sprite, &'static mut Visibility, Option<&'static CrossProp>, Option<&'static CoatProp>, Option<&'static CoatMound>, Option<&'static RopeLine>),
    (Or<(With<CrossProp>, With<CoatProp>, With<CoatMound>, With<RopeLine>)>, Without<Hero>),
>;

#[allow(clippy::too_many_arguments)]
pub fn story_system(
    mut commands: Commands,
    mut story: ResMut<Story>,
    mut controls: ResMut<Controls>,
    sounds: Res<Sounds>,
    terrain: Res<Terrain>,
    mut hz: ResMut<Hazards>,
    mut captions: ResMut<Captions>,
    mut mood: ResMut<Mood>,
    time: Res<Time>,
    mut hq: Query<&mut Hero>,
    mut props: PropQ,
) {
    let Ok(mut h) = hq.single_mut() else { return };
    let dt = time.delta_secs().min(0.05);
    story.t += dt;
    story.lock = (story.lock - dt).max(0.0);
    let ground = h.state == HState::Move;
    let cs = coat_s(&terrain);

    if h.falls > story.falls_seen {
        story.falls_seen = h.falls;
        if h.falls % 2 == 0 {
            captions.sub(SELF, "Ek aur climb.", 3.0);
        }
    }

    // the cross at Base Camp
    if !story.cross_taken && ground && (h.s - CROSS_S).abs() < 10.0 {
        story.cross_taken = true;
        h.state = HState::Pause { t: 1.5, act: crate::hero::Act::Lift };
        h.facing = 1.0;
        story.lock = 1.5;
        play(&mut commands, &sounds.chime, 0.4);
    }
    if story.cross_taken {
        story.cross_t += dt;
    }

    match story.stage {
        Stage::Intro => {
            if story.t > 0.6 {
                captions.day("DAY 1");
                story.go(Stage::Climb);
            }
        }
        Stage::Climb => {
            if ground && h.s > terrain.part_s[10] + 60.0 {
                // WHUMPF. The mountain goes quiet.
                hz.av = None;
                hz.start_avalanche(&terrain, 10, true, 1.3);
                story.lock = 0.9;
                mood.shake = 0.5;
                play(&mut commands, &sounds.whumpf, 1.0);
                story.go(Stage::AvWhumpf);
            }
        }
        Stage::AvWhumpf => {
            if hz.avalanche_running() {
                story.go(Stage::AvSlide);
            }
        }
        Stage::AvSlide => {
            if hz.av.is_none() && h.state == HState::Move {
                story.go(Stage::Aftermath);
            }
        }
        Stage::Aftermath | Stage::CoatDig => {
            story.gate = cs - 16.0; // he stops at it; he can't not look
            if ground && (h.s - (cs - 18.0)).abs() < 24.0 && controls.taps.contains(&Tool::Dig) {
                controls.taps.retain(|t| *t != Tool::Dig); // the shovel is for the coat now
                if story.stage == Stage::Aftermath {
                    story.go(Stage::CoatDig);
                }
                story.coat_taps += 1;
                h.facing = 1.0;
                story.lock = 0.25;
                play(&mut commands, &sounds.crunch, 0.8);
                if story.coat_taps >= 4 {
                    story.go(Stage::CoatHold);
                    h.state = HState::Kneel;
                    story.hide_ui = true;
                }
            }
        }
        Stage::CoatHold => {
            story.lock = 1.0;
            if story.t > 3.4 {
                h.carrying_coat = true;
            }
            if story.t > 4.2 {
                captions.sub(FRIEND, "Jab toofan aaye toh bhaagte nahi...\nbachte hai.", 6.5);
                story.go(Stage::Voice);
            }
        }
        Stage::Voice => {
            story.lock = 1.0;
            if story.t > 7.6 {
                h.state = HState::Rise { t: 0.0 };
                story.hide_ui = false;
                story.gate = f32::MAX;
                story.lock = 0.0;
                story.go(Stage::Climb2);
            }
        }
        Stage::Climb2 => {
            if ground && h.s > terrain.part_s[16] + 60.0 {
                story.go(Stage::Checkpoint);
                h.state = HState::Sit;
                h.facing = 1.0;
                story.hide_ui = true;
            }
        }
        Stage::Checkpoint => {
            story.lock = 1.0;
            hz.dawn = (story.t / 5.0).min(1.0);
            if story.t > 1.8 && story.t - dt <= 1.8 {
                captions.day("DAY 3");
            }
            if story.t > 6.8 && story.t - dt <= 6.8 {
                captions.sub(FRIEND, "Pehli baar yahan aaya tha...\nsocha tha pahaad aur bada dikhega.", 6.5);
            }
            if story.t > 15.5 {
                story.go(Stage::End);
            }
        }
        Stage::End => {
            story.lock = 1.0;
            if story.t > 5.0 && story.t - dt <= 5.0 {
                captions.hint("tap to climb again");
            }
            if story.t > 5.0 && controls.any_tap {
                story.restart = true;
            }
        }
    }

    // ---------------------------------------------------------------- props
    let revealed = matches!(story.stage, Stage::Aftermath | Stage::CoatDig | Stage::CoatHold) || (story.stage == Stage::AvSlide && hz.av.is_some_and(|a| a.hit));
    let c = terrain.point(cs);
    for (mut tr, mut sprite, mut vis, cross, coat, mound, rope) in props.iter_mut() {
        if cross.is_some() {
            let a = if story.cross_taken { (1.0 - (story.cross_t - 0.7) / 0.6).clamp(0.0, 1.0) } else { 1.0 };
            sprite.color = Color::srgba(1., 1., 1., a);
            if story.cross_taken {
                tr.translation.y = terrain.point(CROSS_S).y + 22.0 + (story.cross_t * 14.0).min(10.0);
            }
        }
        if mound.is_some() {
            *vis = if revealed && story.coat_taps < 4 { Visibility::Visible } else { Visibility::Hidden };
            let s = 1.0 - story.coat_taps as f32 * 0.2;
            tr.scale = Vec3::new(s, s, 1.0);
        }
        if coat.is_some() {
            let dug = story.coat_taps as f32 / 4.0;
            *vis = if (revealed || story.stage == Stage::Voice) && !h.carrying_coat { Visibility::Visible } else { Visibility::Hidden };
            tr.translation.y = c.y - 4.0 + dug * 4.0;
            tr.rotation = Quat::from_rotation_z(0.5 * (1.0 - dug));
            let fade = if story.stage == Stage::CoatHold { (1.0 - (story.t - 2.6) / 0.8).clamp(0.0, 1.0) } else { 1.0 };
            sprite.color = Color::srgba(1., 1., 1., fade);
        }
        if rope.is_some() {
            match h.rope {
                Some(a) if !matches!(h.state, HState::Buried) => {
                    let pa = terrain.point(a) + Vec2::new(4.0, 26.0);
                    let pb = h.pos + Vec2::new(0.0, 34.0);
                    let d = pb - pa;
                    *vis = Visibility::Visible;
                    tr.translation = ((pa + pb) / 2.0).extend(9.0);
                    tr.rotation = Quat::from_rotation_z(d.y.atan2(d.x));
                    tr.scale = Vec3::new(d.length().max(1.0), 1.0, 1.0);
                }
                _ => *vis = Visibility::Hidden,
            }
        }
    }
}

/// Restart after the slice ends: a new mountain (new micro-shape), reset everyone.
#[allow(clippy::too_many_arguments)]
pub fn restart_system(
    mut commands: Commands,
    mut story: ResMut<Story>,
    mut captions: ResMut<Captions>,
    mut mood: ResMut<Mood>,
    mut hq: Query<&mut Hero>,
    level: Query<Entity, With<LevelEntity>>,
    assets: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    skill: Res<Skill>,
) {
    if !story.restart {
        return;
    }
    for e in level.iter() {
        commands.entity(e).despawn();
    }
    let t = Terrain::generate();
    spawn_terrain(&mut commands, &t, &assets, &mut images);
    spawn_hazard_props(&mut commands, &t, &assets);
    spawn_story_props(&mut commands, &t, &assets);
    commands.insert_resource(Hazards::new(&t));
    commands.insert_resource(t);
    *story = Story::default();
    *captions = Captions::default();
    *mood = Mood::default();
    if let Ok(mut h) = hq.single_mut() {
        *h = Hero::default();
    }
    // the player's measured skill carries over between runs
    let _ = skill;
}

/// Dev only: EVEREST_SHOTS=<dir> saves a screenshot every EVEREST_SHOT_EVERY seconds;
/// EVEREST_QUIT_AFTER=<secs> exits. Used for automated play-testing.
fn dev_capture(
    mut commands: Commands,
    time: Res<Time>,
    story: Res<Story>,
    skill: Res<Skill>,
    hq: Query<&Hero>,
    terrain: Res<Terrain>,
    mut next: Local<f32>,
    mut n: Local<u32>,
    mut exit: EventWriter<AppExit>,
) {
    use bevy::render::view::screenshot::{save_to_disk, Screenshot};
    let now = time.elapsed_secs();
    if let Ok(q) = std::env::var("EVEREST_QUIT_AFTER") {
        if now > q.parse::<f32>().unwrap_or(1e9) {
            exit.write(AppExit::Success);
        }
    }
    let Ok(dir) = std::env::var("EVEREST_SHOTS") else { return };
    let every: f32 = std::env::var("EVEREST_SHOT_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(2.0);
    if now >= *next {
        *next = now + every;
        *n += 1;
        let tag = hq
            .single()
            .ok()
            .map(|h| format!("{:?}_p{}_{}_st{:.0}_sk{:.2}", story.stage, terrain.part(h.s), state_tag(h.state), h.stamina, skill.diff))
            .unwrap_or_default();
        let path = format!("{dir}/{:04}_{:05.1}_{}.png", *n, now, tag);
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
    }
}

fn state_tag(s: HState) -> &'static str {
    match s {
        HState::Move => "move",
        HState::Slide { .. } => "slide",
        HState::Down { .. } => "down",
        HState::Rise { .. } => "rise",
        HState::Rest => "rest",
        HState::Pause { .. } => "pause",
        HState::Crevasse { .. } => "crevasse",
        HState::Buried => "buried",
        HState::Kneel => "kneel",
        HState::Sit => "sit",
        HState::Crouch => "crouch",
    }
}
