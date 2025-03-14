// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! A collection of [PropertyEditorDefinition] objects for a wide variety of types,
//! including standard Rust types and Fyrox core types.

use crate::inspector::editors::texture_slice::TextureSlicePropertyEditorDefinition;
use crate::{
    absm::{EventAction, EventKind},
    bit::BitField,
    border::Border,
    brush::{Brush, GradientPoint},
    button::Button,
    canvas::Canvas,
    core::{
        algebra::{UnitQuaternion, Vector2, Vector3, Vector4},
        color::Color,
        color_gradient::ColorGradient,
        math::{curve::Curve, Rect, SmoothAngle},
        parking_lot::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard},
        pool::Handle,
        reflect::prelude::*,
        sstorage::ImmutableString,
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
            immutable_string::ImmutableStringPropertyEditorDefinition,
            inherit::InheritablePropertyEditorDefinition,
            inspectable::InspectablePropertyEditorDefinition,
            key::KeyBindingPropertyEditorDefinition,
            matrix2::MatrixPropertyEditorDefinition,
            numeric::NumericPropertyEditorDefinition,
            path::PathPropertyEditorDefinition,
            quat::QuatPropertyEditorDefinition,
            range::RangePropertyEditorDefinition,
            rect::RectPropertyEditorDefinition,
            refcell::RefCellPropertyEditorDefinition,
            string::StringPropertyEditorDefinition,
            style::StyledPropertyEditorDefinition,
            utf32::Utf32StringPropertyEditorDefinition,
            uuid::UuidPropertyEditorDefinition,
            vec::{
                Vec2PropertyEditorDefinition, Vec3PropertyEditorDefinition,
                Vec4PropertyEditorDefinition,
            },
        },
        InspectorEnvironment, InspectorError, PropertyChanged, PropertyFilter,
    },
    key::{HotKeyEditor, KeyBinding, KeyBindingEditor},
    list_view::{ListView, ListViewItem},
    menu::{Menu, MenuItem},
    message::{CursorIcon, UiMessage},
    messagebox::MessageBox,
    nine_patch::{NinePatch, StretchMode},
    numeric::NumericUpDown,
    path::PathEditor,
    popup::Popup,
    progress_bar::ProgressBar,
    range::RangeEditor,
    rect::RectEditor,
    scroll_bar::ScrollBar,
    scroll_panel::ScrollPanel,
    stack_panel::StackPanel,
    style::StyledProperty,
    tab_control::TabControl,
    text::Text,
    text_box::{Position, SelectionRange, TextBox, TextCommitMode},
    tree::{Tree, TreeRoot},
    uuid::UuidEditor,
    vec::VecEditor,
    vector_image::{Primitive, VectorImage},
    widget::Widget,
    window::Window,
    wrap_panel::WrapPanel,
    BuildContext, HorizontalAlignment, Orientation, RcUiNodeHandle, RcUiNodeHandleInner, Thickness,
    UiNode, UserInterface, VerticalAlignment,
};
use fxhash::FxHashMap;
use fyrox_animation::machine::Parameter;
use fyrox_texture::TextureResource;
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    fmt::Debug,
    fmt::Formatter,
    ops::Range,
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};
use strum::VariantNames;

pub mod array;
pub mod bit;
pub mod bool;
pub mod collection;
pub mod color;
pub mod curve;
pub mod enumeration;
pub mod immutable_string;
pub mod inherit;
pub mod inspectable;
pub mod key;
pub mod matrix2;
pub mod numeric;
pub mod path;
pub mod quat;
pub mod range;
pub mod rect;
pub mod refcell;
pub mod string;
mod style;
pub mod texture_slice;
pub mod utf32;
pub mod uuid;
pub mod vec;

