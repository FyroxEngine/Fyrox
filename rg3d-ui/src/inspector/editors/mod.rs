use crate::inspector::editors::bool::BoolPropertyEditorDefinition;
use crate::{
    core::{inspect::PropertyInfo, pool::Handle},
    inspector::{
        editors::{
            color::ColorPropertyEditorDefinition,
            numeric::NumericPropertyEditorDefinition,
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
pub mod color;
pub mod enumeration;
pub mod inspectable;
pub mod numeric;
pub mod quat;
pub mod rect;
pub mod string;
pub mod vec;

pub struct PropertyEditorBuildContext<'a, 'b, 'c> {
    pub build_context: &'a mut BuildContext<'c>,
    pub property_info: &'b PropertyInfo<'b>,
    pub environment: Option<Arc<dyn InspectorEnvironment>>,
    pub definition_container: Arc<PropertyEditorDefinitionContainer>,
    pub sync_flag: u64,
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
    ) -> Result<Option<UiMessage>, InspectorError>;

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

        container.insert(Arc::new(BoolPropertyEditorDefinition));

        container.insert(Arc::new(StringPropertyEditorDefinition));

        container.insert(Arc::new(NumericPropertyEditorDefinition::<f64>::default()));
        container.insert(Arc::new(NumericPropertyEditorDefinition::<f32>::default()));
        container.insert(Arc::new(NumericPropertyEditorDefinition::<i64>::default()));
        container.insert(Arc::new(NumericPropertyEditorDefinition::<u64>::default()));
        container.insert(Arc::new(NumericPropertyEditorDefinition::<i32>::default()));
        container.insert(Arc::new(NumericPropertyEditorDefinition::<u32>::default()));
        container.insert(Arc::new(NumericPropertyEditorDefinition::<i16>::default()));
        container.insert(Arc::new(NumericPropertyEditorDefinition::<u16>::default()));
        container.insert(Arc::new(NumericPropertyEditorDefinition::<i8>::default()));
        container.insert(Arc::new(NumericPropertyEditorDefinition::<u8>::default()));

        container.insert(Arc::new(Vec4PropertyEditorDefinition::<f64>::default()));
        container.insert(Arc::new(Vec4PropertyEditorDefinition::<f32>::default()));
        container.insert(Arc::new(Vec4PropertyEditorDefinition::<i64>::default()));
        container.insert(Arc::new(Vec4PropertyEditorDefinition::<u64>::default()));
        container.insert(Arc::new(Vec4PropertyEditorDefinition::<i32>::default()));
        container.insert(Arc::new(Vec4PropertyEditorDefinition::<u32>::default()));
        container.insert(Arc::new(Vec4PropertyEditorDefinition::<i16>::default()));
        container.insert(Arc::new(Vec4PropertyEditorDefinition::<u16>::default()));
        container.insert(Arc::new(Vec4PropertyEditorDefinition::<i8>::default()));
        container.insert(Arc::new(Vec4PropertyEditorDefinition::<u8>::default()));

        container.insert(Arc::new(Vec3PropertyEditorDefinition::<f64>::default()));
        container.insert(Arc::new(Vec3PropertyEditorDefinition::<f32>::default()));
        container.insert(Arc::new(Vec3PropertyEditorDefinition::<i64>::default()));
        container.insert(Arc::new(Vec3PropertyEditorDefinition::<u64>::default()));
        container.insert(Arc::new(Vec3PropertyEditorDefinition::<i32>::default()));
        container.insert(Arc::new(Vec3PropertyEditorDefinition::<u32>::default()));
        container.insert(Arc::new(Vec3PropertyEditorDefinition::<i16>::default()));
        container.insert(Arc::new(Vec3PropertyEditorDefinition::<u16>::default()));
        container.insert(Arc::new(Vec3PropertyEditorDefinition::<i8>::default()));
        container.insert(Arc::new(Vec3PropertyEditorDefinition::<u8>::default()));

        container.insert(Arc::new(Vec2PropertyEditorDefinition::<f64>::default()));
        container.insert(Arc::new(Vec2PropertyEditorDefinition::<f32>::default()));
        container.insert(Arc::new(Vec2PropertyEditorDefinition::<i64>::default()));
        container.insert(Arc::new(Vec2PropertyEditorDefinition::<u64>::default()));
        container.insert(Arc::new(Vec2PropertyEditorDefinition::<i32>::default()));
        container.insert(Arc::new(Vec2PropertyEditorDefinition::<u32>::default()));
        container.insert(Arc::new(Vec2PropertyEditorDefinition::<i16>::default()));
        container.insert(Arc::new(Vec2PropertyEditorDefinition::<u16>::default()));
        container.insert(Arc::new(Vec2PropertyEditorDefinition::<i8>::default()));
        container.insert(Arc::new(Vec2PropertyEditorDefinition::<u8>::default()));

        container.insert(Arc::new(QuatPropertyEditorDefinition::<f64>::default()));
        container.insert(Arc::new(QuatPropertyEditorDefinition::<f32>::default()));

        container.insert(Arc::new(RectPropertyEditorDefinition::<f64>::new()));
        container.insert(Arc::new(RectPropertyEditorDefinition::<f32>::new()));
        container.insert(Arc::new(RectPropertyEditorDefinition::<i32>::new()));
        container.insert(Arc::new(RectPropertyEditorDefinition::<u32>::new()));
        container.insert(Arc::new(RectPropertyEditorDefinition::<i16>::new()));
        container.insert(Arc::new(RectPropertyEditorDefinition::<u16>::new()));
        container.insert(Arc::new(RectPropertyEditorDefinition::<i8>::new()));
        container.insert(Arc::new(RectPropertyEditorDefinition::<u8>::new()));

        container.insert(Arc::new(ColorPropertyEditorDefinition));

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
