"""Render the hero sprite frames from art/hero/hero.blend.

Needs art/hero/hero.blend (kept out of git: it holds the Mixamo motion data, which may not
be redistributed; keep a private backup). Run inside Blender (the MCP add-on or the Scripting tab):
    exec(open(r"<repo>/tools/blender/hero_sheet.py").read())
    render_all(out_dir)            # or render_row(row, out_dir, coat)

Writes <out_dir>/<plain|coat>/<row>_<col>.png at 4x the cell size (384x352).
tools/hero_pack.py turns those into assets/sprites/hero.png and hero_coat.png.

Every frame is the same rigged model (Mixamo clips on our own climber), so
coat, pack and proportions never change between frames.
"""
import bpy, math, os
from mathutils import Vector, Matrix

SC = bpy.context.scene
ARM = bpy.data.objects["HeroRig"]
O = bpy.data.objects
CELL = (96, 88)
SCALE = 4
GROUND_PX = 86  # feet rest on this pixel row of the cell
VIEW = Vector((-1, 0, 0))  # camera looks along +X: we see his right side (mirrored later so he faces right)
ORTHO = 2.69  # metres across the cell width -> the standing hero is ~66 px tall

COAT = ("CoatRoll", "CoatStrap", "CoatStrap2")


def use(name):
    a = bpy.data.actions["A_" + name]
    ARM.animation_data.action = a
    try:
        ARM.animation_data.action_slot = a.slots[0]
    except Exception:
        pass
    return [int(x) for x in a.frame_range]


def bone_w(name):
    return ARM.matrix_world @ ARM.pose.bones["mixamorig:" + name].head


def ik(on):
    for s in ("Left", "Right"):
        ARM.pose.bones[f"mixamorig:{s}ForeArm"].constraints["RopeIK"].influence = 1.0 if on else 0.0


def rope_update():
    sp = bpy.data.curves["RopeHand"].splines[0]
    L, R = bone_w("LeftHand"), bone_w("RightHand")
    pts = [L + Vector((0, -0.06, 0)), (L + R) / 2 + Vector((0, -0.16, -0.18)), Vector((0, -0.22, 0.90))]
    for p, co in zip(sp.bezier_points, pts):
        p.co = co
        p.handle_left_type = p.handle_right_type = 'AUTO'


def shovel_update():
    sh = O["Shovel"]
    R, L = bone_w("RightHand"), bone_w("LeftHand")
    rh = (ARM.matrix_world.to_3x3() @ ARM.pose.bones["mixamorig:RightHand"].matrix.to_3x3()).normalized()
    d = L - R
    d = d.normalized() if d.length > 1e-4 else Vector((0, -0.3, -1)).normalized()
    z = -d
    x = Vector((0, 0, 1)).cross(z)
    if x.length < 1e-3:
        x = Vector((1, 0, 0))
    x.normalize()
    m = Matrix((x, z.cross(x), z)).transposed().to_4x4()
    m.translation = R + rh.col[1] * 0.07
    sh.matrix_world = m


def axe_point(direction):
    """Turn the axe about its grip so the spike points along `direction` (world)."""
    axe = O["Axe"]
    mw = axe.matrix_world.copy()
    spike = (mw.to_3x3() @ Vector((0, 0, -1))).normalized()
    q = spike.rotation_difference(Vector(direction).normalized())
    rot = q.to_matrix().to_4x4()
    t = mw.translation.copy()
    axe.matrix_world = Matrix.Translation(t) @ rot @ Matrix.Translation(-t) @ mw


def roll_onto_side(deg):
    """Roll the whole rig about the body's long axis through the hips (back -> stomach)."""
    hips = bone_w("Hips")
    head = bone_w("Head")
    axis = (head - hips)
    axis.z = 0
    axis = axis.normalized() if axis.length > 1e-3 else Vector((0, 1, 0))
    rot = Matrix.Rotation(math.radians(deg), 4, axis)
    ARM.matrix_world = Matrix.Translation(hips) @ rot @ Matrix.Translation(-hips) @ ARM.matrix_world


def contacts(name):
    """Frames where the feet are furthest apart (the two heel strikes of a walk cycle)."""
    f0, f1 = use(name)
    gap = []
    for f in range(f0, f1):
        SC.frame_set(f)
        gap.append((abs(bone_w("LeftFoot").y - bone_w("RightFoot").y), f))
    gap.sort(reverse=True)
    first = gap[0][1]
    return f0, f1, first


