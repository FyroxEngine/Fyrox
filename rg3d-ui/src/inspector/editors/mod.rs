use crate::{
    core::{inspect::PropertyInfo, pool::Handle},
    inspector::{
        editors::{
            bool::BoolPropertyEditorDefinition,
            f32::F32PropertyEditorDefinition,
            i32::I32PropertyEditorDefinition,
            quat::QuatPropertyEditorDefinition,
            string::StringPropertyEditorDefinition,
            vec::{
                Vec2PropertyEditorDefinition, Vec3PropertyEditorDefinition,
                Vec4PropertyEditorDefinition,
            },
        },
        InspectorEnvironment, InspectorError,
    },
    message::{PropertyChanged, UiMessage},
    BuildContext, UiNode,
};
use std::{any::TypeId, collections::HashMap, fmt::Debug, sync::Arc};

pub mod bool;
pub mod collection;
pub mod enumeration;
pub mod f32;
pub mod i32;
pub mod quat;
pub mod string;
pub mod vec;

pub const ROW_HEIGHT: f32 = 25.0;

pub struct PropertyEditorBuildContext<'a, 'b, 'c> {
    pub build_context: &'a mut BuildContext<'c>,
    pub property_info: &'b PropertyInfo<'b>,
    pub environment: Option<Arc<dyn InspectorEnvironment>>,
    pub definition_container: Arc<PropertyEditorDefinitionContainer>,
}

pub enum Layout {
    /// Horizontal grid layout. Suitable for simple properties.
    Horizontal,

    /// Vertical grid layout. Suitable for large collections and in situations when you
    /// don't want the editor to be shifted on the right side.
    Vertical,
}

pub trait PropertyEditorDefinition: Debug + Send + Sync {
    fn value_type_id(&self) -> TypeId;

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<Handle<UiNode>, InspectorError>;

    fn create_message(
        &self,
        instance: Handle<UiNode>,
        property_info: &PropertyInfo,
    ) -> Result<UiMessage, InspectorError>;

    fn translate_message(
        &self,
        name: &str,
        owner_type_id: TypeId,
        message: &UiMessage,
    ) -> Option<PropertyChanged>;

    fn layout(&self) -> Layout;
}

#[derive(Clone)]
pub struct PropertyEditorDefinitionContainer {
    definitions: HashMap<TypeId, Arc<dyn PropertyEditorDefinition>>,
}

impl Default for PropertyEditorDefinitionContainer {
    fn default() -> Self {
        Self {
            definitions: Default::default(),
        }
    }
}

impl PropertyEditorDefinitionContainer {
    pub fn new() -> Self {
        let mut container = Self::default();
        container.insert(Arc::new(F32PropertyEditorDefinition));
        container.insert(Arc::new(I32PropertyEditorDefinition));
        container.insert(Arc::new(StringPropertyEditorDefinition));
        container.insert(Arc::new(Vec2PropertyEditorDefinition));
        container.insert(Arc::new(Vec3PropertyEditorDefinition));
        container.insert(Arc::new(Vec4PropertyEditorDefinition));
        container.insert(Arc::new(BoolPropertyEditorDefinition));
        container.insert(Arc::new(QuatPropertyEditorDefinition));
        container
    }

    pub fn insert(
        &mut self,
        definition: Arc<dyn PropertyEditorDefinition>,
    ) -> Option<Arc<dyn PropertyEditorDefinition>> {
        self.definitions
            .insert(definition.value_type_id(), definition)
    }

    pub fn definitions(&self) -> &HashMap<TypeId, Arc<dyn PropertyEditorDefinition>> {
        &self.definitions
    }
}
