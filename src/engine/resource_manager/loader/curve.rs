use crate::engine::resource_manager::loader::BoxedLoaderFuture;
use crate::{
    engine::resource_manager::loader::ResourceLoader,
    resource::curve::{CurveImportOptions, CurveResource, CurveResourceState},
    utils::log::{Log, MessageKind},
};
use fyrox_resource::ResourceState;
use std::{path::PathBuf, sync::Arc};

pub struct CurveLoader;

impl ResourceLoader<CurveResource, CurveImportOptions> for CurveLoader {
    type Output = BoxedLoaderFuture;

    fn load(
        &mut self,
        curve: CurveResource,
        path: PathBuf,
        _default_import_options: CurveImportOptions,
    ) -> Self::Output {
        let fut = async move {
            match CurveResourceState::from_file(&path).await {
                Ok(curve_state) => {
                    Log::writeln(
                        MessageKind::Information,
                        format!("Curve {:?} is loaded!", path),
                    );

                    curve.state().commit(ResourceState::Ok(curve_state));
                }
                Err(error) => {
                    Log::writeln(
                        MessageKind::Error,
                        format!("Unable to load curve from {:?}! Reason {:?}", path, error),
                    );

                    curve.state().commit(ResourceState::LoadError {
                        path,
                        error: Some(Arc::new(error)),
                    });
                }
            }
        };
        Box::pin(fut)
    }
}
