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

#[derive(Debug, PartialEq, Clone, Visit, Default, Serialize, Deserialize)]
pub enum TileContentDescriptor {
    #[default]
    Empty,
    Window(ImmutableString),
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
                    if window_name.is_empty() {
                        Log::warn(
                            "Window name is empty, wrong widget will be used as a \
                        tile content. Assign a unique name to the window used in a docking \
                        manager!",
                        );
                    }

                    let mut window_handle =
                        ui.find_handle(ui.root(), &mut |n| n.name == *window_name);

                    if window_handle.is_none() {
                        for other_window_handle in windows.iter().cloned() {
                            if let Some(window_node) = ui.try_get(other_window_handle) {
                                if &window_node.name == window_name {
                                    window_handle = other_window_handle;
                                }
                            }
                        }
                    }

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
}

#[derive(Debug, PartialEq, Clone, Visit, Default, Serialize, Deserialize)]
pub struct DockingManagerLayoutDescriptor {
    pub floating_windows: Vec<FloatingWindowDescriptor>,
    pub root_tile_descriptor: Option<TileDescriptor>,
}
