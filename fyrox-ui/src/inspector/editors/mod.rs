use crate::{
    core::{inspect::PropertyInfo, pool::Handle},
    inspector::{
        editors::{
            bool::BoolPropertyEditorDefinition,
            color::ColorPropertyEditorDefinition,
            numeric::NumericPropertyEditorDefinition,
            quat::QuatPropertyEditorDefinition,
            range::RangePropertyEditorDefinition,
            rect::RectPropertyEditorDefinition,
            string::StringPropertyEditorDefinition,
            vec::{
                Vec2PropertyEditorDefinition, Vec3PropertyEditorDefinition,
                Vec4PropertyEditorDefinition,
            },
        },
        InspectorEnvironment, InspectorError, PropertyChanged,
    },
    message::UiMessage,
    BuildContext, UiNode, UserInterface,
};
use fxhash::FxHashMap;
use std::{
    any::TypeId,
    cell::{Ref, RefCell},
    fmt::Debug,
    rc::Rc,
};

pub mod array;
pub mod bit;
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
    pub layer_index: usize,
}

pub struct PropertyEditorMessageContext<'a, 'b> {
    pub sync_flag: u64,
    pub instance: Handle<UiNode>,
    pub ui: &'b mut UserInterface,
    pub property_info: &'a PropertyInfo<'a>,
    pub definition_container: Rc<PropertyEditorDefinitionContainer>,
    pub layer_index: usize,
    pub environment: Option<Rc<dyn InspectorEnvironment>>,
}

pub struct PropertyEditorTranslationContext<'b, 'c> {
    pub environment: Option<Rc<dyn InspectorEnvironment>>,
    pub name: &'b str,
    pub owner_type_id: TypeId,
    pub message: &'c UiMessage,
}

pub enum PropertyEditorInstance {
    Simple {
        /// A property editor. Could be any widget that capable of editing a property
        /// value.
        editor: Handle<UiNode>,
    },
    Custom {
        container: Handle<UiNode>,

        /// A property editor. Could be any widget that capable of editing a property
        /// value.
        editor: Handle<UiNode>,
    },
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

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged>;
}

#[derive(Clone, Default)]
pub struct PropertyEditorDefinitionContainer {
    definitions: RefCell<FxHashMap<TypeId, Rc<dyn PropertyEditorDefinition>>>,
}

impl PropertyEditorDefinitionContainer {
    pub fn new() -> Self {
        let container = Self::default();

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
        &self,
        definition: T,
    ) -> Option<Rc<dyn PropertyEditorDefinition>> {
        self.definitions
            .borrow_mut()
            .insert(definition.value_type_id(), Rc::new(definition))
    }

    pub fn definitions(&self) -> Ref<FxHashMap<TypeId, Rc<dyn PropertyEditorDefinition>>> {
        self.definitions.borrow()
    }
}
