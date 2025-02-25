# 0.36.1

Minor release with fixes for some annoying bugs.

## Fixed

- Fixed frustum culling issues for meshes.
- Correctly mark inheritable properties as modified when animating them.
- Show selected item in the asset browser when clearing selection text.
- Fixed incorrect width of the node removal dialog.
- Fixed physics desynchronization issue.
- Fixed docking manager layout saving and loading.
- Improved project name validation (in both `fyrox-template` and the project manager).
- Fixed example shader in the documentation.
- Added names for floating panels in the editor—fixes incorrect layout after loading.

# 0.36

This version unifies versions of sub-crates - now all sub-crates have the same version. This makes it easier to migrate
to future versions.

## Added

- Tile maps.
- UI styling support.
- Project manager to manage multiple Fyrox projects at once.
- Dropdown list docs.
- Implemented PartialEq for sprite sheet animation entities.
- Property editor for SurfaceDataResource.
- Surface data viewer for surface resource.
- `BaseSceneGraph::remove_nodes`.
- Ability to add/remove interaction modes dynamically.
- Shape editing for colliders.
- Shader for sprite-based gizmos (allows to draw sprite-based gizmos on top of everything else).
- `math::get_arbitrary_line_perpendicular`.
- Added ability to specify font-+its size for value indicator in `ScrollBar`.
- Ability to specify font size when building a button.
- Added ability to specify font and font size when creating window title.
- Added surface resource loader.
- Built-in surfaces.
- Added a configurable throttle frame interval for `Executor`.
- Added sanity check for brush operations to protect the editor from being overloaded by huge brushes.
- Messages for `Grid` widget - ability to change rows/columns/draw border/border thickness.
- `Material::texture` helper method.
- `Color::repeat_opaque` method.
- `DrawingContext::push_grid` method.
- `save + save_back` methods for resource.
- Added "refresh" button for asset browser.
- `ResourceDataRef::as_loaded_ref/mut` methods.
- Ability to open assets using double click.
- Multi-selection support for `ListVIew` widget.
- `impl PartialEq for Ray`.
- Add an ability to rotate the editor camera using scene gizmo.
- `impl From<&String> for ImmutableString`.
- Improved material api - `Material::set_property` is now much less verbose.
- Better support for fbx materials from 3DS max.
- Validation for 2d colliders.
- Added folders into asset browser.
- Ability to cut holes in terrain.
- Experimental occlusion culling for light sources.
- `read_pixels_of_type` to get typed pixels instead of raw bytes.
- Added `R32UI` texture format.
- `get_image` for gpu texture.
- Pixel buffer for async framebuffer reads.
- Include cache sizes in rendering statistics (helps in catching uncontrollable GPU memory usage growth).
- Ability to duplicate resources in asset browser.
- Added visible distance for particle systems.
    - Automatically excludes distant particle systems from rendering to improve performance.
    - Can be tweaked on a per-system basis.
- Ability to enable/disable scissor test from custom shaders.
- Ability to specify depth func in custom shaders.
- Added uniform buffers.
- Added `UniformBufferCache` for easier handling of multiple UBOs.
- Added bind groups + mandatory texture binding via render resources.
- Ability to fetch graphics server capabilities.
- Experimental `UniformMemoryAllocator`.
- Frustum culling for light sources.
- Support a saving/restoring the maximized flag of the editor's window.
- Ability to save all opened scenes at once + hotkeys.
- `AxisAlignedBoundingBox::project` method.
- `post_update` callback for `Plugin`.
- Editor plugins container - adds some useful methods for plugin search.
- More dockable windows.
- Ability to copy/paste selection in the curve editor widget.
- Added a configurable limit for message log to prevent excessive bloat.
- Configurable coordinate system for particle systems - allows selecting a coordinate system for generated particles—
  local or world.
- Lighting support for particle systems.
- `ModelResource::instantiate_and_attach` method.
- Ability to add keys on multiple curves at once.
- Hotkey for `zoom to fit` for curve editor widget.
- Useful macros for early return statements. While let-else exists, it still takes more lines of code than it should.
  these macros are much more compact and easier to read.
- `BaseControl::self_size` method.
- Editor ui statistics plugin. Allows tracking the total amount of widget used by the editor, which is useful to find if
  there are "dangling" widgets.
- `DockingManagerLayoutDescriptor::has_window` method.
- Print the total number of drawing commands of ui for the current frame.
- `remove_on_close` flag for `Window` widget.
- Ability to apply custom sorting for children widgets of a widget.
- Ability to sort menu items.
- Track processed ui messages in the editor - helps to find message queue overload.
- `has_component` helper methods.
- `StyleResource` resource type.
- Configurable routing strategy for ui messages.
- Helper methods for easier setting window icon.
- Add a `Zed` editor option into editor settings.
- Added configurable delay for tooltips.
    - Prevents tooltips from popping up instantly on mouse hover, instead there's a configurable (0.55 s by default)
      delay.
    - Removes annoying tooltip popping when moving mouse.
- Added more texture settings - base level, max level, min lod, max lod, lod bias.
- Added home/desktop directories shortcut buttons for file browser widget.
- Ability to focus the current path in the file browser widgets.
- Ability to specify graphics server constructor.
    - Essentially gives an ability to change graphics servers at creation/runtime stages.
    - By default still uses OpenGL graphics server.
- Added kerning support for fonts.
- `BuildContext::send_message` method.
- Added project manager CI.
- Backward compatibility for deserialization of `Untyped->Typed` resource.
- Ability to specify usage for element buffer.
- `info! + warn! + err!` log macros.
- Documentation improvements.
- `Downcast` trait to remove code bloat.
- Added tooltip for shader field in the material editor.
- Toggle button widget.
- Added tags for reflection.
- `WidgetBuilder::with_uniform_margin(..)`.
- Shortcuts for groups in editor settings: allows quickly jumping to a particular settings group.
- Searching functionality for editor settings.
- `impl TypeUuidProvider for Rect<T>`.
- Added property editors for `Option<Rect<T>>`.
- Nine patch widget improvements.
    - Added ability to specify a texture region for atlas support.
    - Remove explicit uv coordinates and calculate them on the fly.
    - Ability to disable drawing of the center region of the nine-patch widget.
    - Configurable tiling mode for nine-patch widget.
    - Easier editing of texture slice using new texture slice editor.
- Thumb widget for draggable things.
- Messages to change vertical and horizontal scrolling of ScrollViewer widget.

## Changed

- Included project license in every source file.
- Reset scene node transform to identity when making it root.
- Take z index into account when linking widgets.
- Split fyrox-template into lib + cli.
- Ability to specify project root dir for template-core.
- Optional app arguments. Prevents crash when trying to parse program arguments.
- Change key bindings to make more intuitive up/down motion.
- Replaced `SurfaceSharedData` into `Resource<SurfaceData>`
    - Surface shared data was essentially a resource of some sort anyway.
    - Allows saving meshes as resources externally.
    - Allows using standard resource pipeline for surface data.
- Simplified camera picking API in the editor.
- Improved terrain brush system.
- Print surface resource kind in the property editor.
- Fixed new object placement.
    - Children objects will stay at (0,0,0).
    - When creating via "Create" menu, a new object will be located in front of the camera.
    - When creating a parent object whose parent is root, it will also be located in front of the camera.
- Ability to specify name column width of inspector widget.
- Save camera projection mode in editor settings.
- Refactored editor camera controller - allows dragging the camera using mmb in 2d mode.
- Sort items of built-in resources.
- Remove native collider when its shape cannot be created.
- Hijack control over animations from animation container in ABSM - now ABSM itself updates the animations it uses,
  and only those that are currently used either by a state or states of active transition.
- Extract the rendering framework into a separate crate.
- Make fbx elements of mesh geometry optional.
    - Prints a warning message and continues reading.
    - This is needed to be able to load "malformed" fbx, that has no mesh geometry, such as animation-only fbx.
- Enable resource hot reloading by default in executor.
- Move `rotateVec2` to shared shader functions.
- Store initial data and file extension (if any) of built-in resources.
- Moved opengl initialization to the rendering framework.
- Use uniform buffer for bone matrices instead of texture matrix storage.
- Use uniform buffer to pass object instance data to shaders.
- Moved camera properties into its own uniform block.
- Switched to uniform buffers across the renderer.
- Pass material properties using uniform buffers.
    - Automatically generate uniform buffer description for material properties.
    - Automatically define uniforms for samplers.
    - No more need to manually define material properties in shaders, just use `properies.your_property_name`.
- Isolated opengl-specific code of gpu program into its own module.
- Use uniform memory allocator to speed up uniform data upload to gpu.
    - Splits rendering of render bundles in two steps: uniform data collection + upload and the actual rendering.
    - More efficient use of memory by using all available space in uniform buffers (prevents having uniform.
      buffers with just 200–300 bytes of memory, where the actual memory block on gpu is 4 kb).
    - It significantly reduces the number of individual data transfers and gapi calls in general.
    - Improves performance by 12–15%.
- Removed redundant buffer binding/unbinding - saves some time on api calls (especially in WebGL, where everything is
  proxied through JS).
- Pass sceneDepth texture to shaders explicitly.
- Use explicit binding for textures. Prevents dozens of `glUniform1i` calls when drawing stuff, thus improving
  performance by 5–10% (more on WebAssembly, where each gl call is passed through JS).
- Refactored shader structure to include resource bindings.
    - Makes shader structure more rigid and removes implicit built-in variables.
    - Makes binding points of resources explicit.
- Turned `Matrix2Editor` into generic-over-size `MatrixEditor`.
- Use immutable string in shader property name.
- Reworked materials.
    - Material now stores only changed shader properties.
    - Move validation from set_property/bind to the renderer where it simply prints an error message to the log
      if something's wrong.
    - Removed fallback value from texture resource binding, it makes no sense to duplicate this info since the correct
      one is stored in the shader anyway.
    - Removed `default` property from texture definition in shaders.
- Collect light info when constructing a render bundle. Removes redundant loop over scene graph nodes.
- Refactor hot reload to allow custom dynamic plugins besides dylib-based.
- Improved gpu texture api.
- Perform checked borrow in node message processing to prevent crashes. Crash could happen if a node is already deleted,
  but its message was still in the queue.
- Replaced component querying from nodes with `ComponentProvider` trait.
- Turned editor inspector into a plugin.
- Cloning physics when cloning Graph to persist Scene settings when saving Scene from the editor.
- TabControl improvements..
- Changed `traverse_iter` to return a pair of handle and ref - much more convenient when there's a need to handle a
  handle with a reference at the same time, no need to do re-borrow which is double work anyway.
- Added `AnimationResource` which decoupled animation tracks data into a shared resource.
    - Significantly reduces memory consumption when cloning animations, since it does not need to clone the tracks
      anymore.
    - Animation resource can be shared across multiple animations using the same tracks.
    - Significantly speeds up the instantiation of animation player scene node.
    - Backward compatibility is preserved.
- Focus search bar's text box when focusing toolbar itself - toolbar focus makes no sense anyway, because it does not
  interact with keyboard, but text box does.
- Node selector usability improvements.
    - Focus search bar on open.
    - Ability to confirm selection by enter key.
    - Bring the first selected item into view on open.
    - Added tab navigation.
- Lazy z-index sorting instead of on-demand.
- Exclude samples buffer from a list of animatable properties.
- Improved property selector.
    - Focus search bar on opening.
    - Tab navigation.
    - Highlight selected properties on rebinding.
    - Ability to confirm selection by hitting the enter key.
- Detached material-related parts of the editor into its own plugin - material editor is now non-existent by default and
  created only when needed, which saves memory (both ram and vram) and cpu/gpu time.
- Detached ragdoll wizard into a separate plugin.
- Move the settings window into a separate plugin.
- Move the animation editor into its own plugin.
- Improved editor plugins api.
- Create animation editor on editor start if animation editor was docked before.
- Move the absm editor to a separate plugin.
- Create save file selector for prefabs on demand.
- Move the curve editor window into its own plugin.
- Move the path fixer into a plugin.
- Use builtin surfaces for meshes created in the editor.
- Migrated to latest `tinyaudio`.
- Removed hardcoded ui widgets constructors. It replaced with user-defined constructors via `ConstructorProvider` trait.
- Sort menu items in alphabetical order in creation menus.
- Replaced hardcoded ui style variables with configurable styles.
- Make tooltips invisible for hit test.
- Move the log panel to `fyrox-ui`.
- Keep the editor running until the active popup is fully shown.
- Change the default path of file browser to `./`.
- Disable log file by default. The log file could be undesirable in some cases, and now it is off by default and can be
  enabled by `Log::set_file_name/set_file` in `fn main`.
- Explicit api to change the log file.
- Replaced proprietary Arial font with Roboto in the editor.
- Do not precompile built-in shaders on engine start.
    - It is faster to compile them on-demand.
    - On WebAssembly such compilation could take 10–15 seconds.
- Detached texture-related code to separate crate. It allows attaching it to `fyrox-ui` to use textures directly without
  using hacky `UntypedResource`.
- Use TextureResource directly in ui code where possible - removes redundant juggling with untyped↔typed conversions.
- Force `Image` widget to use texture size on measurement stage - removes "surprising effect" with collapsed image, if
  width/height is not set explicitly.
