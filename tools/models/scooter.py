# Mobility-Scooter "Leon Mobility INC" fuer KASSE 0 (eigenes Modell, Ausnahme von der
# Regel "keine selbst gebauten Modelle" auf Wunsch der Person, siehe CREDITS.md).
# Baut den Scooter aus einfachen Koerpern und Schrift (Archivo Black, OFL) und speichert
# tools/models/scooter.blend und tools/models/scooter.glb. In Metern, Ursprung = Lage des
# Einkaufswagens (Game::trolley.pos): Der Korb vorn liegt genau dort, wo der Wagenkorb liegt
# (assets.rs CART_BASKET_*), damit die Waren gleich darin liegen. Vorn ist +y (im Spiel -z),
# der Fahrer sitzt bei y = -0,85 (CART_OFFSET).
#   blender -b --factory-startup -P tools/models/scooter.py
import bpy, math, os
from mathutils import Vector

HERE = os.path.dirname(os.path.abspath(__file__))
FONT = os.path.join(HERE, "..", "..", "assets", "fonts", "ArchivoBlack-Regular.ttf")

# Korb wie der Wagenkorb (assets.rs): halbe Breite, laengs (Spiel-z -0,49..0,232), Boden, Oberkante
BX, BY0, BY1, BFLOOR, BTOP = 0.262, -0.232, 0.49, 0.57, 0.80
ROD = 0.008

bpy.ops.wm.read_factory_settings(use_empty=True)
mats = {}


def mat(name, rgb):
    if name not in mats:
        m = bpy.data.materials.new(name)
        m.use_nodes = True
        m.node_tree.nodes["Principled BSDF"].inputs[0].default_value = (*rgb, 1.0)
        mats[name] = m
    return mats[name]


LACK = ("Lack", (0.42, 0.04, 0.05))
CREME = ("Creme", (0.85, 0.8, 0.66))
CHROM = ("Chrom", (0.55, 0.55, 0.58))
GUMMI = ("Gummi", (0.04, 0.04, 0.045))
POLSTER = ("Polster", (0.12, 0.1, 0.1))
SCHRIFT = ("Schrift", (0.95, 0.86, 0.55))
parts = []


def box(c, h, m, rot=None):
    bpy.ops.mesh.primitive_cube_add(size=2.0, location=c)
    o = bpy.context.object
    o.scale = h
    if rot:
        o.rotation_euler = rot
    o.data.materials.append(mat(*m))
    parts.append(o)
    return o


def cyl(c, r, depth, m, rot, verts=16):
    bpy.ops.mesh.primitive_cylinder_add(vertices=verts, radius=r, depth=depth, location=c, rotation=rot)
    o = bpy.context.object
    o.data.materials.append(mat(*m))
    parts.append(o)
    return o


def rod(a, b, r, m):
    a, b = Vector(a), Vector(b)
    d = b - a
    o = cyl((a + b) / 2, r, d.length, m, (0, 0, 0), 8)
    o.rotation_mode = "QUATERNION"
    o.rotation_quaternion = Vector((0, 0, 1)).rotation_difference(d.normalized())
    return o


def wheel(x, y, r, w):
    cyl((x, y, r), r, w, GUMMI, (0, math.pi / 2, 0), 20)
    cyl((x + math.copysign(w / 2 + 0.003, x), y, r), r * 0.5, 0.01, CHROM, (0, math.pi / 2, 0), 12)


def text(s, loc, rot, size, depth=0.004):
    bpy.ops.object.text_add(location=loc, rotation=rot)
    o = bpy.context.object
    o.data.body = s
    o.data.font = bpy.data.fonts.load(FONT)
    o.data.size = size
    o.data.extrude = depth
    o.data.align_x = "CENTER"
    o.data.align_y = "CENTER"
    o.data.resolution_u = 2
    bpy.ops.object.convert(target="MESH")
    o = bpy.context.object
    o.data.materials.clear()
    o.data.materials.append(mat(*SCHRIFT))
    parts.append(o)
    return o


# --- Korb vorn (Drahtkorb wie beim Wagen) ---
box((0, (BY0 + BY1) / 2, BFLOOR - ROD), (BX, (BY1 - BY0) / 2, ROD), CHROM)
for z in (BFLOOR + 0.1, BTOP):
    for x in (-BX, BX):
        rod((x, BY0, z), (x, BY1, z), ROD, CHROM)
    for y in (BY0, BY1):
        rod((-BX, y, z), (BX, y, z), ROD, CHROM)
for t in range(5):
    y = BY0 + (BY1 - BY0) * t / 4
    for x in (-BX, BX):
        rod((x, y, BFLOOR), (x, y, BTOP), ROD, CHROM)
