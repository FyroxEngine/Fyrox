use crate::{
    asset::{
        io::ResourceIo,
        loader::{BoxedLoaderFuture, LoaderPayload, ResourceLoader},
        state::LoadError,
        Resource, ResourceData,
    },
    core::{
        io::FileLoadError, math::Rect, reflect::prelude::*, type_traits::prelude::*,
        visitor::prelude::*,
    },
    resource::texture::TextureResource,
};
use std::{
    any::Any,
    error::Error,
    fmt::{Display, Formatter},
    path::{Path, PathBuf},
    sync::Arc,
};
use strum_macros::{AsRefStr, EnumString, VariantNames};

/// An error that may occur during tile set resource loading.
#[derive(Debug)]
pub enum TileSetResourceError {
    /// An i/o error has occurred.
    Io(FileLoadError),

    /// An error that may occur due to version incompatibilities.
    Visit(VisitError),
}

impl Display for TileSetResourceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(v) => {
                write!(f, "A file load error has occurred {v:?}")
            }
            Self::Visit(v) => {
                write!(
                    f,
                    "An error that may occur due to version incompatibilities. {v:?}"
                )
            }
        }
    }
}

impl From<FileLoadError> for TileSetResourceError {
    fn from(e: FileLoadError) -> Self {
        Self::Io(e)
    }
}

impl From<VisitError> for TileSetResourceError {
    fn from(e: VisitError) -> Self {
        Self::Visit(e)
    }
}

#[derive(
    Copy,
    Clone,
    Hash,
    PartialEq,
    Eq,
    Debug,
    Default,
    Visit,
    Reflect,
    AsRefStr,
    EnumString,
    VariantNames,
    TypeUuidProvider,
)]
#[type_uuid(id = "04a44fec-394f-4497-97d5-fe9e6f915831")]
pub enum TileCollider {
    None,
    #[default]
    Rectangle,
    Mesh,
}

#[derive(Clone, Default, Debug, Reflect, Visit)]
pub struct TileDefinition {
    texture: TextureResource,
    uv_rect: Rect<f32>,
    collider: TileCollider,
}

#[derive(Clone, Default, Debug, Reflect, Visit, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "7b7e057b-a41e-4150-ab3b-0ae99f4024f0")]
pub struct TileSet {
    tiles: Vec<TileDefinition>,
}

impl TileSet {
    /// Load a tile set resource from the specific file path.
    pub async fn from_file(path: &Path, io: &dyn ResourceIo) -> Result<Self, TileSetResourceError> {
        let bytes = io.load_file(path).await?;
        let mut visitor = Visitor::load_from_memory(&bytes)?;
        let mut tile_set = TileSet::default();
        tile_set.visit("TileSet", &mut visitor)?;
        Ok(tile_set)
    }
}

impl ResourceData for TileSet {
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
        let mut visitor = Visitor::new();
        self.visit("TileSet", &mut visitor)?;
        visitor.save_binary(path)?;
        Ok(())
    }

    fn can_be_saved(&self) -> bool {
        true
    }
}

pub type TileSetResource = Resource<TileSet>;

pub struct TileSetLoader;

impl ResourceLoader for TileSetLoader {
    fn extensions(&self) -> &[&str] {
        &["tileset"]
    }

    fn data_type_uuid(&self) -> Uuid {
        <TileSet as TypeUuidProvider>::type_uuid()
    }

    fn load(&self, path: PathBuf, io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture {
        Box::pin(async move {
            let tile_set = TileSet::from_file(&path, io.as_ref())
                .await
                .map_err(LoadError::new)?;
            Ok(LoaderPayload::new(tile_set))
        })
    }
}
