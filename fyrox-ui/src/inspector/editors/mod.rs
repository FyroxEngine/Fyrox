use crate::{
    bit::BitField,
    border::Border,
    brush::{Brush, GradientPoint},
    button::Button,
    canvas::Canvas,
    core::{
        algebra::{UnitQuaternion, Vector2, Vector3, Vector4},
        color::Color,
        color_gradient::ColorGradient,
        curve::Curve,
        math::{Rect, SmoothAngle},
        pool::Handle,
        reflect::{FieldInfo, FieldValue, Reflect},
        uuid::Uuid,
        visitor::prelude::*,
    },
    decorator::Decorator,
    dropdown_list::DropdownList,
    expander::Expander,
    formatted_text::{FormattedText, WrapMode},
    grid::{Grid, GridDimension, SizeMode},
    image::Image,
    inspector::{
        editors::{
            array::ArrayPropertyEditorDefinition,
            bool::BoolPropertyEditorDefinition,
            collection::{CollectionItem, VecCollectionPropertyEditorDefinition},
            color::{ColorGradientPropertyEditorDefinition, ColorPropertyEditorDefinition},
            curve::CurvePropertyEditorDefinition,
            enumeration::{EnumPropertyEditorDefinition, InspectableEnum},
            inherit::InheritablePropertyEditorDefinition,
            inspectable::InspectablePropertyEditorDefinition,
            key::KeyBindingPropertyEditorDefinition,
            numeric::NumericPropertyEditorDefinition,
            quat::QuatPropertyEditorDefinition,
            range::RangePropertyEditorDefinition,
            rect::RectPropertyEditorDefinition,
            refcell::RefCellPropertyEditorDefinition,
            string::StringPropertyEditorDefinition,
            utf32::Utf32StringPropertyEditorDefinition,
            uuid::UuidPropertyEditorDefinition,
            vec::{
                Vec2PropertyEditorDefinition, Vec3PropertyEditorDefinition,
                Vec4PropertyEditorDefinition,
            },
        },
        InspectorEnvironment, InspectorError, PropertyChanged, PropertyFilter,
    },
    key::KeyBinding,
    key::{HotKeyEditor, KeyBindingEditor},
    list_view::{ListView, ListViewItem},
    menu::{Menu, MenuItem},
    message::CursorIcon,
    message::UiMessage,
    messagebox::MessageBox,
    nine_patch::NinePatch,
    numeric::NumericUpDown,
    path::PathEditor,
    popup::Popup,
    progress_bar::ProgressBar,
    range::RangeEditor,
    rect::RectEditor,
    scroll_bar::ScrollBar,
    scroll_panel::ScrollPanel,
    stack_panel::StackPanel,
    tab_control::TabControl,
    text::Text,
    text_box::TextBox,
    tree::{Tree, TreeRoot},
    uuid::UuidEditor,
    vec::VecEditor,
    vector_image::VectorImage,
    widget::Widget,
    window::Window,
    wrap_panel::WrapPanel,
    BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
    VerticalAlignment,
};
use fxhash::FxHashMap;
use std::{
    any::TypeId,
    cell::{Ref, RefCell},
    fmt::Debug,
    ops::Range,
    rc::Rc,
    str::FromStr,
};
use strum::VariantNames;

pub mod array;
pub mod bit;
pub mod bool;
pub mod collection;
pub mod color;
pub mod curve;
pub mod enumeration;
pub mod inherit;
pub mod inspectable;
pub mod key;
pub mod numeric;
pub mod path;
pub mod quat;
pub mod range;
pub mod rect;
pub mod refcell;
pub mod string;
pub mod utf32;
pub mod uuid;
pub mod vec;

pub struct PropertyEditorBuildContext<'a, 'b, 'c, 'd> {
    pub build_context: &'a mut BuildContext<'c>,
    pub property_info: &'b FieldInfo<'b, 'd>,
    pub environment: Option<Rc<dyn InspectorEnvironment>>,
    pub definition_container: Rc<PropertyEditorDefinitionContainer>,
    pub sync_flag: u64,
    pub layer_index: usize,
    pub generate_property_string_values: bool,
    pub filter: PropertyFilter,
}

pub struct PropertyEditorMessageContext<'a, 'b, 'c> {
    pub sync_flag: u64,
    pub instance: Handle<UiNode>,
    pub ui: &'b mut UserInterface,
    pub property_info: &'a FieldInfo<'a, 'c>,
    pub definition_container: Rc<PropertyEditorDefinitionContainer>,
    pub layer_index: usize,
    pub environment: Option<Rc<dyn InspectorEnvironment>>,
    pub generate_property_string_values: bool,
    pub filter: PropertyFilter,
}

