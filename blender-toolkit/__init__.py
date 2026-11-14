# =============================================================================
# PART 1: THE HEADER & THE "ID CARD" OF THE ADDON
# =============================================================================

"""
Blender Toolkit V5 — Blender Addon
Author: Blender Toolkit contributors
Blender: 4.2+

Dynamic Layout System — new in V5:
  - layout_config.json in the addon folder stores every section: name, icon,
    style (BOX / ROLLUP), and the list of operator IDs to display.
  - Developer Mode toggle in addon preferences reveals an in-panel Layout
    Editor: move sections up/down, rename, change style, delete, add new.
  - "Add Script" file browser lets you link any .py file into a section
    without touching code.
  - All 13 original sections are written into layout_config.json on first run
    so nothing is lost.
"""

# --- Why a docstring here? ---
# Triple-quoted strings (""" ... """) are ignored by Python at runtime.
# They are the developer's "noticeboard" — a human-readable changelog
# that explains what changed and why, so future readers (including you!)
# understand the history without trawling through git commits.


# -----------------------------------------------------------------------------
# THE ADDON'S PASSPORT (bl_info)
# -----------------------------------------------------------------------------
# Blender reads this dictionary to show the addon in its Preferences window.
# Think of it as the addon's ID card.
bl_info = {
    "name": "Blender Toolkit",
    "author": "Blender Toolkit contributors",
    # Version bumped to 0.6.0 — matches blender_manifest.toml.
    # If Blender's addon list still shows 0.5.0 after installing, the new
    # code did NOT load and the old cached version is still active.
    "version": (0, 7, 0),
    "blender": (4, 2, 0),
    "location": "View3D > Sidebar > Blender Toolkit",
    "description": "BT production toolkit — transform, align, materials, UV, generators and more",
    "category": "Object",
}

import bpy
import bmesh
import random
import colorsys
import math
import re
import os
import json
import importlib
import sys
from mathutils import Vector, Matrix, Euler
from bpy.props import (FloatProperty, IntProperty, BoolProperty,
                       StringProperty, EnumProperty, FloatVectorProperty)
from bpy.types import PropertyGroup, Panel, Operator


# ─────────────────────────────────────────────────────────────────────────────
# SECTION: EXTENSION REGISTRY (THE "PLUG SOCKET" SYSTEM)
# ─────────────────────────────────────────────────────────────────────────────
class BlenderToolkit:
    """Third-party addons call BlenderToolkit.register_section(label, draw_fn, icon)
    to append collapsible panels at the bottom of the N-panel tab."""
    _sections: list = []

    @classmethod
    def register_section(cls, label: str, draw_fn, icon: str = 'PLUGIN'):
        """Adds a new menu section to our list."""
        if not any(s[0] == label for s in cls._sections):
            cls._sections.append((label, draw_fn, icon))

    @classmethod
    def unregister_section(cls, label: str):
        """Removes a menu section by its label name."""
        cls._sections = [s for s in cls._sections if s[0] != label]

    @classmethod
    def get_sections(cls):
        """Returns the complete list of plugged-in sections so Blender can draw them."""
        return cls._sections


# ─────────────────────────────────────────────────────────────────────────────
# SECTION: LAYOUT CONFIG  (THE JSON FILE MANAGER)
# ─────────────────────────────────────────────────────────────────────────────
# The config file lives right next to __init__.py inside the installed extension.
# On first run _ensure_config() creates it with the original 13 sections.
# After that, every change made in Developer Mode is written back to this file.

_CONFIG_PATH = os.path.join(os.path.dirname(__file__), "layout_config.json")

# The original 13 sections, in order, with their operator lists.
# Each operator is the bl_idname string that Blender uses to call it.
# Style can be "BOX" (always open), "ROLLUP" (collapsible box), or "FLAT" (no box).
_DEFAULT_SECTIONS = [
    {
        "label": "TRANSFORM",
        "icon": "ORIENTATION_GLOBAL",
        "style": "ROLLUP",
        "operators": [
            "bt.random_transform",
            "bt.re_randomise",
            "bt.quick_rotate",
            "bt.drop_to_floor",
            "bt.drop_to_surface",
            "bt.apply_all_transforms",
            "bt.set_pivot_bottom",
            "bt.quick_measure",
            "bt.scale_by_distance",
        ],
    },
    {
        "label": "ALIGN",
        "icon": "ALIGN_CENTER",
        "style": "ROLLUP",
        "operators": [
            "bt.align_multiple",
            "bt.align_to_object",
            "bt.copy_transform",
            "bt.arrange_in_grid",
            "bt.place_along_spline",
            "bt.three_point_align",
        ],
    },
    {
        "label": "SELECTION",
        "icon": "RESTRICT_SELECT_ON",
        "style": "ROLLUP",
        "operators": [
            "bt.select_by_material",
            "bt.select_instances",
            "bt.select_no_material",
            "bt.select_missing_uv",
            "bt.select_non_smoothed",
            "bt.select_unparented",
            "bt.select_by_polycount",
            "bt.random_select",
        ],
    },
    {
        "label": "OBJECT TOOLS",
        "icon": "OBJECT_DATA",
        "style": "ROLLUP",
        "operators": [
            "bt.make_unique",
            "bt.replace_object",
            "bt.make_instances",
            "bt.explode_to_objects",
            "bt.attach_selection",
            "bt.collapse_by_material",
            "bt.mark_as_asset",
            "bt.object_lock",
            "bt.place_at_vertices",
            "bt.subdivision_control",
            "bt.modifier_utility",
            "bt.find_unique_objects",
            "bt.attach_splines",
            "bt.create_collection_from_selection",
            "bt.create_line_at_base",
            "bt.place_name_below",
        ],
    },
    {
        "label": "MESH TOOLS",
        "icon": "MESH_DATA",
        "style": "ROLLUP",
        "operators": [
            "bt.weld_vertices",
            "bt.random_face_remove",
            "bt.random_face_select",
            "bt.random_extrude",
        ],
    },
    {
        "label": "NAMING",
        "icon": "SORTALPHA",
        "style": "ROLLUP",
        "operators": [
            "bt.rename_prefix",
            "bt.rename_suffix",
            "bt.rename_find_replace",
            "bt.rename_number",
        ],
    },
    {
        "label": "MATERIALS",
        "icon": "MATERIAL",
        "style": "ROLLUP",
        "operators": [
            "bt.random_material",
            "bt.random_map_phase",
            "bt.isolate_by_material",
            "bt.remove_materials",
            "bt.merge_duplicate_materials",
            "bt.material_to_wire_color",
            "bt.scale_procedural_maps",
            "bt.apply_default_material",
            "bt.swap_materials",
            "bt.set_viewport_opacity",
            "bt.replace_material",
            "bt.uvw_randomiser",
        ],
    },
    {
        "label": "UV & CLEANUP",
        "icon": "UV_DATA",
        "style": "ROLLUP",
        "operators": [
            "bt.uvw_box_map",
            "bt.uv_transfer",
            "bt.apply_uv_grid",
            "bt.face_uv_transform",
            "bt.select_missing_uv",
            "bt.reload_all_images",
        ],
    },
    {
        "label": "WIRE COLOUR",
        "icon": "SHADING_WIRE",
        "style": "ROLLUP",
        "operators": [
            "bt.wire_color_random",
            "bt.material_to_wire_color",
        ],
    },
    {
        "label": "GENERATORS",
        "icon": "PARTICLES",
        "style": "ROLLUP",
        "operators": [
            "bt.circle_array",
            "bt.floor_generator",
            "bt.simple_scatter",
            "bt.roof_generator",
        ],
    },
    {
        "label": "LIGHTING",
        "icon": "LIGHT",
        "style": "ROLLUP",
        "operators": [
            "bt.three_point_light",
            "bt.studio_light",
            "bt.sun_light",
            "bt.delete_all_lights",
            "bt.randomise_light_hue",
            "bt.randomise_lights_on_off",
            "bt.randomise_light_energy",
            "bt.toggle_light_visibility",
        ],
    },
    {
        "label": "CAMERA TOOLS",
        "icon": "CAMERA_DATA",
        "style": "ROLLUP",
        "operators": [
            "bt.add_camera_rig",
            "bt.focus_on_selection",
            "bt.match_camera_res",
        ],
    },
    {
        "label": "SCENE CLEANUP",
        "icon": "SCENE_DATA",
        "style": "ROLLUP",
        "operators": [
            "bt.purge_unused",
            "bt.remove_empty_collections",
            "bt.remove_missing_uv",
            "bt.poly_count",
        ],
    },
]


def _ensure_config():
    """
    Make sure layout_config.json exists next to __init__.py.
    If it doesn't exist yet we write the default 13-section layout.
    This runs once at register() time and never overwrites existing user changes.
    """
    if not os.path.isfile(_CONFIG_PATH):
        _write_config(_DEFAULT_SECTIONS)


def _read_config():
    """
    Load and return the section list from layout_config.json.
    Falls back to the built-in default if the file is missing or broken.
    """
    try:
        with open(_CONFIG_PATH, "r", encoding="utf-8") as fh:
            data = json.load(fh)
        # Basic validation: must be a list of dicts with at least "label".
        if isinstance(data, list) and all(isinstance(s, dict) and "label" in s for s in data):
            return data
    except Exception:
        pass
    # If the file is bad, silently return defaults so the panel never crashes.
    return list(_DEFAULT_SECTIONS)


def _write_config(sections):
    """
    Save the section list to layout_config.json.
    Called by every operator that modifies the layout (move, rename, delete, etc.).
    json.dump writes human-readable JSON with 2-space indentation.
    """
    try:
        with open(_CONFIG_PATH, "w", encoding="utf-8") as fh:
            json.dump(sections, fh, indent=2, ensure_ascii=False)
    except Exception as e:
        # Never crash Blender just because we couldn't save.
        print(f"[Blender Toolkit] Could not write layout_config.json: {e}")


def _redraw(ctx):
    """
    Force the 3D Viewport sidebar to repaint.
    Blender only redraws panels when it thinks something changed; because we
    store layout state in a JSON file rather than in Blender properties, it
    has no idea anything moved, so we tell it explicitly.
    """
    try:
        for area in ctx.screen.areas:
            if area.type == 'VIEW_3D':
                area.tag_redraw()
    except Exception:
        pass


# ─────────────────────────────────────────────────────────────────────────────
# SECTION: DEVELOPER MODE TOGGLE
# ─────────────────────────────────────────────────────────────────────────────
# Rather than burying the toggle in Blender Preferences (which requires
# navigating menus), we store developer_mode directly on the scene so it
# appears as a single button at the top of the Blender Toolkit panel itself.
# This is simpler, more discoverable, and works reliably across all
# Blender 4.x / 5.x extension install methods.