- Audio initialization errors non-fatal now. It allows running the engine in environments without proper audio output
  support.
- Print editor version in the window title.
- Print editor version in the log on start.
- Replace the hardcoded version of the engine with the one from Cargo.toml. This is a semi-reliable solution, but much
  better than having the hardcoded version.
- Close projection (2d/3d) selector on selection.
- Use toggle button for `track selection` in the world viewer.
- Put the search bar of the world viewer on the same row with other buttons.
- Moved `load_image` to `fyrox-ui` utils.

## Fixed

- Fixed blurry fonts.
- Significantly improved editor performance.
- Improved joint stability after migration to the latest Rapier physics.
- Use z index from the respective message.
- Fixed crash when trying to change window title using the respective message.
- Fixed procedural meshes serialization.
- Fixed inspector syncing when replacing the selected object with another type.
- Fixing Rect tests in fyrox-math.
- `transmute_vec_as_bytes` soundness fix.
- Fixed crash when trying to drag'n'drop non-texture in texture field.
- Refresh asset browser after asset deletion.
- Better validation for colliders.
- Support for chained texture nodes in fbx - fixes normal map import on FBX files made in latest 3ds max/Maya/etc.
- Watch for changes in the current directory and refresh asset browser content.
- Fixed potential crash when cloning ui nodes.
- Fixed tool installation check in project exporter.
    - Do not try to install already installed tools.
    - Prevents accessing the network when there's no actual need.
- Fixed redundant texture binding if it is already bound to pipeline.
- Discard scaling from rotation matrix before passing it to bounding shape - fixes clipping issues of light sources.
- Do not skip light scatter rendering even if there's no fragments lit. It fixes flashing of light scattering.
- Fixed shadow map lod selection condition.
- Speed up access to animation curve data.
- Use `ImmutableString` in `ValueBinding` to make it smaller results in faster copying (32 bytes vs. 16 bytes).
- Prevent render targets from registering multiple times in texture cache.
- Improved performance of render data collection.
- Drop inherited `RUSTFLAGS` for project exporter child processes.
- Fixed crash when rendering large bundles.
- Do not reallocate gpu buffer if there's enough space for data already.
- Ignore buffer write commands when the data is empty.
- Set glsl es precision to `highp`.
- Fixed an invalid editor window size on second startup at the hidpi display.
- Ensure vector images have a set size.
- Fix crash on macOS in notify crate when the path is set first time.
- Reduced code bloat by isolating fallback textures into their own struct.
- Fix wasm tests fails due to using of the deprecated PanicInfo.
- Discard scaling part when calculating light source bounding box.
- Excluded some non-animatable properties from property selector.
- Detached perf of hierarchical properties propagation from graph size.
    - Graph now updates hierarchical properties only for ones that actually changed.
    - Significantly improves performance in static scenes.
- Prevent redundant global transform update for 2d rigid bodies.
- Fixed "teleportation" bug (when a scene node was located at world's origin for one frame and then teleports back
  where it should be).
- Prevent potential nan in `vector_to_quat`.
- Fixed convergence in reverb sound effect.
- Fixed root motion jitter on looping animations - loop boundaries were handled incorrectly, thus leading to error
  accumulation that led to annoying jitter after some iterations.
- Fixed visible borders around point lights.
- Reduced code bloat in the engine internals.
- Fixed transform syncing of colliders.
- Fixed `Inspector` widget syncing issues.
- Fixed crash when deleting multiple animation tracks at once.
- Fix for UI layout, including Grid and Text.
- Fixed crash when trying to fetch intersections from a deleted collider.
- Fixed crash when trying to collect animation events without a root state.
- Fixed crash when using `accurate_world_bounding_box` on some meshes - it would crash if a mesh has no position/bone
  indices/bone weights attributes in its vertex buffer.
- Fixed name of ragdoll joint generated by ragdoll wizard.
- Improved overall editor performance and ui nodes linking in particular.
- Prevent redundant syncing of the editor settings window - saves ~10% of time.
- Prevent the editor from loading the same texture over and over again.
- Fixed keyboard navigation for tree root - fixes annoying issue which causes keyboard focus to stick at tree root.
- Fixed camera preview panel size.
- Fixed deletion of some widgets.
- Fixed arrow visibility of menu item when dynamically changing its items.
- Fixed `MenuItem` performance issues.
- Fixed syncing of material editor shader field.
- Added `fyrox-build-tools` crate which essentially contains build tools from the editor.
- Fixed incorrect texture bindings invalidation - caused weird bug with incorrect textures applied to some objects (very
  noticeable in the ui after resizing the window).
- Use mip mapping for icons in the editor to smooth icons in the editor.
- Fixed background color "leaking" during `Border` widget rendering.
- Fixed syncing bug of R coordinates for volume textures.
- Fixed transform order in visual transform calculation.
- Fixed incorrect memory alignment when deserializing `BinaryBlob`.
- Fixed crash when using nine-patch widget without a texture.
- Fixed crash when dropping non-texture resource on texture field.

## Removed

- Removed redundant data hash calculation in textures.
- Removed redundant field from render data bundle - `is_skinned` flag makes no sense, because it could be derived
  from bone matrix count anyway and it is always defined on the per-instance basis, not per-bundle.
- Remove redundant decal layer index from mesh/terrain/render the data bundle. These are residuals from before
  custom material era, it makes no sense now since decal layer index is defined in materials and these fields simply had
  no effect.
- Removed depth offset.
    - It could be done with shaders.
    - Removed because it adds unnecessary projection matrix juggling for each rendered instance.
- Removed implicit blend shapes storage passing to material shaders - it is now controlled directly from `Mesh` node,
  and it creates temp material to pass blend shape storage explicitly.
- Removed `PersistentIdentifier` and `MatrixStorageCache`.
- Removed `cast_shadows` property from `BaseLight` - this property at some point started to be redundant, because `Base`
  already has such property and the one in `BaseLight` must be deleted to prevent confusion.
- Remove an incorrect error message in the animation editor.
- Removed `Node::query_component_ref/mut`.
    - It duplicates existing functionality.
    - Replaced with `SceneGraphNode::component_ref/mut`.
- Removed redundant boxing when applying animation values - makes animation of arbitrary numeric properies significantly
  faster.

# 0.35

- Version skipped, because of sub-crates version unification. See 0.36 change log for more info.

# 0.34.1 Engine + 0.21.1 Editor

- Fixed crash when trying to create parent for root in the editor
- Dispatch script messages after everything is initialized and updated
- Prevent selection type name from disappearing in the inspector
- Fixed potential crash when undoing asset instantiation
- Fixed rendering issues in 2d projection mode in the editor
- Update visual transform of a widget when render transform changes

# 0.34 Engine + 0.21.0 Editor

## Added

- Code hot reloading for plugins.
- Ability to have multiple scripts on one scene node.
- Static and dynamic batching for meshes.
- Project exporter for automated deployment.
- Configurable build profiles for the editor.
- Ability to have multiple user interface instances.
- GLTF support (available via `gltf` feature).
- Keyboard navigation support in the UI.
- Preview generation for assets in the asset browser.
- Grid for the scene preview.
- `fyrox-template` improvements to generate projects, that supports code hot reloading.
- `AnimationPlayer` + `AnimationBlendingStateMachine` widgets.
- UI prefabs with ability to instantiate them.
- `Pool::try_get_component_of_type` + the same for `MultiBorrowContext`.
- `NodeTrait::on_unlink` method.
- Implemented `ComponentProvider` trait for `Node`.
- `MultiBorrowContext::get/get_mut` methods.
- Ability to remove objects from multiborrow context.
- `newtype_reflect` delegating macro.
- `SceneGraph::change_hierarchy_root` method.
- Ability to change UI scene root.
- Property inheritance for UI widgets.
- Ability to instantiate UI prefabs by dropping prefab into world viewer/scene previewer.
- Ability to open scripts from the editor's inspector.
- `Control::post_draw` method.
- Ability to reorder children of a scene node.
- `SceneGraph::relative_position` + `SceneGraphNode::child_position` methods.
- Ability to reorder nodes/widgets in the world viewer.
- Added more icons for widgets.
- Added support for UI animations in the animation editor.
- Configurable UI update switches.
- Ability to edit ui absm nodes in the absm editor.
- `AbsmEventProvider` widget.
- Ability to enable msaa when initializing graphics context.
- Ability to change corner radius in `Border` widget.
- Ability to draw rectangles with rounded corners in UI drawing context.
- Added layout rounding for `fyrox-ui` which significantly reduced blurring.
- Added support for embedded textures in FBX.
- `Selector` widget.
- Added project dir and scenes to open as cli args to editor.
- `utils::make_cross_primitive` helper method.
- Ability to draw wire circle in the UI drawing context.
- Ability to draw WireCircle primitives in VectorImage widget.
- More tests.
- Vertex buffer API improvements.
- Rendering statistics window for the editor.
- Added shape casting in physics.
- Ability to unassign textures in material editor.
- Allow to set negative playback speed for animations in animation editor.
- `Scene::clone_one_to_one` shortcut for easier scene cloning.
- `fyrox-dylib` crate to be able to link the engine dynamically.
- Ability to link the engine dynamically to the editor.
- Added property editor for untyped textures.
- Added `Plugin::on_loaded` method.
- `NetListener::local_address` method.
- `Model::new` method.
- Ability to disable space optimization of `InheritableVariable` on serialization.
- Added CI for project template for all supported platforms.
- Added diagnostics for degenerated triangles when calculating tangents.
- `Pool::first_ref/first_mut` methods.
- Added release keystore for android project templates.
- Collect rendering statistics on per-scene basis.
- `transmute_slice` helper function.
- Ability to read GPU texture data.
- Experimental histogram-based auto-exposure for HDR (disabled by default).
- Short-path angle interpolation mode for `Curve` - `Curve::angle_at`.
- Property editor for `RcUiNodeHandle` type.
- Adaptive scroll bar thumb.
- Ability to fetch current task pool from resource manager.
- Async icon generation for assets in the asset browser.
- Case-insensitive string comparison helper method `fyrox::core::cmp_strings_case_insensitive`.
- Major performance improvement for searching in the asset browser.
- Configurable interpolation mode for animations.
- Ability to close popups using `Esc` key.
- Added diagnostics for docking manager layout, that warns if a window has empty name.
- Keyboard navigation for tree widget.
- Ability to close windows by `Esc` key.
- Focus opened window automatically.
- Keyboard navigation for `Menu` widget.
- Added `ImmutableString` editor.
- Docs for inspector module.
- Ability to deactivate menus using `Esc` key.
- `PopupMessage::RelayedMessage` to re-cast messages from a popup to a widget.
- `NavigationLayer` widget that handles `Tab`/`Shift+Tab` navigation.
- Ability to switch check box state using space key.
- Ability to click button widget using `Space`/`Enter` keys.
- `accepts_input` for widgets that can be used for keyboard interaction.
- Added keyboard navigation for input fields in the inspector.
- Highlight a widget with keyboard focus.
- `Visitor` docs.
- Ability to open/close drop down list using arrow keys.
- Re-cast `Variant` message on enum property editor.
- Focus popup content (if any) on opening.
- Keyboard navigation for list view widget.
- Focus window content (if any) on opening.
- Optional ability to bring focused item into view in navigation layer.
- Hotkey to run the game from the editor (default is `F5`).
- Ability to increase/decrease `NumericUpDown` widget value by arrow keys.
- Configurable command stack max capacity (prevents the command stack to grow uncontrollably, which could eat a lot of
  memory if the editor is running for a long time).
- Auto-select text on focusing `TextBox` widget.
- Ability to render scene manually.
- Ability to set precision for `VecEditor` widget.
- Ability to switch between shaded and wireframe mode in the scene preview.
- Multi-curve support for the curve editor widget.
- `Color::COLORS` array with pre-defined colors.
- Ability to set different brushes for every curve in the curve editor.
- Apply different colors to curves in the animation editor.
- Show multiple curves at once when selecting tracks in the animation editor.
- Dropdown menu widget.
- Quick-access menu for grid snapping.
- `Create Parent` context menu option for scene nodes.
- Add background curves concept to the curve editor widget.
- Smart placement for newly created objects.
- Added mesh control panel - allows to create physics entities (colliders, rigid bodies, etc) in a few clicks.
- `Reflect::assembly_name` to retrieve assembly name of a type.

## Changed

- Major style improvements for the editor UI.
- Migrated to Rapier 0.18.
- Refactored multiborrow context - removed static size constraint and made borrowing tracking dynamic and more
  efficient.
- Use `Result` instead of `Option` for multiborrowing for better UX.
- Added panic on `Ticket::drop` to prevent dangling pool records.
- Moved generic graph handling code into `fyrox-graph` crate.
- Do not call `Control::update` for every widget:
    - in the editor on complex scenes it improves average performance by 13-20%.
    - you have to set `need_update` flag when building the widget if you need `Control::update` to be called.