for t in range(5):
    x = -BX + 2 * BX * t / 4
    for y in (BY0, BY1):
        rod((x, y, BFLOOR), (x, y, BTOP), ROD, CHROM)
# Korbhalter
for x in (-0.12, 0.12):
    rod((x, 0.05, 0.45), (x, 0.1, BFLOOR - 2 * ROD), 0.012, CHROM)

# --- Vorderteil: Verkleidung bis zum Boden, zwei kleine Vorderraeder ---
box((0, 0.22, 0.3), (0.2, 0.2, 0.18), LACK)
box((0, 0.42, 0.22), (0.2, 0.03, 0.1), LACK, (math.radians(-25), 0, 0))
box((0, 0.44, 0.33), (0.07, 0.012, 0.03), CREME)  # Scheinwerfer
for x in (-0.17, 0.17):
    wheel(x, 0.3, 0.1, 0.06)

# --- Lenksaeule, Lenker, Armaturenbrett mit Schrift (zum Fahrer hin): eigenes Teil, das sich
# im Spiel um die Saeule dreht (scooter_bar.glb, scooter.rs STEER_FOOT/STEER_TOP) ---
n_bar = len(parts)
rod((0, -0.4, 0.2), (0, -0.52, 1.0), 0.03, CHROM)
rod((-0.27, -0.54, 1.0), (0.27, -0.54, 1.0), 0.018, CHROM)
for x in (-0.24, 0.24):
    rod((x, -0.54, 1.0), (x + math.copysign(0.05, x), -0.54, 1.0), 0.026, GUMMI)
dash = box((0, -0.51, 0.93), (0.16, 0.035, 0.07), LACK, (math.radians(-30), 0, 0))
text("LEON", (0, -0.552, 0.955), (math.radians(60), 0, 0), 0.045)
text("MOBILITY INC", (0, -0.532, 0.915), (math.radians(60), 0, 0), 0.026)

bar = parts[n_bar:]
del parts[n_bar:]

# --- Trittbrett ---
box((0, -0.55, 0.15), (0.29, 0.62, 0.05), LACK)
box((0, -0.55, 0.205), (0.25, 0.55, 0.006), GUMMI)

# --- Sitz: Saeule, Polster, Lehne, Armlehnen ---
rod((0, -0.88, 0.2), (0, -0.88, 0.52), 0.035, CHROM)
box((0, -0.88, 0.57), (0.25, 0.2, 0.06), POLSTER)
box((0, -1.06, 0.82), (0.24, 0.035, 0.2), POLSTER, (math.radians(-8), 0, 0))
for x in (-0.27, 0.27):
    box((x, -0.88, 0.72), (0.025, 0.17, 0.02), POLSTER)
    rod((x, -1.02, 0.62), (x, -1.02, 0.72), 0.012, CHROM)

# --- Heck: Verkleidung mit Schrift an beiden Seiten und hinten, Hinterraeder ---
box((0, -1.2, 0.3), (0.26, 0.17, 0.16), LACK)
for s in (-1, 1):
    text("LEON MOBILITY INC", (s * 0.262, -1.2, 0.33), (math.pi / 2, 0, s * math.pi / 2), 0.042)
text("LEON MOBILITY INC", (0, -1.372, 0.33), (math.pi / 2, 0, 0), 0.042)
box((0, -1.372, 0.22), (0.12, 0.004, 0.02), (("Ruecklicht", (0.7, 0.05, 0.05))))
for x in (-0.3, 0.3):
    wheel(x, -1.15, 0.13, 0.08)

# Rumpf und Lenker je zu einem Objekt, getrennt gespeichert
def join(objs, name):
    bpy.ops.object.select_all(action="DESELECT")
    for o in objs:
        o.select_set(True)
    bpy.context.view_layer.objects.active = objs[0]
    bpy.ops.object.join()
    o = bpy.context.object
    o.name = name
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
    return o


def export(o, path):
    bpy.ops.object.select_all(action="DESELECT")
    o.select_set(True)
    bpy.ops.export_scene.gltf(filepath=path, export_format="GLB", use_selection=True)
    print("DIM", o.name, tuple(round(v, 3) for v in o.dimensions),
          "TRIS", sum(len(p.vertices) - 2 for p in o.data.polygons))


body = join(parts, "Scooter")
handlebar = join(bar, "Lenker")
bpy.ops.wm.save_as_mainfile(filepath=os.path.join(HERE, "scooter.blend"))
export(body, os.path.join(HERE, "scooter.glb"))
export(handlebar, os.path.join(HERE, "scooter_bar.glb"))
