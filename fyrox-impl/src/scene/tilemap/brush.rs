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

//! Tile map brush is a set of tiles arranged in arbitrary shape, that can be used to draw on a tile
//! map. A brush resembles a tile set in that they are both collections of tiles, but they are distinct
//! because a brush can be freely organized to suit whatever is most convenient while editing a tile map.
//! The tiles of a brush can be moved at any time without consequence, and so can the pages.
//! A brush is like a painter's toolbox, where the sole purpose is to serve the painter's convenience.
//!
//! In contrast, a tile set is a directory for finding tile data according to a specific position on
//! a specific page. Tiles can be moved in a tile set, but doing so will affect the lookup of tile data.

use crate::{
    asset::{
        io::ResourceIo,
        loader::{BoxedLoaderFuture, LoaderPayload, ResourceLoader},
        manager::ResourceManager,
        state::LoadError,
        Resource, ResourceData,
    },
    core::{
        algebra::{Matrix4, Vector2, Vector3},
        color::Color,
        io::FileLoadError,
        reflect::prelude::*,
        type_traits::prelude::*,
        visitor::prelude::*,
    },
    scene::debug::SceneDrawingContext,
};
use std::{
    error::Error,
    fmt::{Display, Formatter},
    path::{Path, PathBuf},
    sync::Arc,
};

use super::*;

/// An error that may occur during tile map brush resource loading.
#[derive(Debug)]
pub enum TileMapBrushResourceError {
    /// An i/o error has occurred.
    Io(FileLoadError),

    /// An error that may occur due to version incompatibilities.
    Visit(VisitError),
}

impl Display for TileMapBrushResourceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TileMapBrushResourceError::Io(v) => {
                write!(f, "A file load error has occurred {v:?}")
            }
            TileMapBrushResourceError::Visit(v) => {
                write!(
                    f,
                    "An error that may occur due to version incompatibilities. {v:?}"
                )
            }
        }
    }
}

impl From<FileLoadError> for TileMapBrushResourceError {
    fn from(e: FileLoadError) -> Self {
        Self::Io(e)
    }
}

impl From<VisitError> for TileMapBrushResourceError {
    fn from(e: VisitError) -> Self {
        Self::Visit(e)
    }
}

/// A page of tiles within a brush. Having multiple pages allows a brush to be optimized
/// for use in multiple contexts.
#[derive(Default, Debug, Clone, Visit, Reflect)]
pub struct TileMapBrushPage {
    /// The tile that represents this page in the editor
    pub icon: TileDefinitionHandle,
    /// The tiles on this page, organized by position.
    #[reflect(hidden)]
    pub tiles: Tiles,
}

impl TileSource for TileMapBrushPage {
    fn transformation(&self) -> OrthoTransformation {
        OrthoTransformation::default()
    }
    fn get_at(&self, position: Vector2<i32>) -> Option<TileDefinitionHandle> {
        self.tiles.get(&position).copied()
    }
}

impl TileMapBrushPage {
    /// The smallest Rect that contains all the tiles on this page.
    pub fn bounding_rect(&self) -> OptionTileRect {
        let mut result = OptionTileRect::default();
        for pos in self.tiles.keys() {
            result.push(*pos);
        }
        result
    }
    /// The tile definition handle at the given position.
    pub fn find_tile_at_position(&self, position: Vector2<i32>) -> Option<TileDefinitionHandle> {
        self.tiles.get(&position).copied()
    }
    /// The tile definition handles of the tiles at the given positions.
    pub fn get_tiles<I: Iterator<Item = Vector2<i32>>>(&self, iter: I, tiles: &mut Tiles) {
        for pos in iter {
            if let Some(tile) = self.tiles.get(&pos).copied() {
                tiles.insert(pos, tile);
            }
        }
    }

    /// Draw brush outline to the scene drawing context.
    pub fn draw_outline(
        &self,
        ctx: &mut SceneDrawingContext,
        position: Vector2<i32>,
        world_transform: &Matrix4<f32>,
        color: Color,
    ) {
        for (pos, _) in self.tiles.iter() {
            draw_tile_outline(ctx, position + pos, world_transform, color);
        }
    }
}

fn draw_tile_outline(
    ctx: &mut SceneDrawingContext,
    position: Vector2<i32>,
    world_transform: &Matrix4<f32>,
    color: Color,
) {
    ctx.draw_rectangle(
        0.5,
        0.5,
        Matrix4::new_translation(
            &(position.cast::<f32>().to_homogeneous() + Vector3::new(0.5, 0.5, 0.0)),
        ) * world_transform,
        color,
    );
}

