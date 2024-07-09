use crate::command::{CommandContext, CommandTrait};
use fyrox::core::algebra::Vector2;
use fyrox::core::log::Log;
use fyrox::core::Uuid;
use fyrox::scene::tilemap::brush::{BrushTile, TileMapBrushResource};
use fyrox::scene::tilemap::tileset::{TileDefinition, TileSetResource};

#[derive(Debug)]
pub struct AddTileCommand {
    pub tile_set: TileSetResource,
    pub tile: TileDefinition,
}

impl CommandTrait for AddTileCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Add Tile".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.tile_set.data_ref().tiles.push(self.tile.clone());
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.tile = self.tile_set.data_ref().tiles.pop().unwrap();
    }
}

#[derive(Debug)]
pub struct RemoveTileCommand {
    pub tile_set: TileSetResource,
    pub index: usize,
    pub tile: Option<TileDefinition>,
}

impl CommandTrait for RemoveTileCommand {
    fn name(&mut self, _text: &dyn CommandContext) -> String {
        "Remove Tile".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.tile = Some(self.tile_set.data_ref().tiles.remove(self.index));
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.tile_set
            .data_ref()
            .tiles
            .insert(self.index, self.tile.take().unwrap());
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
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.brush.data_ref().tiles.push(self.tile.take().unwrap());
    }
}
