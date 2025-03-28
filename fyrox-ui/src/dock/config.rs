// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::{
    core::{algebra::Vector2, log::Log, pool::Handle, visitor::prelude::*, ImmutableString},
    dock::{Tile, TileBuilder, TileContent},
    message::MessageDirection,
    widget::WidgetBuilder,
    window::WindowMessage,
    Orientation, UiNode, UserInterface,
};
use fyrox_graph::{BaseSceneGraph, SceneGraph};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Default, Serialize, Deserialize)]
pub struct SplitTilesDescriptor {
    pub splitter: f32,
    pub orientation: Orientation,
    pub children: [Box<TileDescriptor>; 2],
}

// Rust trait solver is dumb, it overflows when Visit is derived. To bypass this, we need to
// implement this manually.
impl Visit for SplitTilesDescriptor {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        self.splitter.visit("Splitter", &mut region)?;
        self.orientation.visit("Orientation", &mut region)?;
        self.children.visit("Children", &mut region)?;

        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone, Default, Serialize, Deserialize, Visit)]
pub struct MultiWindowDescriptor {
    pub index: u32,
    pub names: Vec<ImmutableString>,
}

impl MultiWindowDescriptor {
    pub fn has_window(&self, name: &str) -> bool {
        self.names.iter().map(|n| n.as_str()).any(|n| n == name)
    }
}

#[derive(Debug, PartialEq, Clone, Visit, Default, Serialize, Deserialize)]
pub enum TileContentDescriptor {
    #[default]
    Empty,
    Window(ImmutableString),
    MultiWindow(MultiWindowDescriptor),
    SplitTiles(SplitTilesDescriptor),
}