/// Tile map brush is a set of tiles arranged in arbitrary shape, that can be used to draw on a tile
/// map.
#[derive(Default, Debug, Clone, Visit, Reflect, TypeUuidProvider)]
#[type_uuid(id = "23ed39da-cb01-4181-a058-94dc77ecb4b2")]
pub struct TileMapBrush {
    /// The tile set used by this brush. This must match the tile set of any tile map that this
    /// brush is used to edit.
    pub tile_set: Option<TileSetResource>,
    /// The set of pages contained in the brush
    /// Each page is associated with 2D coordinates within a palette of brush pages.
    /// This allows pages to be selected much like tiles are selected, and it allows
    /// users to customize the organization of pages.
    #[reflect(hidden)]
    pub pages: TileGridMap<TileMapBrushPage>,
    /// A count of changes since last save. New changes add +1. Reverting to previous
    /// states add -1. Reverting to a state before the last save can result in negative
    /// values. Saving is unnecessary whenever this value is 0.
    #[reflect(hidden)]
    #[visit(skip)]
    pub change_count: ChangeFlag,
}

impl TileMapBrush {
    /// True if there is a tile at the given position.
    pub fn has_tile_at(&self, page: Vector2<i32>, tile: Vector2<i32>) -> bool {
        let Some(page) = self.pages.get(&page) else {
            return false;
        };
        page.tiles.contains_key(&tile)
    }
    /// True if there is a page at the given position.
    pub fn has_page_at(&self, page: Vector2<i32>) -> bool {
        self.pages.contains_key(&page)
    }
    /// The handle stored at the given position.
    pub fn tile_redirect(&self, handle: TileDefinitionHandle) -> Option<TileDefinitionHandle> {
        self.find_tile_at_position(TilePaletteStage::Tiles, handle.page(), handle.tile())
    }
    /// Returns bounding rectangle of pages in grid coordinates.
    #[inline]
    pub fn pages_bounds(&self) -> OptionTileRect {
        let mut result = OptionTileRect::default();
        for pos in self.pages.keys() {
            result.push(*pos);
        }
        result
    }
    /// The handle of the tile that represents the page at the given position.
    pub fn page_icon(&self, page: Vector2<i32>) -> Option<TileDefinitionHandle> {
        self.pages.get(&page).map(|p| p.icon)
    }
    /// The bounds of the tiles on the given page.
    pub fn tiles_bounds(&self, stage: TilePaletteStage, page: Vector2<i32>) -> OptionTileRect {
        match stage {
            TilePaletteStage::Tiles => {
                let Some(page) = self.pages.get(&page) else {
                    return OptionTileRect::default();
                };
                page.bounding_rect()
            }
            TilePaletteStage::Pages => self.pages_bounds(),
        }
    }

    /// The handle of the tile at the given position, either the icon of a page or the tile stored at that position in the brush.
    pub fn find_tile_at_position(
        &self,
        stage: TilePaletteStage,
        page: Vector2<i32>,
        position: Vector2<i32>,
    ) -> Option<TileDefinitionHandle> {
        match stage {
            TilePaletteStage::Pages => self.pages.get(&position).map(|p| p.icon),
            TilePaletteStage::Tiles => self
                .pages
                .get(&page)
                .and_then(|p| p.find_tile_at_position(position)),
        }
    }

    /// The tile definition handles of the tiles at the given positions on the given page.
    pub fn get_tiles<I: Iterator<Item = Vector2<i32>>>(
        &self,
        stage: TilePaletteStage,
        page: Vector2<i32>,
        iter: I,
        tiles: &mut Tiles,
    ) {
        match stage {
            TilePaletteStage::Pages => {
                for pos in iter {
                    if let Some(handle) = self.pages.get(&pos).map(|p| p.icon) {
                        tiles.insert(pos, handle);
                    }
                }
            }
            TilePaletteStage::Tiles => {
                if let Some(page) = self.pages.get(&page) {
                    page.get_tiles(iter, tiles);
                }
            }
        }
    }

    /// Return true if this brush has no tile set.
    pub fn is_missing_tile_set(&self) -> bool {
        self.tile_set.is_none()
    }

    fn palette_render_loop_without_tile_set<F>(
        &self,
        stage: TilePaletteStage,
        page: Vector2<i32>,
        mut func: F,
    ) where
        F: FnMut(Vector2<i32>, TileRenderData),
    {
        match stage {
            TilePaletteStage::Pages => {
                for k in self.pages.keys() {
                    func(*k, TileRenderData::missing_data());
                }
            }
            TilePaletteStage::Tiles => {
                let Some(page) = self.pages.get(&page) else {
                    return;
                };
                for k in page.tiles.keys() {
                    func(*k, TileRenderData::missing_data());
                }
            }
        }
    }

