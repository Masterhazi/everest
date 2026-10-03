//! One continuous mountain. The route is a single path that only ever climbs:
//! gentle snow, steep snow, rock and ice faces are just different angles of the same line.
//! The mountain's body fills everything below/right of the path.
//!
//! The hero lives at an arc-length `s` along the path. Falling = sliding back down `s`
//! under gravity and friction until the mountain itself stops him.

use crate::LevelEntity;
use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::sprite::Anchor;
use rand::Rng;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Surface {
    Snow,
    Ice,
    Rock,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Zone {
    None,
    Camp,
    Rockfall,
    Crevasse,
    StoryAvalanche,
    Avalanche,
    Seracs,
    Checkpoint,
}

/// Authored shape of the route: (length, angle in degrees, surface, zone).
/// Micro-shape is randomised every run so the mountain never looks ruled.
pub const ROUTE: [(f32, f32, Surface, Zone); 17] = [
    (260.0, 0.0, Surface::Snow, Zone::Camp),            // 0  Base Camp, Day 1
    (240.0, 14.0, Surface::Snow, Zone::None),           // 1  first slope
    (200.0, 32.0, Surface::Snow, Zone::Avalanche),      // 2  first steep snow
    (90.0, 4.0, Surface::Snow, Zone::None),             // 3  shelf
    (150.0, 90.0, Surface::Rock, Zone::Rockfall),       // 4  rock face, rockfall
    (110.0, 6.0, Surface::Rock, Zone::None),            // 5  ledge
    (170.0, 84.0, Surface::Ice, Zone::None),            // 6  ice face
    (90.0, 8.0, Surface::Snow, Zone::None),             // 7
    (150.0, 10.0, Surface::Snow, Zone::Crevasse),       // 8  hidden snow bridge
    (80.0, 6.0, Surface::Snow, Zone::None),             // 9
    (240.0, 30.0, Surface::Snow, Zone::StoryAvalanche), // 10 the first avalanche, the coat
    (100.0, 5.0, Surface::Snow, Zone::None),            // 11
    (280.0, 18.0, Surface::Snow, Zone::Seracs),         // 12 under the ice towers; dusk
    (60.0, 4.0, Surface::Snow, Zone::None),             // 13 night
    (170.0, 80.0, Surface::Ice, Zone::None),            // 14 ice face in the dark
    (220.0, 34.0, Surface::Snow, Zone::Avalanche),      // 15 steep snow at night
    (240.0, 3.0, Surface::Snow, Zone::Checkpoint),      // 16 dawn, first checkpoint (Day 3)
];

pub const WALL_DEG: f32 = 65.0;

#[derive(Clone, Copy, Debug)]
pub struct Seg {
    pub a: Vec2,
    pub b: Vec2,
    pub s0: f32,
    pub len: f32,
    pub dir: Vec2,
    pub deg: f32,
    pub surf: Surface,
    pub zone: Zone,
    /// index into ROUTE
    pub part: usize,
    pub wall: bool,
}

#[derive(Resource, Default)]
pub struct Terrain {
    pub segs: Vec<Seg>,
    /// arc-length where each ROUTE part starts (len = ROUTE.len()+1)
    pub part_s: Vec<f32>,
    pub total: f32,
    /// rope anchors, as arc-lengths
    pub anchors: Vec<f32>,
}

impl Terrain {
    pub fn generate() -> Terrain {
        let mut rng = rand::thread_rng();
        let mut segs = vec![];
        let mut part_s = vec![];
        let mut p = Vec2::new(0.0, 40.0);
        let mut s = 0.0;
        for (part, &(len, deg, surf, zone)) in ROUTE.iter().enumerate() {
            part_s.push(s);
            let wall = deg >= WALL_DEG;
            // split into short pieces with a little wander (less on faces)
            let n = (len / 30.0).ceil().max(1.0) as usize;
            let piece = len / n as f32;
            for k in 0..n {
                let jitter = if wall { rng.gen_range(-3.0..3.0) } else if part == 0 || zone == Zone::Checkpoint { rng.gen_range(-1.5..1.5) } else { rng.gen_range(-7.0..7.0) };
                // never let the path turn back on itself (keeps the mountain solid on the right)
                let d = (deg + jitter).clamp(-4.0, 90.0);
                let _ = k;
                let r = d.to_radians();
                let dir = Vec2::new(r.cos(), r.sin());
                let b = p + dir * piece;
                segs.push(Seg { a: p, b, s0: s, len: piece, dir, deg: d, surf, zone, part, wall });
                p = b;
                s += piece;
            }
        }
        part_s.push(s);
        let anchors = vec![
            part_s[4] - 14.0,  // foot of the rock face
            part_s[6] - 14.0,  // foot of the ice face
            part_s[10] + 20.0, // bottom of the avalanche slope
            part_s[14] - 14.0, // foot of the night ice face
            part_s[15] + 110.0,
        ];
        Terrain { segs, part_s, total: s, anchors }
    }

    pub fn idx(&self, s: f32) -> usize {
        let s = s.clamp(0.0, self.total - 0.01);
        // segments are short and few; binary search on s0
        match self.segs.binary_search_by(|g| g.s0.partial_cmp(&s).unwrap()) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        }
    }
    pub fn seg(&self, s: f32) -> &Seg {
        &self.segs[self.idx(s)]
    }
    pub fn point(&self, s: f32) -> Vec2 {
        let g = self.seg(s);
        g.a + g.dir * (s.clamp(0.0, self.total) - g.s0).clamp(0.0, g.len)
    }
    pub fn part(&self, s: f32) -> usize {
        self.seg(s).part
    }
    pub fn part_range(&self, part: usize) -> (f32, f32) {
        (self.part_s[part], self.part_s[part + 1])
    }
    /// The height of the mountain's surface at world x (for props and debris).
    pub fn top_at_x(&self, x: f32) -> f32 {
        for g in &self.segs {
            if !g.wall && x >= g.a.x && x <= g.b.x && g.b.x > g.a.x {
                let t = (x - g.a.x) / (g.b.x - g.a.x);
                return g.a.y + (g.b.y - g.a.y) * t;
            }
        }
        self.point(self.total).y
    }
}

