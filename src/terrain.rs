//! One continuous mountain. The route is a single path that only ever climbs:
//! gentle snow, steep snow, rock and ice faces are just different angles of the same line.
//! The mountain's body fills everything below/right of the path.
//!
//! The hero lives at an arc-length `s` along the path. Falling = sliding back down `s`
//! under gravity and friction until the mountain itself stops him.

use crate::LevelEntity;
use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
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

fn spawn_terrain_startup(mut commands: Commands, terrain: Res<Terrain>, assets: Res<AssetServer>, mut images: ResMut<Assets<Image>>) {
    spawn_terrain(&mut commands, &terrain, &assets, &mut images);
}

/// A small RGBA texture we sample from while painting the mountain.
struct Tex {
    w: usize,
    h: usize,
    px: Vec<u8>,
}
impl Tex {
    fn load(bytes: &[u8]) -> Tex {
        use bevy::image::{CompressedImageFormats, ImageType};
        let img = Image::from_buffer(bytes, ImageType::Extension("png"), CompressedImageFormats::NONE, true, ImageSampler::Default, RenderAssetUsages::MAIN_WORLD)
            .expect("embedded texture");
        let rgba = img.convert(bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb).unwrap_or(img);
        Tex { w: rgba.width() as usize, h: rgba.height() as usize, px: rgba.data.clone().unwrap_or_default() }
    }
    fn at(&self, x: i32, y: i32) -> [u8; 4] {
        let xx = x.rem_euclid(self.w as i32) as usize;
        let yy = y.rem_euclid(self.h as i32) as usize;
        let i = (yy * self.w + xx) * 4;
        if i + 3 < self.px.len() { [self.px[i], self.px[i + 1], self.px[i + 2], 255] } else { [80, 80, 90, 255] }
    }
}

const TILE: i32 = 512;

/// Paint the mountain into a handful of 512px sprite tiles, once.
/// (No meshes: the 2D mesh pipeline doesn't compile on some old phones' OpenGL ES drivers.)
pub fn spawn_terrain(commands: &mut Commands, t: &Terrain, assets: &AssetServer, images: &mut Assets<Image>) {
    let body = Tex::load(include_bytes!("../assets/sprites/tex_body.png"));
    let rock = Tex::load(include_bytes!("../assets/sprites/tex_rock.png"));
    let ice = Tex::load(include_bytes!("../assets/sprites/tex_ice.png"));
    let snow = Tex::load(include_bytes!("../assets/sprites/tex_snow.png"));
    let skin = |s: Surface| match s {
        Surface::Snow => &snow,
        Surface::Ice => &ice,
        Surface::Rock => &rock,
    };
    let end = t.segs.last().unwrap().b;
    let x0 = -40;
    let x1 = end.x as i32 + 420;
    let y_min = t.segs.iter().map(|g| g.a.y).fold(f32::MAX, f32::min) as i32 - 360;
    let y_max = end.y as i32 + 40;
    let w = (x1 - x0) as usize;
    // surface height and surface type for every pixel column
    let mut top = vec![f32::MIN; w];
    let mut surf = vec![Surface::Snow; w];
    for g in &t.segs {
        if g.b.x - g.a.x < 0.01 {
            continue;
        }
        let (xa, xb) = (g.a.x.floor() as i32, g.b.x.ceil() as i32);
        for x in xa..=xb {
            let i = (x - x0) as usize;
            if i >= w {
                continue;
            }
            let k = ((x as f32 - g.a.x) / (g.b.x - g.a.x)).clamp(0.0, 1.0);
            let y = g.a.y + (g.b.y - g.a.y) * k;
            if y > top[i] {
                top[i] = y;
                surf[i] = g.surf;
            }
        }
    }
    for i in 0..w {
        if top[i] == f32::MIN {
            top[i] = if (i as i32 + x0) < 0 { t.segs[0].a.y } else { end.y };
        }
    }
    // faces: the skin wraps round the vertical walls
    let walls: Vec<(f32, f32, f32, Surface)> = t.segs.iter().filter(|g| g.wall).map(|g| (g.a.x.min(g.b.x), g.a.y, g.b.y, g.surf)).collect();

    let mut ty = y_min;
    while ty < y_max {
        let mut tx = x0;
        while tx < x1 {
            let mut data = vec![0u8; (TILE * TILE * 4) as usize];
            let mut any = false;
            for py in 0..TILE {
                let wy = (ty + TILE - 1 - py) as f32; // image rows go downwards
                for px in 0..TILE {
                    let wx = tx + px;
                    let i = (wx - x0) as usize;
                    if i >= w {
                        continue;
                    }
                    let depth = top[i] - wy;
                    if depth < 0.0 {
                        continue;
                    }
                    let mut c = if depth < 11.0 {
                        skin(surf[i]).at(wx, -(wy as i32))
                    } else {
                        let mut c = body.at(wx, -(wy as i32));
                        // darker deeper in, so the surface reads
                        let f = (1.0 - ((depth - 11.0) / 400.0).min(0.45)) * 0.75;
                        for k in 0..3 {
                            c[k] = (c[k] as f32 * f * [0.95, 1.0, 1.15][k]).min(255.0) as u8;
                        }
                        c
                    };
                    for &(wxw, wy0, wy1, ws) in &walls {
                        let dx = wx as f32 - wxw;
                        if dx >= 0.0 && dx < 14.0 && wy >= wy0 && wy <= wy1 {
                            c = skin(ws).at(wx, -(wy as i32));
                        }
                    }
                    let o = ((py * TILE + px) * 4) as usize;
                    data[o..o + 4].copy_from_slice(&c);
                    any = true;
                }
            }
            if any {
                let img = Image::new(
                    bevy::render::render_resource::Extent3d { width: TILE as u32, height: TILE as u32, depth_or_array_layers: 1 },
                    bevy::render::render_resource::TextureDimension::D2,
                    data,
                    bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::RENDER_WORLD,
                );
                commands.spawn((
                    LevelEntity,
                    Sprite { image: images.add(img), anchor: Anchor::BottomLeft, ..default() },
                    Transform::from_xyz(tx as f32, ty as f32, 2.0),
                ));
            }
            tx += TILE;
        }
        ty += TILE;
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
