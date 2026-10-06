"""Pack the Blender hero renders into the game's sprite sheets.

    python tools/hero_pack.py <render_dir> <out_dir>

<render_dir>/plain/<row>_<col>.png (and optionally coat/...) come from
tools/blender/hero_sheet.py at 4x cell size. Each frame is downscaled to a
96x88 cell, snapped to one palette shared by every frame (so colours never
flicker between frames), given a 1 px dark outline so he reads against snow,
and mirrored so he faces right. Writes hero.png (and hero_coat.png when the
coat renders exist) as a 5-column grid, plus a 3x preview.
"""
import os
import sys
from PIL import Image

CELL = (96, 88)
COLS, ROWS = 5, 10
COLOURS = 28
OUTLINE = (18, 20, 28, 255)
ALPHA_CUT = 110


def load_cells(d):
    cells = {}
    for r in range(ROWS):
        for c in range(COLS):
            p = os.path.join(d, f"{r}_{c}.png")
            if not os.path.exists(p):
                continue
            im = Image.open(p).convert("RGBA").resize(CELL, Image.LANCZOS)
            a = im.getchannel("A").point(lambda v: 255 if v > ALPHA_CUT else 0)
            im.putalpha(a)
            cells[(r, c)] = im.transpose(Image.FLIP_LEFT_RIGHT)
    return cells


def rare(p):
    # small but important colours (rope blue, coat yellow, steel, orange accents) that a plain
    # median cut would merge into the big red/black areas
    r, g, b = p
    return b > r + 20 or (r > 120 and g > 90 and b < 70 and g > r * 0.6) or (min(p) > 110 and max(p) - min(p) < 35)


def cut(px, n):
    strip = Image.new("RGB", (len(px), 1))
    strip.putdata(px)
    q = strip.quantize(colors=n, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
    return q.getpalette()[: 3 * len(set(q.get_flattened_data()))]


def palette_from(all_cells):
    # every opaque pixel from every frame -> one shared palette, plus a few slots for rare colours
    px = []
    for im in all_cells:
        px += [p[:3] for p in im.get_flattened_data() if p[3]]
    few = [p for p in px if rare(p)]
    colours = cut(px, COLOURS - 8) + (cut(few, 8) if few else [])
    pal = Image.new("P", (1, 1))
    pal.putpalette(colours + colours[:3] * (256 - len(colours) // 3))
    return pal


def snap(im, pal):
    rgb = im.convert("RGB").quantize(palette=pal, dither=Image.Dither.NONE).convert("RGB")
    out = rgb.convert("RGBA")
    out.putalpha(im.getchannel("A"))
    return out


def outline(im):
    w, h = im.size
    src = im.load()
    out = im.copy()
    dst = out.load()
    for y in range(h):
        for x in range(w):
            if src[x, y][3]:
                continue
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                nx, ny = x + dx, y + dy
                if 0 <= nx < w and 0 <= ny < h and src[nx, ny][3]:
                    dst[x, y] = OUTLINE
                    break
    return out


def sheet(cells, pal):
    out = Image.new("RGBA", (CELL[0] * COLS, CELL[1] * ROWS), (0, 0, 0, 0))
    for (r, c), im in cells.items():
        out.paste(outline(snap(im, pal)), (c * CELL[0], r * CELL[1]))
    return out


def preview(sh, path):
    bg = Image.new("RGBA", sh.size, (205, 214, 226, 255))
    bg.alpha_composite(sh)
    bg.resize((sh.width * 3, sh.height * 3), Image.NEAREST).save(path)


def main():
    src, dst = sys.argv[1], sys.argv[2]
    os.makedirs(dst, exist_ok=True)
    sets = {name: load_cells(os.path.join(src, name)) for name in ("plain", "coat")
            if os.path.isdir(os.path.join(src, name))}
    pal = palette_from([im for cells in sets.values() for im in cells.values()])
    for name, cells in sets.items():
        sh = sheet(cells, pal)
        fn = "hero.png" if name == "plain" else "hero_coat.png"
        sh.save(os.path.join(dst, fn))
        preview(sh, os.path.join(dst, fn.replace(".png", "_preview.png")))
        print(fn, len(cells), "cells")


if __name__ == "__main__":
    main()