def walk_frames(name):
    """5 frames of one loop, phased so columns 1 and 3 land on heel strikes (footstep sounds)."""
    f0, f1, strike = contacts(name)
    n = f1 - f0
    start = strike - n / 5.0
    return [f0 + int(round((start - f0 + i * n / 5.0))) % n for i in range(5)]


# row -> list of 5 frame specs: (clip, frame, props, extra)
def plan():
    walk = walk_frames("walk_heavy")
    tired = walk_frames("walk_tired")
    return [
        ("idle", [("idle_breathing", f, (), None) for f in (1, 60, 120, 180, 240)]),
        ("walk", [("walk_heavy", f, (), None) for f in walk]),
        ("climb", [("climb_wall", f, (), None) for f in (1, 13, 25, 37, 49)]),
        ("axe", [("climb_wall", f, ("axe",), None) for f in (52, 7, 13, 31, 40)]),
        ("rope", [("idle_breathing", 1 + i * 20, ("rope",), ("ik", i)) for i in range(5)]),
        ("dig", [("kneel_campfire", 107, (), None),
                 ("dig", 37, ("shovel",), None),
                 ("dig", 61, ("shovel",), None),
                 ("dig", 85, ("shovel",), None),
                 ("kneel_campfire", 45, ("axe",), ("probe", None))]),
        ("exhausted", [("kneel_campfire", f, (), None) for f in (43, 75, 150, 107, 280)]),
        ("fall", [("fall_sweep", 15, (), None),
                  ("fall_sweep", 40, (), None),
                  ("fall_sweep", 76, (), ("roll", -90)),
                  ("rise_prone", 1, (), None),
                  ("fall_sweep", 60, (), None)]),
        ("injured", [("rise_prone", 131, (), None),
                     ("rise_prone", 99, (), None),
                     ("rise_prone", 66, (), None),
                     ("rise_prone", 1, (), None),
                     ("rise_prone", 1, ("axe",), ("arrest", None))]),
        ("walk_tired", [("walk_tired", f, (), None) for f in tired]),
        # --- v2: ledges, escapes, tools, weather, story
        ("climb_axe", [("climb_wall", f, ("axe",), None) for f in (1, 13, 25, 37, 49)]),
        ("mantle", [("mantle", f, (), ("follow", None)) for f in (1, 30, 55, 80, 111)]),
        ("catch", [("catch_ledge", f, ("axe",), ("follow", None)) for f in (1, 12, 23, 34, 46)]),
        ("hang", [("hang", f, (), ("follow", None)) for f in (1, 29, 57, 85, 113)]),
        ("dive", [("dive_roll", f, (), None) for f in (1, 12, 22, 32, 44)]),
        ("brace", [("brace_in", 1, (), None), ("brace_in", 20, (), None), ("brace_in", 39, (), None),
                   ("brace", 1, (), None), ("brace", 40, (), None)]),
        ("swim", [("swim", f, (), None) for f in (1, 28, 55, 82, 110)]),
        ("cover", [("cover_face", f, (), None) for f in (1, 60, 120, 240, 400)]),
        ("equip_axe", equip("axe")),
        ("equip_shovel", equip("shovelgrip")),
        ("equip_rope", equip("coilhand")),
        ("wind", [("wind_stagger", f, (), None) for f in (1, 33, 66, 99, 130)]),
        ("teeter", [("teeter", f, (), None) for f in (1, 12, 23, 34, 45)]),
        ("dizzy", [("dizzy", f, (), None) for f in (1, 26, 52, 78, 104)]),
        ("pickup", [("pickup", f, (), None) for f in (1, 50, 90, 130, 200)]),
        ("cry", [("cry", f, (), None) for f in (1, 40, 80, 120, 160)]),
        ("sad", [("sad_idle", f, (), None) for f in (1, 18, 35, 52, 69)]),
    ]


def equip(tool):
    """Reach over the shoulder for a tool: on the pack in cols 0-2, in the hand from col 3.
    The game plays the row backwards to stow it."""
    return [("equip_shoulder", f, (tool,) if i >= 3 else (), None) for i, f in enumerate((1, 13, 25, 37, 51))]


IK_BASE = {"L": Vector((0.05, -0.30, 0.98)), "R": Vector((-0.07, -0.30, 1.02))}
IK_FUMBLE = [((0, 0, 0), (0, 0, 0)), ((0, 0, 0.04), (0, 0, -0.03)), ((-0.03, 0, 0), (0.04, 0, 0)),
             ((0, -0.03, 0.05), (-0.06, 0.02, -0.02)), ((0, 0, 0.01), (0, 0, 0.01))]


