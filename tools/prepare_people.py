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
Format .k0a: b"K0A2", u32 knochen, u32 clips,
             je Knochen: u8 namenslaenge, name, i32 eltern (-1 = Wurzel),
                         f32[12] Ruhepose global (3x4, zeilenweise, Engine-Koordinaten)
             je Clip: u8 namenslaenge, name, f32 fps, u32 bilder, u8 schleife,
                      bilder * knochen * f32[10] lokal: Quaternion xyzw, Position xyz, Skalierung xyz
             Eltern stehen immer vor ihren Kindern. Global = Eltern-Global x Lokal,
             Skinning = Global x Ruhepose^-1.

Animationen: die eigenen Clips der Quaternius-Figuren und Clips aus der Quaternius Universal
Animation Library 1+2 (CC0, assets_src/packs/quaternius_ual/), uebertragen auf das
Figuren-Skelett: Knochen-Richtungen werden pro Bild uebernommen (Ruhepose-Unterschiede
ausgeglichen), die Fuesse danach auf den Boden gesetzt.

Aufruf (headless):
    blender -b --factory-startup -P tools/prepare_people.py -- <kasse0-Ordner> [variante ...]
"""

import bmesh
import bpy
import json
import os
import struct
import sys

from mathutils import Matrix, Quaternion

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
# (Name im Spiel, Quelle, Animation, Schleife). Quelle "q" = eigene Clips der Figur,
# "u1"/"u2" = Universal Animation Library 1/2. Reihenfolge = people::Clip in src/people.rs.
CLIPS = [
    ("idle", "q", "Idle", True),
    ("walk", "q", "Walk", True),
    ("reach", "q", "Interact", True),
    ("run", "q", "Run", True),
    ("grab", "q", "Punch_Right", False),
    ("wave", "q", "Wave", True),
    ("hit", "q", "HitRecieve", False),
    ("death", "q", "Death", False),
    ("crouch_walk", "u1", "Crouch_Fwd_Loop", True),
    ("crouch_idle", "u1", "Crouch_Idle_Loop", True),
    ("zombie_walk", "u2", "Zombie_Walk_Fwd_Loop", True),
    ("zombie_idle", "u2", "Zombie_Idle_Loop", True),
    ("scratch", "u2", "Zombie_Scratch", True),
    ("get_up", "u2", "LayToIdle", False),
    ("fold_arms", "u2", "Idle_FoldArms_Loop", True),
    ("phone", "u2", "Idle_TalkingPhone_Loop", True),
    ("talk", "u1", "Idle_Talking_Loop", True),
    ("pick_up", "u1", "PickUp_Table", False),
    ("push", "u1", "Push_Loop", True),
    ("carry", "u2", "Walk_Carry_Loop", True),
    ("kneel", "u1", "Fixing_Kneeling", True),
    ("no", "u2", "Idle_No_Loop", True),
    ("walk_formal", "u1", "Walk_Formal_Loop", True),
]
UAL = {
    "u1": os.path.join(ROOT, "assets_src", "packs", "quaternius_ual", "UAL1_Standard.glb"),
    "u2": os.path.join(ROOT, "assets_src", "packs", "quaternius_ual", "UAL2_Standard.glb"),
}
# Figuren-Knochen <- UAL-Knochen (Rotation wird uebertragen). Body traegt das Becken
# (Huefte samt Beinen), dazu die Position.
UAL_MAP = {
    "Body": "pelvis", "Abdomen": "spine_01", "Torso": "spine_02", "Chest": "spine_03",
    "Neck": "neck_01", "Head": "Head",
    "UpperLeg.L": "thigh_l", "LowerLeg.L": "calf_l", "Foot.L": "foot_l",
    "UpperLeg.R": "thigh_r", "LowerLeg.R": "calf_r", "Foot.R": "foot_r",
}
for side, s in (("L", "l"), ("R", "r")):
    UAL_MAP.update({
        f"Shoulder.{side}": f"clavicle_{s}", f"UpperArm.{side}": f"upperarm_{s}",
        f"LowerArm.{side}": f"lowerarm_{s}", f"Wrist.{side}": f"hand_{s}",
    })
    for f in ("Index", "Middle", "Ring", "Pinky"):
        for k in (1, 2, 3):
            UAL_MAP[f"{f}{k}.{side}"] = f"{f.lower()}_0{k}_{s}"
    for k in (1, 2, 3):
        UAL_MAP[f"Thumb{k}.{side}"] = f"thumb_0{k}_{s}"

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


def ordered_bones(arm):
    """Knochen so sortiert, dass Eltern vor ihren Kindern stehen."""
    out, seen = [], set()

    def visit(b):
        if b.name in seen:
            return
        if b.parent:
            visit(b.parent)
        seen.add(b.name)
        out.append(b.name)

    for b in arm.data.bones:
        visit(b)
    return out


def write_mat(out, M):
    out += struct.pack("<12f", *[M[r][c] for r in range(3) for c in range(4)])


def set_action(arm, action):
    arm.animation_data_create()
    arm.animation_data.action = action
    if hasattr(arm.animation_data, "action_slot") and getattr(action, "slots", None):
        arm.animation_data.action_slot = action.slots[0]


def find_action(suffix, armature_name=None):
    for a in bpy.data.actions:
        base = a.name.split(".")[0]
        if base == suffix or base.endswith("|" + suffix):
            return a
    return None


def locals_from_globals(G, bones, parent):
    """Engine-Globale -> lokale (Quaternion, Position, Skalierung) je Knochen."""
    rows = []
    for b in bones:
        p = parent[b]
        L = G[b] if p is None else G[p].inverted() @ G[b]
        loc, rot, sca = L.decompose()
        rows.append((rot, loc, sca))
    return rows


def native_frames(arm, action, bones, C):
    set_action(arm, action)
    f0, f1 = int(action.frame_range[0]), int(action.frame_range[1])
    frames = []
    for fr in range(f0, f1):
        bpy.context.scene.frame_set(fr)
        frames.append({b: C @ arm.matrix_world @ arm.pose.bones[b].matrix for b in bones})
    return frames


def retarget_frames(arm, src, action, bones, C):
    """Ein UAL-Clip auf das Figuren-Skelett: Knochen-Richtungen uebernehmen (Vorwaertskinematik
    von Hand, ohne Blender-Auswertung), Fuesse auf den Boden."""
    set_action(src, action)
    tb = arm.data.bones
    sb = src.data.bones
    Wt = arm.matrix_world
    Ws = src.matrix_world
    rest_t = {b: tb[b].matrix_local for b in bones}
    # Ausgleich der Ruhepose: Arme und Beine uebernehmen die Richtung der UAL-Knochen (beide
    # Skelette stehen in T-Pose, kleine Unterschiede werden ausgeglichen). Alle anderen
    # Knochen (Fuesse, Wirbelsaeule, Schultern, Finger) uebernehmen nur die Bewegung relativ
    # zur Ruhepose, damit Fuesse flach und Proportionen der Figur erhalten bleiben.
    limbs = ("UpperArm.", "LowerArm.", "UpperLeg.", "LowerLeg.")
    off = {}
    for t, s_ in UAL_MAP.items():
        if t not in tb or s_ not in sb:
            continue
        if not t.startswith(limbs):
            off[t] = Quaternion()
            continue
        dt = (Wt.to_3x3() @ (tb[t].tail_local - tb[t].head_local)).normalized()
        ds = (Ws.to_3x3() @ (sb[s_].tail_local - sb[s_].head_local)).normalized()
        off[t] = ds.rotation_difference(dt)
    # Groessenverhaeltnis ueber die Hoehe der Huefte (Oberschenkel-Ansatz)
    h_t = (Wt @ tb["UpperLeg.L"].head_local).z
    h_s = (Ws @ sb["thigh_l"].head_local).z
    ratio = h_t / h_s
    pelvis_rest = Ws @ sb["pelvis"].head_local
    body_rest = Wt @ tb["Body"].head_local
    def foot_low(M, side):
        # Ferse (Kopf des Fussknochens) und Zehenspitze (sein Ende)
        tip = M @ Matrix.Translation((0, tb["Foot." + side].length, 0))
        return min(M.translation.z, tip.translation.z)

    foot_rest_z = min(foot_low(Wt @ rest_t["Foot.L"], "L"), foot_low(Wt @ rest_t["Foot.R"], "R"))
    parent = {b: (tb[b].parent.name if tb[b].parent else None) for b in bones}
    f0, f1 = int(action.frame_range[0]), int(action.frame_range[1])
    frames = []
    for fr in range(f0, f1):
        bpy.context.scene.frame_set(fr)
        world = {}  # Figuren-Knochen in Blender-Weltkoordinaten
        for b in bones:
            p = parent[b]
            # Ruhelage relativ zum Elternknochen
            if p is None:
                base = Wt @ rest_t[b]
            else:
                base = world[p] @ rest_t[p].inverted() @ rest_t[b]
            s_name = UAL_MAP.get(b)
            if s_name is None or s_name not in src.pose.bones:
                world[b] = base
                continue
            spb = src.pose.bones[s_name]
            Rs = (Ws @ spb.matrix).to_quaternion()
            Rs0 = (Ws @ sb[s_name].matrix_local).to_quaternion()
            Rt0 = (Wt @ rest_t[b]).to_quaternion()
            Rt = Rs @ Rs0.inverted() @ off[b].inverted() @ Rt0
            loc = base.translation
            if b == "Body":
                ps = (Ws @ spb.matrix).translation
                loc = body_rest + (ps - pelvis_rest) * ratio
            if b.startswith("Foot."):
                # Fuss sitzt am Ende des Unterschenkels (IK-Ziel der Quaternius-Figuren)
                leg = "LowerLeg." + b[-1]
                loc = (world[leg] @ rest_t[leg].inverted() @ rest_t[b]).translation
            world[b] = Matrix.LocRotScale(loc, Rt, base.to_scale())
        # Fuesse auf den Boden: tiefster Fuss auf Ruhehoehe
        low = min(foot_low(world["Foot.L"], "L"), foot_low(world["Foot.R"], "R"))
        lift = Matrix.Translation((0, 0, foot_rest_z - low))
        frames.append({b: C @ lift @ world[b] for b in bones})
    return frames


def export_anims(gender, arm, bones, C):
    """Hierarchie und lokale Knochen-Transformationen (Engine-Koordinaten) aller Clips."""
    fps = bpy.context.scene.render.fps / bpy.context.scene.render.fps_base
    order = ordered_bones(arm)
    parent = {b: (arm.data.bones[b].parent.name if arm.data.bones[b].parent else None) for b in order}
    out = bytearray(b"K0A2")
    out += struct.pack("<II", len(order), len(CLIPS))
    for b in order:
        name = b.encode()
        out += struct.pack("<B", len(name)) + name
        out += struct.pack("<i", order.index(parent[b]) if parent[b] else -1)
        write_mat(out, C @ arm.matrix_world @ arm.data.bones[b].matrix_local)
    sources = {}
    for clip, kind, src_name, loop in CLIPS:
        if kind == "q":
            action = find_action(src_name)
            if action is None:
                raise RuntimeError(f"Animation fehlt: {src_name}")
            frames = native_frames(arm, action, order, C)
        else:
            if kind not in sources:
                before = set(bpy.data.objects)
                bpy.ops.import_scene.gltf(filepath=UAL[kind])
                objs = [o for o in bpy.data.objects if o not in before]
                src = next(o for o in objs if o.type == "ARMATURE")
                for o in objs:
                    if o.type == "MESH":
                        o.hide_viewport = True
                sources[kind] = (src, {a.name for a in bpy.data.actions})
            src, _ = sources[kind]
            action = next((a for a in bpy.data.actions
                           if a.name.split(".")[0] == src_name
                           and a.name.split(".")[0] not in ("",)), None)
            if action is None:
                raise RuntimeError(f"UAL-Animation fehlt: {src_name}")
            frames = retarget_frames(arm, src, action, order, C)
        name = clip.encode()
        out += struct.pack("<B", len(name)) + name
        out += struct.pack("<fIB", fps, len(frames), 1 if loop else 0)
        prev = {}
        for G in frames:
            for b, (rot, loc, sca) in zip(order, locals_from_globals(G, order, parent)):
                # Quaternion-Vorzeichen stetig halten (sonst springt slerp)
                if b in prev and prev[b].dot(rot) < 0.0:
                    rot = -rot
                prev[b] = rot
                out += struct.pack("<10f", rot.x, rot.y, rot.z, rot.w,
                                   loc.x, loc.y, loc.z, sca.x, sca.y, sca.z)
        print(f"  Clip {clip}: {len(frames)} Bilder bei {fps:.0f} fps ({kind}:{src_name})")
        rest_frame(arm)
    path = os.path.join(OUT, f"anim_{gender}.k0a")
    with open(path, "wb") as f:
        f.write(out)
    print(f"  -> {os.path.basename(path)}: {len(order)} Knochen, {len(out) // 1024} KB")
    return order


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
    bones = export_anims(gender, arm, bones, C)
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
