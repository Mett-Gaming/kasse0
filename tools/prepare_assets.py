"""Konvertiert die Quell-Assets in das einfache KASSE-0-Meshformat (.k0m).

Aufruf (headless, beruehrt keine offene Blender-Sitzung):
    blender -b --factory-startup -P tools/prepare_assets.py -- <quaternius_dir> <downloads_dir> <out_dir>

"Grob an unseren Stil anpassen" heisst hier:
  * Polygone reduzieren (Decimate), flache Normalen (Low-Fi-Look)
  * Texturen weg, stattdessen eine Durchschnittsfarbe pro Material
  * Charaktere in eine feste Pose aus der Animation-Library einfrieren
  * Einheitliche Groesse, Boden bei y = 0, Mitte auf x/z = 0, Vorderseite +Z

Format .k0m (little endian):
    b"K0M1", u32 vertex_count, f32[3] bounds_min, f32[3] bounds_max,
    vertex_count * (f32[3] pos, f32[3] normal, f32[3] color)   -- nicht indiziert, Dreiecke
"""

import bpy
import math
import os
import struct
import sys

from mathutils import Matrix, Vector

args = sys.argv[sys.argv.index("--") + 1:]
QUAT, DL, OUT = args[0], args[1], args[2]
os.makedirs(OUT, exist_ok=True)
# eigene Modelle im Projektarchiv
MODELS = os.path.join(os.path.dirname(os.path.abspath(__file__)), "models")

UAL1 = os.path.join(QUAT, "universal-animation-library", "Universal Animation Library[Standard]",
                    "Unreal-Godot", "UAL1_Standard.glb")
MANN_F = os.path.join(QUAT, "universal-animation-library-2", "Universal Animation Library 2[Standard]",
                      "Female Mannequin", "Unreal-Godot", "Mannequin_F.glb")

# Jobs: name, Quelle(n), Optionen
JOBS = [
    # Kunden und Personal: siehe tools/prepare_people.py (animiert)
    # Kassierer:in: sitzend, Haende vorn am Scanner
    dict(name="cashier", src=UAL1, mesh="Mannequin", action="Driving_Loop", frame=10,
         height=1.3, tris=3500, colors={"M_Main": (0.85, 0.85, 0.85), "M_Joints": (0.3, 0.3, 0.3)}),
    # Requisiten
    dict(name="cart", src=os.path.join(DL, "shopping_cart.glb"), size=1.0, tris=2500,
         colors={"_crayfishdiffuse": (0.62, 0.64, 0.66), "02___Default": (0.75, 0.1, 0.08),
                 "03___Default": (0.08, 0.08, 0.08)}),
    # Kasse: Kassen-Station mit Bildschirm, Schublade und Bondrucker (J-Toastie, CC-BY 3.0,
    # poly.pizza/m/ju1CrAV1UY, Datei static.poly.pizza/8e168263-...glb als pos_machine.glb)
    dict(name="register", src=os.path.join(DL, "pos_machine.glb"), size=0.5, tris=1500),
    # Kasse: Stuhl, Zigaretten-Rollo, Kamera, Kartenterminal, Zigaretten
    # Buerostuhl der Kassiererin (Kenney, CC0, poly.pizza/m/CKSz6PB1vO, als desk_chair.glb)
    dict(name="chair", src=os.path.join(DL, "desk_chair.glb"), height=1.05, tris=1500),
    dict(name="shutter", src=os.path.join(DL, "rollershutter_window_01", "rollershutter_window_01_1k.gltf"), size=1.3, tris=800),
    dict(name="camera", src=os.path.join(DL, "security_camera_01", "security_camera_01_1k.gltf"), size=0.4, tris=800),
    # eigenes Modell (tools/models/kartenterminal.blend), schon in Metern, Ursprung = Stange
    dict(name="terminal", src=os.path.join(MODELS, "kartenterminal.glb"), keep=True),
    dict(name="cigarettes", src=os.path.join(DL, "cigarette_pack", "cigarette_pack_1k.gltf"), height=0.09, tris=200),
    # einzelne Zigarette zum Rauchen (Zgeal, CC-BY 3.0, poly.pizza/m/2RL6h8MbDPb), 8,5 cm lang,
    # Datei von static.poly.pizza/18dd85e2-b7aa-4e42-9ae6-a5cb536102f0.glb als cigarette.glb
    dict(name="cigarette", src=os.path.join(DL, "cigarette.glb"), size=0.085, tris=400),
    # Ratte (Poly by Google, CC-BY 3.0)
    dict(name="rat", src=os.path.join(DL, "rat.glb"), size=0.32, tris=600),
    dict(name="wet_floor_sign", src=os.path.join(DL, "WetFloorSign_01", "WetFloorSign_01_1k.gltf"),
         height=0.62, tris=400),
    dict(name="can", src=os.path.join(DL, "russian_food_cans_01", "russian_food_cans_01_1k.gltf"),
         mesh="russian_food_cans_01_can_cond", height=0.11, tris=200,
         colors={"russian_food_cans_01": (0.62, 0.07, 0.05)}),
]

# Waren: siehe tools/prepare_products.py und tools/catalog.json


def reset():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def import_file(path):
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=path)
    return [o for o in bpy.data.objects if o not in before]