/// This structure is passed to [PropertyEditorDefinition::create_instance] in order to allow it to
/// build a widget to allow a property to be edited.
pub struct PropertyEditorBuildContext<'a, 'b, 'c, 'd> {
    /// General context for widget building to be used for creating the editor.
    pub build_context: &'a mut BuildContext<'c>,
    /// The FieldInfo of the property to edit, extracted from the object we are inspecting by reflection.
    pub property_info: &'b FieldRef<'b, 'd>,
    /// Untyped reference to the environment that the Inspector is being used in.
    /// This will often be
    /// [fyroxed_base::inspector::EditorEnvironment](https://docs.rs/fyroxed_base/latest/fyroxed_base/inspector/struct.EditorEnvironment.html)
    /// when the Inspector is being used in Fyroxed, but Inspector widgets can be used in other applications,
    /// and we can access those applications by casting the environment to the appropriate type.
    pub environment: Option<Arc<dyn InspectorEnvironment>>,
    /// The list of the Inspectors property editors.
    /// This allows one property editor to make use of other property editors.
    pub definition_container: Arc<PropertyEditorDefinitionContainer>,
    /// Controls the flags that are included with messages through the [UiMessage::flags] property.
    /// This is used to distinguish sync messages from other messages and is handled automatically by
    /// [InspectorContext](crate::inspector::InspectorContext).
    pub sync_flag: u64,
    /// Editors can be nested within other editors, such as when an array
    /// editor contains editors for each element of the array.
    /// The layer_index indicates how deeply nested the editor widget we
    /// are creating will be.
    pub layer_index: usize,
    /// When true, this indicates that an Inspector should generate strings from `format!("{:?}", field)`, for each field.
    /// Having this in the property editor build context indicates how any Inspectors that are created as part of the new
    /// editor should behave.
    pub generate_property_string_values: bool,
    /// Determines how properties should be filtered in any Inspectors created within the editor that is being built.
    pub filter: PropertyFilter,
    /// Width of the property name column.
    pub name_column_width: f32,
}

/// This structure is passed to [PropertyEditorDefinition::create_message] in order to generate a message that will
/// update the editor widget to the property's current value.
pub struct PropertyEditorMessageContext<'a, 'b, 'c> {
    /// Controls the flags that are included with messages through the [UiMessage::flags] property.
    /// This is used to distinguish sync messages from other messages and is handled automatically by
    /// [InspectorContext](crate::inspector::InspectorContext).
    /// There is no need to put this flag into the message return by the create_message method.
    pub sync_flag: u64,
    /// The handle of widget that the message will be sent to. It should be an editor created by
    /// [PropertyEditorDefinition::create_instance].
    pub instance: Handle<UiNode>,
    /// The UserInterface is provided to make it possible for `create_message` to send whatever messages
    /// are needed directly instead of returning a message. In this case, the sent messages should have their
    /// [UiMessage::flags] set to `sync_flag`.
    pub ui: &'b mut UserInterface,
    /// The FieldInfo of the property to edit, extracted from the object we are inspecting by reflection.
    pub property_info: &'a FieldRef<'a, 'c>,
    /// The list of the Inspectors property editors.
    /// This allows one property editor to make use of other property editors.
    pub definition_container: Arc<PropertyEditorDefinitionContainer>,
    /// Editors can be nested within other editors, such as when an array
    /// editor contains editors for each element of the array.
    /// The layer_index indicates the nesting level of the widget that will receive the created message.
    pub layer_index: usize,
    /// Optional untyped information about the broader application in which
    /// this proprety is being translated. This allows the created message to
    /// adapt to the situation if we can successfully cast the given
    /// [InspectorEnvironment] into a specific type.
    pub environment: Option<Arc<dyn InspectorEnvironment>>,
    /// When true, this indicates that an Inspector should generate strings from `format!("{:?}", field)`, for each field.
    /// Having this in the property editor build context indicates how any Inspectors that are update due to the created message
    /// should behave.
    pub generate_property_string_values: bool,
    /// Determines how properties should be filtered in any Inspectors that are updated by the created message.
    pub filter: PropertyFilter,
    /// Width of the property name column.
    pub name_column_width: f32,
}

