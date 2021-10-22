use crate::inspector::editors::bool::BoolPropertyEditorDefinition;
use crate::inspector::editors::range::RangePropertyEditorDefinition;
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
use std::{any::TypeId, collections::HashMap, fmt::Debug, rc::Rc};

pub mod bool;
pub mod collection;
pub mod color;
pub mod enumeration;
pub mod inspectable;
pub mod numeric;
pub mod quat;
pub mod range;
pub mod rect;
pub mod string;
pub mod vec;

pub struct PropertyEditorBuildContext<'a, 'b, 'c> {
    pub build_context: &'a mut BuildContext<'c>,
    pub property_info: &'b PropertyInfo<'b>,
    pub environment: Option<Rc<dyn InspectorEnvironment>>,
    pub definition_container: Rc<PropertyEditorDefinitionContainer>,
    pub sync_flag: u64,
}

pub struct PropertyEditorMessageContext<'a, 'b> {
    pub sync_flag: u64,
    pub instance: Handle<UiNode>,
    pub ui: &'b mut UserInterface,
    pub property_info: &'a PropertyInfo<'a>,
    pub definition_container: Rc<PropertyEditorDefinitionContainer>,
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

pub trait PropertyEditorDefinition: Debug {
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
    definitions: HashMap<TypeId, Rc<dyn PropertyEditorDefinition>>,
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

        container.insert(BoolPropertyEditorDefinition);

        container.insert(StringPropertyEditorDefinition);

        container.insert(NumericPropertyEditorDefinition::<f64>::default());
        container.insert(NumericPropertyEditorDefinition::<f32>::default());
        container.insert(NumericPropertyEditorDefinition::<i64>::default());
        container.insert(NumericPropertyEditorDefinition::<u64>::default());
        container.insert(NumericPropertyEditorDefinition::<i32>::default());
        container.insert(NumericPropertyEditorDefinition::<u32>::default());
        container.insert(NumericPropertyEditorDefinition::<i16>::default());
        container.insert(NumericPropertyEditorDefinition::<u16>::default());
        container.insert(NumericPropertyEditorDefinition::<i8>::default());
        container.insert(NumericPropertyEditorDefinition::<u8>::default());
        container.insert(NumericPropertyEditorDefinition::<usize>::default());
        container.insert(NumericPropertyEditorDefinition::<isize>::default());

        container.insert(Vec4PropertyEditorDefinition::<f64>::default());
        container.insert(Vec4PropertyEditorDefinition::<f32>::default());
        container.insert(Vec4PropertyEditorDefinition::<i64>::default());
        container.insert(Vec4PropertyEditorDefinition::<u64>::default());
        container.insert(Vec4PropertyEditorDefinition::<i32>::default());
        container.insert(Vec4PropertyEditorDefinition::<u32>::default());
        container.insert(Vec4PropertyEditorDefinition::<i16>::default());
        container.insert(Vec4PropertyEditorDefinition::<u16>::default());
        container.insert(Vec4PropertyEditorDefinition::<i8>::default());
        container.insert(Vec4PropertyEditorDefinition::<u8>::default());
        container.insert(Vec4PropertyEditorDefinition::<usize>::default());
        container.insert(Vec4PropertyEditorDefinition::<isize>::default());

        container.insert(Vec3PropertyEditorDefinition::<f64>::default());
        container.insert(Vec3PropertyEditorDefinition::<f32>::default());
        container.insert(Vec3PropertyEditorDefinition::<i64>::default());
        container.insert(Vec3PropertyEditorDefinition::<u64>::default());
        container.insert(Vec3PropertyEditorDefinition::<i32>::default());
        container.insert(Vec3PropertyEditorDefinition::<u32>::default());
        container.insert(Vec3PropertyEditorDefinition::<i16>::default());
        container.insert(Vec3PropertyEditorDefinition::<u16>::default());
        container.insert(Vec3PropertyEditorDefinition::<i8>::default());
        container.insert(Vec3PropertyEditorDefinition::<u8>::default());
        container.insert(Vec3PropertyEditorDefinition::<usize>::default());
        container.insert(Vec3PropertyEditorDefinition::<isize>::default());

        container.insert(Vec2PropertyEditorDefinition::<f64>::default());
        container.insert(Vec2PropertyEditorDefinition::<f32>::default());
        container.insert(Vec2PropertyEditorDefinition::<i64>::default());
        container.insert(Vec2PropertyEditorDefinition::<u64>::default());
        container.insert(Vec2PropertyEditorDefinition::<i32>::default());
        container.insert(Vec2PropertyEditorDefinition::<u32>::default());
        container.insert(Vec2PropertyEditorDefinition::<i16>::default());
        container.insert(Vec2PropertyEditorDefinition::<u16>::default());
        container.insert(Vec2PropertyEditorDefinition::<i8>::default());
        container.insert(Vec2PropertyEditorDefinition::<u8>::default());
        container.insert(Vec2PropertyEditorDefinition::<usize>::default());
        container.insert(Vec2PropertyEditorDefinition::<isize>::default());

        container.insert(RangePropertyEditorDefinition::<f64>::new());
        container.insert(RangePropertyEditorDefinition::<f32>::new());
        container.insert(RangePropertyEditorDefinition::<i64>::new());
        container.insert(RangePropertyEditorDefinition::<u64>::new());
        container.insert(RangePropertyEditorDefinition::<i32>::new());
        container.insert(RangePropertyEditorDefinition::<u32>::new());
        container.insert(RangePropertyEditorDefinition::<i16>::new());
        container.insert(RangePropertyEditorDefinition::<u16>::new());
        container.insert(RangePropertyEditorDefinition::<i8>::new());
        container.insert(RangePropertyEditorDefinition::<u8>::new());
        container.insert(RangePropertyEditorDefinition::<usize>::new());
        container.insert(RangePropertyEditorDefinition::<isize>::new());

        container.insert(QuatPropertyEditorDefinition::<f64>::default());
        container.insert(QuatPropertyEditorDefinition::<f32>::default());

        container.insert(RectPropertyEditorDefinition::<f64>::new());
        container.insert(RectPropertyEditorDefinition::<f32>::new());
        container.insert(RectPropertyEditorDefinition::<i32>::new());
        container.insert(RectPropertyEditorDefinition::<u32>::new());
        container.insert(RectPropertyEditorDefinition::<i16>::new());
        container.insert(RectPropertyEditorDefinition::<u16>::new());
        container.insert(RectPropertyEditorDefinition::<i8>::new());
        container.insert(RectPropertyEditorDefinition::<u8>::new());
        container.insert(RectPropertyEditorDefinition::<usize>::new());
        container.insert(RectPropertyEditorDefinition::<isize>::new());

        container.insert(ColorPropertyEditorDefinition);

        container
    }

    pub fn insert<T: PropertyEditorDefinition + 'static>(
        &mut self,
        definition: T,
    ) -> Option<Rc<dyn PropertyEditorDefinition>> {
        self.definitions
            .insert(definition.value_type_id(), Rc::new(definition))
    }

    pub fn definitions(&self) -> &HashMap<TypeId, Rc<dyn PropertyEditorDefinition>> {
        &self.definitions
    }
}