- Mutable access to UI in `Control::update`.
- Refactored `Selection` to use dynamic dispatch.
- Refactored the entire editor command system to use dynamic dispatch.
- Split `SceneGraph` trait into object-safe and object-non-safe parts.
- Run most of `Engine::pre_update` logic even if there's no graphics context.
- Moved color space transformation to vertex shader of particle system to increase performance.
- Recalculate world space bounding box of a mesh on `sync_transform` instead of `update`.
- Refactored rectpacker to use plain `Vec` instead of `Pool`.
- Moved rectangle-related code to `rectutils` crate.
- Automatically unregister faulty resources if registering ok one.
- Prevent uvgen to modifying the actual surface data.
- Extracted uvgen module to `uvgen` crate.
- Use simple vec instead of pool in octree.
- Moved `math` + `curve` + `octree` mods to `fyrox-math` crate.
- Moved lightmapper into a `lightmap` crate.
- Support for backwards movement (negative speed) for navmesh agent.
- Moved the engine implementation into `fyrox-impl` crate, `fyrox` crate now is a proxy to it.
- Moved interaction modes panel to the toolbar.
- Made shader methods public to be able to create them manually.
- Show unassigned handles in orange color to attract attention.
- Major refactoring of `TextBox` widget that makes it much more pleasant to work with.
- Major usability improvements for `DockingManager` tiles.
- `Window` widget content is now linked to `NavigationLayer` widget instance.
- Prevented `TextBox` and `NumericUpDown` widgets from sending change messages when they have not changed.
- Reduced width and precision for worldspace position of current selection.
- Use `ImmutableString` for scene nodes and widgets to reduce memory consumption on duplicated strings.
- Do not flush the renderer when changing scenes, to prevent various graphical issues.
- More informative names for curves in the animation editor.
- Change cursor icon when picking/dragging keys in curve editor.
- Major refactoring of coordinate system in the curve editor.
- Keep the animation player selected in the animation editor.
- Changed AABB validity to include zero-size dimensions to allow camera fitting to work with flat objects.
- Prefer prefab roots when selecting nodes in scene.
- `Reflect` trait bound for `Plugin` trait.

## Fixed

- Fixed cascade shadow maps (CSM) rendering.
- Fixed crash when setting particle spawn rate too high.
- Fixed UB when using MultiBorrowContext.
- Fixed visibility of cloned widget.
- Set unique id for widget copies.
- Fixed crash when closing scenes.
- Fixed `Default` impl for `Pool`.
- Fixed rare crash in `TextBox` widget when typing in something
- Fixing double pixel loop (it was looping over y twice) in terrain internals.
- Fixed creating a MenuItem in editor.
- Force ui widget to recalculate layout if it was animated
- Registered property editors for all UI properties.
- Fixed incorrect FBX cluster loading (fixes incorrect rendering of FBX models)
- Fixed crash when selection range is incorrect in the `TextBox` widget.
- Fixed crash in the animation editor when trying to rebind a track referencing deleted scene node.
- Properly expand tree items when building path in file browser widget.
- Fixed doubling of items under disks in file browser widget.
- Fixed track deletion in the animation editor.
- Fixed file browser behaviour on empty file path
- Select current dir in the asset browser.
- Automatically remove disconnected listeners from the log.
- Fixed support of custom layout panel of `ListView` widget.
- Fixed async tasks at WebAssembly target.
- Fixed property inheritance for types with interior mutability.
- Keep selected brush when hovering mouse over a `Decorator` widget.
- Fixed `TabControl` widget headers style.
- Improved SearchBar widget style.
- Fixed incorrect script task handling (it was passing task result to all scripts, instead the one that launched the
  task).
- Prevent particle systems from over-spawn particles when spawn rates are high.
- Fixed incorrect vertex buffer data layout.
- Fixed crash if a selected node was deleted during asset hot reloading.
- Prevent moving a folder into its own subfolder in the asset browser.
- Fixed lightmap saving when corresponding lightmap textures were deleted.
- Sort rectangles back-to-front when rendering to prevent blending issues.
- Back-to-front sorting when rendering nodes with transparency.
- Fixed seams on skybox cubemap.
- Hide `should_be_deleted` field.
- Do not update scripts on disabled nodes.
- Fixed sound context serialization (this bug caused all sound buses to disappear on load)
- Fixed potential crash in audio bus editor.
- Fixed crash when closing the editor.
- Fixed crash `attempt to subtract with overflow` in particle systems.
- Fixed incorrect `Selection::is_empty` implementation.
- Fixed canvas background color leaking to the rendered image on WebAssembly.
- Ignore `target` dir when doing search in the asset browser.
- Fixed accidental enabling/disabling tracks when expanding them in the animation editor.
- Fixed editor layout saving and loading.
- Prevent `Inspector` properties from disappearing when expander is closed.
- Use context menus instead of plain popups in color gradient editor.
- Fixed incorrect extension proposal for in the resource creator.
- Fixed incorrect resource creation in resource creator.
- Fixed sluggish tiles resizing in the docking manager.
- Keep the order of interaction modes the same.
- Fixed bring-into-view for `ScrollPanel` widget - not it does not jump unpredictable.
- Do not pass keyboard input to invisible widgets.
- Handle edge cases properly when calculating curve bounds.
- Fixed "zoom to fit" functionality in the curve editor widget.
- Fixed sliding of the view in the curve editor widget on resizing.
- Fixed frustum culling flag usage.
- Fixed inspector syncing/context changing.
- Fixed crash when trying to get selected entity from empty selection.
- Fixed crash when closing scenes using `X` button on the tabs.

## Removed

- Removed `define_command_stack` macro
- Removed redundant `old_selection` arg from change selection command

# 0.33.1 Engine + 0.20.1 Editor

## Fixed