pub struct PropertyEditorTranslationContext<'b, 'c> {
    pub environment: Option<Rc<dyn InspectorEnvironment>>,
    pub name: &'b str,
    pub owner_type_id: TypeId,
    pub message: &'c UiMessage,
    pub definition_container: Rc<PropertyEditorDefinitionContainer>,
}

#[derive(Clone, Debug, PartialEq, Visit, Reflect)]
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

impl Default for PropertyEditorInstance {
    fn default() -> Self {
        Self::Simple {
            editor: Default::default(),
        }
    }
}

impl PropertyEditorInstance {
    pub fn editor(&self) -> Handle<UiNode> {
        match self {
            PropertyEditorInstance::Simple { editor }
            | PropertyEditorInstance::Custom { editor, .. } => *editor,
        }
    }
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

macro_rules! reg_array_property_editor {
    ($container:ident, $ty:ty, $($count:literal),*) => {
        $(
            $container.insert(ArrayPropertyEditorDefinition::<$ty, $count>::new());
        )*
    }
}

macro_rules! reg_property_editor {
    ($container:ident, $base:ident:$init:ident, $($ty:ty),*) => {
        $(
             $container.insert($base::<$ty>::$init());
        )*
    }
}

macro_rules! reg_inspectables {
    ($container:ident, $($ty:ty),*) => {
        $(
             $container.insert(InspectablePropertyEditorDefinition::<$ty>::new());
        )*
    }
}

impl PropertyEditorDefinitionContainer {
    pub fn new() -> Self {
        let container = Self::default();

        // bool + InheritableVariable<bool>
        container.insert(InheritablePropertyEditorDefinition::<bool>::new());
        container.insert(BoolPropertyEditorDefinition);

        // String + InheritableVariable<String>
        container.insert(StringPropertyEditorDefinition);
        container.insert(InheritablePropertyEditorDefinition::<String>::new());

        // NumericType + InheritableVariable<NumericType>
        reg_property_editor! { container, NumericPropertyEditorDefinition: default, f64, f32, i64, u64, i32, u32, i16, u16, i8, u8, usize, isize }
        reg_property_editor! { container, InheritablePropertyEditorDefinition: new, f64, f32, i64, u64, i32, u32, i16, u16, i8, u8, usize, isize }

        // Vector4<NumericType> + InheritableVariable<Vector4>
        reg_property_editor! { container, Vec4PropertyEditorDefinition: default, f64, f32, i64, u64, i32, u32, i16, u16, i8, u8, usize, isize }
        reg_property_editor! { container, InheritablePropertyEditorDefinition: new,
            Vector4<f64>, Vector4<f32>, Vector4<i64>, Vector4<u64>, Vector4<i32>, Vector4<u32>,
            Vector4<i16>, Vector4<u16>, Vector4<i8>, Vector4<u8>, Vector4<usize>, Vector4<isize>
        }

        // Vector3<NumericType> + InheritableVariable<Vector3>
        reg_property_editor! { container, Vec3PropertyEditorDefinition: default, f64, f32, i64, u64, i32, u32, i16, u16, i8, u8, usize, isize }
        reg_property_editor! { container, InheritablePropertyEditorDefinition: new,
            Vector3<f64>, Vector3<f32>, Vector3<i64>, Vector3<u64>, Vector3<i32>, Vector3<u32>,
            Vector3<i16>, Vector3<u16>, Vector3<i8>, Vector3<u8>, Vector3<usize>, Vector3<isize>
        }

        // Vector2<NumericType> + InheritableVariable<Vector2>
        reg_property_editor! { container, Vec2PropertyEditorDefinition: default, f64, f32, i64, u64, i32, u32, i16, u16, i8, u8, usize, isize }
        reg_property_editor! { container, InheritablePropertyEditorDefinition: new,
            Vector2<f64>, Vector2<f32>, Vector2<i64>, Vector2<u64>, Vector2<i32>, Vector2<u32>,
            Vector2<i16>, Vector2<u16>, Vector2<i8>, Vector2<u8>, Vector2<usize>, Vector2<isize>
        }

        // Range<NumericType> + InheritableVariable<Range<NumericType>>
        reg_property_editor! { container, RangePropertyEditorDefinition: new, f64, f32, i64, u64, i32, u32, i16, u16, i8, u8, usize, isize }
        reg_property_editor! { container, InheritablePropertyEditorDefinition: new,
            Range<f64>, Range<f32>, Range<i64>, Range<u64>, Range<i32>, Range<u32>,
            Range<i16>, Range<u16>, Range<i8>, Range<u8>, Range<usize>, Range<isize>
        }

        // UnitQuaternion + InheritableVariable<UnitQuaternion>
        container.insert(QuatPropertyEditorDefinition::<f64>::default());
        container.insert(InheritablePropertyEditorDefinition::<UnitQuaternion<f64>>::new());
        container.insert(QuatPropertyEditorDefinition::<f32>::default());
        container.insert(InheritablePropertyEditorDefinition::<UnitQuaternion<f32>>::new());

        // Rect<NumericType> + InheritableVariable<Rect<NumericType>>
        reg_property_editor! { container, RectPropertyEditorDefinition: new, f64, f32, i64, u64, i32, u32, i16, u16, i8, u8, usize, isize };
        reg_property_editor! { container, InheritablePropertyEditorDefinition: new,
            Rect<f64>, Rect<f32>, Rect<i64>, Rect<u64>, Rect<i32>, Rect<u32>,
            Rect<i16>, Rect<u16>, Rect<i8>, Rect<u8>, Rect<usize>, Rect<isize>
        }

        // Option<NumericType> + InheritableVariable<Option<NumericType>>
        reg_property_editor! { container, EnumPropertyEditorDefinition: new_optional, f64, f32, i64, u64, i32, u32, i16, u16, i8, u8, usize, isize }
        reg_property_editor! { container, InheritablePropertyEditorDefinition: new,
            Option<f64>, Option<f32>, Option<i64>, Option<u64>, Option<i32>, Option<u32>,
            Option<i16>, Option<u16>, Option<i8>, Option<u8>, Option<usize>, Option<isize>
        }

        // Color + InheritableVariable<Color>
        container.insert(ColorPropertyEditorDefinition);
        container.insert(InheritablePropertyEditorDefinition::<Color>::new());

        // [NumericType; 1..N]
        reg_array_property_editor! { container, f64, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 };
        reg_array_property_editor! { container, f32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 }
        reg_array_property_editor! { container, u64, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 }
        reg_array_property_editor! { container, i64, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 }
        reg_array_property_editor! { container, u32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 }
        reg_array_property_editor! { container, i32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 }
        reg_array_property_editor! { container, u16, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 }
        reg_array_property_editor! { container, i16, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 }
        reg_array_property_editor! { container, u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 }
        reg_array_property_editor! { container, i8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 }
        reg_array_property_editor! { container, u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 }
        reg_array_property_editor! { container, usize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 }
        reg_array_property_editor! { container, isize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 }

        // SmoothAngle
        container.register_inheritable_inspectable::<SmoothAngle>();

        // Uuid + InheritableVariable<Uuid>
        container.insert(UuidPropertyEditorDefinition);
        container.insert(InheritablePropertyEditorDefinition::<Uuid>::new());

        // Color Gradient.
        container.insert(ColorGradientPropertyEditorDefinition);
        container.insert(InheritablePropertyEditorDefinition::<ColorGradient>::new());

        // Key Binding
        container.insert(KeyBindingPropertyEditorDefinition);
        container.insert(InheritablePropertyEditorDefinition::<KeyBinding>::new());

        // Curve
        container.insert(CurvePropertyEditorDefinition);
        container.insert(InheritablePropertyEditorDefinition::<Curve>::new());

        // UI
        container.insert(EnumPropertyEditorDefinition::<Brush>::new());
        container.insert(EnumPropertyEditorDefinition::<Orientation>::new());
        container.insert(EnumPropertyEditorDefinition::<VerticalAlignment>::new());
        container.insert(EnumPropertyEditorDefinition::<HorizontalAlignment>::new());
        container.insert(EnumPropertyEditorDefinition::<WrapMode>::new());
        container.insert(EnumPropertyEditorDefinition::<SizeMode>::new());
        container.insert(EnumPropertyEditorDefinition::<CursorIcon>::new());
        container.insert(EnumPropertyEditorDefinition::<CursorIcon>::new_optional());
        container.insert(EnumPropertyEditorDefinition::<bool>::new_optional());
        container.insert(VecCollectionPropertyEditorDefinition::<GradientPoint>::new());
        container.insert(RefCellPropertyEditorDefinition::<FormattedText>::new());
        container.insert(RefCellPropertyEditorDefinition::<Vec<GridDimension>>::new());
        container.insert(VecCollectionPropertyEditorDefinition::<GridDimension>::new());
        container.insert(Utf32StringPropertyEditorDefinition);
        reg_inspectables!(
            container,
            // Widgets
            Widget,
            Border,
            BitField<u8>,
            BitField<i8>,
            BitField<u16>,
            BitField<i16>,
            BitField<u32>,
            BitField<i32>,
            BitField<u64>,
            BitField<i64>,
            Button,
            Canvas,
            Decorator,
            DropdownList,
            Expander,
            Grid,
            Image,
            HotKeyEditor,
            KeyBindingEditor,
            ListViewItem,
            ListView,
            Menu,
            MenuItem,
            MessageBox,
            NinePatch,
            NumericUpDown<u8>,
            NumericUpDown<i8>,
            NumericUpDown<u16>,
            NumericUpDown<i16>,
            NumericUpDown<u32>,
            NumericUpDown<i32>,
            NumericUpDown<u64>,
            NumericUpDown<i64>,
            NumericUpDown<f32>,
            NumericUpDown<f64>,
            PathEditor,
            Popup,
            ProgressBar,
            RangeEditor<u8>,
            RangeEditor<i8>,
            RangeEditor<u16>,
            RangeEditor<i16>,
            RangeEditor<u32>,
            RangeEditor<i32>,
            RangeEditor<u64>,
            RangeEditor<i64>,
            RangeEditor<f32>,
            RangeEditor<f64>,
            RectEditor<u8>,
            RectEditor<i8>,
            RectEditor<u16>,
            RectEditor<i16>,
            RectEditor<u32>,
            RectEditor<i32>,
            RectEditor<u64>,
            RectEditor<i64>,
            RectEditor<f32>,
            RectEditor<f64>,
            ScrollBar,
            ScrollPanel,
            StackPanel,
            TabControl,
            Text,
            TextBox,
            Tree,
            TreeRoot,
            UuidEditor,
            VecEditor<u8, 2>,
            VecEditor<i8, 2>,
            VecEditor<u16,2>,
            VecEditor<i16,2>,
            VecEditor<u32,2>,
            VecEditor<i32,2>,
            VecEditor<u64,2>,
            VecEditor<i64,2>,
            VecEditor<f32,2>,
            VecEditor<f64,2>,
            VecEditor<u8, 3>,
            VecEditor<i8, 3>,
            VecEditor<u16,3>,
            VecEditor<i16,3>,
            VecEditor<u32,3>,
            VecEditor<i32,3>,
            VecEditor<u64,3>,
            VecEditor<i64,3>,
            VecEditor<f32,3>,
            VecEditor<f64,3>,
            VecEditor<u8, 4>,
            VecEditor<i8, 4>,
            VecEditor<u16,4>,
            VecEditor<i16,4>,
            VecEditor<u32,4>,
            VecEditor<i32,4>,
            VecEditor<u64,4>,
            VecEditor<i64,4>,
            VecEditor<f32,4>,
            VecEditor<f64,4>,
            VectorImage,
            Window,
            WrapPanel,
            // Structs
            GradientPoint,
            Thickness,
            FormattedText,
            GridDimension
        );

        container
    }

