use crate::{
    command::{CommandContext, CommandTrait},
    fyrox::{
        core::{algebra::Vector2, log::Log, Uuid},
        scene::tilemap::{
            brush::{BrushTile, TileMapBrushResource},
            tileset::{TileDefinition, TileDefinitionHandle, TileSetResource},
        },
    },
};

#[derive(Debug)]
pub struct AddTileCommand {
    pub tile_set: TileSetResource,
    pub tile: Option<TileDefinition>,
    pub handle: TileDefinitionHandle,
}

impl CommandTrait for AddTileCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Add Tile".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.handle = self
            .tile_set
            .data_ref()
            .tiles
            .spawn(self.tile.take().unwrap());
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.tile = self.tile_set.data_ref().tiles.try_free(self.handle);
    }
}

#[derive(Debug)]
pub struct RemoveTileCommand {
    pub tile_set: TileSetResource,
    pub handle: TileDefinitionHandle,
    pub tile: Option<TileDefinition>,
}

impl CommandTrait for RemoveTileCommand {
    fn name(&mut self, _text: &dyn CommandContext) -> String {
        "Remove Tile".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.tile = self.tile_set.data_ref().tiles.try_free(self.handle);
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.handle = self
            .tile_set
            .data_ref()
            .tiles
            .spawn(self.tile.take().unwrap());
    }
}

#[derive(Debug)]
pub struct SetBrushTilesCommand {
    pub brush: TileMapBrushResource,
    pub tiles: Vec<BrushTile>,
}

impl SetBrushTilesCommand {
    fn swap(&mut self) {
        std::mem::swap(&mut self.brush.data_ref().tiles, &mut self.tiles);
        Log::verify(self.brush.save_back());
    }
}

impl CommandTrait for SetBrushTilesCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Set Brush Tiles".to_string()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }
}

#[derive(Debug)]
pub struct MoveBrushTilesCommand {
    pub brush: TileMapBrushResource,
    pub positions: Vec<(Uuid, Vector2<i32>)>,
}

impl MoveBrushTilesCommand {
    fn swap(&mut self) {
        let mut brush = self.brush.data_ref();
        for (id, pos) in self.positions.iter_mut() {
            if let Some(index) = brush.tiles.iter_mut().position(|tile| tile.id == *id) {
                let tile = &mut brush.tiles[index];
                std::mem::swap(pos, &mut tile.local_position);
            }
        }
        drop(brush);
        Log::verify(self.brush.save_back());
    }
}

impl CommandTrait for MoveBrushTilesCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Move Brush Tiles".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap()
    }
}

#[derive(Debug)]
pub struct RemoveBrushTileCommand {
    pub brush: TileMapBrushResource,
    pub id: Uuid,
    pub tile: Option<BrushTile>,
}

impl CommandTrait for RemoveBrushTileCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Remove Brush Tile".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        let mut brush = self.brush.data_ref();
        let index = brush
            .tiles
            .iter()
            .position(|tile| tile.id == self.id)
            .unwrap();
        self.tile = Some(brush.tiles.remove(index));
        drop(brush);
        Log::verify(self.brush.save_back());
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.brush.data_ref().tiles.push(self.tile.take().unwrap());
        Log::verify(self.brush.save_back());
    }
}

#[derive(Debug)]
pub struct AddBrushTileCommand {
    pub brush: TileMapBrushResource,
    pub tile: Option<BrushTile>,
}

impl CommandTrait for AddBrushTileCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Add Brush Tile".to_string()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.brush.data_ref().tiles.push(self.tile.take().unwrap());
        Log::verify(self.brush.save_back());
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.tile = self.brush.data_ref().tiles.pop();
        Log::verify(self.brush.save_back());
    }
}
