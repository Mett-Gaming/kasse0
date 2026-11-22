"""Exportiert Kunden und Personal als animierbare Figuren (Skinning).

Quelle: Quaternius Ultimate Modular Men/Women (CC0, via poly.pizza), abgelegt in
assets_src/packs/quaternius_modular_people/. Alle Figuren eines Geschlechts teilen sich ein
Skelett ("CharacterArmature") und dieselben Animationen. Pro Variante (tools/people.json)
werden Kopf, Oberteil, Hose und Schuhe aus verschiedenen Outfits gemischt.

Ausgabe:
  assets/people/<variante>.k0s    Figur in Ruhepose mit je 4 Knochen-Gewichten pro Vertex
  assets/people/anim_<m|f>.k0a    Animationen je Geschlecht als Knochen-Matrizen pro Bild

Format .k0s: b"K0S1", u32 n, f32[3] min, f32[3] max,
             n * (f32[3] pos, f32[3] normal, f32[3] color, f32[4] knochen, f32[4] gewichte)
Format .k0a: b"K0A1", u32 knochen, u32 clips,
             je Clip: u8 namenslaenge, name, f32 fps, u32 bilder,
                      bilder * knochen * f32[12] (3x4-Matrix, zeilenweise, Engine-Koordinaten)

Aufruf (headless):
    blender -b --factory-startup -P tools/prepare_people.py -- <kasse0-Ordner> [variante ...]
"""

import bmesh
import bpy
import json
import os
import struct
import sys

from mathutils import Matrix

ROOT = sys.argv[sys.argv.index("--") + 1]
ONLY = set(sys.argv[sys.argv.index("--") + 2:])
SRC = os.path.join(ROOT, "assets_src", "packs", "quaternius_modular_people")
OUT = os.path.join(ROOT, "assets", "people")
os.makedirs(OUT, exist_ok=True)
MAX_TRIS = 2800
# Koerpergroesse des unveraenderten Freizeit-Outfits je Geschlecht (Meter)
HEIGHT = {"m": 1.78, "f": 1.68}
REFERENCE = {"m": "casual", "f": "casual"}
# Clips im Spiel -> Animation in der Quelle
CLIPS = [("idle", "Idle"), ("walk", "Walk"), ("reach", "Interact"), ("run", "Run")]

with open(os.path.join(ROOT, "tools", "people.json"), encoding="utf-8") as f:
    CFG = json.load(f)

# Blender (Z oben, -Y vorne) -> Engine (Y oben, +Z vorne)
CONV = Matrix(((1, 0, 0, 0), (0, 0, 1, 0), (0, -1, 0, 0), (0, 0, 0, 1)))


def part_of(name):
    n = name.lower()
    if n.endswith("_head"):
        return "head"
    if n.endswith("_body") or n == "backpack":
        return "body"
    if n.endswith("_legs") or n.endswith("_pants"):
        return "legs"
    if n.endswith("_feet"):
        return "feet"
    return None


def color(mat):
    if mat is None:
        return (0.7, 0.7, 0.7)
    col = tuple(mat.diffuse_color[:3])
    if mat.use_nodes:
        bsdf = next((n for n in mat.node_tree.nodes if n.type == "BSDF_PRINCIPLED"), None)
        if bsdf is not None:
            col = tuple(bsdf.inputs[0].default_value[:3])
    return tuple(max(0.0, min(1.0, c)) ** (1 / 2.2) for c in col)


# Materialien, die zum Menschen gehoeren (nicht zur Kleidung)
BODY_MATS = ("skin", "hair", "eyebrow", "eye", "moustache", "earring", "beard")
# Gesicht: wird bei "faceless" in Hautfarbe getaucht (bei den Frauen-Koepfen heissen die Augen "Brown")
FACE_MATS = ("eye", "eyebrows", "brown")
ACCENT_MATS = ("white", "tie", "gold")


def skin_color():
    for m in bpy.data.materials:
        if m.name.lower().startswith("skin"):
            return color(m)
    return (0.8, 0.65, 0.55)


def recolor(variant, part, mat):
    """Farbe eines Materials, mit Uniform und gesichtslosen Koepfen (Mitarbeiter)."""
    c = color(mat)
    name = (mat.name if mat else "").lower().split(".")[0]
    if part == "head" and variant.get("faceless") and name in FACE_MATS:
        return skin_color()
    uni = variant.get("uniform")
    if uni and part in uni and not name.startswith(BODY_MATS):
        if part == "body" and name in ACCENT_MATS and "accent" in uni:
            return tuple(uni["accent"])
        return tuple(uni[part])
    return c