def _dev_mode():
    """
    Returns True when Developer Mode is switched on.
    Reads from the scene property so it is always accessible inside draw().
    Safe to call even before the scene property is registered — returns False.
    """
    try:
        return bpy.context.scene.blender_toolkit.developer_mode
    except Exception:
        return False


# ─────────────────────────────────────────────────────────────────────────────
# SECTION: PROPERTY GROUP (THE MASTER SETTINGS DASHBOARD)
# ─────────────────────────────────────────────────────────────────────────────
class BTProps(PropertyGroup):
    """Holds all custom variables and user settings used by the Blender Toolkit addon."""

    # ── Section visibility toggles (one per original section) ────────────────
    # These power the ROLLUP-style collapsible headers.
    # The names match the section labels after slug-conversion (lower + underscore).
    show_transform:    BoolProperty(default=True)
    show_align:        BoolProperty(default=False)
    show_selection:    BoolProperty(default=False)
    show_object_tools: BoolProperty(default=False)
    show_mesh_tools:   BoolProperty(default=False)
    show_naming:       BoolProperty(default=False)
    show_materials:    BoolProperty(default=False)
    show_uv:           BoolProperty(default=False)
    show_wire:         BoolProperty(default=False)
    show_generators:   BoolProperty(default=False)
    show_lighting:     BoolProperty(default=False)
    show_camera:       BoolProperty(default=False)
    show_scene:        BoolProperty(default=False)

    # ── Random Transform properties ───────────────────────────────────────────
    rt_do_pos:      BoolProperty(name="Position", default=True)
    rt_pos_x_min:   FloatProperty(name="X Min", default=-1.0, unit='LENGTH')
    rt_pos_x_max:   FloatProperty(name="X Max", default=1.0,  unit='LENGTH')
    rt_pos_y_min:   FloatProperty(name="Y Min", default=-1.0, unit='LENGTH')
    rt_pos_y_max:   FloatProperty(name="Y Max", default=1.0,  unit='LENGTH')
    rt_pos_z_min:   FloatProperty(name="Z Min", default=0.0,  unit='LENGTH')
    rt_pos_z_max:   FloatProperty(name="Z Max", default=0.0,  unit='LENGTH')

    rt_do_rot:      BoolProperty(name="Rotation", default=False)
    rt_rot_x_min:   FloatProperty(name="X Min", default=0.0, subtype='ANGLE')
    rt_rot_x_max:   FloatProperty(name="X Max", default=0.0, subtype='ANGLE')
    rt_rot_y_min:   FloatProperty(name="Y Min", default=0.0, subtype='ANGLE')
    rt_rot_y_max:   FloatProperty(name="Y Max", default=0.0, subtype='ANGLE')
    rt_rot_z_min:   FloatProperty(name="Z Min", default=-math.pi, subtype='ANGLE')
    rt_rot_z_max:   FloatProperty(name="Z Max", default=math.pi,  subtype='ANGLE')

    rt_do_scl:       BoolProperty(name="Scale", default=False)
    rt_scl_uniform:  BoolProperty(name="Uniform", default=True)
    rt_scl_x_min:    FloatProperty(name="X Min", default=0.8)
    rt_scl_x_max:    FloatProperty(name="X Max", default=1.2)
    rt_scl_y_min:    FloatProperty(name="Y Min", default=0.8)
    rt_scl_y_max:    FloatProperty(name="Y Max", default=1.2)
    rt_scl_z_min:    FloatProperty(name="Z Min", default=0.8)
    rt_scl_z_max:    FloatProperty(name="Z Max", default=1.2)

    rt_seed: IntProperty(name="Seed", default=0)

    # ── Align properties ──────────────────────────────────────────────────────
    align_axis:   EnumProperty(name="Axis", items=[('X','X',''),('Y','Y',''),('Z','Z','')], default='X')
    align_offset: FloatProperty(name="Offset", default=0.0, unit='LENGTH')
    align_bbox:   BoolProperty(name="Use Bounding Box", default=True)

    # ── Naming properties ─────────────────────────────────────────────────────
    nm_prefix:  StringProperty(name="Prefix", default="")
    nm_suffix:  StringProperty(name="Suffix", default="")
    nm_find:    StringProperty(name="Find",   default="")
    nm_replace: StringProperty(name="Replace",default="")
    nm_base:    StringProperty(name="Base",   default="Object")
    nm_start:   IntProperty(name="Start",     default=1)
    nm_pad:     IntProperty(name="Padding",   default=3, min=1, max=6)

    # ── Wire colour mode ──────────────────────────────────────────────────────
    wire_mode: EnumProperty(name="Mode", items=[
        ('OBJECT',     'Per Object',     'One colour per object',     'OBJECT_DATA', 0),
        ('MATERIAL',   'Per Material',   'Colour follows material',   'MATERIAL',    1),
        ('COLLECTION', 'Per Collection', 'Colour follows collection', 'OUTLINER_COLLECTION', 2),
    ], default='OBJECT')

    # ── Generator: Circle Array ───────────────────────────────────────────────
    gen_ca_count:  IntProperty(name="Count",  default=8,   min=1)
    gen_ca_radius: FloatProperty(name="Radius",default=2.0, unit='LENGTH')
    gen_ca_axis:   EnumProperty(name="Axis", items=[('X','X',''),('Y','Y',''),('Z','Z','')], default='Z')
    gen_ca_align:  BoolProperty(name="Align Instances", default=True)

    # ── Generator: Floor ─────────────────────────────────────────────────────
    gen_fl_w:       FloatProperty(name="Width",      default=4.0,  unit='LENGTH')
    gen_fl_d:       FloatProperty(name="Depth",      default=4.0,  unit='LENGTH')
    gen_fl_tw:      FloatProperty(name="Tile W",     default=0.6,  unit='LENGTH')
    gen_fl_td:      FloatProperty(name="Tile D",     default=0.6,  unit='LENGTH')
    gen_fl_grout:   FloatProperty(name="Grout",      default=0.01, unit='LENGTH')
    gen_fl_thick:   FloatProperty(name="Thickness",  default=0.02, unit='LENGTH')
    gen_fl_vary_z:  FloatProperty(name="Z Vary",     default=0.002,unit='LENGTH')
    gen_fl_pattern: EnumProperty(name="Pattern", items=[
        ('GRID','Grid',''),('BRICK','Brick',''),('HERRING','Herringbone','')], default='GRID')

    # ── Generator: Simple Scatter ─────────────────────────────────────────────
    gen_sc_count:   IntProperty(name="Count",    default=50,  min=1)
    gen_sc_seed:    IntProperty(name="Seed",     default=0)
    gen_sc_scl_min: FloatProperty(name="Scl Min",default=0.8)
    gen_sc_scl_max: FloatProperty(name="Scl Max",default=1.2)
    gen_sc_rot_z:   BoolProperty(name="Rand Rot Z", default=True)

    # ── Generator: UVW Box Map ────────────────────────────────────────────────
    gen_uvw_size:  FloatProperty(name="Box Size", default=1.0, unit='LENGTH')
    gen_uvw_blend: FloatProperty(name="Blend",    default=0.3, min=0.0, max=1.0)

    # ── Subdivision ───────────────────────────────────────────────────────────
    sub_levels_view:   IntProperty(name="Viewport", default=1, min=0, max=6)
    sub_levels_render: IntProperty(name="Render",   default=2, min=0, max=6)
    sub_do_view:       BoolProperty(name="Set Viewport", default=True)
    sub_do_render:     BoolProperty(name="Set Render",   default=True)

    # ── Replace Object ────────────────────────────────────────────────────────
    rep_keep_transforms: BoolProperty(name="Keep Transforms", default=True)
    rep_keep_name:       BoolProperty(name="Keep Name",       default=True)

    # ── Roof Generator ────────────────────────────────────────────────────────
    roof_height:    FloatProperty(name="Height",   default=2.0, unit='LENGTH')
    roof_overhang:  FloatProperty(name="Overhang", default=0.3, unit='LENGTH')
    roof_type:      EnumProperty(name="Type", items=[
        ('HIP','Hip',''), ('GABLE','Gable',''), ('FLAT','Flat','')], default='HIP')

    # ── Scale Mapping ─────────────────────────────────────────────────────────
    map_scale_factor: FloatProperty(name="Scale Factor", default=2.0)
    map_scale_mode:   EnumProperty(name="Mode", items=[
        ('MULTIPLY','Multiply',''), ('SET','Set',''), ('ADD','Add','')], default='MULTIPLY')

    # ── Poly count result ─────────────────────────────────────────────────────
    poly_count_result: StringProperty(name="", default="—")

    # ── Lighting ──────────────────────────────────────────────────────────────
    light_energy_min:   FloatProperty(name="Min Energy", default=100.0)
    light_energy_max:   FloatProperty(name="Max Energy", default=2000.0)

    # ── Viewport Opacity ──────────────────────────────────────────────────────
    viewport_opacity:   FloatProperty(name="Opacity", default=0.5, min=0.0, max=1.0)

    # ── UVW Randomiser ────────────────────────────────────────────────────────
    uvw_rand_offset:    BoolProperty(name="Rand Offset",   default=True)
    uvw_rand_rotation:  BoolProperty(name="Rand Rotation", default=True)
    uvw_rand_scale:     BoolProperty(name="Rand Scale",    default=False)
    uvw_rand_scale_min: FloatProperty(name="Scale Min",    default=0.8)
    uvw_rand_scale_max: FloatProperty(name="Scale Max",    default=1.2)

    # ── Replace Material ──────────────────────────────────────────────────────
    rep_mat_find:    StringProperty(name="Find Material")
    rep_mat_replace: StringProperty(name="Replace With")

    # ── Collection ────────────────────────────────────────────────────────────
    new_coll_name: StringProperty(name="Collection Name", default="New Collection")

    # ── Camera ────────────────────────────────────────────────────────────────
    cam_res: EnumProperty(name="Resolution", items=[
        ('HD',  '1920×1080', ''),
        ('QHD', '2560×1440', ''),
        ('4K',  '3840×2160', ''),
    ], default='HD')

    # ── 3-Point Align ─────────────────────────────────────────────────────────
    tpa_p1: FloatVectorProperty(name="P1", subtype='XYZ')
    tpa_p2: FloatVectorProperty(name="P2", subtype='XYZ')
    tpa_p3: FloatVectorProperty(name="P3", subtype='XYZ')

    # ── Developer Mode toggle ─────────────────────────────────────────────────
    # Stored on the scene so the button appears right inside the panel.
    # Tick this to unlock the Layout Editor controls.
    developer_mode: BoolProperty(
        name="Developer Mode",
        description="Show layout editor controls — move, rename, delete and add sections",
        default=False,
    )  # type: ignore

    # ── Developer-mode inline edit fields ────────────────────────────────────
    dev_edit_label: StringProperty(name="Label", default="")
    dev_edit_icon:  StringProperty(name="Icon",  default="DOT")
    dev_new_label:  StringProperty(name="New Section Name", default="")