    /// Loops through the tiles of the given page and finds the render data for each tile
    /// in the tile set, then passes it to the given function.
    pub fn palette_render_loop<F>(&self, stage: TilePaletteStage, page: Vector2<i32>, mut func: F)
    where
        F: FnMut(Vector2<i32>, TileRenderData),
    {
        let Some(tile_set) = self.tile_set.as_ref() else {
            self.palette_render_loop_without_tile_set(stage, page, func);
            return;
        };
        let mut state = tile_set.state();
        let Some(tile_set) = state.data() else {
            self.palette_render_loop_without_tile_set(stage, page, func);
            return;
        };
        match stage {
            TilePaletteStage::Pages => {
                for (k, p) in self.pages.iter() {
                    let data = tile_set
                        .get_tile_render_data(p.icon.into())
                        .unwrap_or_else(TileRenderData::missing_data);
                    func(*k, data);
                }
            }
            TilePaletteStage::Tiles => {
                let Some(page) = self.pages.get(&page) else {
                    return;
                };
                for (k, &handle) in page.tiles.iter() {
                    let data = tile_set
                        .get_tile_render_data(handle.into())
                        .unwrap_or_else(TileRenderData::missing_data);
                    func(*k, data);
                }
            }
        }
    }

    /// Return the `TileRenderData` needed to render the tile at the given position on the given page.
    /// If there is no tile at that position or the tile set is missing or not loaded, then None is returned.
    /// If there is a tile and a tile set, but the handle of the tile does not exist in the tile set,
    /// then the rendering data for an error tile is returned using `TileRenderData::missing_tile()`.
    pub fn get_tile_render_data(&self, position: ResourceTilePosition) -> Option<TileRenderData> {
        let handle = self.redirect_handle(position)?;
        let mut tile_set = self.tile_set.as_ref()?.state();
        let data = tile_set
            .data()?
            .get_tile_render_data(handle.into())
            .unwrap_or_else(TileRenderData::missing_data);
        Some(data)
    }

    /// The tiles of a brush are references to tiles in the tile set.
    /// This method converts positions within the brush into the handle that points to the corresponding
    /// tile definition within the tile set.
    /// If this brush does not contain a reference at the given position, then None is returned.
    pub fn redirect_handle(&self, position: ResourceTilePosition) -> Option<TileDefinitionHandle> {
        match position.stage() {
            TilePaletteStage::Tiles => {
                let page = self.pages.get(&position.page())?;
                page.tiles.get_at(position.stage_position())
            }
            TilePaletteStage::Pages => self
                .pages
                .get(&position.stage_position())
                .map(|page| page.icon),
        }
    }

    /// The `TileMaterialBounds` taken from the tile set for the tile in the brush at the given position.
    pub fn get_tile_bounds(&self, position: ResourceTilePosition) -> Option<TileMaterialBounds> {
        let handle = self.redirect_handle(position)?;
        self.tile_set
            .as_ref()?
            .state()
            .data()?
            .get_tile_bounds(handle.into())
    }

    /// Load a tile map brush resource from the specific file path.
    pub async fn from_file(
        path: &Path,
        resource_manager: ResourceManager,
        io: &dyn ResourceIo,
    ) -> Result<Self, TileMapBrushResourceError> {
        let bytes = io.load_file(path).await?;
        let mut visitor = Visitor::load_from_memory(&bytes)?;
        visitor.blackboard.register(Arc::new(resource_manager));
        let mut tile_map_brush = Self::default();
        tile_map_brush.visit("TileMapBrush", &mut visitor)?;
        Ok(tile_map_brush)
    }

    fn save(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        let mut visitor = Visitor::new();
        self.visit("TileMapBrush", &mut visitor)?;
        visitor.save_binary(path)?;
        Ok(())
    }
}

impl ResourceData for TileMapBrush {
    fn type_uuid(&self) -> Uuid {
        <Self as TypeUuidProvider>::type_uuid()
    }

    fn save(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        self.save(path)
    }

    fn can_be_saved(&self) -> bool {
        true
    }
}

/// Standard tile map brush loader.
pub struct TileMapBrushLoader {
    /// The resource manager to use to load the brush's tile set.
    pub resource_manager: ResourceManager,
}

impl ResourceLoader for TileMapBrushLoader {
    fn extensions(&self) -> &[&str] {
        &["tile_map_brush"]
    }

    fn data_type_uuid(&self) -> Uuid {
        <TileMapBrush as TypeUuidProvider>::type_uuid()
    }

    fn load(&self, path: PathBuf, io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture {
        let resource_manager = self.resource_manager.clone();
        Box::pin(async move {
            let tile_map_brush = TileMapBrush::from_file(&path, resource_manager, io.as_ref())
                .await
                .map_err(LoadError::new)?;
            Ok(LoaderPayload::new(tile_map_brush))
        })
    }
}

/// An alias to `Resource<TileMapBrush>`.
pub type TileMapBrushResource = Resource<TileMapBrush>;
