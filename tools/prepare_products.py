"""Konvertiert alle Waren-Modelle aus tools/catalog.json nach assets/products/<id>.k0m.

Aufruf (headless):
    blender -b --factory-startup -P tools/prepare_products.py -- <kasse0-Ordner>

Grob an unseren Stil angepasst:
  * Farben: Kenney-Modelle nutzen eine gemeinsame Farbtextur -> Farbe pro Dreieck an der
    UV-Mitte abgetastet. Sonst Materialfarbe (bzw. Override aus dem Katalog).
  * flache Normalen, Polygone begrenzt, Boden bei y = 0, Mitte x/z = 0, Vorderseite +Z.
"""

import bpy
import json
import os
import struct
import sys

from mathutils import Matrix, Vector

ROOT = sys.argv[sys.argv.index("--") + 1]
PACKS = os.path.join(ROOT, "assets_src", "packs")
DL = os.path.join(ROOT, "assets_src", "downloads", "products")
OUT = os.path.join(ROOT, "assets", "products")
os.makedirs(OUT, exist_ok=True)
MAX_TRIS = 700

SRC = {
    "k": lambda n: os.path.join(PACKS, "kenney_food-kit", "Models", "GLB format", n + ".glb"),
    "m": lambda n: os.path.join(PACKS, "kenney_mini-shop_2.1", "Models", "GLB format", n + ".glb"),
    "q": lambda n: os.path.join(PACKS, "ultimate_food_pack_by_quaternius", "Ultimate Food Pack - Oct 2019", "FBX", n + ".fbx"),
    "d": lambda n: os.path.join(DL, {"pasta_box": "pasta_box", "rice": "rice"}.get(n, n) + ".glb"),
}


def import_file(path):
    before = set(bpy.data.objects)
    if path.lower().endswith(".fbx"):
        bpy.ops.import_scene.fbx(filepath=path)
    else:
        bpy.ops.import_scene.gltf(filepath=path)
    return [o for o in bpy.data.objects if o not in before]


_image_cache = {}


def image_pixels(img):
    if img.name not in _image_cache:
        px = [0.0] * len(img.pixels)
        img.pixels.foreach_get(px)
        _image_cache[img.name] = (px, img.size[0], img.size[1])
    return _image_cache[img.name]


def material_info(mat, overrides):
    """(feste Farbe oder None, Bild oder None)"""
    if mat is None:
        return (0.7, 0.7, 0.7), None
    if mat.name in overrides:
        return tuple(overrides[mat.name]), None
    col = (0.7, 0.7, 0.7)
    if mat.use_nodes:
        bsdf = next((n for n in mat.node_tree.nodes if n.type == "BSDF_PRINCIPLED"), None)
        img = next((n.image for n in mat.node_tree.nodes if n.type == "TEX_IMAGE" and n.image), None)
        if img is not None:
            return None, img
        if bsdf is not None:
            col = tuple(bsdf.inputs[0].default_value[:3])
    # lineare Materialfarbe -> Anzeigewert
    return tuple(max(0.0, min(1.0, c)) ** (1 / 2.2) for c in col), None


def sample(img, u, v):
    px, w, h = image_pixels(img)
    x = int((u % 1.0) * (w - 1) + 0.5)
    y = int((v % 1.0) * (h - 1) + 0.5)
    i = (y * w + x) * 4
    return (px[i], px[i + 1], px[i + 2])


def convert(mesh_spec):
    kind, name = mesh_spec["src"].split(":", 1)
    path = SRC[kind](name)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    _image_cache.clear()
    objs = [o for o in import_file(path) if o.type == "MESH" and not o.name.startswith("Icosphere")]
    if not objs:
        raise RuntimeError("keine Meshes: " + path)
    total = sum(len(o.data.polygons) * 2 for o in objs)
    ratio = min(1.0, MAX_TRIS / max(1, total))
    if ratio < 0.98:
        for o in objs:
            o.modifiers.new("dec", "DECIMATE").ratio = ratio
    dg = bpy.context.evaluated_depsgraph_get()
    dg.update()
    conv = Matrix(((1, 0, 0, 0), (0, 0, 1, 0), (0, -1, 0, 0), (0, 0, 0, 1)))
    overrides = mesh_spec.get("colors", {})
    verts = []
    for o in objs:
        ev = o.evaluated_get(dg)
        me = ev.to_mesh()
        me.calc_loop_triangles()
        uv = me.uv_layers.active.data if me.uv_layers.active else None
        mw = conv @ o.matrix_world
        nm = mw.to_3x3().inverted_safe().transposed()
        mats = [material_info(m, overrides) for m in me.materials] or [((0.7, 0.7, 0.7), None)]
        for tri in me.loop_triangles:
            fixed, img = mats[min(tri.material_index, len(mats) - 1)]
            if img is not None and uv is not None:
                us = [uv[li].uv for li in tri.loops]
                c = sample(img, sum(p.x for p in us) / 3, sum(p.y for p in us) / 3)
            else:
                c = fixed
            n = (nm @ tri.normal).normalized()
            for vi in tri.vertices:
                p = mw @ me.vertices[vi].co
                verts.append((p.x, p.y, p.z, n.x, n.y, n.z, c[0], c[1], c[2]))
        ev.to_mesh_clear()
    # normalisieren
    mn = Vector([min(v[i] for v in verts) for i in range(3)])
    mx = Vector([max(v[i] for v in verts) for i in range(3)])
    dim = mx - mn
    s = mesh_spec["h"] / dim.y if "h" in mesh_spec else mesh_spec["s"] / max(dim)
    cx, cz = (mn.x + mx.x) / 2, (mn.z + mx.z) / 2
    out = [((v[0] - cx) * s, (v[1] - mn.y) * s, (v[2] - cz) * s, v[3], v[4], v[5], v[6], v[7], v[8]) for v in verts]
    write(os.path.join(OUT, mesh_spec["id"] + ".k0m"), out)


def write(path, verts):
    mn = [min(v[i] for v in verts) for i in range(3)]
    mx = [max(v[i] for v in verts) for i in range(3)]
    with open(path, "wb") as f:
        f.write(b"K0M1")
        f.write(struct.pack("<I", len(verts)))
        f.write(struct.pack("<6f", *mn, *mx))
        for v in verts:
            f.write(struct.pack("<9f", *v))
    print(f"  -> {os.path.basename(path)}: {len(verts) // 3} Dreiecke, "
          f"{mx[0]-mn[0]:.2f} x {mx[1]-mn[1]:.2f} x {mx[2]-mn[2]:.2f} m")


with open(os.path.join(ROOT, "tools", "catalog.json"), encoding="utf-8") as f:
    catalog = json.load(f)
only = set(sys.argv[sys.argv.index("--") + 2:])
for spec in catalog["meshes"]:
    if only and spec["id"] not in only:
        continue
    print("==", spec["id"])
    try:
        convert(spec)
    except Exception as e:  # noqa: BLE001
        print("  FEHLER", spec["id"], e)
print("FERTIG")