pub struct TerrainPlugin;
impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Terrain::generate()).add_systems(Startup, spawn_terrain_startup);
    }
}

fn spawn_terrain_startup(
    mut commands: Commands,
    terrain: Res<Terrain>,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<ColorMaterial>>,
) {
    spawn_terrain(&mut commands, &terrain, &assets, &mut meshes, &mut mats);
}

fn repeat_tex(assets: &AssetServer, path: &'static str) -> Handle<Image> {
    assets.load_with_settings(path, |s: &mut ImageLoaderSettings| {
        s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::Repeat,
            ..ImageSamplerDescriptor::nearest()
        });
    })
}

/// Simple triangle-list mesh builder with world-space UVs (textures repeat every `tile` px).
struct MeshB {
    pos: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    idx: Vec<u32>,
    tile: f32,
}
impl MeshB {
    fn new(tile: f32) -> Self {
        MeshB { pos: vec![], uv: vec![], idx: vec![], tile }
    }
    fn quad(&mut self, q: [Vec2; 4]) {
        let base = self.pos.len() as u32;
        for p in q {
            self.pos.push([p.x, p.y, 0.0]);
            self.uv.push([p.x / self.tile, -p.y / self.tile]);
        }
        self.idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    fn build(self) -> Mesh {
        let n = self.pos.len();
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.pos)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; n])
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uv)
            .with_inserted_indices(Indices::U32(self.idx))
    }
}