    pub fn insert<T>(&self, definition: T) -> Option<Rc<dyn PropertyEditorDefinition>>
    where
        T: PropertyEditorDefinition + 'static,
    {
        self.definitions
            .borrow_mut()
            .insert(definition.value_type_id(), Rc::new(definition))
    }

    pub fn register_inheritable_vec_collection<T>(&self)
    where
        T: CollectionItem + FieldValue,
    {
        assert!(self
            .insert(VecCollectionPropertyEditorDefinition::<T>::new())
            .is_none());
        assert!(self
            .insert(InheritablePropertyEditorDefinition::<Vec<T>>::new())
            .is_none());
    }

    pub fn register_inheritable_inspectable<T>(&self)
    where
        T: Reflect + FieldValue,
    {
        assert!(self
            .insert(InspectablePropertyEditorDefinition::<T>::new())
            .is_none());
        assert!(self
            .insert(InheritablePropertyEditorDefinition::<T>::new())
            .is_none());
    }

    pub fn register_inheritable_enum<T, E: Debug>(&self)
    where
        T: InspectableEnum + FieldValue + VariantNames + AsRef<str> + FromStr<Err = E> + Debug,
    {
        assert!(self
            .insert(EnumPropertyEditorDefinition::<T>::new())
            .is_none());
        assert!(self
            .insert(InheritablePropertyEditorDefinition::<T>::new())
            .is_none());
    }

    pub fn register_inheritable_option<T>(&self)
    where
        T: InspectableEnum + FieldValue + Default,
    {
        assert!(self
            .insert(EnumPropertyEditorDefinition::<T>::new_optional())
            .is_none());
        assert!(self
            .insert(InheritablePropertyEditorDefinition::<Option<T>>::new())
            .is_none());
    }

    pub fn definitions(&self) -> Ref<FxHashMap<TypeId, Rc<dyn PropertyEditorDefinition>>> {
        self.definitions.borrow()
    }
}
