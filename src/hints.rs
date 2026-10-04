//! Hints, only when the player keeps failing the same way. No popups: a remembered line of advice.
//! Before the coat it's the protagonist muttering to himself; after the coat it's the friend's voice.
//! Second failure of a kind → the line. Fourth → the line again, and the right tool glows for a moment.

use crate::controls::Tool;
use crate::fx::Captions;
use crate::story::{Stage, Story, FRIEND, SELF};
use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Cause {
    /// ran out of strength on a face
    Exhausted,
    /// fell off a face with an anchor just below, unroped
    Unroped,
    /// tired legs on steep snow
    Slip,
    /// long slide on snow, never dug the axe in
    NoArrest,
    /// pushing up an ice face without planting the axe
    IceNoAxe,
    /// axe on rock
    RockAxe,
    RockHit,
    IceHit,
    Crevasse,
    Swept,
    /// walking in the dark without the headlamp
    Dark,
}

impl Cause {
    fn line(self, friend: bool) -> &'static str {
        match (self, friend) {
            (Cause::Exhausted, false) => "Deewar se pehle... saans le le.",
            (Cause::Exhausted, true) => "Thak ke chadhega toh girega. Pehle baith, saans le, phir chadh.",
            (Cause::Unroped, false) => "Kunda wahin tha... rassi daal deta.",
            (Cause::Unroped, true) => "Kunde mein rassi daal de. Girega toh wahin ruk jaayega.",
            (Cause::Slip, false) => "Dhalan pe thake pair... phisalte hain.",
            (Cause::Slip, true) => "Dhalan pe thak gaya toh ruk ja. Thake pair phisalte hain.",
            (Cause::NoArrest, false) => "Kulhadi... barf mein gaad deta.",
            (Cause::NoArrest, true) => "Phisle toh kulhadi barf mein gaad de. Jaldi, der ki toh nahi rukega.",
            (Cause::IceNoAxe, false) => "Barf pe haath nahi tikte... kulhadi.",
            (Cause::IceNoAxe, true) => "Barf ki deewar pe kulhadi gaad, phir khinch. Haath nahi chalte wahan.",
            (Cause::RockAxe, false) => "Patthar pe kulhadi phisal rahi hai.",
            (Cause::RockAxe, true) => "Patthar pe kulhadi nahi, haath aur pair. Kulhadi barf ke liye hai.",
            (Cause::RockHit, false) => "Kankad gire... matlab patthar aa raha hai.",
            (Cause::RockHit, true) => "Upar se kankad gire toh samajh patthar aa raha hai. Side ho ja.",
            (Cause::IceHit, false) => "Woh minar kaanp raha tha...",
            (Cause::IceHit, true) => "Barf ka minar kaanpe toh neeche mat ruk. Nikal wahan se.",
            (Cause::Crevasse, false) => "Woh barf... kuch dabi hui thi.",
            (Cause::Crevasse, true) => "Dabi hui barf pe pehle kulhadi se thok ke dekh. Khokhli ho toh kinare se chal.",
            (Cause::Swept, false) => "Woh awaaz... dhalan se hat jaana tha.",
            (Cause::Swept, true) => "Whumpf sune toh dhalan se hat ja. Neeche bhi nahi, toofan neeche tak aata hai.",
            (Cause::Dark, false) => "Kuch dikh nahi raha... roshni.",
            (Cause::Dark, true) => "Andhere mein bina roshni ke mat chal. Battery bacha ke rakh, din mein band kar.",
        }
    }
    fn tool(self) -> Option<Tool> {
        match self {
            Cause::Exhausted | Cause::Slip => Some(Tool::Rest),
            Cause::Unroped => Some(Tool::Rope),
            Cause::NoArrest | Cause::IceNoAxe | Cause::Crevasse => Some(Tool::Axe),
            Cause::Dark => Some(Tool::Lamp),
            _ => None,
        }
    }
}

#[derive(Resource, Default)]
pub struct HintLog {
    counts: std::collections::HashMap<Cause, u32>,
    pending: Vec<Cause>,
    cooldown: f32,
    /// which tool button glows, and for how long
    pub glow: Option<(Tool, f32)>,
}

impl HintLog {
    pub fn note(&mut self, c: Cause) {
        let n = self.counts.entry(c).or_insert(0);
        *n += 1;
        if *n == 2 || *n == 4 {
            self.pending.push(c);
        }
    }
}

pub fn hints_system(mut log: ResMut<HintLog>, story: Res<Story>, mut captions: ResMut<Captions>, time: Res<Time>) {
    let dt = time.delta_secs().min(0.05);
    log.cooldown = (log.cooldown - dt).max(0.0);
    if let Some((t, left)) = log.glow {
        log.glow = if left > dt { Some((t, left - dt)) } else { None };
    }
    // never talk over a story beat
    let quiet = matches!(story.stage, Stage::AvWhumpf | Stage::CoatHold | Stage::Voice | Stage::Checkpoint | Stage::End | Stage::Intro);
    if quiet || log.cooldown > 0.0 || captions.sub.as_ref().is_some_and(|c| c.t < c.dur) {
        return;
    }
    if let Some(c) = log.pending.first().copied() {
        log.pending.remove(0);
        let friend = matches!(story.stage, Stage::Climb2);
        info!("hint: {:?} ({})", c, if friend { "friend" } else { "self" });
        captions.sub(if friend { FRIEND } else { SELF }, c.line(friend), 5.5);
        if log.counts.get(&c).copied().unwrap_or(0) >= 4 {
            if let Some(t) = c.tool() {
                log.glow = Some((t, 5.0));
            }
        }
        log.cooldown = 18.0;
    }
}