def average_image_color(img):
    try:
        n = len(img.pixels)
        if n == 0:
            return None
        px = [0.0] * n
        img.pixels.foreach_get(px)
        step = max(4, (n // 4 // 4096) * 4)  # hoechstens ~4096 Stichproben
        r = g = b = 0.0
        c = 0
        for i in range(0, n - 3, step):
            r += px[i]
            g += px[i + 1]
            b += px[i + 2]
            c += 1
        return (r / c, g / c, b / c) if c else None
    except Exception as e:  # noqa: BLE001
        print("  Bildfehler", img.name, e)
        return None


def material_color(mat, overrides):
    if mat is None:
        return (0.7, 0.7, 0.7)
    if mat.name in overrides:
        return overrides[mat.name]
    col = (0.7, 0.7, 0.7)
    if mat.use_nodes:
        bsdf = next((n for n in mat.node_tree.nodes if n.type == "BSDF_PRINCIPLED"), None)
        if bsdf is not None:
            inp = bsdf.inputs[0]  # Base Color
            col = tuple(inp.default_value[:3])
            if inp.is_linked:
                node = inp.links[0].from_node
                # durch Mix/Multiply-Knoten zum Bild laufen
                seen = 0
                while node is not None and node.type != "TEX_IMAGE" and seen < 6:
                    nxt = None
                    for i in node.inputs:
                        if i.is_linked:
                            nxt = i.links[0].from_node
                            break
                    node = nxt
                    seen += 1
                if node is not None and node.type == "TEX_IMAGE" and node.image is not None:
                    avg = average_image_color(node.image)
                    if avg:
                        col = avg
    # linear -> sRGB-artig (unser Renderer arbeitet in Anzeigewerten)
    return tuple(max(0.0, min(1.0, c)) ** (1 / 2.2) for c in col)


def apply_pose(objs, job):
    action_name = job.get("action")
    if not action_name:
        return
    if "pose_src" in job:
        import_file(job["pose_src"])  # bringt die Actions mit
        # die Hilfs-Objekte der Library nicht exportieren
    action = bpy.data.actions.get(action_name)
    arm = next((o for o in objs if o.type == "ARMATURE"), None)
    if action is None or arm is None:
        print("  WARNUNG: Pose nicht moeglich", action_name, arm)
        return
    if arm.animation_data is None:
        arm.animation_data_create()
    arm.animation_data.action = action
    # Blender 4.4+/5.x: Action-Slots
    if hasattr(arm.animation_data, "action_slot") and getattr(action, "slots", None):
        try:
            arm.animation_data.action_slot = action.slots[0]
        except Exception as e:  # noqa: BLE001
            print("  Slot-Hinweis:", e)
    bpy.context.scene.frame_set(job.get("frame", 1))


def collect(objs, job):
    depsgraph = bpy.context.evaluated_depsgraph_get()
    wanted = job.get("mesh")
    meshes = [o for o in objs if o.type == "MESH" and (wanted is None or o.name == wanted)
              and not o.name.startswith("Icosphere")]
    if not meshes:
        raise RuntimeError(f"keine Meshes fuer {job['name']}")
    total = sum(len(o.data.polygons) * 2 for o in meshes)
    ratio = min(1.0, job.get("tris", 100000) / max(1, total))
    for o in meshes:
        if ratio < 0.98:
            m = o.modifiers.new("dec", "DECIMATE")
            m.ratio = ratio
    depsgraph = bpy.context.evaluated_depsgraph_get()
    depsgraph.update()

    # Blender (Z oben, -Y vorne) -> Engine (Y oben, +Z vorne)
    conv = Matrix(((1, 0, 0, 0), (0, 0, 1, 0), (0, -1, 0, 0), (0, 0, 0, 1)))
    verts = []
    for o in meshes:
        ev = o.evaluated_get(depsgraph)
        me = ev.to_mesh()
        me.calc_loop_triangles()
        mw = conv @ o.matrix_world
        nm = mw.to_3x3().inverted_safe().transposed()
        cols = [material_color(m, job.get("colors", {})) for m in me.materials] or [(0.7, 0.7, 0.7)]
        for tri in me.loop_triangles:
            c = cols[min(tri.material_index, len(cols) - 1)]
            n = (nm @ tri.normal).normalized()
            for vi in tri.vertices:
                p = mw @ me.vertices[vi].co
                verts.append((p.x, p.y, p.z, n.x, n.y, n.z, c[0], c[1], c[2]))
        ev.to_mesh_clear()
    return verts


def normalize(verts, job):
    if job.get("keep"):  # Groesse und Ursprung des Modells beibehalten
        return verts
    xs = [v[0] for v in verts]
    ys = [v[1] for v in verts]
    zs = [v[2] for v in verts]
    mn = Vector((min(xs), min(ys), min(zs)))
    mx = Vector((max(xs), max(ys), max(zs)))
    dim = mx - mn
    if "height" in job:
        s = job["height"] / dim.y
    else:
        s = job["size"] / max(dim)
    cx, cz = (mn.x + mx.x) / 2, (mn.z + mx.z) / 2
    yaw = math.radians(job.get("yaw", 0.0))
    cy, sy = math.cos(yaw), math.sin(yaw)
    out = []
    for v in verts:
        x, y, z = (v[0] - cx) * s, (v[1] - mn.y) * s, (v[2] - cz) * s
        nx, ny, nz = v[3], v[4], v[5]
        x, z = cy * x + sy * z, -sy * x + cy * z
        nx, nz = cy * nx + sy * nz, -sy * nx + cy * nz
        out.append((x, y, z, nx, ny, nz, v[6], v[7], v[8]))
    return out


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
          f"Groesse {mx[0]-mn[0]:.2f} x {mx[1]-mn[1]:.2f} x {mx[2]-mn[2]:.2f} m")


ONLY = os.environ.get("KASSE0_ONLY")
for job in JOBS:
    if ONLY and job["name"] not in ONLY.split(","):
        continue
    print("==", job["name"])
    reset()
    objs = import_file(job["src"])
    apply_pose(objs, job)
    verts = normalize(collect(objs, job), job)
    write(os.path.join(OUT, job["name"] + ".k0m"), verts)

print("FERTIG")
