//! The mountain for the slice: one continuous climb of ledges joined by walls.
//! Coordinates are world pixels, x in 0..360, y up. Feet stand on `Ledge::y`.

use crate::LevelEntity;
use bevy::prelude::*;
use bevy::sprite::Anchor;

#[derive(Clone, Copy)]
pub struct Ledge {
    pub y: f32,
    pub x0: f32,
    pub x1: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WallKind {
    Rock,
    Ice,
}

#[derive(Clone, Copy)]
pub struct Wall {
    pub x: f32,
    pub y0: f32,
    pub y1: f32,
    pub kind: WallKind,
    /// +1 wall is on the right edge (hero faces right), -1 on the left.
    pub side: f32,
}

impl Wall {
    /// Where the hero's feet sit while on this wall.
    pub fn climb_x(&self) -> f32 {
        self.x - self.side * 14.0
    }
}

pub const LEDGES: [Ledge; 5] = [
    Ledge { y: 40.0, x0: 0.0, x1: 360.0 },   // 0 Base Camp (Day 1)
    Ledge { y: 200.0, x0: 30.0, x1: 360.0 }, // 1 first shelf
    Ledge { y: 430.0, x0: 0.0, x1: 360.0 },  // 2 exposed, windy traverse
    Ledge { y: 600.0, x0: 0.0, x1: 360.0 },  // 3 the avalanche slope
    Ledge { y: 830.0, x0: 0.0, x1: 290.0 },  // 4 first checkpoint (Day 3)
];

pub const WALLS: [Wall; 4] = [
    Wall { x: 342.0, y0: 40.0, y1: 200.0, kind: WallKind::Rock, side: 1.0 },
    Wall { x: 44.0, y0: 200.0, y1: 430.0, kind: WallKind::Ice, side: -1.0 },
    Wall { x: 342.0, y0: 430.0, y1: 600.0, kind: WallKind::Rock, side: 1.0 },
    Wall { x: 22.0, y0: 600.0, y1: 830.0, kind: WallKind::Ice, side: -1.0 },
];

pub const WORLD_TOP: f32 = 1000.0;

// story-relevant places
pub const CROSS_X: f32 = 150.0; // on the camp gear at base camp
pub const ANCHOR_X: f32 = 78.0; // ledge 2
pub const BOULDER_X: f32 = 205.0; // ledge 3 — the leeward side is safe
pub const SHELTER: (f32, f32) = (150.0, 190.0);
pub const COAT_X: f32 = 76.0; // ledge 3
pub const FLAGS_X: f32 = 150.0; // ledge 4

pub fn ledge_at(y: f32) -> Option<usize> {
    LEDGES.iter().position(|l| (l.y - y).abs() < 0.5)
}

#[derive(Component)]
pub struct CrossProp;
#[derive(Component)]
pub struct CoatProp;
#[derive(Component)]
pub struct CoatMound;
#[derive(Component)]
pub struct Debris;
#[derive(Component)]
pub struct Boulder;
#[derive(Component)]
pub struct AvalancheMass;
#[derive(Component)]
pub struct Crack;
#[derive(Component)]
pub struct RopeLine;

pub struct LevelPlugin;
impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_level_startup);
    }
}

fn spawn_level_startup(mut commands: Commands, assets: Res<AssetServer>) {
    spawn_level(&mut commands, &assets);
}

fn tiled(image: Handle<Image>, size: Vec2) -> Sprite {
    Sprite {
        image,
        custom_size: Some(size),
        image_mode: SpriteImageMode::Tiled { tile_x: true, tile_y: true, stretch_value: 1.0 },
        ..default()
    }
}

fn prop(commands: &mut Commands, image: Handle<Image>, pos: Vec3, size: Option<Vec2>) -> Entity {
    commands
        .spawn((
            LevelEntity,
            Sprite { image, custom_size: size, anchor: Anchor::BottomCenter, ..default() },
            Transform::from_translation(pos),
        ))
        .id()
}

