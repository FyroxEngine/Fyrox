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
    message::{MessageData, PropertyChanged, UiMessage},
    node::UINode,
    BuildContext, Control,
};
use std::{any::TypeId, collections::HashMap, fmt::Debug, sync::Arc};

pub mod bool;
pub mod enumeration;
pub mod f32;
pub mod i32;
pub mod quat;
pub mod string;
pub mod vec;

pub struct PropertyEditorBuildContext<'a, 'b, 'c, M: MessageData, C: Control<M, C>> {
    pub build_context: &'a mut BuildContext<'c, M, C>,
    pub property_info: &'b PropertyInfo<'b>,
    pub row: usize,
    pub column: usize,
    pub environment: Option<Arc<dyn InspectorEnvironment>>,
}

pub trait PropertyEditorDefinition<M: MessageData, C: Control<M, C>>: Debug + Send + Sync {
    fn value_type_id(&self) -> TypeId;

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext<M, C>,
    ) -> Result<Handle<UINode<M, C>>, InspectorError>;

    fn create_message(
        &self,
        instance: Handle<UINode<M, C>>,
        property_info: &PropertyInfo,
    ) -> Result<UiMessage<M, C>, InspectorError>;

    fn translate_message(
        &self,
        name: &str,
        owner_type_id: TypeId,
        message: &UiMessage<M, C>,
    ) -> Option<PropertyChanged>;
}

#[derive(Clone)]
pub struct PropertyEditorDefinitionContainer<M: MessageData, C: Control<M, C>> {
    definitions: HashMap<TypeId, Arc<dyn PropertyEditorDefinition<M, C>>>,
}

impl<M: MessageData, C: Control<M, C>> Default for PropertyEditorDefinitionContainer<M, C> {
    fn default() -> Self {
        Self {
            definitions: Default::default(),
        }
    }
}

impl<M: MessageData, C: Control<M, C>> PropertyEditorDefinitionContainer<M, C> {
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
        definition: Arc<dyn PropertyEditorDefinition<M, C>>,
    ) -> Option<Arc<dyn PropertyEditorDefinition<M, C>>> {
        self.definitions
            .insert(definition.value_type_id(), definition)
    }

    pub fn definitions(&self) -> &HashMap<TypeId, Arc<dyn PropertyEditorDefinition<M, C>>> {
        &self.definitions
    }
}