def load_outfit(gender, outfit):
    """Importiert ein Outfit. Liefert (armatur, {teil: [mesh-objekte]})."""
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=os.path.join(SRC, CFG["outfits"][gender][outfit]))
    objs = [o for o in bpy.data.objects if o not in before]
    arm = next(o for o in objs if o.type == "ARMATURE")
    parts = {}
    for o in objs:
        p = part_of(o.name) if o.type == "MESH" else None
        if p:
            parts.setdefault(p, []).append(o)
    return arm, parts


def rest_frame(arm):
    """Alle Animationen weg: Ruhepose."""
    if arm.animation_data:
        arm.animation_data.action = None
    for pb in arm.pose.bones:
        pb.matrix_basis = Matrix.Identity(4)


def engine_transform(gender, meshes):
    """Matrix Welt (Blender) -> Engine: Achsen tauschen, Groesse normieren, Fuesse auf y = 0."""
    dg = bpy.context.evaluated_depsgraph_get()
    ys = []
    for o in meshes:
        for v in o.data.vertices:
            ys.append((CONV @ o.matrix_world @ v.co).y)
    lo, hi = min(ys), max(ys)
    s = HEIGHT[gender] / (hi - lo)
    return Matrix.Scale(s, 4) @ Matrix.Translation((0, -lo, 0)) @ CONV


def mesh_vertices(variant, chosen, role, bone_index, ref_rest, C):
    """Ruhepose-Dreiecke mit Farben und je 4 Knochen-Gewichten (Engine-Koordinaten)."""
    for o in chosen:
        # glTF trennt Vertices an Naehten auf; ohne Verschmelzen reisst die Reduktion dort Loecher
        bm = bmesh.new()
        bm.from_mesh(o.data)
        bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-4)
        bm.to_mesh(o.data)
        bm.free()
        for m in o.modifiers:
            if m.type == "ARMATURE":
                m.show_viewport = False

    # Nur Kopf und Oberkoerper reduzieren: Beine und Schuhe sind schon sparsam und reissen sonst auf
    def tris(o):
        o.data.calc_loop_triangles()
        return len(o.data.loop_triangles)

    fixed = sum(tris(o) for o in chosen if role[o.name] in ("legs", "feet"))
    heavy = [o for o in chosen if role[o.name] in ("head", "body")]
    budget = max(600, MAX_TRIS - fixed)
    ratio = min(1.0, budget / max(1, sum(tris(o) for o in heavy)))
    if ratio < 0.98:
        for o in heavy:
            o.modifiers.new("dec", "DECIMATE").ratio = ratio
    dg = bpy.context.evaluated_depsgraph_get()
    dg.update()
    bones_by_index = {i: b for b, i in bone_index.items()}
    verts = []
    missing = set()
    for o in chosen:
        ev = o.evaluated_get(dg)
        me = ev.to_mesh()
        me.calc_loop_triangles()
        # Eigenes Skelett des Teils -> Referenz-Skelett (manche Outfits haben ein gedrehtes Skelett)
        own = o.parent if o.parent and o.parent.type == "ARMATURE" else None
        retarget = {}

        def to_ref(bone):
            if bone not in retarget:
                if own is None or bone not in own.data.bones or bone not in ref_rest:
                    retarget[bone] = Matrix.Identity(4)
                else:
                    mine = own.matrix_world @ own.data.bones[bone].matrix_local
                    retarget[bone] = ref_rest[bone] @ mine.inverted()
            return retarget[bone]

        M = C @ o.matrix_world
        nm = M.to_3x3().inverted_safe().transposed()
        cols = [recolor(variant, role[o.name], m) for m in me.materials] or [(0.7, 0.7, 0.7)]
        names = [g.name for g in o.vertex_groups]
        weights = []
        for v in me.vertices:
            ws = []
            for g in v.groups:
                if g.weight <= 0.0 or g.group >= len(names):
                    continue
                b = bone_index.get(names[g.group])
                if b is None:
                    missing.add(names[g.group])
                    continue
                ws.append((g.weight, b))
            ws.sort(reverse=True)
            ws = ws[:4]
            total = sum(w for w, _ in ws) or 1.0
            ws = [(w / total, b) for w, b in ws] + [(0.0, 0)] * (4 - len(ws))
            weights.append(ws)
        for tri in me.loop_triangles:
            c = cols[min(tri.material_index, len(cols) - 1)]
            for vi in tri.vertices:
                ws = weights[vi]
                R = C @ to_ref(bones_by_index[ws[0][1]]) @ o.matrix_world
                p = R @ me.vertices[vi].co
                n = (R.to_3x3().inverted_safe().transposed() @ tri.normal).normalized()
                verts.append((p.x, p.y, p.z, n.x, n.y, n.z, c[0], c[1], c[2],
                              *[float(b) for _, b in ws], *[w for w, _ in ws]))
        ev.to_mesh_clear()
    if missing:
        print("  WARNUNG: Knochen fehlen im Referenz-Skelett:", sorted(missing))
    return verts