pub fn spawn_level(commands: &mut Commands, assets: &AssetServer) {
    // the mountain face: two rock buttresses, open sky in the gully between
    commands.spawn((
        LevelEntity,
        Sprite { image: assets.load("sprites/face.png"), custom_size: Some(Vec2::new(360.0, 1002.0)), ..default() },
        Transform::from_xyz(180.0, 501.0 - 40.0, 1.0),
    ));
    // snowfield under base camp
    commands.spawn((
        LevelEntity,
        tiled(assets.load("sprites/snowfield.png"), Vec2::new(360.0, 80.0)),
        Transform::from_xyz(180.0, 0.0, 3.0),
    ));

    for w in WALLS.iter() {
        let img = match w.kind {
            WallKind::Rock => "sprites/wall_rock.png",
            WallKind::Ice => "sprites/wall_ice.png",
        };
        let h = w.y1 - w.y0;
        commands.spawn((
            LevelEntity,
            tiled(assets.load(img), Vec2::new(40.0, h + 8.0)),
            Transform::from_xyz(w.x, w.y0 + h / 2.0 - 4.0, 4.0),
        ));
    }
    for l in LEDGES.iter() {
        let w = l.x1 - l.x0;
        commands.spawn((
            LevelEntity,
            tiled(assets.load("sprites/ledge.png"), Vec2::new(w, 34.0)),
            Transform::from_xyz(l.x0 + w / 2.0, l.y - 15.0, 5.0),
        ));
    }

    // Base Camp
    prop(commands, assets.load("sprites/tent.png"), Vec3::new(92.0, LEDGES[0].y - 4.0, 6.0), None);
    prop(commands, assets.load("sprites/flags.png"), Vec3::new(250.0, LEDGES[0].y - 2.0, 5.5), None);
    prop(commands, assets.load("sprites/camp_gear.png"), Vec3::new(CROSS_X + 8.0, LEDGES[0].y - 4.0, 6.0), Some(Vec2::new(60.0, 65.0)));
    let c = prop(commands, assets.load("sprites/cross.png"), Vec3::new(CROSS_X - 2.0, LEDGES[0].y + 22.0, 6.5), Some(Vec2::new(30.0, 36.0)));
    commands.entity(c).insert(CrossProp);

    // ledge 2: the anchor for the exposed traverse
    prop(commands, assets.load("sprites/anchor.png"), Vec3::new(ANCHOR_X, LEDGES[2].y - 6.0, 6.0), Some(Vec2::new(40.0, 42.0)));

    // ledge 3: the boulder that can shelter him
    let b = prop(commands, assets.load("sprites/boulder.png"), Vec3::new(BOULDER_X, LEDGES[3].y - 6.0, 7.0), None);
    commands.entity(b).insert(Boulder);
    // the coat — hidden until the avalanche churns the slope
    let coat = commands
        .spawn((
            LevelEntity,
            CoatProp,
            Sprite { image: assets.load("sprites/coat_friend.png"), custom_size: Some(Vec2::new(48.0, 39.0)), anchor: Anchor::BottomCenter, ..default() },
            Transform::from_xyz(COAT_X, LEDGES[3].y - 4.0, 6.2).with_rotation(Quat::from_rotation_z(0.5)),
            Visibility::Hidden,
        ))
        .id();
    let _ = coat;
    commands.spawn((
        LevelEntity,
        CoatMound,
        Sprite { image: assets.load("sprites/mound.png"), custom_size: Some(Vec2::new(80.0, 44.0)), anchor: Anchor::BottomCenter, ..default() },
        Transform::from_xyz(COAT_X + 6.0, LEDGES[3].y - 8.0, 6.6),
        Visibility::Hidden,
    ));

    // story machinery, hidden until needed
    commands.spawn((
        LevelEntity,
        AvalancheMass,
        Sprite { image: assets.load("sprites/avalanche.png"), custom_size: Some(Vec2::new(270.0, 216.0)), ..default() },
        Transform::from_xyz(430.0, 820.0, 12.0),
        Visibility::Hidden,
    ));
    commands.spawn((
        LevelEntity,
        Crack,
        Sprite { image: assets.load("sprites/crack.png"), ..default() },
        Transform::from_xyz(262.0, 700.0, 3.0),
        Visibility::Hidden,
    ));
    commands.spawn((
        LevelEntity,
        RopeLine,
        Sprite::from_color(Color::srgb(0.72, 0.42, 0.18), Vec2::new(1.0, 2.0)),
        Transform::from_xyz(0., 0., 9.0),
        Visibility::Hidden,
    ));

    // ledge 4: first checkpoint
    prop(commands, assets.load("sprites/flags.png"), Vec3::new(FLAGS_X, LEDGES[4].y - 2.0, 5.5), None);
}