- Fixed deadlock when deep cloning a texture. Caused the editor to hang up on saving terrains (#598).
- Fixed occasional crash when undoing node creation
- Fixed highlighting for objects that were cloned

# 0.33 Engine + 0.20 Editor

## Added

- UI editor.
- Tasks system for scripts and plugins.
- Implemented dynamic font atlas.
- Batching for 2D graphics.
- Ability to move resources and folders in the Asset Browser.
- Edge highlighting for selection in the editor.
- Added an ability to create resources from asset browser.
- Added height parameter for `Text` and `TextBox` widgets.
- Ability to specify IO abstraction when loading user interface.
- `Utf32StringPropertyEditorDefinition` to edit `Vec<char>` UTF32 strings.
- `RefCellPropertyEditorDefinition` for `RefCell<T>` types.
- Enable reflection + serialization for formatted text and its instances.
- Built in font resource.
- Font resource property editor with font preview.
- Ability to assign fonts from asset browser.
- Reflection for resources.
- UI graph manipulation methods.
- `Screen` widget automatically fits to the current screen size.
- Show type name in world viewer for widgets.
- Ability to specify ignored types for `Reflect::apply_recursively`.
- Preview for curve and hrir resources.
- Ability to open a window at desired position.
- Ability to edit textures as UntypedResource in widgets.
- Implemented `Serialize + Deserialize + Display` traits for `ErasedHandle`.
- Smart positioning for contextual floating panels in the editor.
- `WidgetMessage::Align` + `WindowMessage::OpenAndAlign` messages.
- Ability to invalidate layout for all widgets at once.
- Ability to mark all fields of a struct/enum optional when deserializing: `#[visit(optional)]` can now be
  added to a struct/enum directly, thus overriding all other such attributes on fields.
- Added access to user interface, task pool, graphics context, current scene handle for scripts.
- `PluginsRefMut::get/get_mut/of_type_ref/of_type_mut` methods.
- Added a bunch of `#[inline]` attributes for `Pool` for slight performance improvements.
- Added `AtomicHandle` that can be modified using interrior mutability.
- Ability to pass pixel kind to the `Renderer::render_ui_to_texture` method.
- Show material resource state in the material field editor.
- Ability to scroll to the end of the content for `ScrollViewer` and `ScrollPanel` widgets.
- Ability to save and load light maps into/from a file.
- Ability to repeat clicks of a button while it is hold pressed.
- Ability to open materials for editing from the asset browser.
- Ability to filter a list of types when using reflection for fields iteration.
- Implemented `PartialOrd + Ord` traits for `Handle` type.
- Added icon in the asset browser for material resources.
- Ability to pass generics to `uuid_provider` macro.
- Ability to share navigational mesh across multiple threads.
- Implemented `Reflect` trait for `RwLock`.
- `UserInterface::find_by_name_down_from_root` method to search widgets by name.
- Implemented `Send` trait for UI entities.
- Added user interface resource.
- Collider control panel with ability to fit colliders to parent bounds.
- Property editor for vector image's primitives.
- Show warning in the log when there's script message with no subscribers.
- Implemented `TypeUuidProvider` trait for standard types.
- Ability to specify clear color in `Renderer::render_ui_to_texture`.
- Added icon in the asset browser for shader resources.
- Ability to copy widgets from UI to UI.
- Ability to create ragdolls from `Create` menu.
- Added an ability to rebind tracks in the animation editor.
- Added a set of standard materials, exposed them in the editor.
- Added placeholder texture.
- Ability to fetch resource import options from their respective loaders.
- Implemented `Visit` and `Reflect` traits for `char`.
- Ability to specify icons for assets in respective preview generators.
- `TypedResourceData` trait to be able to set correct default state of a resource.
- Implemented `ResourceData::save` for built-in engine resource types.
- Documentation for LODs.
- Color constants for the colors with names.
- Ability to save resources.
- `ResourceLoader::supports_extension` method.
- Implemented `Error` trait for `VisitError`.
- `Material::set_texture` shortcut.
- Implemented `From<&str>` trait for `ImmutableString`.
- Added normalization option for vertex attribute descriptor.
- Added experimental network abstraction layer.
- Added frustum culling for rectangle node.
- Added camera view pyramid visualization (kudos to [@dasimonde](https://github.com/dasimonde)).
- Added IO abstraction for resource system (kudos to [@jacobtread](https://github.com/jacobtread)).
- Added `Reflect`, `Debug`, `Visit` trait implementations for UI widgets.
- Added `Visit` trait implementation for `usize/isize`.
- Added `ResourceIo::move_file` method.
- Added `ResourceManager::move_resource` method with filtering.
- Added `Drop` message for `FileBrowser` with dropped path.
- Added `ResourceIo::canonicalize_path`.
- Added `Pool::generate_free_handles` methods.
- Added `InteractionMode::make_button` method that creates appropriate button for the mode.

## Changed

- Major editor refactoring to support new UI scenes.
- Moved low level animation modules into fyrox-animation crate.
    - Type aliases for scene specific animation entities + preludes.
    - Texture as generic parameter for sprite sheet animation.
- Turn font into resource + added `TextMessage::Height`.
- Make standard built-in shaders non-procedural by default.
- Refactored internal structure of resources.
    - All resource related data is now stored in `ResourceHeader` instead of being scattered all around in
      `ResourceState`
      variants and even in resource data itself.
    - Backward compatibility is preserved.
    - `ResourceKind` instead of path+flag, refactored resource loader trait.
- Refactored interaction modes in the editor.
- Switched to UntypedResource from SharedTexture in ui
- Simplified usage of `ResourceManager::request/try_request`. No need to write `rm.request<Type, _>`, just
  `rm.request<Type>`.
- Registered Light Panel in floating panels, so it can be docked.
- Made searching in the asset browser smarter.
- GPU resources cache refactoring.
- Speed up access to textures.
- Automatic implementation of `ScriptTrait::id()` method. This implementation now should be removed from your
  scripts.
- Scroll to the end of build log in the editor.
- Prevented build window from closing when a build has failed.
- Tweaked node handle property editor to also work with ui widgets.
- Filter out texture bytes in the property selector to improve performance.
- Enabled clicks repetition mode for scroll bar increase/decrease buttons.
- Keep the editor active if there's any captured ui element.
- Increased scroll bar step for scroll viewer.
- Added filter argument for `aabb_of_descendants`.
- Use abstract EntityId instead of ErasedHandle in animation entities.
- Optimized internals of navigation mesh.
- Prevented serialization of the data of external resources.
- Pass screen size to `Control::update`.
- Ability to clone user interface entirely.
- Refactored scene saving dialogs in the editor to make them more stable.
- Made `Limb::iterate_recursive` method public.
- Switch character rigid body to kinematic when a ragdoll is active.
- Keep menu items highlighted when opening a menu chain.
- Gizmo improvements for navmesh interaction mode.
- Open navmesh panel at the top right of the scene preview when selecting a navmesh scene node.
- Improved visualization in the dependency viewer.
- Made asset import options inspector generic.
- Provide access to material context in the renderer.
- Movement, scale, rotation gizmo improvements.
- Mutable access for ui nodes.
- Preload resources before generating preview for them.
- Made world viewer to accept data provider instead of scene directly.
- Replaced `Cow<Path>` with `&Path` in `ResourceData` trait
- Allow to set materials by drag'n'drop on material editor field.
- Made material fields in the inspector more clickable.
- Improved navigation on navmesh using string pulling algorithm.
- Improved performance of navigation mesh queries.
- Improved text box widget performance.
- `Plane` API improvements.
- Made material editor wider a bit by default.
- Extend resource data constructor with type name.
- Turned material into resource, removed `SharedMaterial` struct.
- Serialize vertex buffer data as a bytes slab.
- Use `Window::pre_present_notify` as recommended in the `winit` docs.
- Refactored sprites rendering to use materials.
- Refactored particle system rendering to use forward renderer.
- More built-in shader variables for lighting.
- Triangle buffer API improvements.
- Use debug message callback instead of message queue for OpenGL errors.
- Enable OpenGL debugging in debug build profile.
- Customizable time-to-live for geometry buffers (allows to create temporary buffers that lives one frame (ttl = 0)).
- Allow to start multiple scenes at editor start up (kudos to [@dasimonde](https://github.com/dasimonde)).
- `push_vertices` + `push_vertices_transform` method for vertex buffer.
- Ability to connect a state with every other state in the ABSM editor (kudos
  to [@Riddhiman007](https://github.com/Riddhiman007))
- Added UUIDs for scene nodes.
- Ability to set navmesh agent path recalculation threshold.
- Reset `modified` flags of inheritable variables when fixing node type.
- Check for node type mismatch on graph resolve and auto-fix this.
- Use type names instead of type ids when reporting inheritance errors.
- Remove orphaned nodes when restoring graph's integrity.
- Code example for `Inspector` widget.
- Pass node handle to surface instance data.
- Check for global `enabled` flag when filtering out cameras for rendering.
- Serialize joints binding local frames.
- Support for touch events in the UI (kudos to [@Bocksdin](https://github.com/Bocksdin)).
- A* pathfinding optimization (kudos to [@TiltedTeapot](https://github.com/TiltedTeapot)).

## Fixed

- Fixed crash of the editor on Wayland.
- Fixed font rendering API.
- Fixed restoration of shallow resource handles of untyped resources.
- Prevent double saving of settings after modification.
- Keep aspect ratio when previewing a texture in the asset browser.
- Filter out non-savable resources in resource creation dialog.
- Fixed offscreen UI rendering in the UI editor.
- Fixed deep cloning of materials: make them embedded after cloning.
- Fixed path filters to correctly handle folders with "extensions".
- Save material when changing its shader property in the material editor.
- Fixed massive footgun with pointers to the graphics pipeline state scattered all around the renderer.
- Prevent creating of multiple thread pool across the engine.
- Fixed crash in the material editor if a material is failed to load.
- Prevent the editor from closing after saving a scene via Ctrl+S.
- Fixed position saving of maximized editor window.
- Fixed crash when assigning non-material resource in a material property.
- Fixed forward pass of standard shader for skinned meshes
- Fixed uuid formatting in visitor.
- Fixed resource extension comparison in the editor by making it case-insensitive.
- Fixed crash when drag'n'dropping assets in scene previewer.
- Fixed OpenGL error handling
- Fixed performance issues when modifying vertex/triangle buffers.
- Fixed crash when editing terrains (kudos to [@Riddhiman007](https://github.com/Riddhiman007))
- Fixed a bug when vertex attribute divisor was ignored.
- Fixed colors for log messages.
- Fixed scene loading in derived mode.
- Fixed text coloring when sending a `WidgetMessage::Foreground` to text.
- Fixed memory leaks on Linux (kudos to [@LordCocoNut](https://github.com/LordCocoNut))
- Fixed invalid GPU resource indexing bug, that caused crashes/quirks in graphics when switching scenes in the editor.

## Removed

- Removed implicit cloning when in `Reflect::into_any` impl for some types.
- Removed redundant memory allocation when fetching fields using reflection.
- Removed redundant memory allocation when iterating over fields.
- Removed `Option` wrapper in typed resource to flatten the internal structure of resources.
- Removed a bunch of redundant clones in the renderer.
- Removed lazy calculations in the navigational mesh.
- Removed unused `soft_boundary_sharpness_factor` param from particle systems (this property was moved to the
  standard particle system material).
- Removed `InteractionModeKind` and replaced it with uuids.

# 0.32

- Do not call `Script::on_os_event` if script is not started yet.
- Borrow instead of move in `Visitor::load_from_memory`.
- Ability to load scenes in two modes - derived and raw.
- Fixed selection issues in the animation editor.
- Fixed path fixer.
- Ability to set resource path.
- `ResourceManager::unregister` to unregister loaded resources.
- Refactored scene loading + plugin interface improvements.
- Bring currently selected node into view when clearing filter in the world viewer.
- Fixed searching in the property selector.
- Bring current selection into view in node selector when clearing filter text.
- Fixed `zoom to fit` in the curve editor when there's no keys.
- Fixed node name formatting in the animation editor's track list.
- Fixed tooltips in the inspector.
- `EditorPlugin::on_post_update` that invoked after any other update methods.
- Fixed selection syncing in node selector.
- `TreeRootMessage::ItemsChanged` to catch the moment when tree root items changes.
- Improved visual style of node handle property editor.
- Ability to set scene node handle via node selector.
- `Sound::try_play` method that will only play the sound if it is not already playing.
- `Flip green channel` option for texture import options: this adds an ability to flip green channels for
  normal maps made in OpenGL Y+ format.
- Resource manager improvements: added base trait with auto-implementation to reduce boilerplate code, mandatory
  `ResourceLoader::data_type_uuid` method to fetch actual data type produced by resource loader,
  `ResourceManager::try_request` - returns an optional resource handle, returns `None` if `T` does not match the
  actual data id (`request` just panics in this case).
- Print an error message to the log when unable to load a resource.
- Resource field property editor improvements: emit transparent geometry to improve mouse picking,
  added margins for elements.
- Exposed resource manager reference to plugin registration context to be able to register custom resource
  loaders that will be used in both the game and the editor.
- `Material::sync_to_shader` method to sync material properties with its shader.
- `parallaxCenter` + `parallaxScale` property for standard shaders.
- Fixed TBN-basis visualization in mesh debug drawing.
- Make all gizmo's X axis match the actual coordinate system's X axis.
- Fixed tooltip in asset browser to show full path without clipping.
- Fixed parallax mapping.
- Fixed binormal vector calculation.
- Added missing `tif` extension for texture loader.
- Fixed build window output in the editor.
- Added fade in/fade out for shadows, that prevents them from popping out of nowhere.
- Added scene gizmo.
- Keep frame alpha when applying lighting for transparent background rendering.
- Rewind sound sources before stopping them.
- Improved camera focusing on a scene object.
- Changed orbital camera controls: drag camera was moved to `Shift+RMB`, added configurable zoom range.
- Added orbital camera mode for editor camera (can be activated by middle mouse button).
- Use `f32` instead of `Duration` for scene sound source's playback time.
- Fixed terrain brush bounds visualization.
- Hotkeys for terrain editor.
- Use `workspace.dependencies` in the projects generated by `fyrox-template` to simplify dependency change.
- Improved editor settings handling.
- `Curve::bounds` + ability to `Zoom to fit` with delay for the curve editor.
- Property editor for `Curve` fields.
- New `fyrox-scripts` crate + flying camera controller script.
- Ability to map ui key back to winit + change key binding property editor.
- Fallback to root directory if `fyrox-template script` cant find `game/src`.
- Added debug impls for gpu texture.
- Fixed seams between terrain chunks.
- Removed obsolete examples and replaced them with [new examples](https://github.com/FyroxEngine/Fyrox-demo-projects).
- Fixed curve editor compatibility with scrolling regions.
- Fixed clipping issues in curve editor.
- Save expanded state of the scene items in the world viewer in the editor settings.
- Fixed invalid keys positioning in the curve editor when selecting them.
- Fixed box selection in the curve editor when mouse is outside.
- Focus currently selected entity when clearing filter text in animation editor.
- Fixed a bunch of potential crashes in the `CurveEditor` widget.
- Fixed HiDPI issues on WebAssembly.
- Removed hardcoded list of supported resource extensions from the editor and use ones from resource loaders.
- `Hrir` resource + async HRTF loading for HRTF sound renderer.
- Fixed texture compression.
- Do not use `glCheckError` in release builds since it has bad performance.
- Set nearest filtration for floating-point textures in the renderer (WebAssembly fix).
- Switch a resource without a loader into error state to prevent infinite loading in some cases.
- Fixed resource loading in WebAssembly.
- Do not render anything if screen size is collapsed into a point.
- Split light map generation in two steps + added async generation for the editor.
- Do not allow to create game projects with a number in beginning of its name.
- Optimized light map data serialization (breaking change, regenerate your lightmaps).
- `BinaryBlob` wrapper to serialize arbitrary sets of data as bytes.
- Print an error to the log instead crashing when unable to generate a lightmap.
- Moved light map into `Graph` from `Scene`.
- Fixed light map internal handles mapping when copying a graph.
- `PathEditor` widget + property editor for `PathBuf` for Inspector.
- Reduce default amount of texels per unit for lightmapper in the editor.
- Ability to specify vcs for a new project in `fyrox-template`
- Set `resolver = 2` for workspaces generated by `fyrox-template`
- Improved joints computational stability.
- `Make Unassigned` button for node handle property editor.
- Do not save invalid window attributes of the main editor window.
- Fixed joints binding.
- Joint rebinding is now optional.
- Fixed potential infinite loop when constructing quaternion from a matrix.
- Ability to set custom name to group command in the editor.
- `Ragdoll` scene node.
- Improved mouse picking for node handle property editor.
- Ragdoll wizard to create ragdolls with a few clicks.
- Power-saving mode for the editor. Editor pauses its execution if its window is unfocused or there's no OS events
  from the main window. This change reduces CPU/GPU resources consumption down to zero when the editor is non-active.
- Do not create a separate region for inheritable variables on serialization if non-modified. This saves quite a
  lot of disk space in derived assets (including saved games).
- Property editors for inheritable vec collections of resources.
- Clamp input time to the actual duration of the buffer when setting sound source's playback time.
- Fixed inability to fetch stream length of ogg/vorbis.
- `GenericBuffer::duration` is now using integer arithmetics which does not suffer from precision
  issues (unlike floating point numbers).
- Decoders now returns channel duration in samples, not in seconds.
- Send text box message on changes only if its commit mode is immediate.
- Fixed severity for messages from inability to load editor settings.
- Added vec property editors for collections of resources.
- Property editor for `Vec<T>` will now use appropriate property editor for `T` instead of implicit usage
  of `InspectablePropertyEditor`.
- Fixed incorrect focusing of an asset in the asset browser.
- Fixed emitted message direction for `TextBox` widget.
- `Show in Asset Browser` button for resource fields in the inspector.
- Take sound source gain into account when using HRTF renderer.
- Fixed visualization of bones list of a surface in the editor.
- Reduced HRTF sound renderer latency.
- Fixed animation events collection for blend-by-index ABSM nodes.
- Improved ABSM events collection API.
- Ability to fetch animation events from ABSM layers.
- Fixed property reversion: now it reverts only modified ones.
- Ability to revert all inheritable properties at once of a scene node.
- `Reflect::enumerate_fields_recursively` allows you to iterate over descendant fields of an object
  while getting info about each field.
- Update only currently active scene in the editor.
- Navmesh path smoothing improvements and fixes. Prevent smoothing from cutting corners.
- `A*` path finder API improvements.
- Debug drawing for NavMesh scene node.
- Light scattering now takes light intensity into account.
- Prevent loading the same scene multiple times.
- Clear UI in the editor when changing scenes to prevent potential visual desync.
- Fixed potential panic when handling UI messages with invalid target widget handle.
- Fixed doubling of the text when printing text in `TextBox` widget on some platforms.
- Ability to duplicate animation tracks in the animation editor.
- Ability to set an ID of animation tracks.
- Fixed potential panic on invalid handles of `Rapier` entities when fetching contacts.
- Ability to close tabs in `TabControl` widget using middle mouse button.
- Visualize directional lights as arrows in the editor.
- Ability to draw arrows in scene debug drawing context.
- Migrated to `winit 0.29`.
- Fixed `Rect::clip_by` method.
- Removed `VecExtensions` trait, because its functionality was already added in the standard library.
- `Popup` widget improvements: `Placement::target` method, ability to create popups without adding them
  to the UI.
- Fixed potential infinite loop in the `Menu` widget.
- Added context menu to the file browser to be able to create folders and remove files.
- Significantly improved test coverage for `fyrox-core` and `fyrox-resource` crates (kudos to
  [@san-smith](https://github.com/san-smith))
- Optional node deletion dialog to warn if a node is referenced somewhere in the graph.
- Fixed potential double free issue in the vertex buffer.
- Fixed unsoundness of type-erasure in the vertex buffer.
- `Graph::find_references_to` to search for node references in the graph.
- `Reflect::apply_recursively` for recursive iteration over the descendant fields of an object.
- Added `try` reserved keyword for `fyrox-template`.
- Built-in sky box for `Camera` scene node.
- Improved search in the World Viewer.
- Make `TriangleDefinition` trivially-copyable.
- Major UI documentation improvements.
- Docs for `VectorImage`, `ScrollPanel`, `RectEditor`, `RangeEditor`, `ProgressBar`, `ListView`, `Canvas`,
  `SearchBar`, `ScrollViewer`, `Expander`, `KeyBindingEditor`, `HotKeyEditor`, `Tree`, widgets.
- Major book improvements.

# 0.31

- Multi-scene editing
- Docs for `Window` widget
- Fixed opengl es usage when opengl is not supported
- Docs for `Decorator` widget
- Added `crv` extension for `CurveLoader`
- Basic editor plugins support
- Updated deps
- Expose all editor fields so they can be accessible outside
- Docs for `UuidEditor` widget
- Use user_data field of physics entities to store handle to engine entity
- Ability to encode/decode handles to/from u128
- Ability to fetch all contact pairs from 2d/3d physics worlds
- Docs for `MessageBox` widget
- `Graph::aabb_of_descendants`
- Aabb api improvements
- Ability to open asset of a node from the world viewer
- Improved `impl_component_provider` macro to accept `field.foo.ab` chains
- Docs for navmesh node
- Useful shortcuts for behaviour trees
- Fixed standard materials for new serialization format
- Inverter node for behaviour trees
- Docs and examples for `VertexBuffer`
- Added `VertexTrait` to prevent using a vertex type with different layout
- Improved `surface` mod docs
- Added `elapsed_time` in `PluginContext`
- Use all texture channels in sprite fragment shader
- Load editor's docking manager layout on reconfiguration
- Open window of a tile when restoring docking manager layout
- Ability to save/load editor's docking manager layout
- Prevent panic in ui search methods
- Ability to apply saved docking manager layout + improved layout saving
- Ability to save docking manager layout
- Changed error to warning when unable to load missing options file
- Fixed crash when exiting the editor
- Fixed opening arbitrary files from asset browser
- Ability to open scenes from asset browser
- User-defined data for tabs
- Ability to add and remove tabs in the `TabControl` widget via messages
- Added a nine patch widget
- Fixed tab control's content alignment
- `can_be_closed` flag for `TabControl` tabs
- Ability to close tabs in `TabControl` widget
- Docs for `TabControl` widget
- Ability to catch the moment when the active tab of `TabControl` changed
- Docs for `ScrollBar` widget
- Docs for `Popup` widget
- Docs for `NumericUpDown` widget
- Ability to change `StackPanel`'s orientation via message
- Ability to change `WrapPanel`'s orientation via message
- Docs for `WrapPanel` widget
- Docs for `CheckBox` widget
- Docs for `Widget`
- Docs for `TextBox` widget
- Docs for `StackPanel` widget
- Docs for `Grid` widget
- Docs for `Image` widget
- Docs for `Text` widget
- Fyrox-ui docs
- Docs for `Button` widget
- Access to current transform of `TransformStack`
- Docs for `Border`
- Ability to pass doc comments in `define_constructor` macro
- Docs for `BuildContext`
- Docs for `UiNode`
- Iterative font atlas packing.
- Docs for `Thickness`
- Docs for widget alignments
- Docs for `BaseControl`
- Update hierarchical data when instantiating a prefab
- Docs for `trait Control`
- Hotkey to focus editor's camera on a selected object
- Helper methods for `Frustum`
- Ability to focus editor's camera on an object
- Helper methods for `TextureKind`
- Camera fitting functionality
- Aabb helper methods
- Save editor settings only if they were modified by user
- `Camera::frustum`
- Fixed camera preview + added camera preview control panel
- Automatically select newly created scene nodes in the editor

# 0.30

- Ability to change graph root to arbitrary graph node.
- Ability to change graph root in the editor.
- Optional checkerboard background for `Image` widget.
- Simplified animation blending.
- Mutable access to curve key's value.
- Added property validation for the animation editor.
- Track validation for the animation editor.
- Ability to set widget's tooltip via message.
- Correctly sync track names in the animation editor.
- Ability to change target nodes on animation tracks.
- Preserve parent when extracting a sub-graph from a graph.
- Refactored editor scene structure to allow modifying the root node.
- Play sound buffer resource when inspecting it in the asset browser.
- Show textured quad in resources previewer when inspecting a texture.
- Configurable scroll speed for `ScrollViewer` widget + speed up scrolling 2x.
- Helper methods to quickly check a resource state.
- Helper methods to access script components faster.
- Improved range property editor.
- `Enter State` for state menu in absm editor. Works the same as double click, removes confusion for ppl that does not
  get used to double-click on things.
- Leave preview mode when closing or changing scenes in the editor.
- Prevent panic when trying to generate random number from an empty range.
- Serialize delay line samples as POD array.
- Optional ability to save current scene in text form for debugging.
- Do not render disabled sprite nodes.
- Fixed property inheritance subtle bugs.
- Do not allow revering a property value in the editor if there's no parent.
- Do not save content of non-modified inheritable variables.
- Fixed directional light docs.
- Fixed `Node::is_x,as_x,as_x_mut` methods.
- `Graph::try_get_script_of + try_get_script_of_mut` methods
- `Base::root_resource` - allows you to find root resource in dependency graph.
- Prevent deadlock on self-referencing model resources
- UUID for widgets.
- Save editor's window position and size into editor's settings.
- Apply local scaling of terrain to heightfield collider.
- `MachineLayer::is_all_animations_of_state_ended`
- Ability to fetch all animations of a state in ABSM layer.
- Added `IsAnimationEnded` condition for ABSM transitions.
- ABSM state actions. Allows you to rewind/enable/disable specific animations when entering/leaving a state.
- Fixed incorrect "state enter" event sent from source instead of dest.
- Added a collection of built-in resources for resource manager. This collection is used on resource deserialization
  step to restore references to built-in resources.
- Pre-compile built-in shaders on engine startup.
- Ability to change camera zoom speed in the editor.
- `Plugin::before_rendering`
- Matrix storage cache to prevent driver synchronization steps.
- Persistent identifiers for render entities.
- Improved deserialization performance.
- Use `fast_image_resize` crate to generate mip maps (which gave 5x performance boost).
- Configurable filter for mip-map generation for textures.
- Fixed tooltip position - it now does not go outside of screen bounds.
- "Immutable collection" reflection attribute for collection fields that prevent changing collection size.
- Ability to get typed data of specific mip level of a texture.
- Ability to fetch specific mip level data of textures.
- Ability to set height map of terrain chunks directly from an image.
- Dependency graph visualizer for asset browser.
- Resource dependency graph.
- Ability to flatten terrain slopes.
- Return local height value at intersection point in ray-terrain test.
- Cleaned editor's command API.
- Removed visibility cache.
- Ability to index graph with `Handle<T: NodeTrait>`
- `Handle::transmute`
- Doc comments support for reflection.
- Show doc comments for selected entity in a separate window.
- Moved logger to `fyrox_core`.
- Resource system refactoring to support user-defined resources.
- Blackboard for visitor to pass arbitrary data when serializing/deserializing.
- Added missing recalculation of terrain bounding box.
- `Texture::deep_clone`
- `Log::verify_message`
- `R32F` + `R16F` texture formats.
- `data_of_type` methods to reinterpret inner texture data storage to a particular type.
- Debug drawing for scene nodes.
- Configurable polygon rasterization mode for scenes (gbuffer only).
- Ability to set polygon rasterization mode to select between solid and wireframe rendering.
- Force `Framebuffer::draw_x` methods to accept element range to draw.
- Proper culling for terrains.
- Refactored rendering: scene nodes can now supply renderer with data. `NodeTrait::collect_render_data` is now used to
  supply renderer with data.
- Batch generation is now done on per-camera (which includes light sources for shadows) basis.
- Added a method to link nodes while keeping child's global position and rotation.
- LODs for terrains.
- Limits for collider shape values.
- Added doc example for `Graph::begin_multi_borrow`.
- Fixed samplers type collision when rendering with materials with different sampler types.
- Unbind texture from other samplers when setting it to a new one.
- Fixed half-float textures + fixed volume textures mip maps.
- `RGB16F` texture format.
- Use texture-based matrix storage for "unlimited" bone matrices. Raises matrix count per surface from 64
  to 255.
- Fixed texture alignment issues.
- Use correct sampler index when changing texture data.
- Set new mip count for texture when changing its data.
- Fixed texture binding bug.
- Warning instead of panic when there's not enough space for bone matrices.
- Rename `visitor::Node` to `visitor::VisitorNode` to prevent confusing import in IDEs.
- `InheritableVariable::take`
- Ability to change size of terrain height map and layer masks.
- Ability to add chunks from any side of the terrain.
- Fixed crash when deleting a navmesh edge.
- Improved package description.
- Make navmesh panel floating by default + open it when a navmesh is selected.
- Navigational mesh refactoring.
- Navigational mesh scene node.
- Pass light intensity into lightmapper.
- "Headless" mode for `Executor` - suitable for server-side of multiplayer games.
- Added editor's window icon.
- Blend shape support.
- Changed sidebar to be inspector in the view dropdown menu.
- Tweaked step values for transform properties.
- Limits for vec editor.
- Generic `Vector<T,N>` property editor.
- Added support for min, max, step property attributes for vecN.
- Ability to create/destroy audio output device on demand.
- Migrate to `tinyaudio` as audio output backend
- Use `RcUiNodeHandle` for context menus. This ensures that context menu will be destroyed when it is
  not used anymore.
- Fixed multiple lightmapping issues.
- Fixed incorrect `sRGB` conversion for WASM.
- Android support.
- Ability to run the engine without graphics/window/sound by making these parts optional.
- Update to latest `winit` + `glutin`.
- Ability to change value in `NumericUpDown` widget by dragging
- Removed "Scene Graph" item from world viewer + made breadcrumbs much more compact.
- Put interaction mode panel on top of scene previewer.
- Added ability to search assets in the asset browser.
- `SearchBar` widget.
- Ability to hide path text box in file browser widget.
- Hide path field in the asset browser.
- Tooltip for asset items in the asset browser that shows full asset path.
- Improved simple tooltip style.
- Optional ability to suppress closing menus by clicking on non-empty menu.
- Added `No Scene` reminder in the editor and how to create/load a scene.
- Editor UI style improvements.
- `DrawingContext::push_arc+push_rounded_rect`
- Ability to enable/disable debug geometry for camera/light sources.
- Show indices of input sockets of ABSM nodes.
- Keep animations enabled on import.
- Blend space support.
- Added help menu (with `Open Book` and `Open API Reference` items)
- Ability to create special (much faster) bindings to position/scale/rotation of nodes in the animation
  editor.
- Ability to reimport animations in the animation editor.
- New example: render to texture.
- Audio bus graph.
- Root motion support.
- Audio panel rework to support audio bus graphs.
- Sound effect API improvements.
- Keep recent files list sorted and up-to-date.
- Fixed incorrect sound panning in HRTF mode.
- Ability to get unique material instances when cloning a surface.
- Validation for sound node
- Audio preview panel
- Do not play sounds in the editor automatically. Sounds can only be played from the audio preview panel
  instead. fixes the issue when you have a scene with multiple sounds, but since they're playing, their playback
  position
  changes and these changes sneak in the saved scene preventing from defining strict playback position
- Ability to partially update global properties of a hierachy of nodes.
- Do not crash if a root node in the previewer died.
- Fixed deadlock when selecting object's property in animation editor.
- Ability to set pre-generated particles in particle systems.
- Provided access to standard shader names.
- Print texture resource name when failed to create its GPU version.
- Rebuild terrain's geometry on deserialization.
- Automatic, reflection-based resource handle mapping.
- Ability to ignore some type when doing property inheritance.
- Support for hash maps in the property selector.
- Expose material fields via reflection.
- Keep flags of `ScrollBarMessage` when responding to value message.
- Delegating implementation of `Debug` trait for `ImmutableString`.
- Added reflection for hash maps.
- Reflection system refactoring to support types with interior mutability (`Mutex`, `RefCell`, etc.)
- Ability to rewind particle systems to a particular time.
- Determinism for particle systems.
- Fixed preview mode for particle systems.
- Ability to "rewind" particle systems in particle system control panel.
- Fixed `ParticleSystem::clear_particles` for emitters that does not resurrect their particles.
- Fixed potential panic in formatted text on missing glyphs.
- Supply `PluginContext` with performance statistics for the previous frame.
- Property editor for `ColorGradient`s.
- Simplified `color_over_lifetime` field in particle systems.
- Improved color gradient API.
- Fixed incorrect activation of transition/states during the preview mode in the ABSM editor.
- Compound conditions for ABSM transitions
- Fixed off-screen UI rendering compatibility with HDR pipeline.
- Refactored scene node lifetime management - this mainly fixes the bug when a node with `Some(lifetime)` would crash
  the editor. The same is applied to play-once sounds. `Node::update` now does not manage node's lifetime anymore,
  instead
  there's `Node::is_alive`.
- Fixed incorrect handling of user-defined forces of rigid bodies. A body was pushed continuously using
  previously set force.
- Configurable size for light pictograms in the editor
- `ActiveStateChanged` event now contains both previous and new states.
- Message passing for scripts with multiple routing strategies
- `Graph::find_map/find_up_map/find_up_by_name`
- Improved `Graph::find_x` methods - returns `Option<(Handle<Node>, &Node)>` now, that removes another
  borrow if there's a need to borrow it at a call site.

# 0.29

- Animation system rework.
- Animation Editor.
- Animation Blending State Machine Editor.
- Fixed potential crash when joint was initialized earlier than connected rigid bodies.
- Model instantiation scaling now used for prefab preview.
- Fixed lots of potential sources of panic in perspective and ortho projections.
- Fixed editor's camera movement speed setting for 3D mode.
- Standard "two-side" shader - useful for foliage and grass.
- Sprite sheet editor
- Support for `Vector(2/3/4)<f32/f64/u8/i8/u16/i16/u32/i32/u64/i64>` types in serializer.
- Sprite sheet animation now uses frames coordinates instead of explicit uv rectangles for each frame.
- Sprite sheet animation now has a texture associated with it.
- Fixed reflection fallback in case of missing field setter.
- Ability to set uv rect for Image widget
- Scene settings window for the editor - gives you an ability to edit scene settings: change
  physics integration parameters, ambient lighting color, various flags, etc.
- Prevent crash when adding a new surface to a Mesh node in the editor
- Fixed directory/file duplicates in file browser widget when double-clicking on an item.
- Show use count for materials in Inspector
- Replace `Arc<Mutex<Material>>` with `SharedMaterial` new-type.
- Ability to assign a unique copy of a material to an object.
- Replace `Arc<Mutex<Material>>` with `SurfaceSharedData`
- Clear collections before deserialization
- Property inheritance for collections
- Fixed incorrect material replacement when loading a scene with an FBX with custom materials.
- Added Blender material slots names in FBX loader
- Access to `procedural` flag for `SurfaceData`
- Property editor for mesh's surface data.
- Validation for scene nodes
    - Helps to find invalid cases like:
    - Missing joint bodies or invalid types of bodies (i.e. use 2d rigid body for 3d joint)
    - Wrongly attached colliders (not being a child of a rigid body)
    - Shows small exclamation mark if there's something wrong with a node
- Share tooltip across widgets on clone
- Fixed color picker: brightness-saturation grid wasn't visible
- Added support for Collider intersection check (kudos to [@Thomas Hauth](https://github.com/ThomasHauth))
- Animation system refactoring
    - Use curves for numeric properties.
    - Ability to animate arbitrary numeric properties via reflection.
- Prevent crash in case of invalid node handle in animation
- `Curve::value_at` optimization - 2x performance improvement of using binary search for spans.
- `Curve::add_key` optimized insertion using binary search.
- Node Selector widget - allows you to pick a node from a scene.
- Merge `Inspect` trait functionality into `Reflect` trait - it is now possible to obtain fields metadata
  while iterating over them.
- Property Selector widget - allows you to pick a property path from an object that supports `Reflect` trait.
- `Reflect` implementation for `Uuid`
- `fyrox::gui::utils::make_cross` - small helper to create a vector image of a cross
- `FieldInfo::type_name` - allows to get type name of a field without using unstable
  `std::any::type_name_of_val`
- `PathVertex::g_score` penalty for A* pathfinding (kudos to [@cordain](https://github.com/Cordain))
- Added `Default`, `Debug`,`Clone` impls for `RawMesh`
- Name and uuid for `Curve`
- Send curve when adding new keys in the `CurveEditor` widget
- Preserve curve and keys id in the curve editor widget
- Correctly wrap `Audio Panel` in docking manager tile (kudos to [@iRaiko](https://github.com/iRaiko))
- `AsyncSceneLoader` - cross-platform (wasm included) asynchronous scene loader
- Added support for wasm in fyrox-template - now fyrox-template generates `executor-wasm` crate which is a special
  version of executor for webassembly
- Non-blocking resource waiting before processing scene scripts
- Added missing property editor for sound status
- Sync sound buffer first, then playback position
- Property editor for `Machine` type.
- Rectangle+RectangleFilled primitives for `VectorImage` widget
- Draw x values in curve editor widget at the top of the view
- Ability to show/hide axes values in the curve editor widget
- Use messages to modify view position and zoom in the curve editor (helps to catch the moment when zoom or view
  position changes)
- Fixed UI messages not being passed to plugins based on when they happened during frame (kudos to
  [@bolshoytoster](https://github.com/bolshoytoster))
- Ability to explicitly set animation time slice instead of length.
- Cloning a node now produces exact clone.
- Ability to set min, max values, step, precision for numericupdown widget
- Prevent panic when trying to iterate over pool items using reflection
- Split `Model::retarget_animations` in two separate methods
- Smart movement move for move gizmo (kudos to [@Zoltan Haindrich](https://github.com/kgyrtkirk))
- `Reflect::set_field_by_path`
- Ability to add zones for highlighting in the `CurveEditor`
- Ability to zoom non-uniformly via shift or ctrl pressed during zooming in the `CurveEditor` widget
- Animation signals rework
    - uuid instead of numeric identifier
    - added name for signals
    - removed getters/setters
    - added more signal management methods
- `Animation::pop_signal`
- Refactored animation blending state machine to support animation layers
- `Visit` impl for `HashSet`
- Ability to set layer mask in the absm editor
- Added animation system documentation.
- `Graph::try_get_of_type+try_get_mut_of_type`
- Rename `InheritableVariable` methods to remove ambiguity
- `Model::retarget_animations_to_player`
- Use correct property editor for `PoseWeight`
- Show handles of absm entities in the editor
- Show more info on absm nodes
    - PlayAnimation nodes shows name of the animation
    - blend nodes shows the amount of animations blended
- `AnimationContainer::find_by_name_ref/mut`
- Ability to search various animation entities by their names
- Add more information to panic messages in `fyrox-template` (kudos to [@lenscas](https://github.com/lenscas))
- Check for reserved names in `fyrox-template` (kudos to [@TheEggShark](https://github.com/TheEggShark))
- Ability to enable/disable scene nodes
- Basic support for headless mode for server part of games (kudos to [@martin-t](https://github.com/martin-t))
- Removed `Scene::remove_node`
- Rename `NodeTrait::clean_up` -> `NodeTrait::on_removed_from_graph`
- Fixed colorization in the world viewer
- Ability to disable steps of update pipeline of the graph
- Preview mode for animation player, animation blending state machine, particle system nodes.
- Rename colliding `ParticleSystem::set_enabled` method to `play`
- Particle system preview control panel
- Property editor for `Uuid` type.
- Restrict `Reflect` trait on `Debug`.
- Optional ability to `Copy Value as String` for properties in `Inspector` widget
- Pass animation signal name to animation event - makes much easier to respond to multiple animation events with the
  same name
- Ability to maximize ui windows
- `Animation::take_events`
- `Reflect::type_name`
- Show type name of selected object in the inspector
- Fixed multiple nodes parenting in the world viewer
- Apply grid snapping when instantiating a prefab
- Added range selection for tree widget (Shift + Click)
- Docking manager now collapses tiles when closing a docked window
- Improved search bar style in the world viewer
- Improved breadcrumbs in the world viewer
- `HotKey` + `KeyBinding` + respective property editors
- Ability to change editor controls.

# 0.28

- Preview for prefab instantiation.
- Drag preview nodes are now input-transparent.
- Expand/collapse trees by double click.
- Fixed move/rotate/scale gizmo behaviour for mouse events.
- Fixed fallback to defaults when editor's config is corrupted.
- Save `Track Selection` option in the editor's config.
- Clear breadcrumbs when changing scene in the editor.
- Fixed 1-frame delay issues in the editor.
- Emit MouseUp message before Drop message.
- Fixed UI "flashing" in the editor in some cases.
- Do not silently discard UI messages from nodes that were already be deleted.
- Show node handle in breadcrumbs in the editor.
- Provide direct read-only access to current dragging context in UI.
- Fixed crash when trying to select a node by invalid handle in the editor.
- Highlight invalid handles in the Inspector.
- Discard "leftover" debug geometry when undoing actions in the editor.
- Some menus in the editor now more intuitive now.
- Fixed critical bug with incorrect unpack alignment for RGB textures - this causes hard crash in some
  cases.
- Do not try to reload a resource if it is already loading.
- Ability to set desired frame rate for `Executor` (default is 60 FPS).
- Ability to paste editor's clipboard content to selected node (paste-as-child functionality).
- Ability to render into transparent window while keeping the transparency of untouched pixels (see
  `transparent` example).
- Ability to specify custom window builder in `Executor` + a way to disable vsync in `Executor`.
- `MultiBorrowContext` for `Pool` and `Graph::begin_multi_borrow`, helps you to borrow multiple mutable
  references to different items.
- Speed up code generation in proc-macros.
- Correctly map handles in instances after property inheritance (fixed weird bugs when handles to nodes
  in your scripts mapped to incorrect ones)
- Refactored script processing:
    - Added `ScriptTrait::on_start` - it is guaranteed to be called after all scripts in scene are initialized, useful
      when a script depends on some other script
    - Script processing is now centralized, not scattered as before.
    - More deterministic update path (`on_init` -> `on_start` -> `on_update` -> `on_destroy`)
- Fixed crash when modifying text in a text box via message and then trying to type something.
- `ButtonBuilder::with_text_and_font`
- Show node names in for fields of `Handle<Node>` fields of structs in the editor.
- Fixed crash in the editor when a script has resource field.
- Ability to clone behaviour trees.
- Automatic node handle mapping via reflection.
- Removed `ScriptTrait::remap_handles` method.
- Pass elapsed time to scripts.
- Do not call `ScriptTrait::on_os_event` if scene is disabled.
- Make world viewer filtering case-insensitive.
- Correctly set self handle and sender for graph's root node.
- `#[inline]` attributes for "hot" methods.
- Fixed panic when rigid body is a root node of a scene.
- `Base::has_script` + `Base::try_get_script` + `Base::try_get_script_mut` helper methods, it is now easier
  to fetch scripts on scene nodes.
- Ability to change selected node type in the editor (useful to change scene root type).
- Optimized script trait parameter passing, script context now passed by reference instead of value.
- Script context now have access to all plugins, which makes possible create cross plugin interaction.
- Removed requirement of scripts api to provide parent plugin's uuid.
- There is no more need to define uuid for plugins.
- Do not update scene scripts if it is disabled.
- `Graph::find_first_by_script` - helps you find a node by its script type.
- Added missing property editors for `Inspector` widget.
- Save editor's scene camera settings (position, orientation, zoom, etc.) per scene.
- Skip-chars list to be able to treat some chars like white space.
- Optional text shadow effect.
- Ctrl+Shift+Arrow to select text word-by-word in text box widget.
- Added navmesh settings to editor's settings panel.
- Make text box widget to accept text messages + special messages for text box widget.
- Set 500 ms double click interval (previously it was 750 ms).
- Fixed text selection in case of custom ui scaling.
- Fixed `TextBox::screen_pos_to_text_pos` - incorrect usage of `char_code` as index was leading to incorrect screen
  position to text position mapping.
- Ability to scroll text in the text box widget.
- `Rect::with_position` + `Rect::with_size` methods.
- Fixed caret position when removing text from text box in specific cases.
- Fixed crash when typing spaces at the end of text box with word wrap.
- Fixed caret position when adding text to the multiline text box widget.
- Fixed new line addition in the text box widget.
- Ability to select words (or whitespace spans) in the text box by double click.
- Emit double click after mouse down event (not before).
- Fixed caret blinking in the text box widget for various scenarios.
- Ctrl+LeftArrow and Ctrl+RightArrow to skip words in the text box widget.
- Allow setting caret position in the text box widget by clicking outside of line bounds.
- `raycast2d` example.
- Fixed text deletion in text box by `Delete` key + selection fixes.
- Fixed selection by Ctrl+Home, Ctrl+End in the text box widget.
- Fixed selected text highlighting in the text box widget.
- Fixed panic when Ctrl+C in a text box when selection is right-to-left.
- Ability to focus/unfocus a widget by sending a message.
- Added `TextBox` example.
- Removed `is_modified` flag from `PropertyInfo`.
- Ability to revert inherited property to parent's prefab value.
- Replaced manual property inheritance with reflection.
- Added `fields` and `fields_mut` for `Reflect` trait.
- Property inheritance for scripts.
- Ability to extract selection as a prefab.
- Fixed tooltips for complex properties in `Inspector` widget.
- Allow selecting build profile when running a game from the editor.
- `NodeHandle` wrapper to bypass some limitations of `Inspector` widget.
- Return result instead of unwrap and panic in `make_relative_path` - fixed some issues with symlinks in the
  editor.
- Added missing `Reflect` implementation for scripts made in `fyrox-template`.
- Added dependencies optimization for projects generated in `fyrox-template`.
- Provided access to some sound engine methods to plugins (`set_sound_gain` and `sound_gain`)
- Fixed style for ArrayPropertyEditor widget.
- Do not emit events for disabled animation signals.
- Sprite sheet animations with signals.
- Fixed terrain rendering - there's no more seams between layers with skybox content.
- Ability to set blending equation in draw parameters in the renderer.
- Ability to set blend function separately for RGB and Alpha in the renderer.
- Ignore invisible menus when closing menu chain by out-of-bounds click.
- Make some buttons in the editor smaller and less bright, add more tooltips.
- Use images for `Expand all`, `Collapse all`, `Locate Selection` buttons in world viewer.
- Fixed potential infinite loops when performing some math operations.
- Smoothing for cascaded shadow maps.
- Fixed script property editor - no more weird bugs in the editor when setting/editing/removing scripts from
  a node.
- Fixed cascaded shadow maps for directional lights.
- Added `Frustum::center` method.
- Fixed list of panels in `View` menu in the editor.
- Create tool tips for interaction modes hidden by default.
- Reload settings when reconfiguring the editor.
- Added list of recent scenes to `File` menu in the editor - makes easier to switch between most used scenes.
- Ability to add, remove, set items for `MenuItem` widget
- Correctly highlight selected interaction mode button
- More hotkeys for the editor
    - `[5]` - activate navmesh edit mode
    - `[6]` - activate terrain edit mode
- Ability to set `Selected` flag to `Decorator` widget on build stage
- Added `Invert drag` option for camera settings in the editor.
- Fixed incorrect rendering of `Luminance` and `LuminanceAlpha` textures.
- Fixed closing menus by clicking outside them.
- Direct access to all fields in all widgets.
- Force `TextBox` widget to consume all input messages, this fixes hot keys triggering in the editor while
  typing something in text fields.

# 0.27.1

- Fixed `Operation failed! Reason: Modify { value: dyn Reflect }` error.
- Fixed inability to edit properties of 2d collider shape
- Fixed inability to edit some properties of Joint2D.
- Added property editor for `Color Grading Lut` property of the Camera node.
- Fixed panic when editing cascades properties of directional light.
- Prevent panic when there's invalid bone handle.
- Hide `data` field from inspector for Surface, because it has no proper property editor.
- Fixed terrain layer deletion from the Inspector.

# 0.27

- Added compile-time reflection (huge thanks to [@toyboot4e](https://github.com/toyboot4e))
- Most editor commands were removed and replaced by universal command based on reflection.
- Backward compatibility for native engine data formats was dropped - use FyroxEd 0.13 to convert your scenes to newer
  version.
- Fixed panic when loading an FBX model with malformed animation curves (when there is only 1 or 2 components animated
  instead of 3, X and Y, but not Z for example).
- ABSM editor now have smaller default size and fits on small screens.
- Asset previewer now plays model animations
- Fixed critical FBX importer bug, that caused malformed animations.
- Ability to define "playable" time slice for animations.
- Fixed editor update rate, previously it was very high and that caused some weird issues.
- Proper support for all resource types in Inspector
- Show ABSM resources in the asset browser
- Ability to edit sound import options in the asset browser
- Dynamic type casting for script instances
- Provide access to parameters in ABSM
- Fixed transition instantiation in ABSM - it incorrectly handled "invert rule" flag.
- Prevent panic when deleting a node from script methods.
- Dynamic type casting for plugin instances
- Two-step ABSM instantiation - at first step you load all animations in parallel (async) and on second step you
  create actual ABSM instance.
- Wait for all resources to load before initialize scripts - this prevents panicking when trying to access
  not yet loaded resource in script methods.
- Default instantiation scaling options for 3D models - allows you to scale 3D models automatically on instantiation.
- Graph event broadcaster - allows you to receive `Added` and `Removed` events for nodes.
- Correctly initialize scripts of nodes that created at runtime.
- Component provider for scripts - allows you to provide access to inner script components via unified interface.
- Disable automatic texture compression - having compression enabled for all kinds of textures is not good, because
  there could be some textures with gradients, and they'll have significant distortion.
- `Pool::drain` - allows you to remove all objects from a pool while processing every object via closure.
- `Script::on_deinit` - allows you to execute any code for cleanup.
- Added `NodeHandleMap` - a small wrapper over map that have some methods that makes node handle mapping much
  shorter.
- Correctly handle missing properties in Inspector for various objects.
- Provide access to main application window from plugins.
- Allow chaining `ScriptConstructorContainer::add` calls
- Removed `screen_size` parameter from `UserInterface::new`
- Ability to remove render passes.
- Run the game in a separate process from the editor.
- Provide access to default engine's user interface instance for plugins.
- `--override-scene` parameter for Executor
- `ButtonContent` improvements - it is now possible to re-create button's text field using `ButtonMessage::Content`
- Provide access to control flow switch for plugins.
- `Plugin::on_ui_message`
- Two-step plugins initialization:
    - `PluginConstructor` trait defines a method that creates an instance of `Plugin` trait, instance of plugin
      constructor is used to create plugins on demand. It is needed because engine has deferred plugin initialization.
- `Framework` is removed, its functionality was merged with plugins.
- Simplified `ScriptConstructorContainer::add` definition, there were redundant generic parameters that just add
  visual clutter.
- Implemented `Clone+Debug` traits for `NavmeshAgent`
- Fixed spam in log in the editor when any file was changed.
- High DPI screens support for the editor.
- Newly created cameras in the editor are now enabled by default.
- Added "Preview" option for cameras in world viewer.
- Refactored joints:
    - Joints binding now is fully automatic and it is based on world transform of the joint, no need to manually
      set local frames.
    - Rebinding happens when a joint changes its position
    - Joints editing in the editor is now much more intuitive
- Improved debug visualization for physics.
- Read-only mode for NumericUpDown and Vec2/Vec3/Vec4 widgets
- Show global coordinates of current selection in the scene previewer
- BitField widget - it helps you to edit numbers as bit containers, allowing you to switch separate bits
- More compact editors for properties in Inspector
- NumericUpDown widget does not use word wrapping by default anymore
- CheckBox widget can now be switched only by left mouse button
- Ability to disable contacts between connected bodies of a joint
- `style` parameter for project template generator - it defines which scene will be used by default - either `2d`
  or `3d`
- Ability to select portion of the texture to render in `Rectangle` nodes.
- Ability to generate script skeleton for template generator
- HSL color model
- Ability to copy log enties to the clipboard
- `Log` API improvements
- Visualize cameras in the editor
- Context menu for asset items, it is now possible to open, delete, show-in-explorer items and also
  to copy file name and full file path to the clipboard.
- Visualize point and spot lights in the editor.

# 0.26

This release is mostly to fix critical bugs of 0.25 and add missing functionality that stops you from using scripting
system.

- Added project template generator
- Fixed invisible selected item in drop-down list widget.
- Correctly sync node names in `World Viewer`
- Reset editor's camera projection mode switch when creating new scene
- Fixed doubling scene entities in `World Viewer` when loading scene via `StartupData`
- More logging for renderer
- Fixed shader cache - now the engine won't re-compile shaders each 20 seconds.
- Temporarily disable `Lifetime` property editing because it causes crashes
- Do not show `dirty` flag of `Transform` in the `Inspector`
- Provide access to property editors container for editor's `Inspector` - it is now possible
  to register your own property editors
- Fixed panic when syncing `Inspector` for an entity with `Option<Texture>` field.
- Added `handle_object_property_changed` and `handle_collection_property_changed` macros to reduce
  boilerplate code in script property handling.
- Added ability to restore resource handles for scripts
- Fixed selection visualization in `Asset Browser`
- Validation for sky box cube map generator

## Migration guide

There are no breaking changes in this release.

# 0.25

- Static plugin system
- User-defined scripts
- Play mode for the editor
- Animation Blending State Machine (ABSM) editor.
- Some of sound entities were integrated in the scene graph.
- New `Sound` and `Listener` scene nodes.
- Sound buffer import options.
- `ResourceManager::request_sound_buffer` now accepts only path to sound buffer.
- Prefab inheritance improvements - now most of the properties of scene nodes are inheritable.
- Access to simulation properties of the physics.
- Engine and Resource manager are nonserializable anymore, check migration guide to find how to create
  save files in the correct way.
- `Node` enumeration was removed and replaced with dynamic dispatch. This allows you to define your own
  types of scene nodes.
- `Base` is not a scene node anymore, it was replaced with `Pivot` node (see migration guide for more info)
- `Base` now has `cast_shadows` property, respective property setters/getters was removed from `Mesh` and
  `Terrain` nodes.
- Ability to bring ListView item into view.
- Logger improvements: event subscriptions + collecting timestamps
- Log panel improvements in the editor: severity filtering, color differentiation.
- Scene nodes now have more or less correct local bounds (a bounding box that can fit the node).
- Improved picking in the editor: now it is using precise hit test against node's geometry.
- "Ignore back faces" option for picking in the editor: allows you to pick through "back" of polygon
  faces, especially useful for closed environment.
- Rotation ribbons were replaced with torus, it is much easier to select desired rotation mode.
- New material for gizmos in the editor, that prevent depth issues.
- New expander for TreeView widget, `V` and `>` arrows instead of `+` and `-` signs.
- ScrollBar widget is much thinner by default.
- Editor settings window now based on Inspector widget, which provides uniform way of data visualization.
- `DEFAULT_FONT` singleton was removed, it is replaced with `default_font`
- Shortcuts improvements in the editor.
- Overall UI performance improvements.
- Ability to disable clipping of widget bounds to parent bounds.
- Layout and render transform support for widgets - allows you to scale/rotate/translate widgets.
- Ability to make widget lowermost in hierarchy.
- Animation blending state machine refactoring, optimizations and stability improvements.
- Animation blending state machines are now stored in special container which stored in the Scene.
- Docking manager now shows anchors only for its windows.
- Model previewer now has much more intuitive controls.
- NumericUpDown don't panic anymore on edges of numeric bounds (i.e when trying to do `i32::MAX_VALUE + 1`)
- DoubleClick support for UI.
- Update rate fix for editor, it fixes annoying issue with flickering in text boxes.
- `UserInterface::hit_test_unrestricted` which performs hit test that is not restricted to current
  picking restriction stack.
- WASM renderer fixes.
- `Pool::try_free` which returns `Option<T>` on invalid handles, instead of panicking.
- Light source for model previewer
- Default skybox for editor and model previewer cameras
- `Color` API improvements.
- `#[reflect(expand)]` and `#[reflect(expand_subtree)]` were removed from `Inspect` proc-macro
- Correct field name generation for enum variants
- Ability to draw Bézier curves in the UI.
- Fix for navmesh agent navigation of multilayer navigational meshes.
- Improvements for serializer, now it allows you correctly recover from serialization errors.

## Migration guide

**WARNING:** This release **does not** provide legacy sound system conversion to new one, which means if
any of your scene had any sound, they will be lost!

Now there is limited access to `fyrox_sound` entities, there is no way to create sound contexts, sounds,
effects manually. You have to use respective scene nodes (`Sound`, `Listener`) and `Effect` from
`fyrox::scene::sound` module (and children modules).

### Nodes

Since `Node` enumeration was removed, there is a new way of managing nodes:

- `Node` now is just `Box<dyn NodeTrait>` wrapped in a new-type-struct.
- Pattern matching was replaced with `cast` and `cast_mut` methods.
- In addition to `cast/cast_mut` there are two more complex methods for polymorphism: `query_component_ref` and
  `query_component_mut` which are able to extract references to internal parts of the nodes. This now has only one
  usage - `Light` enumeration was removed and `PointLight`, `SpotLight`, `DirectionalLight` provides unified access
  to `BaseLight` component via `query_component_ref/query_component_mut`. `query_component` could be a bit slower,
  since it might involve additional branching while attempting to query component.
- `Base` node was replaced with `Pivot` node (and respective `PivotBuilder`), it happend due to problems with
  `Deref<Target = Base>/DerefMut` implementation, if `Base` is implementing `NodeTrait` then it must implement `Deref`
  but implementing `Deref` for `Base` causes infinite deref coercion loop.
- To be able to create custom scene nodes and having the ability to serialize/deserialize scene graph with such
  nodes, `NodeConstructorContainer` was added. It contains a simple map `UUID -> NodeConstructor` which allows to
  pick the right node constructor based on type uuid at deserialization stage.

#### Replacing `BaseBuilder` with `PivotBuilder`

It is very simply, just wrap `BaseBuilder` with a `PivotBuilder` and call `build` on `PivotBuilder` instance:

```rust
// Before
fn create_pivot_node(graph: &mut Graph) -> Handle<Node> {
    BaseBuilder::new().build(graph)
}

// After
fn create_pivot_node(graph: &mut Graph) -> Handle<Node> {
    PivotBuilder::new(BaseBuilder::new()).build(graph)
}
```

#### Pattern matching replacement

Pattern matching was replaced with 4 new methods `cast/cast_mut/query_component_ref/query_component_mut`:

```rust
fn set_sprite_color(node: &mut Node, color: Color) {
    // Use `cast_mut` when you are sure about the real node type.
    if let Some(sprite) = node.cast_mut::<Sprite>() {
        sprite.set_color(color);
    }
}

fn set_light_color(node: &mut Node, color: Color) {
    // Use query_component_mut if you unsure what is the exact type of the node.
    // In this example the `node` could be either PointLight, SpotLight, DirectionalLight,
    // since they're all provide access to `BaseLight` via `query_component_x` the function
    // will work with any of those types.
    if let Some(base_light) = node.query_component_mut::<BaseLight>() {
        base_light.set_color(color);
    }
}
```

### Listener

Now there is no need to manually sync position and orientation of the sound listener, all you need to do
instead is to create `Listener` node and attach it to your primary camera (or other scene node). Keep
in mind that the engine supports only one listener, which means that only one listener can be active
at a time. The engine will not stop you from having multiple listeners active, however only first (the
order is undefined) will be used to output sound.

### Sound sources

There is no more 2D/3D separation between sounds, all sounds in 3D by default. Every sound source now is
a scene node and can be created like so:

```rust
let sound = SoundBuilder::new(
BaseBuilder::new().with_local_transform(
TransformBuilder::new()
.with_local_position(position)
.build(),
),
)
.with_buffer(buffer.into())
.with_status(Status::Playing)
.with_play_once(true)
.with_gain(gain)
.with_radius(radius)
.with_rolloff_factor(rolloff_factor)
.build(graph);
```

Its API mimics `fyrox_sound` API so there should be now troubles in migration.

### Effects

Effects got a thin wrapper around `fyrox_sound` to make them compatible with `Sound` scene nodes, a reverb
effect instance can be created like so:

```rust
let reverb = ReverbEffectBuilder::new(BaseEffectBuilder::new().with_gain(0.7))
.with_wet(0.5)
.with_dry(0.5)
.with_decay_time(3.0)
.build( & mut scene.graph.sound_context);
```

A sound source can be attached to an effect like so:

```rust
graph
.sound_context
.effect_mut( self .reverb)
.inputs_mut()
.push(EffectInput {
sound,
filter: None,
});
```

### Filters

Effect input filters API remain unchanged.

### Engine initialization

`Engine::new` signature has changed to accept `EngineInitParams`, all previous argument were moved to the
structure. However, there are some new engine initialization parameters, like `serialization_context` and
`resource_manager`. Previously `resource_manager` was created implicitly, currently it has to be created
outside and passed to `EngineInitParams`. This is because of new `SerializationContext` which contains
a set of constructors for various types that may be used in the engine and be added by external plugins.
Typical engine initialization could look something like this:

```rust
use fyrox::engine::{Engine, EngineInitParams};
use fyrox::window::WindowBuilder;
use fyrox::engine::resource_manager::ResourceManager;
use fyrox::event_loop::EventLoop;
use std::sync::Arc;
use fyrox::engine::SerializationContext;

fn init_engine() {
    let evt = EventLoop::new();
    let window_builder = WindowBuilder::new()
        .with_title("Test")
        .with_fullscreen(None);
    let serialization_context = Arc::new(SerializationContext::new());
    let mut engine = Engine::new(EngineInitParams {
        window_builder,
        resource_manager: ResourceManager::new(serialization_context.clone()),
        serialization_context,
        events_loop: &evt,
        vsync: false,
    })
        .unwrap();
}
```

## Serialization

Engine and ResourceManager both are non-serializable anymore. It changes approach of creating save files in games.
Previously you used something like this (following code snippets are modified versions of `save_load` example):

```rust
const SAVE_FILE: &str = "save.bin";

fn save(game: &mut Game) {
    let mut visitor = Visitor::new();

    game.engine.visit("Engine", visitor)?; // This no longer works
    game.game_scene.visit("GameScene", visitor)?;

    visitor.save_binary(Path::new(SAVE_FILE)).unwrap();
}

fn load(game: &mut Game) {
    if Path::new(SAVE_FILE).exists() {
        if let Some(game_scene) = game.game_scene.take() {
            game.engine.scenes.remove(game_scene.scene);
        }

        let mut visitor = block_on(Visitor::load_binary(SAVE_FILE)).unwrap();

        game.engine.visit("Engine", visitor)?; // This no longer works
        game.game_scene.visit("GameScene", visitor)?;
    }
}
```

However, on practice this approach could lead to some undesirable side effects. The main problem with the old
approach is that when you serialize the engine, it serializes all scenes you have. This fact is more or less
ok if you have only one scene, but if you have two and more scenes (for example one for menu and one for
game level) it writes/reads redundant data. The second problem is that you cannot load saved games asynchronously
using the old approach, because it takes mutable access of the engine and prevents you from off-threading work.

The new approach is much more flexible and do not have such issues, instead of saving the entire state of the
engine, you just save and load only what you actually need:

```rust
const SAVE_FILE: &str = "save.bin";

fn save(game: &mut Game) {
    if let Some(game_scene) = game.game_scene.as_mut() {
        let mut visitor = Visitor::new();

        // Serialize game scene first.
        game.engine.scenes[game_scene.scene]
            .save("Scene", &mut visitor)
            .unwrap();
        // Then serialize the game scene.
        game_scene.visit("GameScene", &mut visitor).unwrap();

        // And call save method to write everything to disk.
        visitor.save_binary(Path::new(SAVE_FILE)).unwrap();
    }
}

// Notice that load is now async.
async fn load(game: &mut Game) {
    // Try to load saved game.
    if Path::new(SAVE_FILE).exists() {
        // Remove current scene first.
        if let Some(game_scene) = game.game_scene.take() {
            game.engine.scenes.remove(game_scene.scene);
        }

        let mut visitor = Visitor::load_binary(SAVE_FILE).await.unwrap();

        let scene = SceneLoader::load("Scene", &mut visitor)
            .unwrap()
            .finish(game.engine.resource_manager.clone())
            .await;

        let mut game_scene = GameScene::default();
        game_scene.visit("GameScene", &mut visitor).unwrap();

        game_scene.scene = game.engine.scenes.add(scene);
        game.game_scene = Some(game_scene);
    }
}
```

As you can see in the new approach you save your scene and some level data, and on load - you load the scene, add
it to the engine as usual and load level's data. The new approach is a bit more verbose, but it is much more
flexible.

# 0.24

## Engine

- 2D games support (with 2D physics as well)
- Three new scene nodes was added: RigidBody, Collider, Joint. Since rigid body, collider and joint are graph nodes
  now, it is possible to have complex hierarchies built with them.
- It is possible to attach rigid body to any node in scene graph, its position now will be correct in this case (
  previously it was possible to have rigid bodies attached only on root scene nodes).
- New `Inspector` widget + tons of built-in property editors (with the ability to add custom editors)
- `Inspect` trait + proc macro for lightweight reflection
- UI now using dynamic dispatch allowing you to add custom nodes and messages easily
- fyrox-sound optimizations (30% faster)
- Linear interpolation for sound samples when sampling rate != 1.0 (much better quality than before)
- Color fields in material editor now editable
- Window client area is now correctly filled by the renderer on every OS, not just Windows.
- NumericRange removal (replaced with standard Range + extension trait)
- Sort files and directories in FileBrowser/FileSelector widgets
- RawStreaming data source for sound
- Renderer performance improvements (2.5x times faster)
- UI layout performance improvements
- Prevent renderer from eating gigabytes of RAM
- Use `#[inline]` attribute to enable cross-crate inlining
- `ImmutableString` for faster hashing of static strings
- `SparseBuffer` as a lightweight analog for `Pool` (non-generational version)
- Support diffuse color in FBX materials
- Frustum culling fixes for terrain
- Shaders don't print empty lines when compiles successfully.
- `Pool` improvements
- Impl `IntoIterator` for references to `Pool`
- Cascaded shadow maps for directional light sources
- `spawn_at` + `spawn_at_handle` for `Pool`
- Preview for drag'n'drop
- `Grid` widget layout performance optimizations (**1000x** performance improvement - this is not a typo)
- `query_component` for UI widgets
- Curve resource
- Remove all associated widgets of a widget when deleting the widget (do not leave dangling objects)
- World bounding box calculation fix
- Heavy usage of invalidation in UI routines (prevents checking tons of widgets every frame)
- Migrate to `parking-lot` synchronization primitives
- Migrate to `FxHash` (faster hashing)
- `Log::verify` to log errors of `Result<(), Error`
- Custom scene node properties support
- `Alt+Click` prevents selection in `Tree` widget
- Ability to change camera projection (Perspective or Orthographic)
- Smart position selection for popups (prevents them from appearing outside screen bounds)
- High-quality mip-map generation using Lanczos filter.

## Editor

- `Inspector` widget integration, which allowed to remove tons of boilerplate code
- Middle mouse button camera dragging
- Q/E + Space to move camera up/down
- Working directory message is much less confusing now
- Ability to edit sound sources in the editor
- Checkerboard colorization fix in the world viewer
- Search in the world viewer
- Floating brush panel for terrain editor
- Editor camera has manual exposure (not affected by auto-exposure)
- Curve editor
- Automatically select an newly created instance of a scene node
- Grid snapping fix
- Angle snapping
- Edit properties of multiple selected objects at once.
- Context menu for scene items in world viewer
- `Create child` for scene item context menu
- Import options editor for asset browser
- Hot reload for textures.

## Breaking changes and migration guide

There are lots of breaking changes in this version, however all of them mostly related to the code and scenes made in
previous version _should_ still be loadable.

### Convert old scenes to new format

At first, install the rusty-editor from crates.io and run it:

```shell
cargo install rusty-editor
rusty-editor
```

And then just re-save your scenes one-by-one. After this all your scenes will be converted to the newest version.
Keep in mind that the editor from GitHub repo (0.25+) is not longer have backward compatibility/conversion code!

### 2D scenes

2D scene were completely removed and almost every 2D node were removed, there is only one "2D" node left - Rectangle.
2D now implemented in 3D scenes, you have to use orthographic camera for that. There is no migration guide for 2D scenes
because 2D had rudimentary support, and I highly doubt that there is any project that uses 2D of the engine.

## Resource management

Resource manager has changed its API and gained some useful features that should save you some time.

`request_texture` now accepts only one argument - path to texture, second argument was used to pass
`TextureImportOptions`. Import options now should be located in a separate options file. For example, you have a
`foo.jpg` texture and you want to change its import options (compression, wrapping modes, mip maps, etc.). To do this
you should create `foo.jpg.options` file in the same directory near your file with following content (each field is
optional):

```text
(
    minification_filter: LinearMipMapLinear,
    magnification_filter: Linear,
    s_wrap_mode: Repeat,
    t_wrap_mode: Repeat,
    anisotropy: 16,
    compression: NoCompression,
)
```

The engine will read this file when you'll call `request_texture` and it will apply the options on the first load.
This file is not mandatory, you can always set global import defaults in resource manage by calling
`set_texture_import_options`.

`request_model` have the same changes, there is only one argument and import options were moved to options file:

```text
(
    material_search_options: RecursiveUp
)
```

Again, all fields aren't mandatory and the entire file can be omitted, global import defaults can be set by calling
`set_model_import_options`.

### Physics

Old physics was replaced with new scene nodes: RigidBody, Collider, Joint. Old scenes will be automatically converted
on load, you should convert your scenes as soon as possible using the editor (open your scene and save it, that will
do the conversion).

Now there are two ways of adding a rigid body to a scene node:

- If you want your object to have a rigid body (for example a crate with box rigid body), your object must be
  **child** object of a rigid body. Graphically it can be represented like this:

```text
- Rigid Body
  - Crate3DModel
  - Cuboid Collider     
```

- If you want your object to have a rigid body that should move together with your object (to simulate hit boxes for
  example), then rigid body must be child object of your object. Additionally it should be marked as `Kinematic`,
  otherwise it will be affected by simulation (simply speaking it will fall on ground). Graphically it can be
  represented like this:

```text
- Limb
  - Rigid Body
     - Capsule Collider
```

#### Migration

This section will help you to migrate to new physics.

##### Rigid bodies

Rigid body and colliders now can be created like so:

```rust
use fyrox_impl::{
    core::{algebra::Vector3, pool::Handle},
    scene::{
        base::BaseBuilder,
        collider::{ColliderBuilder, ColliderShape},
        node::Node,
        rigidbody::RigidBodyBuilder,
        transform::TransformBuilder,
        Scene,
    },
};

fn create_capsule_rigid_body(scene: &mut Scene) -> Handle<Node> {
    RigidBodyBuilder::new(
        BaseBuilder::new()
            .with_local_transform(
                // To position, rotate rigid body you should use engine's transform.
                TransformBuilder::new()
                    .with_local_position(Vector3::new(1.0, 2.0, 3.0))
                    .build(),
            )
            .with_children(&[
                // It is very important to add at least one child collider node, otherwise rigid
                // body will not do collision response.
                ColliderBuilder::new(
                    BaseBuilder::new().with_local_transform(
                        // Colliders can have relative position to their parent rigid bodies.
                        TransformBuilder::new()
                            .with_local_position(Vector3::new(0.0, 0.5, 0.0))
                            .build(),
                    ),
                )
                    // Rest of properties can be set almost as before.
                    .with_friction(0.2)
                    .with_restitution(0.1)
                    .with_shape(ColliderShape::capsule_y(0.5, 0.2))
                    .build(&mut scene.graph),
            ]),
    )
        // Rest of properties can be set almost as before.
        .with_mass(2.0)
        .with_ang_damping(0.1)
        .with_lin_vel(Vector3::new(2.0, 1.0, 3.0))
        .with_ang_vel(Vector3::new(0.1, 0.1, 0.1))
        .build(&mut scene.graph)
}
```

##### Joints

Joints can be created in a similar way:

```rust
fn create_ball_joint(scene: &mut Scene) -> Handle<Node> {
    JointBuilder::new(BaseBuilder::new())
        .with_params(JointParams::BallJoint(BallJoint {
            local_anchor1: Vector3::new(1.0, 0.0, 0.0),
            local_anchor2: Vector3::new(-1.0, 0.0, 0.0),
            limits_local_axis1: Vector3::new(1.0, 0.0, 0.0),
            limits_local_axis2: Vector3::new(1.0, 0.0, 0.0),
            limits_enabled: true,
            limits_angle: 45.0,
        }))
        .with_body1(create_capsule_rigid_body(scene))
        .with_body2(create_capsule_rigid_body(scene))
        .build(&mut scene.graph)
}
```

##### Raycasting

Raycasting located in `scene.graph.physics`, there were almost no changes to it, except now it returns handles to
scene nodes instead of raw collider handles.

##### Contact info

Contact info can now be queried from the collider node itself, via `contacts()` method.

```rust
fn query_contacts(collider: Handle<Node>, graph: &Graph) -> impl Iterator<Item=ContactPair> {
    graph[collider].as_collider().contacts(&graph.physics)
}
```