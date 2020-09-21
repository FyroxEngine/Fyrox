# rg3d-ui

Retained mode, general purpose, graphics API agnostic user interface library. Inspired by WPF.

## Features

- More than 28 widgets
- Full TTF/OTF fonts support
- Powerful layout system
- Fully customizable - you can construct visual tree of any complexity: for example tree view item can have any sub-widgets as content.
- GAPI-agnostic - crate does not know anything about rendering backend: it can be OpenGL, DirectX, Vulkan, Metal, or even built-in OS drawing API.
- OS-agnostic - similar look of all widgets across all operation systems and window managers.
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
- [x] Vec3 editor
- [x] Menu
- [x] Menu item
- [x] Message box
- [x] Wrap panel
- [x] User defined widget

## Limitations

- Since this library is OS-, GAPI-agnostic it cannot create native OS' windows and it cannot render anything on screen. Instead it uses internal draw buffer which holds list of commands, which has to be interpreted in your game/app. This is very flexible way, but it has some limitations: multiwindow (native) configuration is hard to implement, you have to implement your own UI renderer what can be difficult if you not familiar with anything like this.
- There is still no keyboard navigation, it is planned but not with high priority.
- No support for right-to-left text (arabic, hebrew, etc.)

## Performance

- In general rg3d-ui is fast, however it can be slow if used incorrectly. Since this library uses very complex layout system, it may work slow if there are lots of ui elements being moved (i.e. when scrolling). Hopefully it has built-in layout caching system and it relies on layout invalidation so it won't do layout calculations each frame - only if something significant changed (position, size, etc.).
- Batching of render commands can be difficult, because the library extensively uses clipping, and each clipping geometry has to be drawn into stencil buffer as separate draw call. Rendering still has to be optimized, it is inefficient for now.

## Screenshots

[![editor](https://raw.githubusercontent.com/mrDIMAS/rusty-editor/master/screenshots/1.png)](https://github.com/mrDIMAS/rusty-editor/)

## Contributing

- Writing user interface library is very challenging for one person, so any help is appreciated.

## Documentation

TODO.

## Samples 

TODO.

There are two projects uses this UI library: 

- [rusty-editor](https://github.com/mrDIMAS/rusty-editor/)
- [rusty-shooter](https://github.com/mrDIMAS/rusty-shooter)

However, it can be too difficult to understand how to use library from those projects, so standalone samples should be added. This is still a TODO.