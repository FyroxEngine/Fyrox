# Architecture

** WORK IN PROGRESS **

This document describes high-level architecture and basic concepts of Fyrox. It should help you to understand
basics of the engine's architecture and find a right place for your modifications.

## Overview

Fyrox is a monolithic game engine with very few replaceable parts. This means that Fyrox itself has relatively
strong coupling between modules. However, some of its parts can be used as standalone crates - core, UI and 
sound are independent of the engine. Internal coupling is one-way in most of the places, this means that, for
instance, a renderer **is** dependent on a scene, but scene does **not** know anything about the renderer.
This fact makes changes in the engine very easy even for beginners.

Fyrox consists of the four crates - fyrox-core, fyrox-sound, fyrox-ui, and Fyrox itself. fyrox-core, fyrox-sound and
fyrox-ui are **standalone** crates and can be used separately, the only place where these three are meet is the
Fyrox. Previously each crate had a separate repository, but then I decided to put everything in single repository
because it was too much of a pain to build any project that uses the engine.

Another important fact is that Fyrox **does not** use ECS, instead it uses generational arenas (pools in Fyrox's
terminology) for efficient memory management (fast allocation/deallocation, CPU cache efficiency). This means
that you are working with good old structures which are placed in contiguous memory block (pool). Once an
object was placed in a pool, you get a handle to the object which can be used to access (borrow) the object
when you need. Such approach allows you to make any relations between the objects - handle is just a pair of 
numbers, it won't cause issues with borrow checker. For more info check 
[pool.rs](https://github.com/FyroxEngine/Fyrox/blob/master/fyrox-core/src/pool.rs).

### Core

Core contains some of very common algorithms and data structures that are used across other parts of the engine.
It contains linear algebra, accelerating structures, color-space functions, etc. In other words it contains 
"building blocks" that are very widely used across other parts of the engine. 

### Renderer

Fyrox uses a combination of deferred + forward renderers. The deferred renderer is used to render opaque objects,
when the forward renderer is used to render transparent objects. The renderer provides lots of very common 
graphical effects. The renderer is suitable for most of the needs, however it is not flexible enough yet and 
there is no way of using custom shaders yet.

### User Interface

Fyrox uses custom user interface library. It is node-based, has very powerful layout system, uses messages
to communicate between widgets, supports styling. The library has 30+ widgets (including docking manager,
windows, file browsers, etc). Please keep in mind that the library does not render anything, instead it
just prepares a set of drawing commands which can be used with any kind of renderer - a software (GDI for
instance) or a hardware (OpenGL, DirectX, Vulkan, Metal, etc.).

### Sound

Fyrox uses software sound engine [fyrox-sound](https://github.com/FyroxEngine/Fyrox/tree/master/fyrox-sound).
The sound engine provides support for binaural sound rendering using HRTF, which gives excellent sound 
spatialization. 

## Code Map

Code map should help you find a right place for your modifications. This is the most boring part of the 
document, here is the table of contents for your comfort:

- [fyrox-core](#fyrox-core)
    - [math](#mathmodrs)
- [fyrox-ui](#fyrox-ui)
    - [widgets](#borderrs)
- [fyrox-sound](#fyrox-sound)
    - [buffer](#buffermodrs)
    - [decoder](#decodermodrs)
    - [device](#devicemodrs)
- [Fyrox](#fyrox)

### fyrox-core

As it was already said up above, fyrox-core is just a set of useful algorithms. If you want to add a thing
that will be used in dependent crates then you're in the right place. Here is the very brief description
of each module.

#### math/mod.rs

The module contains some intersection check algorithms, vector projection methods (tri-planar mapping for 
example), generic rectangle struct, and other stuff that cannot be easily classified.

#### math/aabb.rs

The module contains Axis-Aligned Bounding Box (AABB) structure. It is used as a bounding volume to accelerate
spatial checks (like ray casting, coarse intersection checks, etc).

#### math/frustum.rs

The module contains Frustum structure. Its purpose (in the project) is to perform visibility checks - for
example camera is using frustum culling to determine which objects must be rendered on screen. 

#### math/plane.rs

The module contains Plane structure that basically represents just plane equation. Planes are used for 
intersection tests.

#### math/ray.rs

The module contains Ray structure which is used for intersection tests. For example the engine uses rays
in lightmapper to do ray tracing to calculate shadows.

#### math/triangulator.rs

The module contains a set of triangulation algorithms for polygon triangulation. There are two algorithms:
simple one is for quadrilaterals, and generic one is the ear-clipping algorithm. The stuff from the module is
used to triangulate polygons in 3d models to make them suitable to render on GPU.

#### color.rs

The module contains structure and methods to work with colors in HSV and RGB color spaces, and to convert 
colors HSV <-> RGB.

#### color_gradient.rs

The module contains simple linear color gradient with multiple points. It is widely used in particle systems
to change color of particles over time. For example a spark starts from white color and becomes more red over
time and finally becomes black.

#### lib.rs

The module contains BiDirHashMap and very few other algorithms.

#### numeric_range.rs

The module contains fool-proof numeric range - there is no way to create a range with incorrect bounds - bounds
will be determined at the sampling moment.

#### octree.rs

The module contains Octree acceleration structure which is used to accelerate ray casting, point-probing, 
box intersection tests and so on.

#### pool.rs

The module contains the heart of the engine: pool is one of the most commonly used structure in the engine.
Its purpose is to provide a storage for objects of a type in a contiguous block of memory. Any object
can be accessed later by a handle. Handles are some sort of indices, but with additional information that 
is used to check if handle is valid.

#### profiles.rs

The module contains a simple intrusive profiler. It uses special macro (scope_profile!()) to time a scope.

#### rectpack.rs

The module contains rectangle packing algorithm (most commonly known as "bin-packing problem"). It is used
to create texture atlases.

#### visitor.rs

The module contains node-based serializer/deserializer (visitor). Everything in the engine serialized by 
this serializer. It supports serialization of basic types, many std types (including
Rc/Arc) and user-defined types.

### fyrox-ui

fyrox-ui is a standalone, graphics API-agnostic, node-based, general-purpose user interface library.

#### lib.rs

The module contains UserInterface structure and Control trait. 

#### border.rs

The module contains Border widget which is basically just a rectangle with variable width of borders, and
an ability to be a container for child widgets.

#### brush.rs

The module contains Brush structure which describes a way of drawing graphics primitives. 

#### button.rs

The module contains Button widget and its builder.

#### canvas.rs

The module contains Canvas widget and its builder. Canvas is a simple container for child widgets,
it allows setting position of children widgets directly. 

#### check_box.rs

The module contains CheckBox widget and its builder. CheckBox is a three-state (checked, unchecked,
undefined) switch.

#### color.rs

The module contains widgets to work with colors and their builders. There are separate widgets to change
hue, saturation, brightness and compound color selector widget.

#### decorator.rs

The module contains Decorator widget and its builder. Decorator is a simple container for child widgets,
it has built-in behaviour for most common actions: it changes appearance when mouse enters or leaves bounds,
when user clicks on it, etc. 

#### dock.rs

The module contains DockingManager and Tile widgets and their builders. Docking manager is able to combine
multiple Window widgets in its tiles, each tile can be resized, docked or undocked. This is the must-have
widget for any kind of editor.

#### draw.rs

The module is responsible for "drawing". It is in quotes, because it does not actually draw anything, it just 
emits drawing commands in a queue for later interpretation. 

#### dropdown_list.rs

The module contains DropdownList widget and its builder. DropdownList is a small field that shows selected
item and a "popup" that contains list of items.

#### expander.rs

The module contains Expander widget and its builder. Expander is a collapsable container for child widgets
with a field that describes a content. 

#### file_browser.rs

The module contains a set of widgets that displays file system. FileBrowser widget is a simple file system
tree, FileSelector is a window with FileBrowser and few buttons. 

#### formatted_text.rs

The module is responsible for text formatting and "rendering". The latter is in quotes, because the library 
just uses glyph info from a font to layout each glyph in a line of text with word wrapping and other 
useful features (like text size calculation, etc).

#### grid.rs

The module contains Grid widget and its builder. Grid is a powerful layout widget, it allows arranging child
widgets in a series of rows and columns.

#### image.rs

The module contains Image widget and its builder. Image is a simple rectangle with a texture.

#### list_view.rs

The module contains ListView widget and its builder. ListView is a container for items which arranged 
as a stack. ListView supports item selection.

#### menu.rs

The module contains menu widgets. Menu here means a classic menu which is a strip with root items and a 
set of child sub-menus.

#### message.rs

The module contains all supported messages for every widget in the library. 

#### messagebox.rs

#### node.rs

#### numeric.rs

#### popup.rs

#### progress_bar.rs

#### scroll_bar.rs

#### scroll_panel.rs

#### scroll_viewer.rs

#### stack_panel.rs

#### tab_control.rs

#### text.rs

#### tree.rs

#### ttf.rs

#### utils.rs

#### vec.rs

#### vector_image.rs

#### widget.rs

#### window.rs

#### wrap_panel.rs

### fyrox-sound

fyrox-sound is a standalone sound engine with multiple renderers and high-quality sound. The sound engine
provides support for binaural sound rendering using HRTF, which gives excellent sound spatialization.

#### buffer/mod.rs

#### buffer/generic.rs

#### buffer/streaming.rs

#### decoder/mod.rs

#### decoder/vorbis.rs

#### decoder/wav.rs

#### device/mod.rs

#### device/alsa.rs

#### device/coreaudio.rs

#### device/dsound.rs

#### device/dummy.rs

### Fyrox

The engine itself. It has: a renderer, resource manager, animations, scenes, and various utilities like 
lightmapper, uv-mapper, navigation mesh, logger, pathfinder.

#### animation/mod.rs
#### animation/machine/mod.rs
#### animation/machine/blend_nodes.rs
#### engine/mod.rs
#### engine/error.rs
#### engine/resource_manager.rs
#### renderer/mod.rs
#### renderer/framework/mod.rs
#### renderer/framework/framebuffer.rs
#### renderer/framework/geometry_buffer.rs