# ─────────────────────────────────────────────────────────────────────────────
# SECTION: HELPER UTILITIES (THE "BEHIND-THE-SCENES" WORKBENCH)
# ─────────────────────────────────────────────────────────────────────────────
_VALID_ICONS = None

def _safe_icon(icon: str) -> str:
    """Returns the icon name if Blender knows it, otherwise falls back to 'DOT'."""
    global _VALID_ICONS
    if _VALID_ICONS is None:
        try:
            _VALID_ICONS = set(
                bpy.types.UILayout.bl_rna.functions['label']
                .parameters['icon'].enum_items.keys()
            )
        except Exception:
            _VALID_ICONS = set()
    if not icon: return 'NONE'
    if _VALID_ICONS and icon not in _VALID_ICONS: return 'DOT'
    return icon

def I(icon: str) -> str:
    """Shorthand for _safe_icon — keeps draw() calls readable."""
    return _safe_icon(icon)

def _label_to_key(label: str) -> str:
    """
    Converts a section label like "OBJECT TOOLS" into a valid Python
    property name like "show_object_tools" that we can use as a toggle key.
    re.sub() replaces any non-word character (spaces, hyphens, slashes) with _.
    """
    return "show_" + re.sub(r'\W+', '_', label.lower()).strip('_')

def _section_header(layout, props, prop_name: str, label: str, icon: str = 'DOT'):
    """
    Draws the ▶/▼ toggle row for a collapsible section.
    Returns True when the section is currently expanded so the caller
    knows whether to draw the section body.
    """
    row = layout.row(align=True)
    row.prop(props, prop_name,
             icon=I('TRIA_DOWN') if getattr(props, prop_name) else I('TRIA_RIGHT'),
             icon_only=True, emboss=False)
    try:
        row.label(text=label, icon=_safe_icon(icon))
    except TypeError:
        row.label(text=label)
    return getattr(props, prop_name)

def _rand_hsv_color():
    h = random.random()
    s = 0.65 + random.random() * 0.35
    v = 0.55 + random.random() * 0.45
    r, g, b = colorsys.hsv_to_rgb(h, s, v)
    return (r, g, b)

def _get_or_create_gn_mod(obj, name: str):
    for mod in obj.modifiers:
        if mod.type == 'NODES' and mod.name == name: return mod
    mod = obj.modifiers.new(name=name, type='NODES')
    return mod

def _ensure_gn_tree(mod, name: str):
    if mod.node_group and mod.node_group.name == name: return mod.node_group
    tree = bpy.data.node_groups.new(name=name, type='GeometryNodeTree')
    mod.node_group = tree
    tree.nodes.clear()
    tree.interface.new_socket('Geometry', in_out='INPUT',  socket_type='NodeSocketGeometry')
    tree.interface.new_socket('Geometry', in_out='OUTPUT', socket_type='NodeSocketGeometry')
    return tree

def _link(tree, from_node, from_sock, to_node, to_sock):
    tree.links.new(from_node.outputs[from_sock], to_node.inputs[to_sock])

def _add_light(ctx, name, ltype, energy, loc, rot, color=(1,1,1), radius=0.5):
    bpy.ops.object.light_add(type=ltype, location=loc)
    light = ctx.active_object
    light.name = name
    light.data.energy = energy
    light.data.color  = color
    light.rotation_euler = Euler(rot)
    if hasattr(light.data, 'shadow_soft_size'): light.data.shadow_soft_size = radius
    return light

def _build_circle_array_gn(mod, count, radius, axis, align):
    tree = _ensure_gn_tree(mod, "BT_CircleArray")
    nodes = tree.nodes; links = tree.links
    nodes.clear()
    gi   = nodes.new('NodeGroupInput');   gi.location  = (-400, 0)
    go   = nodes.new('NodeGroupOutput');  go.location  = (800, 0)
    circ = nodes.new('GeometryNodeMeshCircle'); circ.location = (-200, 120)
    circ.inputs['Vertices'].default_value = count
    circ.inputs['Radius'].default_value   = radius
    circ.fill_type = 'NONE'
    m2p  = nodes.new('GeometryNodeMeshToPoints'); m2p.location = (0, 120)
    iop  = nodes.new('GeometryNodeInstanceOnPoints'); iop.location = (250, 0)
    iop.inputs['Pick Instance'].default_value = False
    rot_node = None
    if align:
        rot_node = nodes.new('GeometryNodeAlignRotationToVector')
        rot_node.location = (250, -150)
        axis_map = {'X': 'X', 'Y': 'Y', 'Z': 'Z'}
        rot_node.axis = axis_map.get(axis, 'Z')
        links.new(m2p.outputs['Points'], rot_node.inputs['Vector'])
    real = nodes.new('GeometryNodeRealizeInstances'); real.location = (550, 0)
    links.new(circ.outputs['Mesh'], m2p.inputs['Mesh'])
    links.new(gi.outputs['Geometry'],  iop.inputs['Instance'])
    links.new(m2p.outputs['Points'],   iop.inputs['Points'])
    if rot_node: links.new(rot_node.outputs['Rotation'], iop.inputs['Rotation'])
    links.new(iop.outputs['Instances'], real.inputs['Geometry'])
    links.new(real.outputs['Geometry'], go.inputs['Geometry'])

def _build_floor_gn(mod, w, d, tw, td, grout, thick, vary_z, pattern):
    tree = _ensure_gn_tree(mod, "BT_FloorGenerator")
    nodes = tree.nodes; links = tree.links
    nodes.clear()
    gi   = nodes.new('NodeGroupInput');   gi.location = (-400, 0)
    go   = nodes.new('NodeGroupOutput');  go.location = (900, 0)
    cols = max(1, int(w / (tw + grout)))
    rows = max(1, int(d / (td + grout)))
    grid = nodes.new('GeometryNodeMeshGrid'); grid.location = (-150, 100)
    grid.inputs['Size X'].default_value = w
    grid.inputs['Size Y'].default_value = d
    grid.inputs['Vertices X'].default_value = cols + 1
    grid.inputs['Vertices Y'].default_value = rows + 1
    m2p  = nodes.new('GeometryNodeMeshToPoints'); m2p.location = (100, 100)
    m2p.inputs['Position'].hide = True
    tile = nodes.new('GeometryNodeMeshCube'); tile.location = (-150, -100)
    tile.inputs['Size'].default_value = (tw, td, thick)
    iop  = nodes.new('GeometryNodeInstanceOnPoints'); iop.location = (320, 0)
    real = nodes.new('GeometryNodeRealizeInstances'); real.location = (600, 0)
    links.new(grid.outputs['Mesh'],      m2p.inputs['Mesh'])
    links.new(m2p.outputs['Points'],     iop.inputs['Points'])
    links.new(tile.outputs['Mesh'],      iop.inputs['Instance'])
    links.new(iop.outputs['Instances'],  real.inputs['Geometry'])
    links.new(real.outputs['Geometry'],  go.inputs['Geometry'])

def _build_scatter_gn(mod, count, seed, scl_min, scl_max, rot_z):
    tree = _ensure_gn_tree(mod, "BT_SimpleScatter")
    nodes = tree.nodes; links = tree.links
    nodes.clear()
    gi   = nodes.new('NodeGroupInput');  gi.location  = (-500, 0)
    go   = nodes.new('NodeGroupOutput'); go.location  = (900, 0)
    dist = nodes.new('GeometryNodeDistributePointsOnFaces'); dist.location = (-200, 100)
    dist.inputs['Count'].default_value   = count
    dist.inputs['Seed'].default_value    = seed
    iop  = nodes.new('GeometryNodeInstanceOnPoints'); iop.location = (150, 0)
    rnd_scl = nodes.new('FunctionNodeRandomValue'); rnd_scl.location = (-200, -150)
    rnd_scl.data_type = 'FLOAT'
    rnd_scl.inputs['Min'].default_value = scl_min
    rnd_scl.inputs['Max'].default_value = scl_max
    sc3  = nodes.new('ShaderNodeCombineXYZ'); sc3.location = (0, -150)
    links.new(rnd_scl.outputs[1], sc3.inputs['X'])
    links.new(rnd_scl.outputs[1], sc3.inputs['Y'])
    links.new(rnd_scl.outputs[1], sc3.inputs['Z'])
    links.new(sc3.outputs['Vector'], iop.inputs['Scale'])
    if rot_z:
        rnd_rot = nodes.new('FunctionNodeRandomValue'); rnd_rot.location = (-200, -280)
        rnd_rot.data_type = 'FLOAT'
        rnd_rot.inputs['Min'].default_value = 0.0
        rnd_rot.inputs['Max'].default_value = math.pi * 2
        eu = nodes.new('ShaderNodeCombineXYZ'); eu.location = (0, -280)
        eu.inputs['X'].default_value = 0.0; eu.inputs['Y'].default_value = 0.0
        links.new(rnd_rot.outputs[1], eu.inputs['Z'])
        links.new(eu.outputs['Vector'], iop.inputs['Rotation'])
    real = nodes.new('GeometryNodeRealizeInstances'); real.location = (600, 0)
    links.new(gi.outputs['Geometry'],   dist.inputs['Mesh'])
    links.new(gi.outputs['Geometry'],   iop.inputs['Instance'])
    links.new(dist.outputs['Points'],   iop.inputs['Points'])
    links.new(iop.outputs['Instances'], real.inputs['Geometry'])
    links.new(real.outputs['Geometry'], go.inputs['Geometry'])

