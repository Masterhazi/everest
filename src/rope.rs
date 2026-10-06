//! The rope, from the anchor's ring to his harness. Slack rope sags between them; a rope that
//! is holding him (or he's below the anchor, hanging on it) runs straight. Drawn as short
//! two-tone pieces so it reads as a twisted kernmantle rope, not a line.

use crate::hero::{HState, Hero, HERO_SCALE};
use crate::hero_frames::HIPS;
use crate::terrain::Terrain;
use bevy::prelude::*;

const PIECES: usize = 40;
const DARK: Color = Color::srgb(0.13, 0.30, 0.62);
const LIGHT: Color = Color::srgb(0.40, 0.62, 0.92);
/// how much it hangs when he's close to the anchor (px); less as the rope pays out
const SAG: f32 = 16.0;
/// the rope runs out at this distance (see hero_system)
const LENGTH: f32 = 320.0;

#[derive(Component)]
pub struct RopePiece(usize);

pub struct RopePlugin;
impl Plugin for RopePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_rope);
    }
}

fn spawn_rope(mut commands: Commands) {
    for i in 0..PIECES {
        let c = if i % 2 == 0 { DARK } else { LIGHT };
        commands.spawn((RopePiece(i), Sprite::from_color(c, Vec2::ONE), Transform::from_xyz(0.0, 0.0, 9.0), Visibility::Hidden));
    }
}

pub fn rope_system(terrain: Res<Terrain>, hero: Query<&Hero>, mut pieces: Query<(&RopePiece, &mut Transform, &mut Visibility)>) {
    let Ok(h) = hero.single() else { return };
    let anchor = h.rope.filter(|_| h.state != HState::Buried);
    let Some(a) = anchor else {
        for (_, _, mut vis) in &mut pieces {
            *vis = Visibility::Hidden;
        }
        return;
    };
    // the ring on top of the anchor stake, and his harness in the frame he's showing
    let ring = terrain.point(a) + Vec2::new(4.0, 26.0);
    let (row, col) = (h.cell / 5, h.cell % 5);
    let (hx, hy) = HIPS.get(row).map(|r| r[col]).unwrap_or((0.0, 34.0));
    let harness = h.pos + Vec2::new(hx * h.facing, hy) * HERO_SCALE;
    let dist = (harness - ring).length();
    let loaded = matches!(h.state, HState::Slide { .. }) || h.s < a;
    let sag = if loaded { 0.0 } else { SAG * (1.0 - dist / LENGTH).clamp(0.0, 1.0) * (dist / 60.0).min(1.0) };
    let at = |t: f32| ring.lerp(harness, t) - Vec2::Y * sag * 4.0 * t * (1.0 - t);
    for (piece, mut tr, mut vis) in &mut pieces {
        let (p0, p1) = (at(piece.0 as f32 / PIECES as f32), at((piece.0 + 1) as f32 / PIECES as f32));
        let d = p1 - p0;
        tr.translation = ((p0 + p1) / 2.0).extend(9.0);
        tr.rotation = Quat::from_rotation_z(d.y.atan2(d.x));
        tr.scale = Vec3::new(d.length() + 0.5, 2.0, 1.0);
        *vis = Visibility::Visible;
    }
}
