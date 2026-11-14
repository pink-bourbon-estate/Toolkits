Blender Toolkit — Contents & Component Reference

This document provides a complete directory of all tools and operators available in the modularised Blender Toolkit.
Core Components

    blender_toolkit_core.py: The main shell environment containing the UI panel definition, global property dashboard, shared helper utilities, and the dynamic script loader.

    tool_template.py: A blank boilerplate template for developers to easily script and register new operators into the library framework.

Transform Tools

    random_transform.py: Applies random position, rotation, and scale variations to selected objects within customisable parameters.

    re_randomise.py: Rerolls the random seed and re-applies the previous random transform values.

    drop_to_floor.py: Moves selected objects downward so their lowest bounding box point rests exactly at Z=0.

    drop_to_surface.py: Uses raycasting to drop selected objects directly onto the surface of the active target object.

    apply_all_transforms.py: Freezes and applies all current location, rotation, and scale data.

    quick_rotate.py: Rotates objects by precise, preset standard angles (90°, 45°, 30°, 10°) on user-chosen axes.

    set_pivot_bottom.py: Moves the object's origin pivot point to the bottom-centre of its bounding box.

Align Tools

    align_multiple.py: Lines up multiple objects sequentially along an axis, automatically factoring in their bounding box widths to prevent clipping.

    arrange_in_grid.py: Arranges selected objects into an ordered N×M grid pattern on the floor plane.

    align_to_object.py: Snaps the position and rotation of selected objects to perfectly match the active object.

    place_along_spline.py: Distributes selected objects evenly along the length of a chosen curve spline.

    three_point_align.py: Calculates a surface normal from three coordinate points and rotates the active object to lay flat against that plane.

    copy_transform.py: Copies location, rotation, and scale values from the active object directly to all other selections.

Selection Tools

    select_by_material.py: Scans the scene and highlights all objects sharing the active object's material.

    select_instances.py: Selects all linked duplicates that share the exact same underlying mesh data.

    select_no_material.py: Highlights bare, unpainted meshes that lack any assigned material slots.

    select_by_polycount.py: Filters and selects objects that fall within a specified polygon count threshold.

    select_non_smoothed.py: Enters Edit Mode and selects all flat-shaded faces to assist with smoothing corrections.

    select_unparented.py: Selects all orphan objects that do not belong to a parent hierarchy.

    random_select.py: Randomly drops a specified percentage of objects from the current selection to thin out crowds.

Object Tools

    make_unique.py: Severs linked data so each selected object receives its own independent mesh copy.

    replace_object.py: Swaps the mesh geometry of all selected objects with the active object's geometry.

    make_instances.py: Replaces selected objects' meshes with the active object's mesh as memory-efficient linked instances.

    explode_to_objects.py: Separates disconnected, loose mesh parts into entirely individual objects.

    attach_selection.py: Joins all selected objects into the single active object.

    collapse_by_material.py: Groups and joins multiple separate objects together based on their shared paint materials.

    object_lock.py: Locks or unlocks all transform values to prevent accidental movement in complex scenes.

    subdivision_control.py: Batch-applies or modifies Subdivision Surface viewport and render levels simultaneously.

    modifier_utility.py: Batch-adds a specified modifier (Bevel, Weld, Solidify, etc.) to all selected objects safely.

    place_at_vertices.py: Spawns an instance of the active object exactly at every corner vertex of a target mesh.

    scale_by_distance.py: Dynamically scales objects up or down based on their physical distance from the active object.

    quick_measure.py: Calculates the precise bounding box dimensions (X, Y, Z) and prints them to the interface.

    mark_as_asset.py: Automatically tags selected items for inclusion in the Blender Asset Browser.

    find_unique_objects.py: Selects objects that possess entirely unique, un-cloned mesh data.

    attach_splines.py: Joins multiple separate curve paths into a single combined curve object.

    create_collection_from_selection.py: Moves all selected items into a newly generated Outliner collection folder.

    create_line_at_base.py: Draws a 2D rectangular footprint on the ground directly beneath objects.

    place_name_below.py: Generates floating 3D text labels underneath objects displaying their exact names.

Mesh Tools

    random_face_remove.py: Randomly deletes a user-defined percentage of faces to create procedural damage or gaps.

    random_face_select.py: Enters Edit Mode and highlights a random scattering of faces for variation texturing.

    random_extrude.py: Selects random faces and pulls them outward at varying heights to generate structural greeble.

    weld_vertices.py: Fuses overlapping or tightly packed corner dots based on a millimetre distance threshold.