pub fn spawn_terrain(
    commands: &mut Commands,
    t: &Terrain,
    assets: &AssetServer,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<ColorMaterial>,
) {
    const BOTTOM: f32 = -700.0;
    // the mountain's body
    let mut body = MeshB::new(64.0);
    for g in &t.segs {
        if g.b.x - g.a.x > 0.01 {
            body.quad([Vec2::new(g.a.x, BOTTOM), Vec2::new(g.b.x, BOTTOM), g.b, g.a]);
        }
    }
    // keep going past the end so the summit side isn't a cliff into nothing
    let end = t.segs.last().unwrap().b;
    body.quad([Vec2::new(end.x, BOTTOM), Vec2::new(end.x + 900.0, BOTTOM), Vec2::new(end.x + 900.0, end.y + 60.0), end]);
    commands.spawn((
        LevelEntity,
        Mesh2d(meshes.add(body.build())),
        MeshMaterial2d(mats.add(ColorMaterial { color: Color::srgb(0.55, 0.6, 0.75), texture: Some(repeat_tex(assets, "sprites/tex_body.png")), ..default() })),
        Transform::from_xyz(0.0, 0.0, 2.0),
    ));

    // the skin: snow / ice / rock along the path, a little thickness into the mountain
    for (surf, path, tile, z) in [
        (Surface::Rock, "sprites/tex_rock.png", 40.0, 3.0),
        (Surface::Ice, "sprites/tex_ice.png", 44.0, 3.1),
        (Surface::Snow, "sprites/tex_snow.png", 40.0, 3.2),
    ] {
        let mut m = MeshB::new(tile);
        for g in t.segs.iter().filter(|g| g.surf == surf) {
            let n = Vec2::new(g.dir.y, -g.dir.x); // into the mountain
            let th = if g.wall { 14.0 } else { 11.0 };
            // overlap neighbours slightly so joints don't show
            let a = g.a - g.dir * 2.0;
            let b = g.b + g.dir * 2.0;
            m.quad([a + n * th, b + n * th, b + Vec2::new(0.0, 1.5), a + Vec2::new(0.0, 1.5)]);
        }
        if m.pos.is_empty() {
            continue;
        }
        commands.spawn((
            LevelEntity,
            Mesh2d(meshes.add(m.build())),
            MeshMaterial2d(mats.add(ColorMaterial { texture: Some(repeat_tex(assets, path)), ..default() })),
            Transform::from_xyz(0.0, 0.0, z),
        ));
    }

    // props that sit on the surface
    let on = |s: f32| t.point(s);
    let prop = |commands: &mut Commands, img: &str, p: Vec2, z: f32, size: Option<Vec2>| {
        commands
            .spawn((
                LevelEntity,
                Sprite { image: assets.load(img.to_string()), custom_size: size, anchor: Anchor::BottomCenter, ..default() },
                Transform::from_translation(p.extend(z)),
            ))
            .id()
    };
    prop(commands, "sprites/tent.png", on(70.0) - Vec2::Y * 3.0, 6.0, None);
    prop(commands, "sprites/flags.png", on(205.0) - Vec2::Y * 2.0, 5.5, None);
    prop(commands, "sprites/camp_gear.png", on(CROSS_S + 10.0) - Vec2::Y * 3.0, 6.0, Some(Vec2::new(60.0, 65.0)));
    let c = prop(commands, "sprites/cross.png", on(CROSS_S) + Vec2::Y * 22.0, 6.5, Some(Vec2::new(30.0, 36.0)));
    commands.entity(c).insert(crate::story::CrossProp);
    for &a in &t.anchors {
        prop(commands, "sprites/anchor.png", on(a) - Vec2::Y * 4.0, 6.0, Some(Vec2::new(30.0, 32.0)));
    }
    let (c0, _) = t.part_range(16);
    prop(commands, "sprites/flags.png", on(c0 + 90.0) - Vec2::Y * 2.0, 5.5, None);
}

/// Where the cross waits at Base Camp.
pub const CROSS_S: f32 = 150.0;