def _build_uvw_box_gn(mod, size, blend):
    tree = _ensure_gn_tree(mod, "BT_UVWBoxMap")
    nodes = tree.nodes; links = tree.links
    nodes.clear()
    gi   = nodes.new('NodeGroupInput');   gi.location  = (-800, 0)
    go   = nodes.new('NodeGroupOutput');  go.location  = (600, 0)
    pos  = nodes.new('GeometryNodeInputPosition'); pos.location = (-600, 100)
    nor  = nodes.new('GeometryNodeInputNormal');   nor.location = (-600, -50)
    div  = nodes.new('ShaderNodeVectorMath'); div.location = (-400, 100)
    div.operation = 'DIVIDE'
    div.inputs[1].default_value = (size, size, size)
    links.new(pos.outputs['Position'], div.inputs[0])
    sep_p = nodes.new('ShaderNodeSeparateXYZ'); sep_p.location = (-200, 100)
    sep_n = nodes.new('ShaderNodeSeparateXYZ'); sep_n.location = (-200, -50)
    links.new(div.outputs['Vector'], sep_p.inputs['Vector'])
    links.new(nor.outputs['Normal'], sep_n.inputs['Vector'])
    uv_top   = nodes.new('ShaderNodeCombineXYZ'); uv_top.location   = (0, 200)
    uv_front = nodes.new('ShaderNodeCombineXYZ'); uv_front.location = (0, 50)
    uv_side  = nodes.new('ShaderNodeCombineXYZ'); uv_side.location  = (0, -100)
    links.new(sep_p.outputs['X'], uv_top.inputs['X']);   links.new(sep_p.outputs['Y'], uv_top.inputs['Y'])
    links.new(sep_p.outputs['X'], uv_front.inputs['X']); links.new(sep_p.outputs['Z'], uv_front.inputs['Y'])
    links.new(sep_p.outputs['Y'], uv_side.inputs['X']);  links.new(sep_p.outputs['Z'], uv_side.inputs['Y'])
    abs_n = nodes.new('ShaderNodeVectorMath'); abs_n.location = (-200, -200)
    abs_n.operation = 'ABSOLUTE'
    links.new(nor.outputs['Normal'], abs_n.inputs[0])
    sep_a = nodes.new('ShaderNodeSeparateXYZ'); sep_a.location = (0, -250)
    links.new(abs_n.outputs['Vector'], sep_a.inputs['Vector'])
    mix1 = nodes.new('ShaderNodeMixRGB'); mix1.location = (250, 100)
    mix1.blend_type = 'MIX'
    links.new(sep_a.outputs['Z'], mix1.inputs['Fac'])
    links.new(uv_front.outputs['Vector'], mix1.inputs['Color1'])
    links.new(uv_top.outputs['Vector'],   mix1.inputs['Color2'])
    mix2 = nodes.new('ShaderNodeMixRGB'); mix2.location = (400, 0)
    mix2.blend_type = 'MIX'
    links.new(sep_a.outputs['X'], mix2.inputs['Fac'])
    links.new(mix1.outputs['Color'], mix2.inputs['Color1'])
    links.new(uv_side.outputs['Vector'], mix2.inputs['Color2'])
    store = nodes.new('GeometryNodeStoreNamedAttribute'); store.location = (550, 0)
    store.data_type = 'FLOAT2'
    store.domain    = 'CORNER'
    store.inputs['Name'].default_value = 'UVMap'
    links.new(gi.outputs['Geometry'],   store.inputs['Geometry'])
    links.new(mix2.outputs['Color'],    store.inputs['Value'])
    links.new(store.outputs['Geometry'], go.inputs['Geometry'])

def _build_roof_gn(mod, height, overhang, roof_type):
    tree = _ensure_gn_tree(mod, "BT_RoofGenerator")
    nodes = tree.nodes; links = tree.links
    nodes.clear()
    gi  = nodes.new('NodeGroupInput');   gi.location  = (-400, 0)
    go  = nodes.new('NodeGroupOutput');  go.location  = (600, 0)
    ext = nodes.new('GeometryNodeExtrudeMesh'); ext.location = (0, 0)
    ext.mode = 'FACES'
    ext.inputs['Offset Scale'].default_value = height
    ext.inputs['Individual'].default_value   = False
    scl = nodes.new('GeometryNodeScaleElements'); scl.location = (250, 0)
    scl.domain = 'FACE'
    scl.inputs['Scale'].default_value = 0.0
    links.new(gi.outputs['Geometry'], ext.inputs['Mesh'])
    links.new(ext.outputs['Mesh'],    scl.inputs['Geometry'])
    links.new(scl.outputs['Geometry'], go.inputs['Geometry'])


# ─────────────────────────────────────────────────────────────────────────────
# SECTION: LAYOUT EDITOR OPERATORS
# These are the "buttons" in the Developer Mode editor strip.
# Each one reads the config, mutates the sections list, then writes it back.
# ─────────────────────────────────────────────────────────────────────────────

class BT_OT_SectionMoveUp(Operator):
    """Move a panel section one position higher in the list."""
    bl_idname = "bt.section_move_up"
    bl_label  = "Move Section Up"
    # bl_options 'REGISTER' means the action is undoable with Ctrl+Z in Blender.
    bl_options = {'REGISTER', 'UNDO'}

    # 'index' is the position of the section we want to move.
    # It is set by the panel code when it draws the "▲" button.
    index: IntProperty()  # type: ignore

    def execute(self, ctx):
        sections = _read_config()
        i = self.index
        # Can't move the first section any higher.
        if i <= 0 or i >= len(sections):
            return {'CANCELLED'}
        # Swap section[i] with section[i-1] — a classic list swap.
        sections[i - 1], sections[i] = sections[i], sections[i - 1]
        _write_config(sections)
        # Force the N-panel to redraw immediately.
        for area in ctx.screen.areas:
            if area.type == 'VIEW_3D':
                area.tag_redraw()
        return {'FINISHED'}


class BT_OT_SectionMoveDown(Operator):
    """Move a panel section one position lower in the list."""
    bl_idname = "bt.section_move_down"
    bl_label  = "Move Section Down"
    bl_options = {'REGISTER', 'UNDO'}

    index: IntProperty()  # type: ignore

    def execute(self, ctx):
        sections = _read_config()
        i = self.index
        # Can't move the last section any lower.
        if i < 0 or i >= len(sections) - 1:
            return {'CANCELLED'}
        sections[i], sections[i + 1] = sections[i + 1], sections[i]
        _write_config(sections)
        for area in ctx.screen.areas:
            if area.type == 'VIEW_3D':
                area.tag_redraw()
        return {'FINISHED'}


class BT_OT_SectionRename(Operator):
    """Rename a panel section."""
    bl_idname = "bt.section_rename"
    bl_label  = "Rename Section"
    bl_options = {'REGISTER', 'UNDO'}

    index:    IntProperty()   # type: ignore
    new_name: StringProperty()  # type: ignore

    def execute(self, ctx):
        sections = _read_config()
        i = self.index
        if i < 0 or i >= len(sections):
            return {'CANCELLED'}
        new = self.new_name.strip()
        if not new:
            self.report({'WARNING'}, "Name cannot be empty.")
            return {'CANCELLED'}
        sections[i]['label'] = new
        _write_config(sections)
        for area in ctx.screen.areas:
            if area.type == 'VIEW_3D':
                area.tag_redraw()
        return {'FINISHED'}


class BT_OT_SectionDelete(Operator):
    """Delete a panel section (operators in it are unaffected — just hidden from the panel)."""
    bl_idname = "bt.section_delete"
    bl_label  = "Delete Section"
    bl_options = {'REGISTER', 'UNDO'}

    index: IntProperty()  # type: ignore

    def execute(self, ctx):
        sections = _read_config()
        i = self.index
        if i < 0 or i >= len(sections):
            return {'CANCELLED'}
        sections.pop(i)
        _write_config(sections)
        for area in ctx.screen.areas:
            if area.type == 'VIEW_3D':
                area.tag_redraw()
        return {'FINISHED'}


class BT_OT_SectionAdd(Operator):
    """Add a new empty section to the panel."""
    bl_idname = "bt.section_add"
    bl_label  = "Add Section"
    bl_options = {'REGISTER', 'UNDO'}

    label: StringProperty(default="NEW SECTION")  # type: ignore

    def execute(self, ctx):
        sections = _read_config()
        new_section = {
            "label": self.label.strip() or "NEW SECTION",
            "icon": "DOT",
            "style": "ROLLUP",
            "operators": [],
        }
        sections.append(new_section)
        _write_config(sections)
        for area in ctx.screen.areas:
            if area.type == 'VIEW_3D':
                area.tag_redraw()
        return {'FINISHED'}


class BT_OT_SectionCycleStyle(Operator):
    """
    Cycle a section's display style through BOX → ROLLUP → FLAT.
    BOX   = always-visible box with no header toggle.
    ROLLUP = collapsible box with a ▶/▼ header.
    FLAT  = no box, buttons drawn directly on the panel.
    """
    bl_idname = "bt.section_cycle_style"
    bl_label  = "Cycle Section Style"
    bl_options = {'REGISTER', 'UNDO'}

    index: IntProperty()  # type: ignore

    def execute(self, ctx):
        sections = _read_config()
        i = self.index
        if i < 0 or i >= len(sections):
            return {'CANCELLED'}
        cycle = ['BOX', 'ROLLUP', 'FLAT']
        current = sections[i].get('style', 'ROLLUP')
        next_style = cycle[(cycle.index(current) + 1) % len(cycle)] if current in cycle else 'ROLLUP'
        sections[i]['style'] = next_style
        _write_config(sections)
        for area in ctx.screen.areas:
            if area.type == 'VIEW_3D':
                area.tag_redraw()
        return {'FINISHED'}


class BT_OT_SectionSetIcon(Operator):
    """Set a section's icon by name (type a valid Blender icon name)."""
    bl_idname = "bt.section_set_icon"
    bl_label  = "Set Section Icon"
    bl_options = {'REGISTER', 'UNDO'}

    index: IntProperty()   # type: ignore
    icon:  StringProperty()  # type: ignore

    def execute(self, ctx):
        sections = _read_config()
        i = self.index
        if i < 0 or i >= len(sections):
            return {'CANCELLED'}
        sections[i]['icon'] = self.icon.strip().upper() or 'DOT'
        _write_config(sections)
        for area in ctx.screen.areas:
            if area.type == 'VIEW_3D':
                area.tag_redraw()
        return {'FINISHED'}