Naming Tools

    rename_prefix.py: Attaches a custom text string to the very beginning of selected object names.

    rename_suffix.py: Attaches a custom text string to the very end of selected object names.

    rename_find_replace.py: Searches object names for a specific word and bulk-replaces it with another.

    rename_number.py: Renames objects sequentially using a base string and zero-padded numbering.

Materials

    random_material.py: Randomly assigns existing user materials to selected objects for quick look-development.

    random_map_phase.py: Randomises the X and Y texture mapping offsets so repeating patterns look unique per object.

    scale_procedural_maps.py: Multiplies, sets, or adds to the scale of Mapping nodes across all selected materials.

    isolate_by_material.py: Hides all objects in the viewport EXCEPT those sharing the active material.

    remove_materials.py: Strips all material slots from selected models entirely, returning them to bare geometry.

    merge_duplicate_materials.py: Cleans up imported materials with .001/.002 extensions and consolidates them.

    material_to_wire_color.py: Adapts viewport wireframe colours to match the base diffuse paint of assigned materials.

    apply_default_material.py: Scans for unpainted meshes and coats them with a default grey Principled BSDF material.

    swap_materials.py: Neatly swaps the assigned materials between exactly two selected models.

    replace_material.py: Globally searches the scene for one specific material and replaces all instances of it with another.

    uvw_randomiser.py: Intelligently jitters Mapping node offsets, rotations, and scales for organic variation.

    set_viewport_opacity.py: Overlays a ghosted, semi-transparent view state on objects in the 3D viewport.

UV & Cleanup

    uvw_box_map.py: Applies an automated triplanar box-map UV projection using a generated Geometry Nodes modifier.

    select_missing_uv.py: Locates and selects any mesh in the scene lacking a UV coordinate layer.

    uv_transfer.py: Clones UV mapping data from the active object to selected models via a baked Data Transfer modifier.

    apply_uv_grid.py: Generates and applies a 1024x1024 checkerboard texture to visually test for UV map stretching.

    face_uv_transform.py: Automatically splits the workspace to open the UV Editor, enters Edit Mode, and highlights all UVs.

    reload_all_images.py: Forces Blender to actively refresh and reload all image texture paths from the hard drive.

Wire Colour

    wire_color_random.py: Colour-codes viewport lines randomly, sorting by either individual object, collection folder, or material type.

Generators

    circle_array.py: Leverages Geometry Nodes to instantly arrange cloned instances in a customisable circular array.

    floor_generator.py: Procedurally generates a 3D tiled floor with adjustable tile dimensions, grout width, and thickness.

    simple_scatter.py: Sprinkles object instances naturally across a target surface with varied scaling and Z-axis rotation.

    roof_generator.py: Extrudes any flat polygon building footprint upwards and collapses the top to form a pitched roof.

Lighting

    three_point_light.py: Instantly spawns a classic Key, Fill, and Back lighting rig for professional subject illumination.

    studio_light.py: Generates a clean Top, Front, and Side lighting setup ideal for product showcases.

    sun_light.py: Spawns a directional sun lamp angled at 45° for standard outdoor daylight simulation.

    delete_all_lights.py: Selects and permanently removes every light source across the entire scene.

    randomise_light_hue.py: Overrides selected lamps with a random, highly saturated neon colour tint.

    randomise_light_energy.py: Randomises the brightness wattage of selected lamps within a constrained minimum and maximum range.

    toggle_light_visibility.py: Master switch to toggle viewport and render visibility for all lights simultaneously.

    randomise_lights_on_off.py: Simulates random lighting patterns by flipping a 50/50 virtual coin to turn individual lights on or off.

Camera Tools

    add_camera_rig.py: Deploys a new camera heavily constrained to physically track a movable target marker.

    focus_on_selection.py: Calculates the mathematical centre of selected models and precisely frames the camera to face it.

    match_camera_res.py: One-click setup configuring render output to common broadcast dimensions (1080p HD, 1440p QHD, 4K).

Scene Cleanup

    purge_unused.py: Empties the digital recycle bin, recursively deleting all orphaned and unused background data.

    remove_empty_collections.py: Audits the Outliner tree and deletes any collection folders that lack objects or sub-folders.

    poly_count.py: Calculates and prints the total geometric polygon count for both the selection and the entire file.

    remove_missing_uv.py: Functions as an auditor to select objects lacking UVs, ensuring they are queued for cleanup or deletion.