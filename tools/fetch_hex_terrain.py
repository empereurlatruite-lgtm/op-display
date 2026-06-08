#!/usr/bin/env python3
"""
Fetch a real Foxhole hex's texture + heightmap from the community tiles
(pickles976/Foxhole-Map-3D-Tiles, heightmap by Derp / NoUDerp) and crop it to
the hex bounding box for use as the op-board terrain.

Usage:  python3 fetch_hex_terrain.py ENDLESS
Writes: web/<slug>.png (texture, cyan water recolored) and
        web/<slug>-height.png (grayscale heightmap, white = high).

Coordinate math mirrors Foxhole-Map-3D's config.js / utils.js OffsetToPosition.
The whole world heightmap is 16384x16384; each hex is HEX_W x HEX_H * SCALE.
"""
import sys, io, os, urllib.request
from PIL import Image

TILES = "https://raw.githubusercontent.com/pickles976/Foxhole-Map-3D-Tiles/main"

# from regions.js — [x (east/west, *HEX_W), y (north/south, *HEX_H)]
REGIONS = {
    "ENDLESS": ([1.5, 0], "endless-shore"),
    "DEADLANDS": ([0, 0], "deadlands"),
    "FARRANAC": ([-1.5, 0], "farranac-coast"),
    "GODCROFTS": ([2.25, 0.5], "godcrofts"),
    "STONECRADLE": ([-1.5, 1], "stonecradle"),
    # add more as needed, keyed like regions.js
}

# from config.js
RATIO = 1.1021839
HEX_H = 1900
HEX_W = 2197
MAP = 16384
SCALE = MAP / (HEX_H * 7)
SIZE = 256            # tile zoom; smaller = more detail, more downloads (256 ~= 2x sharper than 512)
DS = SIZE // 128      # tiles are stored downscaled to 128px


def fetch(layer, x, yf):
    url = f"{TILES}/{layer}/{x}_{yf}_{SIZE}.png"
    for _ in range(3):
        try:
            with urllib.request.urlopen(url, timeout=20) as r:
                return Image.open(io.BytesIO(r.read()))
        except Exception as e:
            err = e
    print("  fail", url, err)
    return None


def crop_layer(layer, bb):
    ix0, ix1 = int(bb[0] // SIZE), int(bb[2] // SIZE)
    ir0, ir1 = int(bb[1] // SIZE), int(bb[3] // SIZE)
    ny = MAP // SIZE
    mode = "L" if layer == "heightmaps" else "RGB"
    canvas = Image.new(mode, ((ix1 - ix0 + 1) * 128, (ir1 - ir0 + 1) * 128))
    for i in range(ir0, ir1 + 1):
        for x in range(ix0, ix1 + 1):
            im = fetch(layer, x, ny - 1 - i)
            if im is None:
                continue
            canvas.paste(im.convert(mode).resize((128, 128)),
                         ((x - ix0) * 128, (i - ir0) * 128))
    ox, oy = ix0 * SIZE, ir0 * SIZE
    return canvas.crop((round((bb[0] - ox) / DS), round((bb[1] - oy) / DS),
                        round((bb[2] - ox) / DS), round((bb[3] - oy) / DS)))


def main():
    key = (sys.argv[1] if len(sys.argv) > 1 else "ENDLESS").upper()
    off, slug = REGIONS[key]
    cx = MAP / (RATIO * 2) + off[0] * HEX_W * SCALE
    cy = MAP / 2 - off[1] * HEX_H * SCALE
    hw, hh = HEX_W * SCALE / 2, HEX_H * SCALE / 2
    bb = (cx - hw, cy - hh, cx + hw, cy + hh)
    print(f"{key} center=({cx:.0f},{cy:.0f}) bbox={tuple(round(v) for v in bb)}")

    web = os.path.join(os.path.dirname(__file__), "..", "web")

    h = crop_layer("heightmaps", bb).convert("L")
    h.save(os.path.join(web, f"{slug}-height.png"))

    t = crop_layer("texturemaps", bb).convert("RGB")
    px = t.load()
    for y in range(t.height):
        for x in range(t.width):
            r, g, b = px[x, y]
            if b > 110 and g > 110 and r < 120 and b >= r and g >= r:
                px[x, y] = (38, 66, 104)  # cyan water/padding -> Foxhole blue
    t.save(os.path.join(web, f"{slug}.png"))
    print(f"wrote web/{slug}.png and web/{slug}-height.png {t.size}")


if __name__ == "__main__":
    main()