class BT_OT_AddScript(Operator):
    """
    Open a file browser, pick a .py script, copy it into this addon's
    directory and add its operator IDs to the chosen section.

    The script is copied so the addon stays self-contained inside the
    extension folder — no dangling external references.
    """
    bl_idname      = "bt.section_add_script"
    bl_label       = "Add Script to Section"
    bl_options     = {'REGISTER', 'UNDO'}
    # These are special Blender properties that activate the file browser.
    filepath: StringProperty(subtype='FILE_PATH')  # type: ignore
    filter_glob: StringProperty(default="*.py", options={'HIDDEN'})  # type: ignore
    # Which section index to add the script to.
    section_index: IntProperty(default=0)  # type: ignore

    def invoke(self, ctx, event):
        # Open Blender's built-in file browser dialog.
        ctx.window_manager.fileselect_add(self)
        return {'RUNNING_MODAL'}

    def execute(self, ctx):
        import shutil

        src = self.filepath
        if not src.endswith('.py') or not os.path.isfile(src):
            self.report({'ERROR'}, "Please select a .py file.")
            return {'CANCELLED'}

        # Destination: put the script in a 'custom_scripts' subfolder of the addon.
        addon_dir   = os.path.dirname(__file__)
        scripts_dir = os.path.join(addon_dir, "custom_scripts")
        os.makedirs(scripts_dir, exist_ok=True)

        dest = os.path.join(scripts_dir, os.path.basename(src))
        shutil.copy2(src, dest)

        # Import the new module to discover operator bl_idnames it registers.
        spec = importlib.util.spec_from_file_location("_bt_temp_script", dest)
        mod  = importlib.util.module_from_spec(spec)
        try:
            spec.loader.exec_module(mod)
        except Exception as e:
            self.report({'WARNING'}, f"Script loaded but could not be executed for inspection: {e}")
            mod = None

        # Collect any Operator subclasses the script defines.
        op_ids = []
        if mod:
            for name in dir(mod):
                obj = getattr(mod, name, None)
                if (obj and isinstance(obj, type)
                        and issubclass(obj, bpy.types.Operator)
                        and hasattr(obj, 'bl_idname')):
                    op_ids.append(obj.bl_idname)

        # Update the config.
        sections = _read_config()
        i = self.section_index
        if 0 <= i < len(sections):
            existing = sections[i].get('operators', [])
            for oid in op_ids:
                if oid not in existing:
                    existing.append(oid)
            sections[i]['operators'] = existing
            _write_config(sections)

        # Reload the addon's tool modules so the new operators appear immediately.
        _load_tools()
        for m in _TOOL_MODULES:
            if hasattr(m, 'register'):
                try: m.register()
                except Exception: pass

        self.report({'INFO'}, f"Added {os.path.basename(src)} → section {i} ({len(op_ids)} operators found)")
        for area in ctx.screen.areas:
            if area.type == 'VIEW_3D':
                area.tag_redraw()
        return {'FINISHED'}


class BT_OT_MoveOperator(Operator):
    """Move an operator (by bl_idname) from one section to another."""
    bl_idname = "bt.section_move_operator"
    bl_label  = "Move Operator"
    bl_options = {'REGISTER', 'UNDO'}

    op_id:       StringProperty()  # type: ignore
    from_index:  IntProperty()     # type: ignore
    to_index:    IntProperty()     # type: ignore

    def execute(self, ctx):
        sections = _read_config()
        fi = self.from_index
        ti = self.to_index
        if fi < 0 or fi >= len(sections) or ti < 0 or ti >= len(sections):
            return {'CANCELLED'}
        from_ops = sections[fi].get('operators', [])
        to_ops   = sections[ti].get('operators', [])
        if self.op_id in from_ops:
            from_ops.remove(self.op_id)
            if self.op_id not in to_ops:
                to_ops.append(self.op_id)
            sections[fi]['operators'] = from_ops
            sections[ti]['operators'] = to_ops
            _write_config(sections)
        for area in ctx.screen.areas:
            if area.type == 'VIEW_3D':
                area.tag_redraw()
        return {'FINISHED'}


class BT_OT_RemoveOperatorFromSection(Operator):
    """Remove an operator entry from a section's display list (does NOT delete the .py file)."""
    bl_idname = "bt.section_remove_operator"
    bl_label  = "Remove from Section"
    bl_options = {'REGISTER', 'UNDO'}

    op_id:        StringProperty()  # type: ignore
    section_index: IntProperty()   # type: ignore

    def execute(self, ctx):
        sections = _read_config()
        i = self.section_index
        if i < 0 or i >= len(sections):
            return {'CANCELLED'}
        ops = sections[i].get('operators', [])
        if self.op_id in ops:
            ops.remove(self.op_id)
            sections[i]['operators'] = ops
            _write_config(sections)
        for area in ctx.screen.areas:
            if area.type == 'VIEW_3D':
                area.tag_redraw()
        return {'FINISHED'}


class BT_OT_OperatorMoveUp(Operator):
    """Move a script one position UP inside its section."""
    bl_idname = "bt.operator_move_up"
    bl_label  = "Move Script Up"
    bl_options = {'REGISTER', 'UNDO'}

    section_index: IntProperty()  # type: ignore
    op_index:      IntProperty()  # type: ignore

    def execute(self, ctx):
        sections = _read_config()
        si, oi = self.section_index, self.op_index
        if si < 0 or si >= len(sections):
            return {'CANCELLED'}
        ops = sections[si].get('operators', [])
        # Already at the top of its section — nothing to do.
        if oi <= 0 or oi >= len(ops):
            return {'CANCELLED'}
        ops[oi - 1], ops[oi] = ops[oi], ops[oi - 1]
        sections[si]['operators'] = ops
        _write_config(sections)
        _redraw(ctx)
        return {'FINISHED'}


class BT_OT_OperatorMoveDown(Operator):
    """Move a script one position DOWN inside its section."""
    bl_idname = "bt.operator_move_down"
    bl_label  = "Move Script Down"
    bl_options = {'REGISTER', 'UNDO'}

    section_index: IntProperty()  # type: ignore
    op_index:      IntProperty()  # type: ignore

    def execute(self, ctx):
        sections = _read_config()
        si, oi = self.section_index, self.op_index
        if si < 0 or si >= len(sections):
            return {'CANCELLED'}
        ops = sections[si].get('operators', [])
        if oi < 0 or oi >= len(ops) - 1:
            return {'CANCELLED'}
        ops[oi], ops[oi + 1] = ops[oi + 1], ops[oi]
        sections[si]['operators'] = ops
        _write_config(sections)
        _redraw(ctx)
        return {'FINISHED'}


# Blender can garbage-collect strings returned by an EnumProperty callback,
# which shows up as corrupted menu entries. Keeping a module-level reference
# to the most recent list is the standard workaround.
_ENUM_KEEPALIVE = []

def _section_enum_items(self, context):
    """
    Builds the 'Move to section' dropdown contents fresh each time it opens,
    so newly added sections appear without needing an addon reload.
    """
    global _ENUM_KEEPALIVE
    sections = _read_config()
    items = []
    for i, s in enumerate(sections):
        label = s.get('label', f'Section {i}')
        items.append((str(i), label, f"Move this script into {label}"))
    if not items:
        items = [('0', 'No sections', '')]
    _ENUM_KEEPALIVE = items
    return _ENUM_KEEPALIVE


class BT_OT_MoveOperatorPopup(Operator):
    """Pick a destination section and move this script into it."""
    bl_idname = "bt.section_move_operator_popup"
    bl_label  = "Move Script to Section"
    bl_options = {'REGISTER', 'UNDO'}

    op_id:      StringProperty()  # type: ignore
    from_index: IntProperty()     # type: ignore
    target:     EnumProperty(name="Move to", items=_section_enum_items)  # type: ignore

    def invoke(self, ctx, event):
        return ctx.window_manager.invoke_props_dialog(self)

    def draw(self, ctx):
        col = self.layout.column()
        col.label(text=self.op_id, icon=I('SCRIPT'))
        col.prop(self, 'target')

    def execute(self, ctx):
        sections = _read_config()
        fi = self.from_index
        try:
            ti = int(self.target)
        except (TypeError, ValueError):
            return {'CANCELLED'}
        if fi < 0 or fi >= len(sections) or ti < 0 or ti >= len(sections):
            return {'CANCELLED'}
        if fi == ti:
            return {'CANCELLED'}  # moving to itself is a no-op

        from_ops = sections[fi].get('operators', [])
        to_ops   = sections[ti].get('operators', [])
        if self.op_id in from_ops:
            from_ops.remove(self.op_id)
        if self.op_id not in to_ops:
            to_ops.append(self.op_id)
        sections[fi]['operators'] = from_ops
        sections[ti]['operators'] = to_ops
        _write_config(sections)
        _redraw(ctx)
        self.report({'INFO'}, f"Moved {self.op_id} → {sections[ti].get('label')}")
        return {'FINISHED'}


class BT_OT_ResetLayout(Operator):
    """Restore the original 13-section layout by overwriting layout_config.json with defaults."""
    bl_idname = "bt.layout_reset"
    bl_label  = "Reset Layout to Defaults"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        _write_config(_DEFAULT_SECTIONS)
        for area in ctx.screen.areas:
            if area.type == 'VIEW_3D':
                area.tag_redraw()
        self.report({'INFO'}, "Blender Toolkit layout reset to defaults.")
        return {'FINISHED'}


# ─────────────────────────────────────────────────────────────────────────────
# SECTION: THE MAIN USER INTERFACE PANEL (DRAWING THE SIDEBAR MENU)
# ─────────────────────────────────────────────────────────────────────────────

# This dictionary maps a section label to its dedicated draw function.
# For complex sections that need more than a button list (e.g. Random Transform
# which has sub-properties, grid flows, and quick-rotate rows), a custom draw
# function is registered here. For any section NOT in this dict, the panel
# falls back to drawing one operator button per entry in "operators".
_SECTION_DRAW_OVERRIDES = {}

