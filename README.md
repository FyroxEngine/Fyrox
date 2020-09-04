# rg3d-ui

Retained mode, general purpose, graphics API agnostic user interface library. Inspired by WPF.

## Widgets
- [x] Button
- [x] Border
- [x] Canvas
- [x] Check box
- [x] Decorator
- [x] Drop-down list
- [x] Grid
- [x] Image
- [x] List view
- [x] Popup
- [x] Progress bar
- [x] Scroll bar
- [x] Scroll content presenter
- [x] Scroll viewer
- [x] Stack panel
- [x] Tab control
- [x] Text
- [x] Text box
- [x] Tree
- [x] Window
- [x] File browser
- [x] Docking manager
- [x] NumericUpDown
- [x] Vec3 editor
- [x] Menu
- [x] Message box
- [x] Wrap panel

## Limitations

- Since this library is OS-, GAPI-agnostic it cannot create native OS' windows and it cannot render anything on screen. Instead it uses internal draw buffer which holds list of commands, which has to be interpreted in your game/app. This is very flexible way, but it has some limitations: multiwindow (native) configuration is hard to implement, you have to implement your own UI renderer what can be difficult if you not familiar with anything like this.
- There is still no keyboard navigation, it is planned but not with high priority.

## Performance

- In general rg3d-ui is fast, however it can be slow if used incorrectly. Since this library uses very complex layout system, it may work slow if there are lots of ui elements being moved (i.e. when scrolling). Hopefully it has built-in layout caching system and it relies on layout invalidation so it won't do layout calculations each frame - only if something significant changed (position, size, etc.).
- Batching of render commands can be difficult, because the library extensively uses clipping, and each clipping geometry has to be drawn into stencil buffer as separate draw call. Rendering still has to be optimized, it is inefficient for now.

## Documentation

TODO.

## Samples 

TODO.

There are two projects uses this UI library: 

- [rusty-editor](https://github.com/mrDIMAS/rusty-editor/)
- [rusty-shooter](https://github.com/mrDIMAS/rusty-shooter)

However it can be too difficult to understand how to use library from those projects, so standalone samples should be added. This is still a TODO.

