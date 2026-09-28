//! The slice's beats. No objectives, no text boxes — only the mountain, a few
//! words of memory, and what the player chooses to do.

use crate::controls::{Controls, Tool};
use crate::fx::{play, Captions, Mood, Sounds};
use crate::hero::{HState, Hero};
use crate::level::*;
use crate::LevelEntity;
use bevy::prelude::*;
use rand::Rng;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    Intro,
    Climb,
    Whumpf,
    Slide,
    Buried,
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
    pub roped: bool,
    gust_phase: u8,
    gust_timer: f32,
    pub gust_warn: bool,
    pub gust_active: bool,
    pub mass_active: bool,
    pub mass_t: f32,
    pub wave_x: f32,
    contact: bool,
    pub sheltered: bool,
    swept_t: f32,
    pub dig_left: i32,
    pub coat_taps: i32,
    pub min_x: f32,
    falls_seen: u32,
    pub restart: bool,
}

impl Default for Story {
    fn default() -> Self {
        Story {
            stage: Stage::Intro,
            t: 0.0,
            lock: 0.0,
            hide_ui: false,
            cross_taken: false,
            cross_t: 0.0,
            roped: false,
            gust_phase: 0,
            gust_timer: 4.0,
            gust_warn: false,
            gust_active: false,
            mass_active: false,
            mass_t: 0.0,
            wave_x: 999.0,
            contact: false,
            sheltered: false,
            swept_t: 0.0,
            dig_left: 0,
            coat_taps: 0,
            min_x: 0.0,
            falls_seen: 0,
            restart: false,
        }
    }
}

impl Story {
    fn go(&mut self, s: Stage) {
        info!("stage -> {:?}", s);
        self.stage = s;
        self.t = 0.0;
    }
}

// Friend (warm) and protagonist (cold white) subtitle colours.
pub const FRIEND: Color = Color::srgb(0.98, 0.86, 0.6);
pub const SELF: Color = Color::srgb(0.85, 0.9, 0.97);

pub struct StoryPlugin;
impl Plugin for StoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Story>().add_systems(Update, dev_capture);
    }
}

type PropQ<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Transform,
        &'static mut Sprite,
        &'static mut Visibility,
        Option<&'static CrossProp>,
        Option<&'static CoatProp>,
        Option<&'static CoatMound>,
        Option<&'static AvalancheMass>,
        Option<&'static RopeLine>,
        Option<&'static Crack>,
    ),
    (Or<(With<CrossProp>, With<CoatProp>, With<CoatMound>, With<AvalancheMass>, With<RopeLine>, With<Crack>)>, Without<Hero>),
>;

