//! Tile map brush is a set of tiles arranged in arbitrary shape, that can be used to draw on a tile
//! map.

use crate::{
    asset::{
        io::ResourceIo,
        loader::{BoxedLoaderFuture, LoaderPayload, ResourceLoader},
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
    scene::{debug::SceneDrawingContext, tilemap::tileset::TileDefinitionHandle},
};
use fyrox_core::math::Rect;
use std::{
    any::Any,
    error::Error,
    fmt::{Display, Formatter},
    path::{Path, PathBuf},
    sync::Arc,
};

/// Brush tile is a building block of a brush.
#[derive(PartialEq, Debug, Clone, Visit, Reflect)]
pub struct BrushTile {
    /// Handle of the tile definition in a tile set.
    pub definition_handle: TileDefinitionHandle,
    /// Local position of tile (in grid coordinates).
    pub local_position: Vector2<i32>,
    /// Unique id of the tile.
    pub id: Uuid,
}

impl Default for BrushTile {
    fn default() -> Self {
        Self {
            definition_handle: Default::default(),
            local_position: Default::default(),
            id: Uuid::new_v4(),
        }
    }
}

impl BrushTile {
    /// Draws brush outline to the scene drawing context.
    pub fn draw_outline(
        &self,
        ctx: &mut SceneDrawingContext,
        position: Vector2<i32>,
        world_transform: &Matrix4<f32>,
        color: Color,
    ) {
        ctx.draw_rectangle(
            0.5,
            0.5,
            Matrix4::new_translation(
                &((self.local_position + position)
                    .cast::<f32>()
                    .to_homogeneous()
                    + Vector3::new(0.5, 0.5, 0.0)),
            ) * world_transform,
            color,
        );
    }
}

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

/// Tile map brush is a set of tiles arranged in arbitrary shape, that can be used to draw on a tile
/// map.
#[derive(Default, PartialEq, Debug, Clone, Visit, Reflect, TypeUuidProvider)]
#[type_uuid(id = "23ed39da-cb01-4181-a058-94dc77ecb4b2")]
pub struct TileMapBrush {
    /// Tiles of the brush.
    pub tiles: Vec<BrushTile>,
}

impl TileMapBrush {
    /// Returns bounding rectangle of the tile map brush in grid coordinates.
    #[inline]
    pub fn bounding_rect(&self) -> Rect<i32> {
        let mut min = Vector2::repeat(i32::MAX);
        let mut max = Vector2::repeat(i32::MIN);

        for tile in self.tiles.iter() {
            min = tile.local_position.inf(&min);
            max = tile.local_position.sup(&max);
        }

        Rect::from_points(min, max)
    }

    /// Draw brush outline to the scene drawing context.
    pub fn draw_outline(
        &self,
        ctx: &mut SceneDrawingContext,
        position: Vector2<i32>,
        world_transform: &Matrix4<f32>,
        color: Color,
    ) {
        for tile in self.tiles.iter() {
            tile.draw_outline(ctx, position, world_transform, color);
        }
    }

    /// Tries to find a tile with the given id.
    pub fn find_tile(&self, id: &Uuid) -> Option<&BrushTile> {
        self.tiles.iter().find(|tile| tile.id == *id)
    }

    /// Tries to find a tile with the given id.
    pub fn find_tile_mut(&mut self, id: &Uuid) -> Option<&mut BrushTile> {
        self.tiles.iter_mut().find(|tile| tile.id == *id)
    }

    /// Load a tile map brush resource from the specific file path.
    pub async fn from_file(
        path: &Path,
        io: &dyn ResourceIo,
    ) -> Result<Self, TileMapBrushResourceError> {
        let bytes = io.load_file(path).await?;
        let mut visitor = Visitor::load_from_memory(&bytes)?;
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
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

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
pub struct TileMapBrushLoader {}

impl ResourceLoader for TileMapBrushLoader {
    fn extensions(&self) -> &[&str] {
        &["tile_map_brush"]
    }

    fn data_type_uuid(&self) -> Uuid {
        <TileMapBrush as TypeUuidProvider>::type_uuid()
    }

    fn load(&self, path: PathBuf, io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture {
        Box::pin(async move {
            let tile_map_brush = TileMapBrush::from_file(&path, io.as_ref())
                .await
                .map_err(LoadError::new)?;
            Ok(LoaderPayload::new(tile_map_brush))
        })
    }
}

/// An alias to `Resource<TileMapBrush>`.
pub type TileMapBrushResource = Resource<TileMapBrush>;