def _draw_transform(layout, ctx):
    """Custom draw for the TRANSFORM section — preserves the original rich UI."""
    p = ctx.scene.blender_toolkit
    box = layout.box(); col = box.column(align=True)
    row = col.row(align=True)
    row.prop(p, 'rt_do_pos', text="", icon=I('EMPTY_ARROWS'))
    row.label(text="Position")
    if p.rt_do_pos:
        g = col.grid_flow(row_major=True, columns=2, align=True)
        g.prop(p, 'rt_pos_x_min', text="X Min"); g.prop(p, 'rt_pos_x_max', text="X Max")
        g.prop(p, 'rt_pos_y_min', text="Y Min"); g.prop(p, 'rt_pos_y_max', text="Y Max")
        g.prop(p, 'rt_pos_z_min', text="Z Min"); g.prop(p, 'rt_pos_z_max', text="Z Max")
    col.separator(factor=0.5)
    row = col.row(align=True)
    row.prop(p, 'rt_do_rot', text="", icon=I('DRIVER_ROTATIONAL_DIFFERENCE'))
    row.label(text="Rotation")
    if p.rt_do_rot:
        g = col.grid_flow(row_major=True, columns=2, align=True)
        g.prop(p, 'rt_rot_x_min', text="X Min"); g.prop(p, 'rt_rot_x_max', text="X Max")
        g.prop(p, 'rt_rot_y_min', text="Y Min"); g.prop(p, 'rt_rot_y_max', text="Y Max")
        g.prop(p, 'rt_rot_z_min', text="Z Min"); g.prop(p, 'rt_rot_z_max', text="Z Max")
    col.separator(factor=0.5)
    row = col.row(align=True)
    row.prop(p, 'rt_do_scl', text="", icon=I('FULLSCREEN_ENTER'))
    row.label(text="Scale")
    row.prop(p, 'rt_scl_uniform', text="Uniform", toggle=True)
    if p.rt_do_scl:
        if p.rt_scl_uniform:
            g = col.grid_flow(row_major=True, columns=2, align=True)
            g.prop(p, 'rt_scl_x_min', text="Min"); g.prop(p, 'rt_scl_x_max', text="Max")
        else:
            g = col.grid_flow(row_major=True, columns=2, align=True)
            g.prop(p, 'rt_scl_x_min', text="X Min"); g.prop(p, 'rt_scl_x_max', text="X Max")
            g.prop(p, 'rt_scl_y_min', text="Y Min"); g.prop(p, 'rt_scl_y_max', text="Y Max")
            g.prop(p, 'rt_scl_z_min', text="Z Min"); g.prop(p, 'rt_scl_z_max', text="Z Max")
    col.separator(factor=0.8)
    row = col.row(align=True)
    row.operator("bt.random_transform", icon=I('SHADERFX'))
    row.operator("bt.re_randomise",     icon=I('FILE_REFRESH'), text="")
    col.separator(factor=0.5)
    sub = col.row(align=True)
    sub.label(text="Quick Rotate:", icon=I('ORIENTATION_GIMBAL'))
    for ang in (90, 45, 30, 10):
        op = col.row(align=True)
        for ax in ('X','Y','Z'):
            o = op.operator("bt.quick_rotate", text=f"{ax} {ang}°")
            o.axis = ax; o.angle = ang
    col.separator(factor=0.5)
    col.operator("bt.drop_to_floor",        icon=I('IMPORT'))
    col.operator("bt.drop_to_surface",       icon=I('SNAP_FACE'))
    col.operator("bt.apply_all_transforms",  icon=I('CHECKMARK'))
    col.separator(factor=0.5)
    col.operator("bt.set_pivot_bottom",  icon=I('OBJECT_ORIGIN'))
    col.operator("bt.quick_measure",     icon=I('DRIVER_DISTANCE'))
    col.separator(factor=0.5)
    col.operator("bt.scale_by_distance", icon=I('FULLSCREEN_ENTER'))

def _draw_align(layout, ctx):
    p = ctx.scene.blender_toolkit
    box = layout.box(); col = box.column(align=True)
    row = col.row(align=True)
    row.prop(p, 'align_axis', expand=True)
    col.prop(p, 'align_offset', icon=I('ARROW_LEFTRIGHT'))
    col.prop(p, 'align_bbox',   icon=I('PIVOT_BOUNDBOX'))
    col.operator("bt.align_multiple",     icon=I('ALIGN_JUSTIFY'))
    col.separator(factor=0.5)
    col.operator("bt.align_to_object",    icon=I('OBJECT_DATA'))
    col.separator(factor=0.5)
    col.operator("bt.copy_transform",     icon=I('COPYDOWN'))
    col.separator(factor=0.5)
    col.operator("bt.arrange_in_grid",    icon=I('GRID'))
    col.separator(factor=0.5)
    col.operator("bt.place_along_spline", icon=I('CURVE_PATH'))
    col.separator(factor=0.5)
    sub = col.box()
    sub.label(text="3-Point Align", icon=I('NORMALS_FACE'))
    sub.prop(p, 'tpa_p1')
    sub.prop(p, 'tpa_p2')
    sub.prop(p, 'tpa_p3')
    sub.operator("bt.three_point_align", icon=I('CHECKMARK'))

def _draw_object_tools(layout, ctx):
    p = ctx.scene.blender_toolkit
    box = layout.box(); col = box.column(align=True)
    col.operator("bt.make_unique",          icon=I('UNLINKED'))
    col.operator("bt.replace_object",       icon=I('OBJECT_DATAMODE'))
    col.operator("bt.make_instances",       icon=I('LINKED'))
    col.operator("bt.explode_to_objects",   icon=I('PARTICLES'))
    col.operator("bt.attach_selection",     icon=I('MESH_DATA'))
    col.operator("bt.collapse_by_material", icon=I('MATERIAL'))
    col.separator(factor=0.5)
    col.operator("bt.mark_as_asset",        icon=I('ASSET_MANAGER'))
    col.separator(factor=0.5)
    row = col.row(align=True)
    lock_op = row.operator("bt.object_lock", text="Lock",   icon=I('LOCKED'))
    lock_op.lock = True
    unlock_op = row.operator("bt.object_lock", text="Unlock", icon=I('UNLOCKED'))
    unlock_op.lock = False
    col.separator(factor=0.5)
    col.operator("bt.place_at_vertices",    icon=I('VERTEXSEL'))
    col.separator(factor=0.5)
    sub = col.box()
    sub.label(text="Subdivision", icon=I('MOD_SUBSURF'))
    row = sub.row(align=True)
    row.prop(p, 'sub_do_view');   row.prop(p, 'sub_levels_view',   text="")
    row = sub.row(align=True)
    row.prop(p, 'sub_do_render'); row.prop(p, 'sub_levels_render', text="")
    sub.operator("bt.subdivision_control", icon=I('CHECKMARK'))
    col.separator(factor=0.5)
    col.operator("bt.modifier_utility",               icon=I('MODIFIER'))
    col.separator(factor=0.5)
    col.operator("bt.find_unique_objects",              icon=I('UNLINKED'))
    col.operator("bt.attach_splines",                   icon=I('CURVE_PATH'))
    col.operator("bt.create_collection_from_selection", icon=I('OUTLINER_COLLECTION'))
    col.operator("bt.create_line_at_base",              icon=I('IPO_CONSTANT'))
    col.operator("bt.place_name_below",                 icon=I('FONT_DATA'))

def _draw_naming(layout, ctx):
    p = ctx.scene.blender_toolkit
    box = layout.box(); col = box.column(align=True)
    row = col.row(align=True); row.prop(p, 'nm_prefix'); row.operator("bt.rename_prefix", icon=I('ADD'), text="")
    row = col.row(align=True); row.prop(p, 'nm_suffix'); row.operator("bt.rename_suffix", icon=I('ADD'), text="")
    col.separator(factor=0.5)
    col.prop(p, 'nm_find'); col.prop(p, 'nm_replace')
    col.operator("bt.rename_find_replace", icon=I('VIEWZOOM'))
    col.separator(factor=0.5)
    col.prop(p, 'nm_base'); col.prop(p, 'nm_start'); col.prop(p, 'nm_pad')
    col.operator("bt.rename_number", icon=I('LINENUMBERS_ON'))

def _draw_materials(layout, ctx):
    p = ctx.scene.blender_toolkit
    box = layout.box(); col = box.column(align=True)
    col.operator("bt.random_material",           icon=I('SHADERFX'))
    col.operator("bt.random_map_phase",          icon=I('RNDCURVE'))
    col.operator("bt.isolate_by_material",       icon=I('HIDE_OFF'))
    col.operator("bt.remove_materials",          icon=I('X'))
    col.operator("bt.merge_duplicate_materials", icon=I('AUTOMERGE_ON'))
    col.operator("bt.material_to_wire_color",    icon=I('SHADING_WIRE'))
    col.separator(factor=0.5)
    sub = col.box()
    sub.label(text="Scale Mapping Nodes", icon=I('DRIVER_TRANSFORM'))
    row = sub.row(align=True)
    row.prop(p, 'map_scale_mode', expand=True)
    sub.prop(p, 'map_scale_factor')
    sub.operator("bt.scale_procedural_maps", icon=I('CHECKMARK'))
    col.separator(factor=0.5)
    col.operator("bt.apply_default_material", icon=I('MATERIAL_DATA'))
    col.operator("bt.swap_materials",         icon=I('ARROW_LEFTRIGHT'))
    col.separator(factor=0.5)
    sub2 = col.box()
    sub2.label(text="Set Viewport Opacity", icon=I('XRAY'))
    sub2.prop(p, 'viewport_opacity')
    sub2.operator("bt.set_viewport_opacity", icon=I('CHECKMARK'))
    col.separator(factor=0.5)
    sub3 = col.box()
    sub3.label(text="Replace Material", icon=I('VIEWZOOM'))
    sub3.prop(p, 'rep_mat_find')
    sub3.prop(p, 'rep_mat_replace')
    sub3.operator("bt.replace_material", icon=I('CHECKMARK'))
    col.separator(factor=0.5)
    sub4 = col.box()
    sub4.label(text="UVW Randomiser", icon=I('RNDCURVE'))
    row = sub4.row(align=True)
    row.prop(p, 'uvw_rand_offset',   toggle=True)
    row.prop(p, 'uvw_rand_rotation', toggle=True)
    row.prop(p, 'uvw_rand_scale',    toggle=True)
    if p.uvw_rand_scale:
        g = sub4.grid_flow(columns=2, align=True)
        g.prop(p, 'uvw_rand_scale_min')
        g.prop(p, 'uvw_rand_scale_max')
    sub4.operator("bt.uvw_randomiser", icon=I('CHECKMARK'))

def _draw_uv(layout, ctx):
    p = ctx.scene.blender_toolkit
    box = layout.box(); col = box.column(align=True)
    sub = col.box()
    sub.label(text="UVW Box Map (GN)", icon=I('MOD_UVPROJECT'))
    sub.prop(p, 'gen_uvw_size')
    sub.prop(p, 'gen_uvw_blend')
    sub.operator("bt.uvw_box_map", icon=I('CHECKMARK'))
    col.separator(factor=0.5)
    col.operator("bt.uv_transfer",        icon=I('LOOP_FORWARDS'))
    col.operator("bt.apply_uv_grid",      icon=I('TEXTURE'))
    col.operator("bt.face_uv_transform",  icon=I('UV_FACESEL'))
    col.operator("bt.select_missing_uv",  icon=I('QUESTION'))
    col.separator(factor=0.5)
    col.operator("bt.reload_all_images",  icon=I('FILE_REFRESH'))

