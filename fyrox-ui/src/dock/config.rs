use crate::{
    core::{pool::Handle, uuid::Uuid, visitor::prelude::*},
    dock::{Tile, TileContent},
    Orientation, UiNode, UserInterface,
};

#[derive(Debug, PartialEq, Clone, Default)]
pub struct SplitTiles {
    splitter: f32,
    orientation: Orientation,
    children: [Box<ConfigEntry>; 2],
}

// Rust trait solver is dumb, it overflows when Visit is derived. To bypass this, we need to
// implement this manually.
impl Visit for SplitTiles {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        self.splitter.visit("Splitter", &mut region)?;
        self.orientation.visit("Orientation", &mut region)?;
        self.children.visit("Children", &mut region)?;

        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone, Visit, Default)]
pub enum ConfigEntry {
    #[default]
    Empty,
    Window(Uuid),
    SplitTiles(SplitTiles),
}

impl ConfigEntry {
    pub fn from_tile(tile: &Tile, ui: &UserInterface) -> Self {
        match tile.content {
            TileContent::Empty => Self::Empty,
            TileContent::Window(window) => {
                Self::Window(ui.try_get_node(window).map(|w| w.id).unwrap_or_default())
            }
            TileContent::VerticalTiles { splitter, tiles } => Self::SplitTiles(SplitTiles {
                splitter,
                orientation: Orientation::Vertical,
                children: Self::from_tile_handle_slice(tiles, ui),
            }),
            TileContent::HorizontalTiles { splitter, tiles } => Self::SplitTiles(SplitTiles {
                splitter,
                orientation: Orientation::Horizontal,
                children: Self::from_tile_handle_slice(tiles, ui),
            }),
        }
    }

    fn from_tile_handle(handle: Handle<UiNode>, ui: &UserInterface) -> Self {
        ui.try_get_node(handle)
            .and_then(|t| t.query_component::<Tile>())
            .map(|t| Self::from_tile(t, ui))
            .unwrap_or_else(|| Self::Empty)
    }

    fn from_tile_handle_slice(slice: [Handle<UiNode>; 2], ui: &UserInterface) -> [Box<Self>; 2] {
        [
            Box::new(Self::from_tile_handle(slice[0], ui)),
            Box::new(Self::from_tile_handle(slice[1], ui)),
        ]
    }
}