def write_skin(path, verts):
    mn = [min(v[i] for v in verts) for i in range(3)]
    mx = [max(v[i] for v in verts) for i in range(3)]
    with open(path, "wb") as f:
        f.write(b"K0S1")
        f.write(struct.pack("<I", len(verts)))
        f.write(struct.pack("<6f", *mn, *mx))
        for v in verts:
            f.write(struct.pack("<17f", *v))
    print(f"  -> {os.path.basename(path)}: {len(verts) // 3} Dreiecke, "
          f"{mx[0]-mn[0]:.2f} x {mx[1]-mn[1]:.2f} x {mx[2]-mn[2]:.2f} m")


def export_anims(gender, arm, bones, C):
    """Knochen-Matrizen (Engine-Koordinaten) aller Clips eines Geschlechts."""
    Ci = C.inverted()
    fps = bpy.context.scene.render.fps / bpy.context.scene.render.fps_base
    out = bytearray(b"K0A1")
    out += struct.pack("<II", len(bones), len(CLIPS))
    for clip, src in CLIPS:
        action = next((a for a in bpy.data.actions if a.name.split(".")[0].endswith("|" + src)), None)
        if action is None:
            raise RuntimeError(f"Animation fehlt: {src}")
        arm.animation_data_create()
        arm.animation_data.action = action
        if hasattr(arm.animation_data, "action_slot") and getattr(action, "slots", None):
            arm.animation_data.action_slot = action.slots[0]
        f0, f1 = int(action.frame_range[0]), int(action.frame_range[1])
        frames = list(range(f0, f1))  # Schleife: letztes Bild = erstes
        name = clip.encode()
        out += struct.pack("<B", len(name)) + name
        out += struct.pack("<fI", fps, len(frames))
        for fr in frames:
            bpy.context.scene.frame_set(fr)
            for b in bones:
                pb = arm.pose.bones[b]
                D = arm.matrix_world @ pb.matrix @ pb.bone.matrix_local.inverted() @ arm.matrix_world.inverted()
                E = C @ D @ Ci
                out += struct.pack("<12f", *[E[r][c] for r in range(3) for c in range(4)])
        print(f"  Clip {clip}: {len(frames)} Bilder bei {fps:.0f} fps")
    path = os.path.join(OUT, f"anim_{gender}.k0a")
    with open(path, "wb") as f:
        f.write(out)
    print(f"  -> {os.path.basename(path)}: {len(bones)} Knochen, {len(out) // 1024} KB")


def reference(gender):
    """Referenz-Skelett und Engine-Transformation eines Geschlechts (frische Szene)."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    arm, parts = load_outfit(gender, REFERENCE[gender])
    rest_frame(arm)
    bpy.context.view_layer.update()
    meshes = [o for ps in parts.values() for o in ps]
    C = engine_transform(gender, meshes)
    bones = [b.name for b in arm.data.bones]
    ref_rest = {b.name: arm.matrix_world @ b.matrix_local for b in arm.data.bones}
    return arm, bones, C, ref_rest


for gender in ("m", "f"):
    print("== Skelett", gender)
    arm, bones, C, ref_rest = reference(gender)
    export_anims(gender, arm, bones, C)
    bone_index = {b: i for i, b in enumerate(bones)}
    for variant in CFG["variants"]:
        if variant["g"] != gender or (ONLY and variant["id"] not in ONLY):
            continue
        print("==", variant["id"])
        bpy.ops.wm.read_factory_settings(use_empty=True)
        loaded = {}
        chosen, role = [], {}
        for part in ("head", "body", "legs", "feet"):
            outfit = variant[part]
            if outfit not in loaded:
                a, parts = load_outfit(gender, outfit)
                rest_frame(a)
                loaded[outfit] = parts
            for o in loaded[outfit].get(part, []):
                chosen.append(o)
                role[o.name] = part
        bpy.context.view_layer.update()
        write_skin(os.path.join(OUT, f"{variant['id']}.k0s"),
                   mesh_vertices(variant, chosen, role, bone_index, ref_rest, C))
print("FERTIG")