def _draw_wire(layout, ctx):
    p = ctx.scene.blender_toolkit
    box = layout.box(); col = box.column(align=True)
    col.prop(p, 'wire_mode', expand=True)
    col.operator("bt.wire_color_random",      icon=I('COLORSET_03_VEC'))
    col.operator("bt.material_to_wire_color", icon=I('MATERIAL'))

def _draw_generators(layout, ctx):
    p = ctx.scene.blender_toolkit
    box = layout.box(); col = box.column(align=True)
    sub = col.box()
    sub.label(text="Circle Array", icon=I('MESH_CIRCLE'))
    sub.prop(p, 'gen_ca_count'); sub.prop(p, 'gen_ca_radius')
    row = sub.row(align=True); row.prop(p, 'gen_ca_axis', expand=True)
    sub.prop(p, 'gen_ca_align')
    sub.operator("bt.circle_array", icon=I('CHECKMARK'))
    col.separator(factor=0.5)
    sub = col.box()
    sub.label(text="Floor Generator", icon=I('MESH_GRID'))
    g = sub.grid_flow(row_major=True, columns=2, align=True)
    g.prop(p, 'gen_fl_w'); g.prop(p, 'gen_fl_d')
    g.prop(p, 'gen_fl_tw'); g.prop(p, 'gen_fl_td')
    g.prop(p, 'gen_fl_grout'); g.prop(p, 'gen_fl_thick')
    sub.prop(p, 'gen_fl_vary_z')
    sub.prop(p, 'gen_fl_pattern')
    sub.operator("bt.floor_generator", icon=I('CHECKMARK'))
    col.separator(factor=0.5)
    sub = col.box()
    sub.label(text="Simple Scatter", icon=I('PARTICLES'))
    sub.prop(p, 'gen_sc_count')
    g = sub.grid_flow(row_major=True, columns=2, align=True)
    g.prop(p, 'gen_sc_scl_min'); g.prop(p, 'gen_sc_scl_max')
    sub.prop(p, 'gen_sc_rot_z')
    sub.operator("bt.simple_scatter", icon=I('CHECKMARK'))
    col.separator(factor=0.5)
    sub = col.box()
    sub.label(text="Roof Generator", icon=I('HOME'))
    sub.prop(p, 'roof_type')
    sub.prop(p, 'roof_height')
    sub.prop(p, 'roof_overhang')
    sub.operator("bt.roof_generator", icon=I('CHECKMARK'))

def _draw_lighting(layout, ctx):
    p = ctx.scene.blender_toolkit
    box = layout.box(); col = box.column(align=True)
    col.operator("bt.three_point_light", icon=I('LIGHT_SUN'))
    col.operator("bt.studio_light",      icon=I('LIGHT_AREA'))
    col.operator("bt.sun_light",         icon=I('LIGHT_SUN'))
    col.operator("bt.delete_all_lights", icon=I('TRASH'))
    col.separator(factor=0.5)
    col.label(text="Light Control", icon=I('LIGHT'))
    col.operator("bt.randomise_light_hue",     icon=I('COLORSET_03_VEC'))
    col.operator("bt.randomise_lights_on_off", icon=I('SHADERFX'))
    sub = col.box()
    sub.label(text="Randomise Energy", icon=I('LIGHT_SUN'))
    sub.prop(p, 'light_energy_min')
    sub.prop(p, 'light_energy_max')
    sub.operator("bt.randomise_light_energy", icon=I('CHECKMARK'))
    col.separator(factor=0.5)
    row = col.row(align=True)
    op  = row.operator("bt.toggle_light_visibility", text="Show All Lights", icon=I('LIGHT'))
    op.visible = True
    op2 = row.operator("bt.toggle_light_visibility", text="Hide All Lights", icon=I('LIGHT'))
    op2.visible = False

def _draw_camera(layout, ctx):
    p = ctx.scene.blender_toolkit
    box = layout.box(); col = box.column(align=True)
    col.operator("bt.add_camera_rig",     icon=I('CAMERA_DATA'))
    col.operator("bt.focus_on_selection", icon=I('ZOOM_IN'))
    col.separator(factor=0.5)
    sub = col.box()
    sub.label(text="Set Resolution", icon=I('RENDER_STILL'))
    sub.prop(p, 'cam_res', expand=True)
    sub.operator("bt.match_camera_res", icon=I('CHECKMARK'))

def _draw_scene(layout, ctx):
    p = ctx.scene.blender_toolkit
    box = layout.box(); col = box.column(align=True)
    col.operator("bt.purge_unused",              icon=I('TRASH'))
    col.operator("bt.remove_empty_collections",  icon=I('OUTLINER_COLLECTION'))
    col.operator("bt.remove_missing_uv",         icon=I('UV_DATA'))
    col.separator(factor=0.5)
    row = col.row(align=True)
    row.operator("bt.poly_count", icon=I('MESH_DATA'), text="Count Polys")
    col.label(text=p.poly_count_result, icon=I('INFO'))

# Populate the override registry with all original 13 sections.
# Key = section label exactly as it appears in _DEFAULT_SECTIONS.
_SECTION_DRAW_OVERRIDES = {
    "TRANSFORM":    _draw_transform,
    "ALIGN":        _draw_align,
    "OBJECT TOOLS": _draw_object_tools,
    "NAMING":       _draw_naming,
    "MATERIALS":    _draw_materials,
    "UV & CLEANUP": _draw_uv,
    "WIRE COLOUR":  _draw_wire,
    "GENERATORS":   _draw_generators,
    "LIGHTING":     _draw_lighting,
    "CAMERA TOOLS": _draw_camera,
    "SCENE CLEANUP":_draw_scene,
}

# Sections whose body is drawn by a simple fallback (just operator buttons).
# SELECTION and MESH TOOLS both fit this pattern — no sub-properties needed.
# Nothing to add here; they use the generic button loop in the main draw().


class BT_PT_Library(Panel):
    """
    The main N-panel tab for Blender Toolkit.
    This class is the "canvas" that Blender calls every time it redraws the
    3D Viewport sidebar. The draw() method reads layout_config.json on every
    call so any change made by the editor operators is reflected immediately.
    """
    bl_label       = "Blender Toolkit"
    bl_idname      = "BT_PT_Library"
    bl_space_type  = 'VIEW_3D'
    bl_region_type = 'UI'
    # bl_category is the tab name shown in the N-panel strip on the right.
    # This MUST be 'Blender Toolkit' — it is what separates us from BT Scatter.
    bl_category    = 'Blender Toolkit'

    def draw(self, ctx):
        layout = self.layout
        p      = ctx.scene.blender_toolkit
        dev    = _dev_mode()

        # ── DEVELOPER MODE TOGGLE (always visible at the top) ─────────────────
        # This single row lets you flip Developer Mode on/off without going
        # anywhere. When off it is a small unobtrusive row; when on the full
        # editor strip appears under every section below.
        top_row = layout.row(align=True)
        icon = I('TOOL_SETTINGS') if dev else I('PREFERENCES')
        top_row.prop(p, 'developer_mode', text="Dev Mode", icon=icon, toggle=True)
        if dev:
            top_row.operator("bt.layout_reset", text="", icon=I('LOOP_BACK'))

        # ── ADD SECTION (top, so it is never hidden below a long panel) ───────
        # This used to sit at the very bottom, which meant scrolling past every
        # section to reach it — easy to miss entirely.
        if dev:
            add_box = layout.box()
            add_row = add_box.row(align=True)
            add_row.prop(p, 'dev_new_label', text="", icon=I('GREASEPENCIL'))
            add_op = add_row.operator("bt.section_add", text="Add Section", icon=I('ADD'))
            add_op.label = p.dev_new_label or "NEW SECTION"

        layout.separator(factor=0.5)

        # ── READ THE CONFIG ───────────────────────────────────────────────────
        # _read_config() opens layout_config.json on every draw call.
        # This is intentional: it lets external tools edit the JSON directly
        # and see results on the next redraw without reloading the addon.
        # The JSON read is a fast local file operation; Blender caches the panel
        # redraw so it doesn't happen every frame, only on UI events.
        sections = _read_config()

        # ── DRAW EACH SECTION ─────────────────────────────────────────────────
        for idx, sec in enumerate(sections):
            label = sec.get('label', 'SECTION')
            icon  = sec.get('icon', 'DOT')
            style = sec.get('style', 'ROLLUP')
            ops   = sec.get('operators', [])

            # Build a stable toggle property key from the label.
            # e.g. "OBJECT TOOLS" → "show_object_tools"
            toggle_key = _label_to_key(label)

            # If BTProps doesn't have this toggle yet (custom section added
            # in Developer Mode), create it dynamically.
            if not hasattr(p, toggle_key) and toggle_key not in BTProps.__annotations__:
                try:
                    BTProps.__annotations__[toggle_key] = BoolProperty(default=False)
                    # Re-register so Blender knows about the new property.
                    bpy.utils.unregister_class(BTProps)
                    bpy.utils.register_class(BTProps)
                except Exception:
                    pass

            expanded = getattr(p, toggle_key, False) if hasattr(p, toggle_key) else False

            # ── DEVELOPER MODE EDITOR STRIP PER SECTION ───────────────────────
            if dev:
                editor_row = layout.row(align=True)
                # ▲ Move up
                up = editor_row.operator("bt.section_move_up",   text="", icon=I('TRIA_UP'))
                up.index = idx
                # ▼ Move down
                dn = editor_row.operator("bt.section_move_down", text="", icon=I('TRIA_DOWN'))
                dn.index = idx
                # Style cycle button — shows current style as text
                cy = editor_row.operator("bt.section_cycle_style", text=style)
                cy.index = idx
                # Label shown (not editable inline — use rename operator below)
                editor_row.label(text=label)
                # Rename button (opens a popup dialog for entering new name)
                # We use invoke_props_dialog via a small helper trick.
                rn = editor_row.operator("bt.section_rename_popup", text="", icon=I('GREASEPENCIL'))
                rn.index = idx
                rn.current_name = label
                # Delete button
                dl = editor_row.operator("bt.section_delete", text="", icon=I('TRASH'))
                dl.index = idx
                # Add Script button
                as_op = editor_row.operator("bt.section_add_script", text="", icon=I('FILEBROWSER'))
                as_op.section_index = idx

            # ── SECTION HEADER (the ▶/▼ toggle row or plain label) ────────────
            if style == 'BOX':
                # BOX style: always expanded, no toggle.
                hdr = layout.box()
                hdr.label(text=label, icon=I(icon))
                body_layout = hdr
                draw_body = True
            elif style == 'FLAT':
                # FLAT style: label row then buttons directly on the panel, no box.
                layout.label(text=label, icon=I(icon))
                body_layout = layout
                draw_body = True
            else:
                # ROLLUP style (default): collapsible header, body inside a box.
                draw_body = _section_header(layout, p, toggle_key, label, icon)
                body_layout = layout

            # ── SECTION BODY ──────────────────────────────────────────────────
            # Two completely different views depending on Dev Mode:
            #
            #   DEV MODE OFF → "display view". Sections with a hand-crafted
            #       draw function get their full rich UI (sliders, grids,
            #       sub-boxes). This is what an end user sees.
            #
            #   DEV MODE ON  → "edit view". EVERY section, including the rich
            #       ones, is drawn as a plain list of scripts with per-script
            #       controls. A rich draw function is Python, so its contents
            #       can't be reordered from the UI — swapping to the list view
            #       is what makes every script movable.
            #
            # Flipping Dev Mode off restores the pretty layout, now reordered.
            if draw_body:
                if dev:
                    col = body_layout.box().column(align=True)
                    if ops:
                        for oi, op_id in enumerate(ops):
                            row = col.row(align=True)
                            # ▲ / ▼ reorder this script within its section
                            mu = row.operator("bt.operator_move_up", text="", icon=I('TRIA_UP'))
                            mu.section_index = idx; mu.op_index = oi
                            md = row.operator("bt.operator_move_down", text="", icon=I('TRIA_DOWN'))
                            md.section_index = idx; md.op_index = oi

                            # Show the script name, trimmed of the "bt." prefix
                            # so the list stays readable in a narrow sidebar.
                            short = op_id[4:] if op_id.startswith("bt.") else op_id
                            row.label(text=short)

                            # → move this script into a different section
                            mv = row.operator("bt.section_move_operator_popup",
                                              text="", icon=I('FORWARD'))
                            mv.op_id = op_id; mv.from_index = idx

                            # X remove from this section (does not delete the file)
                            rm = row.operator("bt.section_remove_operator",
                                              text="", icon=I('X'))
                            rm.op_id = op_id; rm.section_index = idx
                    else:
                        col.label(text="(empty — use the folder icon to add a script)",
                                  icon=I('INFO'))
                else:
                    # ── DISPLAY VIEW ──────────────────────────────────────────
                    if label in _SECTION_DRAW_OVERRIDES:
                        _SECTION_DRAW_OVERRIDES[label](body_layout, ctx)
                    else:
                        col = (body_layout.box().column(align=True)
                               if style == 'ROLLUP' else body_layout.column(align=True))
                        if ops:
                            for op_id in ops:
                                # If a tool module failed to load its operator
                                # won't exist; show a warning rather than letting
                                # the whole panel raise and render blank.
                                try:
                                    col.operator(op_id)
                                except Exception:
                                    col.label(text=f"missing: {op_id}", icon=I('ERROR'))
                        else:
                            col.label(text="(empty)", icon=I('INFO'))

        # ── DEVELOPER MODE FOOTER ─────────────────────────────────────────────
        # Section count, as a quick sanity check that nothing has gone missing.
        if dev:
            layout.separator(factor=0.5)
            layout.label(text=f"{len(sections)} sections", icon=I('INFO'))

        # ── EXTENSION SECTIONS (THIRD-PARTY PLUG-INS) ────────────────────────
        # These come after the config-driven sections and are always shown.
        for label, draw_fn, ext_icon in BlenderToolkit.get_sections():
            toggle_key = "_ext_" + re.sub(r'\W+', '_', label.lower())
            if not hasattr(p, toggle_key):
                try:
                    BTProps.__annotations__[toggle_key] = BoolProperty(default=False)
                except Exception:
                    pass
            if _section_header(layout, p, toggle_key if hasattr(p, toggle_key) else 'show_scene',
                               label, ext_icon):
                sub_box = layout.box()
                draw_fn(sub_box, ctx)


