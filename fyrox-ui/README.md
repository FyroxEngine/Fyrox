# fyrox-ui

Retained mode, general purpose, graphics API agnostic user interface library. Inspired by WPF.

**NOTE:** even though this crate has `fyrox` prefix in its name, it can be used separately without any issues.

## Features

- More than 28 widgets
- Full TTF/OTF fonts support
- Powerful layout system
- Fully customizable - you can construct visual trees of any complexity: for example a tree view item can have any sub-widgets as contents.
- GAPI-agnostic - this crate does not know anything about the rendering backend: it can be OpenGL, DirectX, Vulkan, Metal, or even built-in OS drawing API.
- OS-agnostic - similar look of all widgets across all operating systems and window managers.
- Extendable - full support of user-defined widgets.

## Widgets
- [x] Button
- [x] Border
- [x] Canvas
- [x] Color picker
- [x] Color field
- [x] Check box
- [x] Decorator
- [x] Drop-down list
- [x] Grid
- [x] Image
- [x] List view
- [x] Popup
- [x] Progress bar
- [x] Scroll bar
- [x] Scroll panel
- [x] Scroll viewer
- [x] Stack panel
- [x] Tab control
- [x] Text
- [x] Text box
- [x] Tree
- [x] Window
- [x] File browser
- [x] File selector
- [x] Docking manager
- [x] NumericUpDown
- [x] Vector2/Vector3/Vector4 editor
- [x] Quaternion editor
- [x] Menu
- [x] Menu item
- [x] Message box
- [x] Wrap panel
- [x] Curve editor
- [x] Bit Field
- [x] User defined widget
- [x] Inspector

## Limitations

- Since this library is OS-, GAPI-agnostic it cannot create native OS' windows, and it cannot render anything on screen. Instead, it uses an internal draw buffer which holds a list of commands, which has to be interpreted in your game/app. This is very a flexible way, but it has some limitations: multiwindow (native) configuration is hard to implement, you have to implement your own UI renderer what can be difficult if you not familiar with anything like this.
- There is still no keyboard navigation, it is planned but not with high priority.
- No support for right-to-left text (arabic, hebrew, etc.)

## Performance

- In general fyrox-ui is fast, however it can be slow if used incorrectly. Since this library uses a very complex layout system, it may work slow if there are lots of ui elements being moved (i.e. when scrolling). Hopefully it has built-in layout caching system, and it relies on layout invalidation, so it won't do layout calculations each frame - only if something significant changed (position, size, etc.).
- Batching of render commands can be difficult, because this library extensively uses clipping, and each clipping geometry has to be drawn into the stencil buffer as separate draw call. Rendering still has to be optimized, it is inefficient for now.

## Styling

fyrox-ui uses a bit unusual way of styling - you have to replace entire sub-graphs of widget's visual trees. What does that mean? fyrox-ui uses graph to build visual trees of any complexity, each widget is a set of nodes in the graph. For example a button is a set of background and foreground widgets, background widget usually defines appearance and foreground shows a content. Content of a button can be any widget, in most common cases it is either a text or an image. So to change appearance of a button you have to define your own background widget at the building stage, by default fyrox-ui uses Decorator widget which just changes its foreground brush when it receives MouseEnter, MouseLeave, etc. message. This fact significantly complicates **minor** styling (like change a color), but it is super flexible approach and allows to build your own unique style. Most of widget builders provides a way to change its parts, some of them still may lack such functionality, but this should eventually be fixed.

## Screenshots

![editor](https://raw.githubusercontent.com/FyroxEngine/Fyrox/master/pics/editor.png)
![absm editor](https://fyrox.rs/assets/absm_editor_full.png)
![sound](https://fyrox.rs/assets/reverb_properties.png)

## Contributing

- Writing a user interface library is very challenging for one person, so any help is appreciated.

## Documentation

TODO.

## Samples 

TODO.

There are two projects using this UI library:

- [Fyroxed](https://github.com/FyroxEngine/Fyrox/)
- [rusty-shooter](https://github.com/mrDIMAS/rusty-shooter)

However, it can be too difficult to understand how to use the library from those projects, so standalone samples should be added. This is still a TODO.