impl TileContentDescriptor {
    pub fn from_tile(tile_content: &TileContent, ui: &UserInterface) -> Self {
        match tile_content {
            TileContent::Empty => Self::Empty,
            TileContent::Window(window) => Self::Window(
                ui.try_get(*window)
                    .map(|w| w.name.clone())
                    .unwrap_or_default(),
            ),
            TileContent::MultiWindow { index, windows } => {
                Self::MultiWindow(MultiWindowDescriptor {
                    index: *index,
                    names: windows
                        .iter()
                        .map(|window| {
                            ui.try_get(*window)
                                .map(|w| w.name.clone())
                                .unwrap_or_default()
                        })
                        .collect(),
                })
            }
            TileContent::VerticalTiles { splitter, tiles } => {
                Self::SplitTiles(SplitTilesDescriptor {
                    splitter: *splitter,
                    orientation: Orientation::Vertical,
                    children: TileDescriptor::from_tile_handle_slice(tiles, ui),
                })
            }
            TileContent::HorizontalTiles { splitter, tiles } => {
                Self::SplitTiles(SplitTilesDescriptor {
                    splitter: *splitter,
                    orientation: Orientation::Horizontal,
                    children: TileDescriptor::from_tile_handle_slice(tiles, ui),
                })
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone, Visit, Default, Serialize, Deserialize)]
pub struct TileDescriptor {
    pub content: TileContentDescriptor,
}

impl TileContentDescriptor {
    pub fn has_window(&self, window: &str) -> bool {
        match self {
            TileContentDescriptor::Empty => false,
            TileContentDescriptor::Window(window_name) => window_name.as_str() == window,
            TileContentDescriptor::MultiWindow(windows) => windows.has_window(window),
            TileContentDescriptor::SplitTiles(tiles) => {
                for tile in tiles.children.iter() {
                    if tile.content.has_window(window) {
                        return true;
                    }
                }
                false
            }
        }
    }
}

fn find_window(
    window_name: &ImmutableString,
    ui: &mut UserInterface,
    windows: &[Handle<UiNode>],
) -> Handle<UiNode> {
    if window_name.is_empty() {
        Log::warn(
            "Window name is empty, wrong widget will be used as a \
        tile content. Assign a unique name to the window used in a docking \
        manager!",
        );
    }

    let window_handle = ui.find_handle(ui.root(), &mut |n| n.name == *window_name);

    if window_handle.is_none() {
        for other_window_handle in windows.iter().cloned() {
            if let Some(window_node) = ui.try_get(other_window_handle) {
                if &window_node.name == window_name {
                    return other_window_handle;
                }
            }
        }
    }
    window_handle
}

impl TileDescriptor {
    pub(super) fn from_tile_handle(handle: Handle<UiNode>, ui: &UserInterface) -> Self {
        ui.try_get(handle)
            .and_then(|t| t.query_component::<Tile>())
            .map(|t| Self {
                content: TileContentDescriptor::from_tile(&t.content, ui),
            })
            .unwrap_or_default()
    }

    fn from_tile_handle_slice(slice: &[Handle<UiNode>; 2], ui: &UserInterface) -> [Box<Self>; 2] {
        [
            Box::new(Self::from_tile_handle(slice[0], ui)),
            Box::new(Self::from_tile_handle(slice[1], ui)),
        ]
    }

    pub fn create_tile(
        &self,
        ui: &mut UserInterface,
        windows: &[Handle<UiNode>],
    ) -> Handle<UiNode> {
        TileBuilder::new(WidgetBuilder::new())
            .with_content(match &self.content {
                TileContentDescriptor::Empty => TileContent::Empty,
                TileContentDescriptor::Window(window_name) => {
                    let window_handle = find_window(window_name, ui, windows);
                    if window_handle.is_some() {
                        ui.send_message(WindowMessage::open(
                            window_handle,
                            MessageDirection::ToWidget,
                            false,
                            true,
                        ));

                        TileContent::Window(window_handle)
                    } else {
                        TileContent::Empty
                    }
                }
                TileContentDescriptor::MultiWindow(MultiWindowDescriptor { index, names }) => {
                    let handles = names
                        .iter()
                        .map(|n| find_window(n, ui, windows))
                        .filter(|h| h.is_some())
                        .collect::<Vec<_>>();
                    match handles.len() {
                        0 => TileContent::Empty,
                        1 => TileContent::Window(handles[0]),
                        _ => TileContent::MultiWindow {
                            index: *index,
                            windows: handles,
                        },
                    }
                }
                TileContentDescriptor::SplitTiles(split_tiles) => match split_tiles.orientation {
                    Orientation::Vertical => TileContent::VerticalTiles {
                        splitter: split_tiles.splitter,
                        tiles: [
                            split_tiles.children[0].create_tile(ui, windows),
                            split_tiles.children[1].create_tile(ui, windows),
                        ],
                    },
                    Orientation::Horizontal => TileContent::HorizontalTiles {
                        splitter: split_tiles.splitter,
                        tiles: [
                            split_tiles.children[0].create_tile(ui, windows),
                            split_tiles.children[1].create_tile(ui, windows),
                        ],
                    },
                },
            })
            .build(&mut ui.build_ctx())
    }
}

#[derive(Debug, PartialEq, Clone, Visit, Default, Serialize, Deserialize)]
pub struct FloatingWindowDescriptor {
    pub name: ImmutableString,
    pub position: Vector2<f32>,
    pub size: Vector2<f32>,
    #[serde(default = "default_is_open")]
    pub is_open: bool,
}

fn default_is_open() -> bool {
    true
}

#[derive(Debug, PartialEq, Clone, Visit, Default, Serialize, Deserialize)]
pub struct DockingManagerLayoutDescriptor {
    pub floating_windows: Vec<FloatingWindowDescriptor>,
    pub root_tile_descriptor: Option<TileDescriptor>,
}

impl DockingManagerLayoutDescriptor {
    pub fn has_window<S: AsRef<str>>(&self, window: S) -> bool {
        self.root_tile_descriptor
            .as_ref()
            .is_some_and(|desc| desc.content.has_window(window.as_ref()))
    }
}