# ─────────────────────────────────────────────────────────────────────────────
# POPUP RENAME OPERATOR
# Draws a small Blender modal dialog so the user can type a new section name.
# ─────────────────────────────────────────────────────────────────────────────
class BT_OT_SectionRenamePopup(Operator):
    """Open a small dialog to rename a section."""
    bl_idname = "bt.section_rename_popup"
    bl_label  = "Rename Section"
    bl_options = {'REGISTER', 'UNDO'}

    index:        IntProperty()              # type: ignore
    current_name: StringProperty()           # type: ignore
    new_name:     StringProperty(name="New Name")  # type: ignore

    def invoke(self, ctx, event):
        # Pre-fill the field with the current name so the user can edit it.
        self.new_name = self.current_name
        # invoke_props_dialog draws a small popup with OK/Cancel.
        return ctx.window_manager.invoke_props_dialog(self)

    def draw(self, ctx):
        self.layout.prop(self, 'new_name')

    def execute(self, ctx):
        sections = _read_config()
        i = self.index
        if i < 0 or i >= len(sections):
            return {'CANCELLED'}
        new = self.new_name.strip()
        if not new:
            self.report({'WARNING'}, "Name cannot be empty.")
            return {'CANCELLED'}
        sections[i]['label'] = new
        _write_config(sections)
        for area in ctx.screen.areas:
            if area.type == 'VIEW_3D':
                area.tag_redraw()
        return {'FINISHED'}


# ─────────────────────────────────────────────────────────────────────────────
# SECTION: REGISTRATION (THE ON / OFF POWER SWITCH)
# ─────────────────────────────────────────────────────────────────────────────
_CLASSES = [
    BTProps,
    BT_PT_Library,
    # Layout editor operators
    BT_OT_SectionMoveUp,
    BT_OT_SectionMoveDown,
    BT_OT_SectionRename,
    BT_OT_SectionRenamePopup,
    BT_OT_SectionDelete,
    BT_OT_SectionAdd,
    BT_OT_SectionCycleStyle,
    BT_OT_SectionSetIcon,
    BT_OT_AddScript,
    BT_OT_MoveOperator,
    BT_OT_OperatorMoveUp,
    BT_OT_OperatorMoveDown,
    BT_OT_MoveOperatorPopup,
    BT_OT_RemoveOperatorFromSection,
    BT_OT_ResetLayout,
]

_TOOL_MODULES = []

def _load_tools():
    """
    Auto-discover and load operator modules from subfolders next to __init__.py.

    When the addon is installed as a .zip (Blender 4.2+ extension), every tool
    subfolder (transform/, materials/, etc.) ends up in the SAME directory as
    this __init__.py — NOT inside an extra tools/ subfolder.

    os.walk() recursively visits every sub-directory, so new tool folders
    added later are found automatically without changing this function.
    """
    global _TOOL_MODULES
    _TOOL_MODULES.clear()

    current_dir = os.path.dirname(__file__)
    # Skip the custom_scripts subfolder during the first-pass discovery because
    # those scripts get explicitly loaded by BT_OT_AddScript.
    skip_dirs = {'custom_scripts', '__pycache__'}

    for root, dirs, files in os.walk(current_dir):
        # Prune directories we don't want to descend into.
        dirs[:] = [d for d in dirs if d not in skip_dirs and not d.startswith('.')]

        # ── GUARD: never import .py files sitting in the addon ROOT ──────────
        # Only __init__.py belongs at the root. Any OTHER root-level .py is a
        # leftover/backup copy of the addon itself (e.g. blender_toolkit_core.py).
        # Importing one is actively harmful: it re-registers a rival BTProps,
        # clobbers Scene.blender_toolkit (losing developer_mode), and registers a
        # duplicate BT_PT_Library panel that silently replaces the real one.
        # Real tools always live in subfolders (transform/, materials/, ...),
        # so skipping the root costs us nothing.
        if os.path.abspath(root) == os.path.abspath(current_dir):
            continue

        for file in files:
            if file.endswith(".py") and file != "__init__.py":
                rel_path   = os.path.relpath(os.path.join(root, file), current_dir)
                module_name = rel_path.replace(os.sep, ".")[:-3]  # strip ".py"

                if module_name in sys.modules:
                    mod = importlib.reload(sys.modules[module_name])
                else:
                    try:
                        mod = importlib.import_module(
                            f".{module_name}",
                            package=__package__ or "blender_toolkit"
                        )
                    except ImportError:
                        try:
                            mod = importlib.import_module(module_name)
                        except Exception:
                            continue  # silently skip unimportable files
                _TOOL_MODULES.append(mod)

    # Also load any scripts in custom_scripts/ that were added via Add Script.
    custom_dir = os.path.join(current_dir, "custom_scripts")
    if os.path.isdir(custom_dir):
        for fname in os.listdir(custom_dir):
            if fname.endswith(".py"):
                fpath = os.path.join(custom_dir, fname)
                spec  = importlib.util.spec_from_file_location(f"bt_custom.{fname[:-3]}", fpath)
                mod   = importlib.util.module_from_spec(spec)
                try:
                    spec.loader.exec_module(mod)
                    _TOOL_MODULES.append(mod)
                except Exception as e:
                    print(f"[Blender Toolkit] Could not load custom script {fname}: {e}")


def register():
    """
    register() is called by Blender when the addon is enabled.
    Everything we want Blender to know about must be registered here:
    property groups, panels, operators.
    """
    # 1. Make sure layout_config.json exists (creates it with defaults if missing).
    _ensure_config()

    # 2. Register every class in _CLASSES with Blender.
    for cls in _CLASSES:
        bpy.utils.register_class(cls)

    # 3. Attach our property group to every Scene so blender_toolkit is always accessible.
    bpy.types.Scene.blender_toolkit = bpy.props.PointerProperty(type=BTProps)

    # 4. Auto-discover and register all tool operator modules.
    _load_tools()
    for mod in _TOOL_MODULES:
        if hasattr(mod, 'register'):
            try:
                mod.register()
            except Exception as e:
                print(f"[Blender Toolkit] Could not register module: {e}")


def unregister():
    """
    unregister() is called when the addon is disabled.
    We undo everything register() did, in reverse order.
    """
    # Unregister tool modules first (they depend on our classes).
    for mod in reversed(_TOOL_MODULES):
        if hasattr(mod, 'unregister'):
            try:
                mod.unregister()
            except Exception:
                pass
    _TOOL_MODULES.clear()

    # Remove the scene property.
    del bpy.types.Scene.blender_toolkit

    # Unregister our own classes in reverse so dependencies are removed last.
    for cls in reversed(_CLASSES):
        try:
            bpy.utils.unregister_class(cls)
        except Exception:
            pass