#[allow(clippy::too_many_arguments)]
pub fn story_system(
    mut commands: Commands,
    mut story: ResMut<Story>,
    controls: Res<Controls>,
    sounds: Res<Sounds>,
    mut captions: ResMut<Captions>,
    mut mood: ResMut<Mood>,
    time: Res<Time>,
    mut hq: Query<&mut Hero>,
    mut props: PropQ,
    assets: Res<AssetServer>,
) {
    let Ok(mut h) = hq.single_mut() else { return };
    let dt = time.delta_secs().min(0.05);
    story.t += dt;
    story.lock = (story.lock - dt).max(0.0);
    let tapped = |t: Tool| controls.taps.contains(&t);
    let ground = h.state == HState::Ground;

    // "Ek aur climb." — muttered after every second fall
    if h.falls > story.falls_seen {
        story.falls_seen = h.falls;
        if h.falls % 2 == 0 {
            captions.sub(SELF, "Ek aur climb.", 3.0);
        }
    }

    // ---------------------------------------------------------------- the cross (base camp)
    if !story.cross_taken && h.ledge == 0 && ground && (h.pos.x - CROSS_X).abs() < 12.0 {
        story.cross_taken = true;
        h.state = HState::Pause { t: 1.5 };
        h.facing = 1.0;
        story.lock = 1.5;
        play(&mut commands, &sounds.chime, 0.4);
    }
    if story.cross_taken {
        story.cross_t += dt;
    }

    // ---------------------------------------------------------------- rope on the exposed ledge
    if h.ledge == 2 && ground && !story.roped && tapped(Tool::Rope) && (h.pos.x - ANCHOR_X).abs() < 32.0 {
        story.roped = true;
        play(&mut commands, &sounds.chink, 0.5);
    }
    if story.roped && h.ledge != 2 {
        story.roped = false;
    }

    // ---------------------------------------------------------------- wind gusts (ledge 2 and above)
    if h.ledge >= 2 && matches!(story.stage, Stage::Climb | Stage::Climb2) {
        story.gust_timer -= dt;
        if story.gust_timer <= 0.0 {
            story.gust_phase = (story.gust_phase + 1) % 3;
            story.gust_timer = match story.gust_phase {
                1 => {
                    play(&mut commands, &sounds.gust, 0.9);
                    1.3
                }
                2 => 1.7,
                _ => rand::thread_rng().gen_range(3.5..6.0),
            };
        }
    } else {
        story.gust_phase = 0;
    }
    story.gust_warn = story.gust_phase == 1;
    story.gust_active = story.gust_phase == 2;

    // ---------------------------------------------------------------- stages
    match story.stage {
        Stage::Intro => {
            if story.t > 0.6 {
                captions.day("DAY 1");
                story.go(Stage::Climb);
            }
        }
        Stage::Climb => {
            if h.ledge == 3 && ground && h.pos.x < 292.0 {
                // WHUMPF. Everything goes quiet.
                story.go(Stage::Whumpf);
                story.lock = 1.1;
                h.facing = 1.0;
                mood.shake = 0.5;
                play(&mut commands, &sounds.whumpf, 1.0);
            }
        }
        Stage::Whumpf => {
            if story.t > 1.1 {
                story.go(Stage::Slide);
                story.mass_active = true;
                story.mass_t = 0.0;
                play(&mut commands, &sounds.rumble, 1.0);
            }
        }
        Stage::Slide => {
            if !story.contact && story.wave_x <= h.pos.x + 6.0 {
                story.contact = true;
                if ground && h.pos.x >= SHELTER.0 && h.pos.x <= SHELTER.1 {
                    story.sheltered = true;
                    h.state = HState::Crouch;
                    h.facing = 1.0;
                } else {
                    h.state = HState::Swept;
                    story.swept_t = 0.0;
                }
            }
            if story.contact {
                if story.sheltered {
                    if !story.mass_active {
                        h.state = HState::Ground;
                        deposit_debris(&mut commands, &assets);
                        story.go(Stage::Aftermath);
                    }
                } else {
                    story.swept_t += dt;
                    h.pos.x = (story.wave_x + 30.0).clamp(COAT_X + 40.0, LEDGES[3].x1 - 10.0);
                    h.facing = -1.0;
                    mood.shake = mood.shake.max(0.3);
                    if story.swept_t > 1.0 {
                        h.state = HState::Buried;
                        story.dig_left = 8;
                        deposit_debris(&mut commands, &assets);
                        story.go(Stage::Buried);
                    }
                }
            }
        }
        Stage::Buried => {
            if tapped(Tool::Dig) {
                story.dig_left -= 1;
                play(&mut commands, &sounds.crunch, 0.9);
                if story.dig_left <= 0 {
                    h.state = HState::Rise { t: 0.0 };
                    story.go(Stage::Aftermath);
                }
            }
        }
        Stage::Aftermath | Stage::CoatDig => {
            story.min_x = COAT_X + 16.0; // he stops at it; he can't not look
            if ground && (h.pos.x - (COAT_X + 20.0)).abs() < 26.0 && tapped(Tool::Dig) {
                if story.stage == Stage::Aftermath {
                    story.go(Stage::CoatDig);
                }
                story.coat_taps += 1;
                h.facing = -1.0;
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
            if story.t > 3.4 && !h.carrying_coat {
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
                story.min_x = 0.0;
                story.lock = 0.0;
                story.go(Stage::Climb2);
            }
        }
        Stage::Climb2 => {
            if h.ledge == 4 && ground && h.pos.x >= FLAGS_X - 34.0 {
                story.go(Stage::Checkpoint);
                h.state = HState::Sit;
                h.facing = 1.0;
                story.hide_ui = true;
            }
        }
        Stage::Checkpoint => {
            story.lock = 1.0;
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

    // ---------------------------------------------------------------- the avalanche mass
    if story.mass_active {
        story.mass_t += dt;
        let t = story.mass_t;
        let p = if t < 1.4 {
            Vec2::new(430.0, 820.0).lerp(Vec2::new(330.0, 612.0), t / 1.4)
        } else {
            Vec2::new(330.0 - (t - 1.4) * 175.0, 612.0)
        };
        story.wave_x = p.x - 110.0;
        if p.x < -200.0 {
            story.mass_active = false;
        }
        for (mut tr, _, mut vis, .., mass, _, _) in props.iter_mut() {
            if mass.is_some() {
                tr.translation.x = p.x;
                tr.translation.y = p.y + (t * 20.0).sin() * 3.0;
                *vis = if story.mass_active { Visibility::Visible } else { Visibility::Hidden };
            }
        }
    }

    // ---------------------------------------------------------------- props
    let revealed = matches!(story.stage, Stage::Buried | Stage::Aftermath | Stage::CoatDig | Stage::CoatHold)
        || (story.stage == Stage::Slide && story.contact);
    for (mut tr, mut sprite, mut vis, cross, coat, mound, _, rope, crack) in props.iter_mut() {
        if cross.is_some() {
            // lifted, held for a moment, gone into his pocket
            let a = if story.cross_taken { (1.0 - (story.cross_t - 0.7) / 0.6).clamp(0.0, 1.0) } else { 1.0 };
            sprite.color = Color::srgba(1., 1., 1., a);
            if story.cross_taken {
                tr.translation.y = LEDGES[0].y + 22.0 + (story.cross_t * 14.0).min(10.0);
            }
        }
        if crack.is_some() {
            *vis = if matches!(story.stage, Stage::Whumpf | Stage::Slide) { Visibility::Visible } else { Visibility::Hidden };
        }
        if mound.is_some() {
            *vis = if revealed && story.coat_taps < 4 { Visibility::Visible } else { Visibility::Hidden };
            let s = 1.0 - story.coat_taps as f32 * 0.2;
            tr.scale = Vec3::new(s, s, 1.0);
        }
        if coat.is_some() {
            let dug = story.coat_taps as f32 / 4.0;
            let show = revealed || (matches!(story.stage, Stage::Voice) && !h.carrying_coat);
            *vis = if show && !h.carrying_coat { Visibility::Visible } else { Visibility::Hidden };
            // a sleeve pokes out of the debris; each dig frees more of it
            tr.translation.y = LEDGES[3].y - 4.0 + dug * 4.0;
            tr.rotation = Quat::from_rotation_z(0.5 * (1.0 - dug));
            let fade = if story.stage == Stage::CoatHold { (1.0 - (story.t - 2.6) / 0.8).clamp(0.0, 1.0) } else { 1.0 };
            sprite.color = Color::srgba(1., 1., 1., fade);
        }
        if rope.is_some() {
            if story.roped {
                let a = Vec2::new(ANCHOR_X + 4.0, LEDGES[2].y + 30.0);
                let b = h.pos + Vec2::new(0.0, 34.0);
                let d = b - a;
                *vis = Visibility::Visible;
                tr.translation = ((a + b) / 2.0).extend(9.0);
                tr.rotation = Quat::from_rotation_z(d.y.atan2(d.x));
                tr.scale = Vec3::new(d.length().max(1.0), 1.0, 1.0);
            } else {
                *vis = Visibility::Hidden;
            }
        }
    }
}

fn deposit_debris(commands: &mut Commands, assets: &AssetServer) {
    let mut rng = rand::thread_rng();
    for x in [128.0, 250.0, 300.0, 340.0] {
        let s = rng.gen_range(0.6..1.0);
        commands.spawn((
            LevelEntity,
            Debris,
            Sprite {
                image: assets.load("sprites/mound.png"),
                custom_size: Some(Vec2::new(70.0 * s, 44.0 * s)),
                anchor: bevy::sprite::Anchor::BottomCenter,
                ..default()
            },
            Transform::from_xyz(x, LEDGES[3].y - 8.0, 6.4),
        ));
    }
}

/// Restart after the slice ends: rebuild the mountain, reset everyone.
pub fn restart_system(
    mut commands: Commands,
    mut story: ResMut<Story>,
    mut captions: ResMut<Captions>,
    mut mood: ResMut<Mood>,
    mut hq: Query<&mut Hero>,
    level: Query<Entity, With<LevelEntity>>,
    assets: Res<AssetServer>,
) {
    if !story.restart {
        return;
    }
    for e in level.iter() {
        commands.entity(e).despawn();
    }
    spawn_level(&mut commands, &assets);
    *story = Story::default();
    *captions = Captions::default();
    *mood = Mood::default();
    if let Ok(mut h) = hq.single_mut() {
        *h = Hero::default();
    }
}

/// Dev only: EVEREST_SHOTS=<dir> saves a screenshot every EVEREST_SHOT_EVERY seconds;
/// EVEREST_QUIT_AFTER=<secs> exits. Used for automated play-testing.
fn dev_capture(
    mut commands: Commands,
    time: Res<Time>,
    story: Res<Story>,
    hq: Query<&Hero>,
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
        let h = hq.single().ok();
        let tag = h.map(|h| format!("{:?}_L{}_s{:.0}", story.stage, h.ledge, h.stamina)).unwrap_or_default();
        let path = format!("{dir}/{:04}_{:05.1}_{}.png", *n, now, tag);
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
    }
}