/// The details relevant to translating a message from an editor widget into
/// a [PropertyChanged] message that an [Inspector](crate::inspector::Inspector) widget
/// can use to update the inspected property based on the messages from the editor.
pub struct PropertyEditorTranslationContext<'b, 'c> {
    /// Optional untyped information about the broader application in which
    /// this proprety is being translated. This allows the translation to
    /// adapt to the situation if we can successfully cast the given
    /// [InspectorEnvironment] into a specific type.
    ///
    /// When the environment is not None, it is often an
    /// [fyroxed_base::inspector::EditorEnvironment](https://docs.rs/fyroxed_base/latest/fyroxed_base/inspector/struct.EditorEnvironment.html)
    /// which may be accessed using EditorEnvironment::try_get_from.
    /// For example, the EditorEnvironment can be used by
    /// [fyroxed_base::inspector::editors::script::ScriptPropertyEditor](https://docs.rs/fyroxed_base/latest/fyroxed_base/inspector/editors/script/struct.ScriptPropertyEditor.html)
    /// to translate the UUID of a script into an actual
    /// [fyrox::script::Script](https://docs.rs/fyrox/latest/fyrox/script/struct.Script.html)
    /// when it receives a
    /// [ScriptPropertyEditorMessage::Value](https://docs.rs/fyroxed_base/latest/fyroxed_base/inspector/editors/script/enum.ScriptPropertyEditorMessage.html#variant.Value).
    pub environment: Option<Arc<dyn InspectorEnvironment>>,
    /// The name of the property being edited.
    /// This comes from [ContextEntry::property_name](crate::inspector::ContextEntry).
    pub name: &'b str,
    /// The original message that may be translated, if it represents a change in the property.
    pub message: &'c UiMessage,
    /// The list of the Inspectors property editors.
    /// This allows one property editor to make use of other property editors.
    pub definition_container: Arc<PropertyEditorDefinitionContainer>,
}

/// A widget handle that is to act as an editor in an [Insector](crate::inspector::Inspector), with or without
/// a custom container widget to show the name of the property that is being edited.
#[derive(Clone, Debug, PartialEq, Visit, Reflect)]
pub enum PropertyEditorInstance {
    /// A property editor that is to be given a default container, which is just a label to the left
    /// of the editor to show the name of the property being edited.
    Simple {
        /// A property editor. Could be any widget that capable of editing a property
        /// value.
        editor: Handle<UiNode>,
    },
    /// A property editor that comes with its own custom container.
    Custom {
        /// A widget that contains the editor.
        /// It should include a label to identify the property being edited.
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

/// The trait for all property editor definitions which are capable of providing
/// and editor widget to an [Inspector](crate::inspector::Inspector) and helping
/// the inspector handle the necessary messages to and from that widget.
pub trait PropertyEditorDefinition: Debug + Send + Sync {
    /// The type of property that the editor will edit.
    fn value_type_id(&self) -> TypeId;

    /// Build a widget that an [Inspector](crate::inspector::Inspector) can use to edit this property.
    /// The returned value is either a simple property editor instance which contains just a
    /// UiNode handle, or else it is a custom editor instance that contains both
    /// the handle of the editor and the handle of the container.
    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError>;

    /// Create a message that will tell the editor widget to update itself with the current value
    /// of the property. This is called by [InspectorContext::sync](crate::inspector::InspectorContext::sync).
    ///
    /// Despite the name, this method is also permitted to send messages directly to the widget instead
    /// of returning anything. If messages are sent directly, they should have their [UiMessage::flags] set
    /// to [PropertyEditorMessageContext::sync_flag], as this is required to identify the message a sync message
    /// and prevent potential infinite message loops.
    ///
    /// If a message is returned, the caller is responsible for setting `flags` and sending the message.
    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError>;

    /// Translate messages from the editor widget created by [PropertyEditorDefinition::create_message] into
    /// [PropertyChanged] messages that the [Inspector](crate::inspector::Inspector) widget can use to apply updates.
    /// The given [PropertyEditorTranslationContext] contains all the relevant details of the message to be
    /// translated.
    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged>;
}

/// One entry from the list of editor definitions in a [PropertyEditorDefinitionContainer].
pub struct PropertyEditorDefinitionContainerEntry {
    /// A type representing the source of `property_editor`.
    /// This value is set equal to [PropertyEditorDefinitionContainer::context_type_id] when
    /// this entry is created by inserting `property_editor`.
    /// The value of this type can be used to indicate whether this property editor definition
    /// comes from a plugin.
    pub source_type_id: TypeId,
    /// The PropertyEditorDefinition to be used by some inspector to create
    /// and control its child widgets.
    pub property_editor: Box<dyn PropertyEditorDefinition>,
}

/// This is a list of [PropertyEditorDefinition] which is indexed by the type that each
/// editor edits, as specified by [PropertyEditorDefinition::value_type_id].
/// It also records where each entry in the list came from so that it can know whether
/// a property editor is built-in to the Fyroxed or whether it was added by a plugin.
/// This allows entries to be removed when a plugin is unloaded.
pub struct PropertyEditorDefinitionContainer {
    /// A type representing the source of PropertyEditorDefinitions that are added in the future.
    /// For each added PropertyEditorDefinition entry, [PropertyEditorDefinitionContainerEntry::source_type_id]
    /// is set equal to this TypeId. By default this begins as `().type_id()`, and then it can be modified
    /// with a plugin is loaded to cause all definitions added after that point to be marked as being from
    /// that plugin.
    pub context_type_id: Mutex<TypeId>,
    definitions: RwLock<FxHashMap<TypeId, PropertyEditorDefinitionContainerEntry>>,
}

impl Debug for PropertyEditorDefinitionContainer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "PropertyEditorDefinitionContainer")
    }
}

