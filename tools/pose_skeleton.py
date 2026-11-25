# Legt das Skelett von OpenGameArt ("LowPoly Rigged skeleton (With IK)", zisongbr, CC0) hin:
# auf dem Ruecken, eine Hand greift nach oben, Beine leicht gespreizt. Die Pose wird ins Mesh
# gebacken und als GLB gespeichert, das prepare_assets.py dann wie alle Modelle umwandelt.
#   blender -b --factory-startup assets_src\downloads\ogs_skeleton\Skeleton_a\Skel_base.blend
#           -P tools/pose_skeleton.py -- assets_src\downloads\ogs_skeleton\skeleton_lying.glb
import bpy, math, sys
from mathutils import Matrix, Vector
out = sys.argv[sys.argv.index("--") + 1]
arm = bpy.data.objects["Skel_armature"]
mesh = bpy.data.objects["Skel_base"]
bpy.context.view_layer.objects.active = arm
bpy.ops.object.mode_set(mode="POSE")
# IK-Ziele im Armature-Raum verschieben: Arme und Beine abgespreizt
def move(name, d):
    pb = arm.pose.bones.get(name)
    if pb is None:
        print("fehlt", name); return
    pb.matrix = Matrix.Translation(Vector(d)) @ pb.matrix
    bpy.context.view_layer.update()
for n in ("Right_hand_ik", "Left_hand_ik", "Right_Foot_IK", "Left_Foot_IK"):
    pb = arm.pose.bones[n]
    print(n, tuple(round(v, 2) for v in pb.head))
# rechte Hand angewinkelt nach oben (greift ins Leere), linke liegt weit ab, Beine gespreizt
move("Right_hand_ik", (0.25, -0.35, 1.0))
move("Left_hand_ik", (-0.55, 0.0, -0.15))
move("Right_Foot_IK", (0.25, 0.0, 0.05))
move("Left_Foot_IK", (-0.15, 0.0, 0.0))

bpy.ops.object.mode_set(mode="OBJECT")
# Pose ins Mesh backen, Armature weg
bpy.context.view_layer.objects.active = mesh
for m in list(mesh.modifiers):
    if m.type == "ARMATURE":
        bpy.ops.object.modifier_apply(modifier=m.name)
mw = mesh.matrix_world.copy()
mesh.parent = None
mesh.matrix_world = mw
bpy.data.objects.remove(arm)
# auf den Ruecken legen: Kopf nach -X, Gesicht nach oben
mesh.matrix_world = Matrix.Rotation(math.radians(90), 4, "X") @ Matrix.Rotation(math.radians(90), 4, "Z") @ Matrix.Rotation(math.radians(-90), 4, "X") @ mesh.matrix_world
for o in list(bpy.data.objects):
    if o is not mesh:
        bpy.data.objects.remove(o)
bpy.ops.object.select_all(action="DESELECT")
mesh.select_set(True)
bpy.ops.export_scene.gltf(filepath=out, use_selection=True, export_format="GLB")
print("DIM", tuple(round(v, 2) for v in mesh.dimensions))
