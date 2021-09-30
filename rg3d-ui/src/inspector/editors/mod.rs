use crate::{
    core::{inspect::PropertyInfo, pool::Handle},
    inspector::{
        editors::{
            bool::BoolPropertyEditorDefinition,
            f32::F32PropertyEditorDefinition,
            int::I32PropertyEditorDefinition,
            int::{
                I16PropertyEditorDefinition, I64PropertyEditorDefinition,
                I8PropertyEditorDefinition, U16PropertyEditorDefinition,
                U32PropertyEditorDefinition, U64PropertyEditorDefinition,
                U8PropertyEditorDefinition,
            },
            quat::QuatPropertyEditorDefinition,
            rect::RectPropertyEditorDefinition,
            string::StringPropertyEditorDefinition,
            vec::{
                Vec2PropertyEditorDefinition, Vec3PropertyEditorDefinition,
                Vec4PropertyEditorDefinition,
            },
        },
        InspectorEnvironment, InspectorError,
    },
    message::{PropertyChanged, UiMessage},
    BuildContext, UiNode, UserInterface,
};
use std::{any::TypeId, collections::HashMap, fmt::Debug, sync::Arc};

pub mod bool;
pub mod collection;
pub mod enumeration;
pub mod f32;
pub mod int;
pub mod quat;
pub mod rect;
pub mod string;
pub mod vec;

pub struct PropertyEditorBuildContext<'a, 'b, 'c> {
    pub build_context: &'a mut BuildContext<'c>,
    pub property_info: &'b PropertyInfo<'b>,
    pub environment: Option<Arc<dyn InspectorEnvironment>>,
    pub definition_container: Arc<PropertyEditorDefinitionContainer>,
}

pub struct PropertyEditorMessageContext<'a, 'b> {
    pub sync_flag: u64,
    pub instance: Handle<UiNode>,
    pub ui: &'b mut UserInterface,
    pub property_info: &'a PropertyInfo<'a>,
    pub definition_container: Arc<PropertyEditorDefinitionContainer>,
}

pub enum Layout {
    /// Horizontal grid layout. Suitable for simple properties.
    Horizontal,

    /// Vertical grid layout. Suitable for large collections and in situations when you
    /// don't want the editor to be shifted on the right side.
    Vertical,
}

pub struct PropertyEditorInstance {
    /// Title of a property editor. Usually just a text with a property display name.
    ///
    /// Could be [`Handle::NONE`], in this case inspector will automatically create
    /// a text widget with property name.
    pub title: Handle<UiNode>,

    /// A property editor. Could be any widget that capable of editing a property
    /// value.
    pub editor: Handle<UiNode>,
}

pub trait PropertyEditorDefinition: Debug + Send + Sync {
    fn value_type_id(&self) -> TypeId;

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError>;

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<UiMessage, InspectorError>;

    fn translate_message(
        &self,
        name: &str,
        owner_type_id: TypeId,
        message: &UiMessage,
    ) -> Option<PropertyChanged>;

    fn layout(&self) -> Layout {
        Layout::Horizontal
    }
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
        container.insert(Arc::new(I8PropertyEditorDefinition));
        container.insert(Arc::new(U8PropertyEditorDefinition));
        container.insert(Arc::new(I16PropertyEditorDefinition));
        container.insert(Arc::new(U16PropertyEditorDefinition));
        container.insert(Arc::new(I32PropertyEditorDefinition));
        container.insert(Arc::new(U32PropertyEditorDefinition));
        container.insert(Arc::new(I64PropertyEditorDefinition));
        container.insert(Arc::new(U64PropertyEditorDefinition));
        container.insert(Arc::new(StringPropertyEditorDefinition));
        container.insert(Arc::new(Vec2PropertyEditorDefinition));
        container.insert(Arc::new(Vec3PropertyEditorDefinition));
        container.insert(Arc::new(Vec4PropertyEditorDefinition));
        container.insert(Arc::new(BoolPropertyEditorDefinition));
        container.insert(Arc::new(QuatPropertyEditorDefinition));
        container.insert(Arc::new(RectPropertyEditorDefinition::<f32>::new()));
        container.insert(Arc::new(RectPropertyEditorDefinition::<i32>::new()));
        container.insert(Arc::new(RectPropertyEditorDefinition::<u32>::new()));
        container.insert(Arc::new(RectPropertyEditorDefinition::<i16>::new()));
        container.insert(Arc::new(RectPropertyEditorDefinition::<u16>::new()));
        container.insert(Arc::new(RectPropertyEditorDefinition::<i8>::new()));
        container.insert(Arc::new(RectPropertyEditorDefinition::<u8>::new()));
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