impl Default for PropertyEditorDefinitionContainer {
    fn default() -> Self {
        Self {
            context_type_id: Mutex::new(().type_id()),
            definitions: Default::default(),
        }
    }
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

macro_rules! reg_matrix_property_editor {
    ($container:ident, $base:ident[$rows:expr, $columns:expr]:$init:ident, $($ty:ty),*) => {
        $(
             $container.insert($base::<$rows, $columns, $ty>::$init());
        )*
    }
}

impl PropertyEditorDefinitionContainer {
    pub fn empty() -> Self {
        Self::default()
    }

    /// A container with property editors for Fyrox core types and Rust standard types.
    pub fn with_default_editors() -> Self {
        let container = Self::default();

        // bool + InheritableVariable<bool>
        container.insert(InheritablePropertyEditorDefinition::<bool>::new());
        container.insert(BoolPropertyEditorDefinition);

        // String
        container.insert(StringPropertyEditorDefinition);
        container.insert(InheritablePropertyEditorDefinition::<String>::new());
        container.insert(VecCollectionPropertyEditorDefinition::<String>::new());

        // ImmutableString
        container.insert(ImmutableStringPropertyEditorDefinition);
        container.insert(InheritablePropertyEditorDefinition::<ImmutableString>::new());
        container.insert(VecCollectionPropertyEditorDefinition::<ImmutableString>::new());

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

        reg_matrix_property_editor! { container, MatrixPropertyEditorDefinition[2, 2]: default, f64, f32, i64, u64, i32, u32, i16, u16, i8, u8, usize, isize }
        reg_matrix_property_editor! { container, MatrixPropertyEditorDefinition[3, 3]: default, f64, f32, i64, u64, i32, u32, i16, u16, i8, u8, usize, isize }
        reg_matrix_property_editor! { container, MatrixPropertyEditorDefinition[4, 4]: default, f64, f32, i64, u64, i32, u32, i16, u16, i8, u8, usize, isize }

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
        reg_property_editor! { container, InheritablePropertyEditorDefinition: new,
            Option<Rect<f64>>, Option<Rect<f32>>, Option<Rect<i64>>, Option<Rect<u64>>, Option<Rect<i32>>, Option<Rect<u32>>,
            Option<Rect<i16>>, Option<Rect<u16>>, Option<Rect<i8>>, Option<Rect<u8>>, Option<Rect<usize>>, Option<Rect<isize>>
        }
        reg_property_editor! { container, EnumPropertyEditorDefinition: new_optional,
            Rect<f64>, Rect<f32>, Rect<i64>, Rect<u64>, Rect<i32>, Rect<u32>,
            Rect<i16>, Rect<u16>, Rect<i8>, Rect<u8>, Rect<usize>, Rect<isize>
        }

        // Option<NumericType> + InheritableVariable<Option<NumericType>>
        reg_property_editor! { container, EnumPropertyEditorDefinition: new_optional, f64, f32, i64, u64, i32, u32, i16, u16, i8, u8, usize, isize }
        reg_property_editor! { container, InheritablePropertyEditorDefinition: new,
            Option<f64>, Option<f32>, Option<i64>, Option<u64>, Option<i32>, Option<u32>,
            Option<i16>, Option<u16>, Option<i8>, Option<u8>, Option<usize>, Option<isize>
        }

        // Path
        container.insert(PathPropertyEditorDefinition);
        container.insert(VecCollectionPropertyEditorDefinition::<PathBuf>::new());

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
        container.register_inheritable_styleable_enum::<Brush, _>();
        container.register_inheritable_enum::<Orientation, _>();
        container.register_inheritable_enum::<VerticalAlignment, _>();
        container.register_inheritable_enum::<HorizontalAlignment, _>();
        container.register_inheritable_enum::<WrapMode, _>();
        container.register_inheritable_enum::<Primitive, _>();
        container.register_inheritable_enum::<SizeMode, _>();
        container.insert(EnumPropertyEditorDefinition::<CursorIcon>::new());
        container.insert(EnumPropertyEditorDefinition::<CursorIcon>::new_optional());
        container.insert(EnumPropertyEditorDefinition::<bool>::new_optional());
        container.insert(InheritablePropertyEditorDefinition::<Option<bool>>::new());
        container.insert(InheritablePropertyEditorDefinition::<Option<CursorIcon>>::new());

        container.register_inheritable_vec_collection::<GradientPoint>();
        container.register_inheritable_vec_collection::<Primitive>();

        container.insert(RefCellPropertyEditorDefinition::<FormattedText>::new());

        container.insert(VecCollectionPropertyEditorDefinition::<GridDimension>::new());
        container.insert(RefCellPropertyEditorDefinition::<Vec<GridDimension>>::new());
        container.insert(InheritablePropertyEditorDefinition::<
            RefCell<Vec<GridDimension>>,
        >::new());

        container.insert(Utf32StringPropertyEditorDefinition);
        container.insert(InheritablePropertyEditorDefinition::<Vec<char>>::new());

        container.insert(InheritablePropertyEditorDefinition::<Thickness>::new());

        container.register_inheritable_enum::<EventKind, _>();

        container.register_inheritable_enum::<StretchMode, _>();

        container.insert(InspectablePropertyEditorDefinition::<EventAction>::new());
        container.register_inheritable_vec_collection::<EventAction>();

        container.insert(EnumPropertyEditorDefinition::<Parameter>::new());

        container.insert(EnumPropertyEditorDefinition::<TextCommitMode>::new());
        container.insert(InheritablePropertyEditorDefinition::<TextCommitMode>::new());

        container.insert(EnumPropertyEditorDefinition::<SelectionRange>::new_optional());
        container.insert(InheritablePropertyEditorDefinition::<Option<SelectionRange>>::new());

        container.register_inheritable_inspectable::<Position>();

        container.insert(EnumPropertyEditorDefinition::<RcUiNodeHandle>::new_optional());
        container.insert(InspectablePropertyEditorDefinition::<RcUiNodeHandle>::new());
        container.insert(InspectablePropertyEditorDefinition::<RcUiNodeHandleInner>::new());
        container.insert(InspectablePropertyEditorDefinition::<
            Arc<Mutex<RcUiNodeHandleInner>>,
        >::new());

        container.insert(TextureSlicePropertyEditorDefinition);

        // Styled.
        container.insert(InheritablePropertyEditorDefinition::<StyledProperty<f32>>::new());
        container.insert(StyledPropertyEditorDefinition::<f32>::new());

        container.insert(InheritablePropertyEditorDefinition::<StyledProperty<Color>>::new());
        container.insert(StyledPropertyEditorDefinition::<Color>::new());

        container.insert(InheritablePropertyEditorDefinition::<
            StyledProperty<Thickness>,
        >::new());
        container.insert(StyledPropertyEditorDefinition::<Thickness>::new());

        container.insert(InheritablePropertyEditorDefinition::<
            StyledProperty<TextureResource>,
        >::new());
        container.insert(StyledPropertyEditorDefinition::<TextureResource>::new());

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

    /// Add an already boxed dynamic PropertyEditorDefinition to the list.
    /// If this container already had a PropertyEditorDefinition for the same type,
    /// the old property editor is removed and returned.
    pub fn insert_raw(
        &self,
        definition: Box<dyn PropertyEditorDefinition>,
    ) -> Option<PropertyEditorDefinitionContainerEntry> {
        self.definitions.write().insert(
            definition.value_type_id(),
            PropertyEditorDefinitionContainerEntry {
                source_type_id: *self.context_type_id.lock(),
                property_editor: definition,
            },
        )
    }

    /// Consume a given collection of property editors and add each entry into this collection.
    /// *Every* entry from the given collection is marked as having the current source type;
    /// whatever sources they may have had in their original container is forgotten.
    pub fn merge(&self, other: Self) {
        for (_, definition) in other.definitions.into_inner() {
            self.insert_raw(definition.property_editor);
        }
    }

    /// Move a PropertyEditorDefinition into the list, where it will automatically be boxed.
    /// If this container already had a PropertyEditorDefinition for the same type,
    /// the old property editor is removed and returned.
    pub fn insert<T>(&self, definition: T) -> Option<PropertyEditorDefinitionContainerEntry>
    where
        T: PropertyEditorDefinition + 'static,
    {
        self.definitions.write().insert(
            definition.value_type_id(),
            PropertyEditorDefinitionContainerEntry {
                source_type_id: *self.context_type_id.lock(),
                property_editor: Box::new(definition),
            },
        )
    }

    /// Inserts the default property editor for `Vec<T>` and `InheritableVariable<Vec<T>>`.
    /// Panic if these types already have editor definitions.
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

    /// Insert a [InspectablePropertyEditorDefinition] for the given type.
    /// This is a creates a generic property editor that is just a nested
    /// inspector for the properties of the value, with an [Expander]
    /// to allow the inner inspector to be hidden.
    ///
    /// A property editor definition for `InheritableVariable<T>` is also inserted.
    ///
    /// Panic if these types already have editor definitions.
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

    pub fn register_inheritable_styleable_inspectable<T>(&self)
    where
        T: Reflect + FieldValue,
    {
        assert!(self
            .insert(InspectablePropertyEditorDefinition::<T>::new())
            .is_none());
        assert!(self
            .insert(InheritablePropertyEditorDefinition::<T>::new())
            .is_none());
        assert!(self
            .insert(InheritablePropertyEditorDefinition::<StyledProperty<T>>::new())
            .is_none());
        assert!(self
            .insert(StyledPropertyEditorDefinition::<T>::new())
            .is_none());
    }

    /// Insert property editor definitions to allow enum T to be edited
    /// using a dropdown list, as well as `InheritableVariable<T>`.
    ///
    /// Panic if these types already have editor definitions.
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

    pub fn register_inheritable_styleable_enum<T, E: Debug>(&self)
    where
        T: InspectableEnum + FieldValue + VariantNames + AsRef<str> + FromStr<Err = E> + Debug,
    {
        assert!(self
            .insert(EnumPropertyEditorDefinition::<T>::new())
            .is_none());
        assert!(self
            .insert(InheritablePropertyEditorDefinition::<T>::new())
            .is_none());
        assert!(self
            .insert(InheritablePropertyEditorDefinition::<StyledProperty<T>>::new())
            .is_none());
        assert!(self
            .insert(StyledPropertyEditorDefinition::<T>::new())
            .is_none());
    }

    /// Insert property editor definitions to allow `Option<T>` to be edited
    /// as well as `InheritableVariable<T>`.
    ///
    /// Panic if these types already have editor definitions.
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

    /// Direct read-only access to all the editor definitions.
    pub fn definitions(
        &self,
    ) -> RwLockReadGuard<FxHashMap<TypeId, PropertyEditorDefinitionContainerEntry>> {
        self.definitions.read()
    }

    /// Direct and unrestricted access to all the editor definitions.
    pub fn definitions_mut(
        &self,
    ) -> RwLockWriteGuard<FxHashMap<TypeId, PropertyEditorDefinitionContainerEntry>> {
        self.definitions.write()
    }
}
