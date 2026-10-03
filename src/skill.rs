//! Quiet adaptive difficulty. No menus, no model — a running skill estimate fed by what the
//! player actually does, and a difficulty that drifts towards it.
//!
//! Signals: clean self-arrests, dodged rocks and ice, escaped avalanches, probing a snow bridge
//! (good) vs. running out of strength on a face, slips, hits, being swept (bad).
//! The first slopes act as a silent calibration: early observations move the estimate more.
//!
//! What it tunes (see hazards.rs / hero.rs): warning times, how often rocks and ice fall,
//! avalanche likelihood, slip chance, how fast stamina drains. Story beats never change.

use bevy::prelude::*;

#[derive(Resource)]
pub struct Skill {
    /// 0 = struggling, 1 = very capable
    pub est: f32,
    /// 0..1, what the mountain currently throws at them (follows `est`, slowly)
    pub diff: f32,
    pub observations: u32,
}

impl Default for Skill {
    fn default() -> Self {
        Skill { est: 0.45, diff: 0.4, observations: 0 }
    }
}

impl Skill {
    /// Positive = the player handled something well.
    pub fn event(&mut self, delta: f32) {
        // calibration: the first handful of observations count more
        let weight = 1.6 / (1.0 + self.observations as f32 * 0.12) + 0.4;
        self.est = (self.est + delta * weight).clamp(0.0, 1.0);
        self.observations += 1;
    }
}

pub fn skill_system(mut skill: ResMut<Skill>, time: Res<Time>) {
    let dt = time.delta_secs().min(0.05);
    // difficulty eases toward the estimate over ~20 s, never jumps
    let target = 0.15 + 0.8 * skill.est;
    skill.diff += (target - skill.diff) * (dt / 20.0).min(1.0);
}