def setup_render():
    SC.render.engine = 'CYCLES'
    SC.cycles.device = 'CPU'
    SC.cycles.samples = 12
    SC.cycles.use_denoising = False
    SC.render.film_transparent = True
    SC.view_settings.exposure = 0.45
    SC.render.resolution_x = CELL[0] * SCALE
    SC.render.resolution_y = CELL[1] * SCALE
    SC.render.resolution_percentage = 100
    cam = SC.camera
    cam.data.type = 'ORTHO'
    cam.data.ortho_scale = ORTHO


def frame(spec, path, coat):
    clip, f, props, extra = spec
    arm_mw = ARM.matrix_world.copy()
    axe_mb = O["Axe"].matrix_basis.copy()
    use(clip)
    ik(False)
    if extra and extra[0] == "ik":
        oL, oR = IK_FUMBLE[extra[1]]
        O["IK_L"].location = IK_BASE["L"] + Vector(oL)
        O["IK_R"].location = IK_BASE["R"] + Vector(oR)
        ik(True)
    if extra and extra[0] == "probe":
        O["IK_R"].location = (-0.12, -0.50, 0.80)
        ARM.pose.bones["mixamorig:RightForeArm"].constraints["RopeIK"].influence = 1.0
    SC.frame_set(f)
    bpy.context.view_layer.update()
    if extra and extra[0] == "roll":
        roll_onto_side(extra[1])
        bpy.context.view_layer.update()
    O["Axe"].hide_render = "axe" not in props
    O["Shovel"].hide_render = "shovel" not in props
    O["ShovelGrip"].hide_render = "shovelgrip" not in props
    O["CoilHand"].hide_render = "coilhand" not in props
    O["RopeHand"].hide_render = "rope" not in props
    O["RopeCoil"].hide_render = True
    # tools live on the pack unless they are in his hands
    O["AxeBag"].hide_render = "axe" in props
    O["ShovelBag"].hide_render = "shovel" in props or "shovelgrip" in props
    O["CoilBag"].hide_render = "coilhand" in props
    for n in COAT:
        O[n].hide_render = not coat
    if "rope" in props:
        rope_update()
    if "shovel" in props:
        shovel_update()
    if extra and extra[0] == "probe":
        axe_point((0, -0.45, -1))
    if extra and extra[0] == "arrest":
        axe_point((0, -1, -0.3))
    bpy.context.view_layer.update()
    # centre on the hips horizontally; the ground stays on the same pixel row in every cell,
    # except for ledge rows, where the camera follows him up and the hands mark the ledge
    hp = bone_w("Hips")
    h = ORTHO * CELL[1] / CELL[0]
    cz = h / 2 - (CELL[1] - GROUND_PX) / CELL[1] * h
    if extra and extra[0] == "follow":
        cz += hp.z - 0.95
    META[path] = {"hands": px((bone_w("LeftHand") + bone_w("RightHand")) / 2, hp.y, cz, h),
                  "hips": px(hp, hp.y, cz, h), "ground_z": round((cz - (h / 2 - (CELL[1] - GROUND_PX) / CELL[1] * h)) * CELL[1] / h, 2)}
    cam = SC.camera
    cam.location = Vector((0, hp.y, cz)) - VIEW * 6
    cam.rotation_euler = VIEW.to_track_quat('-Z', 'Y').to_euler()
    SC.render.filepath = path
    bpy.ops.render.render(write_still=True)
    ARM.matrix_world = arm_mw
    O["Axe"].matrix_basis = axe_mb
    ik(False)


META = {}


def px(p, cy, cz, h):
    """World point -> cell pixel (x from the left after mirroring so he faces right, y from the top)."""
    x = CELL[0] / 2 - (p.y - cy) / ORTHO * CELL[0]  # +y (behind him) is left once mirrored
    y = CELL[1] / 2 - (p.z - cz) / h * CELL[1]
    return [round(x, 1), round(y, 1)]


def render_row(row, out_dir, coat=False):
    import json
    setup_render()
    name, specs = plan()[row]
    d = os.path.join(out_dir, "coat" if coat else "plain")
    os.makedirs(d, exist_ok=True)
    for col, spec in enumerate(specs):
        path = os.path.join(d, f"{row}_{col}.png")
        frame(spec, path, coat)
        if not coat:
            mp = os.path.join(out_dir, "meta.json")
            meta = json.load(open(mp)) if os.path.exists(mp) else {}
            meta[f"{row}_{col}"] = dict(META[path], row=name)
            json.dump(meta, open(mp, "w"), indent=0)
    return name


def render_all(out_dir):
    for coat in (False, True):
        for r in range(len(plan())):
            render_row(r, out_dir, coat)
