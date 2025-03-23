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

//! Inspector is a widget, that allows you to generate visual representation for internal fields an arbitrary
//! structure or enumeration recursively. It's primary usage is provide unified and simple way of introspection.
//! See [`Inspector`] docs for more info and usage examples.

use crate::{
    border::BorderBuilder,
    check_box::CheckBoxBuilder,
    core::{
        algebra::Vector2,
        pool::Handle,
        reflect::{prelude::*, CastError, Reflect},
        type_traits::prelude::*,
        uuid_provider,
        visitor::prelude::*,
    },
    define_constructor,
    expander::ExpanderBuilder,
    formatted_text::WrapMode,
    grid::{Column, GridBuilder, Row},
    inspector::editors::{
        PropertyEditorBuildContext, PropertyEditorDefinitionContainer, PropertyEditorInstance,
        PropertyEditorMessageContext, PropertyEditorTranslationContext,
    },
    menu::{ContextMenuBuilder, MenuItemBuilder, MenuItemContent, MenuItemMessage},
    message::{MessageDirection, UiMessage},
    popup::{PopupBuilder, PopupMessage},
    stack_panel::StackPanelBuilder,
    text::TextBuilder,
    utils::{make_arrow, make_simple_tooltip, ArrowDirection},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, RcUiNodeHandle, Thickness, UiNode, UserInterface, VerticalAlignment,
};
use copypasta::ClipboardProvider;

use fyrox_core::log::Log;
use fyrox_graph::{
    constructor::{ConstructorProvider, GraphNodeConstructor},
    BaseSceneGraph, SceneGraph,
};
use std::{
    any::{Any, TypeId},
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    sync::Arc,
};

pub mod editors;

/// Messages representing a change in a collection:
/// either adding an item, removing an item, or updating an existing item.
#[derive(Debug, Clone, PartialEq)]
pub enum CollectionChanged {
    /// An item should be added in the collection.
    Add(ObjectValue),
    /// An item in the collection should be removed.
    Remove(usize),
    /// An item in the collection has changed one of its properties.
    ItemChanged {
        /// Index of an item in the collection.
        index: usize,
        /// The change to the item.
        property: FieldKind,
    },
}

impl CollectionChanged {
    define_constructor!(CollectionChanged:Add => fn add(ObjectValue), layout: false);
    define_constructor!(CollectionChanged:Remove => fn remove(usize), layout: false);
    define_constructor!(CollectionChanged:ItemChanged => fn item_changed(index: usize, property: FieldKind), layout: false);
}

/// Changes that can happen to inheritable variables.
#[derive(Debug, Clone)]
pub enum InheritableAction {
    /// Revert the variable to the value that it originally inherited.
    Revert,
}

/// An enum of the ways in which a property might be changed by an editor.
#[derive(Debug, Clone)]
pub enum FieldKind {
    /// A collection has been changed in the given way.
    Collection(Box<CollectionChanged>),
    /// A property of a nested object has been changed in the given way.
    Inspectable(Box<PropertyChanged>),
    /// A new value is being assigned to the property.
    Object(ObjectValue),
    /// The state of an inheritable property is changing, such as being reverted
    /// to match the value in the original.
    Inheritable(InheritableAction),
}

/// An action for some property.
#[derive(Debug)]
pub enum PropertyAction {
    /// A property needs to be modified with given value.
    Modify {
        /// New value for a property.
        value: Box<dyn Reflect>,
    },
    /// An item needs to be added to a collection property.
    AddItem {
        /// New collection item.
        value: Box<dyn Reflect>,
    },
    /// An item needs to be removed from a collection property.
    RemoveItem {
        /// Index of an item.
        index: usize,
    },
    /// Revert value to parent.
    Revert,
}

impl PropertyAction {
    /// Creates action from a field definition. It is recursive action, it traverses the tree
    /// until there is either FieldKind::Object or FieldKind::Collection. FieldKind::Inspectable
    /// forces new iteration.
    pub fn from_field_kind(field_kind: &FieldKind) -> Self {
        match field_kind {
            FieldKind::Object(ref value) => Self::Modify {
                value: value.clone().into_box_reflect(),
            },
            FieldKind::Collection(ref collection_changed) => match **collection_changed {
                CollectionChanged::Add(ref value) => Self::AddItem {
                    value: value.clone().into_box_reflect(),
                },
                CollectionChanged::Remove(index) => Self::RemoveItem { index },
                CollectionChanged::ItemChanged { ref property, .. } => {
                    Self::from_field_kind(property)
                }
            },
            FieldKind::Inspectable(ref inspectable) => Self::from_field_kind(&inspectable.value),
            FieldKind::Inheritable { .. } => Self::Revert,
        }
    }

    /// Tries to apply the action to a given target.
    #[allow(clippy::type_complexity)]
    pub fn apply(
        self,
        path: &str,
        target: &mut dyn Reflect,
        result_callback: &mut dyn FnMut(Result<Option<Box<dyn Reflect>>, Self>),
    ) {
        match self {
            PropertyAction::Modify { value } => {
                let mut value = Some(value);
                target.resolve_path_mut(path, &mut |result| {
                    if let Ok(field) = result {
                        if let Err(value) = field.set(value.take().unwrap()) {
                            result_callback(Err(Self::Modify { value }))
                        } else {
                            result_callback(Ok(None))
                        }
                    } else {
                        result_callback(Err(Self::Modify {
                            value: value.take().unwrap(),
                        }))
                    }
                });
            }
            PropertyAction::AddItem { value } => {
                let mut value = Some(value);
                target.resolve_path_mut(path, &mut |result| {
                    if let Ok(field) = result {
                        field.as_list_mut(&mut |result| {
                            if let Some(list) = result {
                                if let Err(value) = list.reflect_push(value.take().unwrap()) {
                                    result_callback(Err(Self::AddItem { value }))
                                } else {
                                    result_callback(Ok(None))
                                }
                            } else {
                                result_callback(Err(Self::AddItem {
                                    value: value.take().unwrap(),
                                }))
                            }
                        })
                    } else {
                        result_callback(Err(Self::AddItem {
                            value: value.take().unwrap(),
                        }))
                    }
                })
            }
            PropertyAction::RemoveItem { index } => target.resolve_path_mut(path, &mut |result| {
                if let Ok(field) = result {
                    field.as_list_mut(&mut |result| {
                        if let Some(list) = result {
                            if let Some(value) = list.reflect_remove(index) {
                                result_callback(Ok(Some(value)))
                            } else {
                                result_callback(Err(Self::RemoveItem { index }))
                            }
                        } else {
                            result_callback(Err(Self::RemoveItem { index }))
                        }
                    })
                } else {
                    result_callback(Err(Self::RemoveItem { index }))
                }
            }),
            PropertyAction::Revert => {
                // Unsupported due to lack of context (a reference to parent entity).
                result_callback(Err(Self::Revert))
            }
        }
    }
}

/// Trait of values that can be edited by an Inspector through reflection.
pub trait Value: Reflect + Send {
    fn clone_box(&self) -> Box<dyn Value>;

    fn into_box_reflect(self: Box<Self>) -> Box<dyn Reflect>;
}

impl<T> Value for T
where
    T: Reflect + Clone + Debug + Send,
{
    fn clone_box(&self) -> Box<dyn Value> {
        Box::new(self.clone())
    }

    fn into_box_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        Box::new(*self.into_any().downcast::<T>().unwrap())
    }
}

/// An untyped value that is created by an editor and sent in a message
/// to inform the inspected object that one of its properties should change.
#[derive(Debug)]
pub struct ObjectValue {
    pub value: Box<dyn Value>,
}

impl Clone for ObjectValue {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone_box(),
        }
    }
}

impl PartialEq for ObjectValue {
    fn eq(&self, other: &Self) -> bool {
        // Cast fat pointers to thin first.
        let ptr_a = &*self.value as *const _ as *const ();
        let ptr_b = &*other.value as *const _ as *const ();
        // Compare thin pointers.
        std::ptr::eq(ptr_a, ptr_b)
    }
}

impl ObjectValue {
    pub fn cast_value<T: 'static>(&self, func: &mut dyn FnMut(Option<&T>)) {
        (*self.value).as_any(&mut |any| func(any.downcast_ref::<T>()))
    }

    pub fn cast_clone<T: Clone + 'static>(&self, func: &mut dyn FnMut(Option<T>)) {
        (*self.value).as_any(&mut |any| func(any.downcast_ref::<T>().cloned()))
    }

    pub fn try_override<T: Clone + 'static>(&self, value: &mut T) -> bool {
        let mut result = false;
        (*self.value).as_any(&mut |any| {
            if let Some(self_value) = any.downcast_ref::<T>() {
                *value = self_value.clone();
                result = true;
            }
        });
        false
    }

    pub fn into_box_reflect(self) -> Box<dyn Reflect> {
        self.value.into_box_reflect()
    }
}

impl PartialEq for FieldKind {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (FieldKind::Collection(l), FieldKind::Collection(r)) => std::ptr::eq(&**l, &**r),
            (FieldKind::Inspectable(l), FieldKind::Inspectable(r)) => std::ptr::eq(&**l, &**r),
            (FieldKind::Object(l), FieldKind::Object(r)) => l == r,
            _ => false,
        }
    }
}

impl FieldKind {
    pub fn object<T: Value>(value: T) -> Self {
        Self::Object(ObjectValue {
            value: Box::new(value),
        })
    }
}

/// The details of a change to some field of some object due to being edited in an inspector.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyChanged {
    /// The name of the edited property.
    pub name: String,
    /// The details of the change.
    pub value: FieldKind,
}

impl PropertyChanged {
    pub fn path(&self) -> String {
        let mut path = self.name.clone();
        match self.value {
            FieldKind::Collection(ref collection_changed) => {
                if let CollectionChanged::ItemChanged {
                    ref property,
                    index,
                } = **collection_changed
                {
                    match property {
                        FieldKind::Inspectable(inspectable) => {
                            path += format!("[{}].{}", index, inspectable.path()).as_ref();
                        }
                        _ => path += format!("[{index}]").as_ref(),
                    }
                }
            }
            FieldKind::Inspectable(ref inspectable) => {
                path += format!(".{}", inspectable.path()).as_ref();
            }
            FieldKind::Object(_) | FieldKind::Inheritable { .. } => {}
        }
        path
    }

    pub fn is_inheritable(&self) -> bool {
        match self.value {
            FieldKind::Collection(ref collection_changed) => match **collection_changed {
                CollectionChanged::Add(_) => false,
                CollectionChanged::Remove(_) => false,
                CollectionChanged::ItemChanged { ref property, .. } => match property {
                    FieldKind::Inspectable(inspectable) => inspectable.is_inheritable(),
                    FieldKind::Inheritable(_) => true,
                    _ => false,
                },
            },
            FieldKind::Inspectable(ref inspectable) => inspectable.is_inheritable(),
            FieldKind::Object(_) => false,
            FieldKind::Inheritable(_) => true,
        }
    }
}

/// Messages to and from the inspector to keep the inspector and the inspected object in sync.
#[derive(Debug, Clone, PartialEq)]
pub enum InspectorMessage {
    /// Message sent to the inspector to replace the context of the inspector so it can inspect a new object.
    Context(InspectorContext),
    /// Message sent from the inspector to notify the world that the object has been edited according to the
    /// given PropertyChanged struct.
    PropertyChanged(PropertyChanged),
}

impl InspectorMessage {
    define_constructor!(InspectorMessage:Context => fn context(InspectorContext), layout: false);
    define_constructor!(InspectorMessage:PropertyChanged => fn property_changed(PropertyChanged), layout: false);
}

/// This trait allows dynamically typed context information to be
/// passed to an [Inspector] widget.
/// Since an Inspector might be used in applications other than Fyroxed,
/// Inspector does not assume that InspectorEnvironment must always be
/// [fyroxed_base::inspector::EditorEnvironment](https://docs.rs/fyroxed_base/latest/fyroxed_base/inspector/struct.EditorEnvironment.html).
/// Instead, when a property editor needs to talk to the application using the Inspector,
/// it can attempt to cast InspectorEnvironment to whatever type it might be.
pub trait InspectorEnvironment: Any + Send + Sync {
    fn name(&self) -> String;
    fn as_any(&self) -> &dyn Any;
}

/// Inspector is a widget, that allows you to generate visual representation for internal fields an arbitrary
/// structure or enumeration recursively. It's primary usage is provide unified and simple way of introspection.
///
/// ## Example
///
/// An instance of inspector widget could be created like so:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::{pool::Handle, reflect::prelude::*},
/// #     inspector::{
/// #         editors::{
/// #             enumeration::EnumPropertyEditorDefinition,
/// #             inspectable::InspectablePropertyEditorDefinition,
/// #             PropertyEditorDefinitionContainer,
/// #         },
/// #         InspectorBuilder, InspectorContext,
/// #     },
/// #     widget::WidgetBuilder,
/// #     BuildContext, UiNode,
/// # };
/// # use std::sync::Arc;
/// # use strum_macros::{AsRefStr, EnumString, VariantNames};
/// # use fyrox_core::uuid_provider;
///
/// #[derive(Reflect, Debug, Clone)]
/// struct MyObject {
///     foo: String,
///     bar: u32,
///     stuff: MyEnum,
/// }
///
/// uuid_provider!(MyObject = "391b9424-8fe2-4525-a98e-3c930487fcf1");
///
/// // Enumeration requires a bit more traits to be implemented. It must provide a way to turn
/// // enum into a string.
/// #[derive(Reflect, Debug, Clone, AsRefStr, EnumString, VariantNames)]
/// enum MyEnum {
///     SomeVariant,
///     YetAnotherVariant { baz: f32 },
/// }
///
/// uuid_provider!(MyEnum = "a93ec1b5-e7c8-4919-ac19-687d8c99f6bd");
///
/// fn create_inspector(ctx: &mut BuildContext) -> Handle<UiNode> {
///     // Initialize an object first.
///     let my_object = MyObject {
///         foo: "Some string".to_string(),
///         bar: 42,
///         stuff: MyEnum::YetAnotherVariant { baz: 123.321 },
///     };
///
///     // Create a new property editors definition container.
///     let definition_container = PropertyEditorDefinitionContainer::with_default_editors();
///
///     // Add property editors for our structure and enumeration, so the inspector could use these
///     // property editors to generate visual representation for them.
///     definition_container.insert(InspectablePropertyEditorDefinition::<MyObject>::new());
///     definition_container.insert(EnumPropertyEditorDefinition::<MyEnum>::new());
///
///     // Generate a new inspector context - its visual representation, that will be used
///     // by the inspector.
///     let context = InspectorContext::from_object(
///         &my_object,
///         ctx,
///         Arc::new(definition_container),
///         None,
///         1,
///         0,
///         true,
///         Default::default(),
///         150.0
///     );
///
///     InspectorBuilder::new(WidgetBuilder::new())
///         .with_context(context)
///         .build(ctx)
/// }
/// ```
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct Inspector {
    pub widget: Widget,
    #[reflect(hidden)]
    #[visit(skip)]
    pub context: InspectorContext,
}

impl ConstructorProvider<UiNode, UserInterface> for Inspector {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>().with_variant("Inspector", |ui| {
            InspectorBuilder::new(WidgetBuilder::new().with_name("Inspector"))
                .build(&mut ui.build_ctx())
                .into()
        })
    }
}

crate::define_widget_deref!(Inspector);

impl Inspector {
    pub fn context(&self) -> &InspectorContext {
        &self.context
    }
}

/// Default margines for editor containers.
pub const HEADER_MARGIN: Thickness = Thickness {
    left: 2.0,
    top: 1.0,
    right: 4.0,
    bottom: 1.0,
};

/// An error that may be produced by an Inspector.
#[derive(Debug)]
pub enum InspectorError {
    /// An error occurred due to reflection when some value did not have its expected type.
    CastError(CastError),
    /// The object type has changed and the inspector context is no longer valid.
    OutOfSync,
    /// An error message produced by some editor with specialized details unique to that editor.
    /// For example, an array editor might complain if there is no editor definition for the type
    /// of its elements.
    Custom(String),
    /// As an inspector contains multiple editors, it can potentially produce multiple errors.
    Group(Vec<InspectorError>),
}

impl From<CastError> for InspectorError {
    fn from(e: CastError) -> Self {
        Self::CastError(e)
    }
}

/// Stores the association between a field in an object and an editor widget in an [Inspector].
#[derive(Clone, Debug)]
pub struct ContextEntry {
    /// The name of the field being edited, as found in [FieldMetadata::name].
    pub property_name: String,
    /// The name of the field being edited, as found in [FieldMetadata::display_name].
    pub property_display_name: String,
    /// The name of the field being edited, as found in [FieldMetadata::tag].
    pub property_tag: String,
    /// The type of the property being edited, as found in [PropertyEditorDefinition::value_type_id](editors::PropertyEditorDefinition::value_type_id).
    pub property_value_type_id: TypeId,
    /// The list of property editor definitions being used by the inspector.
    pub property_editor_definition_container: Arc<PropertyEditorDefinitionContainer>,
    /// The handle of the widget that is editing the property.
    pub property_editor: Handle<UiNode>,
    /// The result of `format!("{:?}", field)`, if generated. Otherwise, this string is empty.
    /// Generating these strings is controlled by the `generate_property_string_values` parameter in [InspectorContext::from_object].
    pub property_debug_output: String,
    /// The widget that contains the editor widget. It provides a label to identify which property is being edited.
    /// Storing the handle here allows us to which editor the user is indicating if the mouse is over the area
    /// surrounding the editor instead of the editor itself.
    pub property_container: Handle<UiNode>,
}

impl PartialEq for ContextEntry {
    fn eq(&self, other: &Self) -> bool {
        // Cast fat pointers to thin first.
        let ptr_a = &*self.property_editor_definition_container as *const _ as *const ();
        let ptr_b = &*other.property_editor_definition_container as *const _ as *const ();

        self.property_editor == other.property_editor
            && self.property_name == other.property_name
            && self.property_value_type_id ==other.property_value_type_id
            // Compare thin pointers.
            && std::ptr::eq(ptr_a, ptr_b)
    }
}

/// The handles of a context menu when right-clicking on an [Inspector].
#[derive(Default, Clone)]
pub struct Menu {
    /// The handle of the "Copy Value as String" menu item.
    pub copy_value_as_string: Handle<UiNode>,
    /// The reference-counted handle of the menu as a whole.
    pub menu: Option<RcUiNodeHandle>,
}

/// The widget handle and associated information that represents what an [Inspector] is currently displaying.
#[derive(Clone)]
pub struct InspectorContext {
    /// The handle of a UI node containing property editor widgets.
    /// This would usually be a vertical Stack widget, but any widget will sever the same purpose
    /// so long as it produces messages that are recognized by the
    /// [PropertyEditorDefinitions](crate::inspector::editors::PropertyEditorDefinition)
    /// contained in [InspectorContext::property_definitions].
    ///
    /// To ensure this, the widget should be composed of widgets produced by
    /// [PropertyEditorDefinition::create_instance](crate::inspector::editors::PropertyEditorDefinition::create_instance).
    pub stack_panel: Handle<UiNode>,
    /// The context menu that opens when right-clicking on the inspector.
    pub menu: Menu,
    /// List of the editors in this inspector, in order, with each entry giving the editor widget handle, the name of the field being edited,
    /// and so on.
    pub entries: Vec<ContextEntry>,
    /// List if property definitions that are by [sync](InspectorContext::sync) to update the widgets of [stack_panel](InspectorContext::stack_panel),
    /// with the current values of properties that may have changed.
    pub property_definitions: Arc<PropertyEditorDefinitionContainer>,
    /// Untyped information from the application that is using the inspector. This can be used by editors that may be
    /// supplied by that application, if those editors know the actual type of this value to be able to successfully cast it.
    pub environment: Option<Arc<dyn InspectorEnvironment>>,
    /// The value given to [UiMessage::flags] when [sync](InspectorContext::sync) is generating update messages for
    /// editor widgets. This identifies sync messages and prevents the Inspector from trying to translate them,
    /// and thereby it prevents sync messages from potentially creating an infinite message loop.
    pub sync_flag: u64,
    /// Type id of the object for which the context was created.
    pub object_type_id: TypeId,
    /// A width of the property name column.
    pub name_column_width: f32,
}

impl PartialEq for InspectorContext {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

fn object_type_id(object: &dyn Reflect) -> TypeId {
    let mut object_type_id = None;
    object.as_any(&mut |any| object_type_id = Some(any.type_id()));
    object_type_id.unwrap()
}

impl Default for InspectorContext {
    fn default() -> Self {
        Self {
            stack_panel: Default::default(),
            menu: Default::default(),
            entries: Default::default(),
            property_definitions: Arc::new(
                PropertyEditorDefinitionContainer::with_default_editors(),
            ),
            environment: None,
            sync_flag: 0,
            object_type_id: ().type_id(),
            name_column_width: 150.0,
        }
    }
}

impl Debug for InspectorContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "InspectorContext")
    }
}

/// Convert a layer_index into a margin thickness.
/// An editor's layer_index indicates how deeply nested it is within other editors.
/// For example, an array editor will contain nested editors for each element of the array,
/// and those nested editors will have the array editors index_layer + 1.
/// Deeper layer_index values correspond to a thicker left margin.
pub fn make_property_margin(layer_index: usize) -> Thickness {
    let mut margin = HEADER_MARGIN;
    margin.left += 10.0 + layer_index as f32 * 10.0;
    margin
}

fn make_expander_margin(layer_index: usize) -> Thickness {
    let mut margin = HEADER_MARGIN;
    margin.left += layer_index as f32 * 10.0;
    margin
}

fn make_expander_check_box(
    layer_index: usize,
    property_name: &str,
    property_description: &str,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    let description = if property_description.is_empty() {
        property_name.to_string()
    } else {
        format!("{property_name}\n\n{property_description}")
    };

    let handle = CheckBoxBuilder::new(
        WidgetBuilder::new()
            .with_vertical_alignment(VerticalAlignment::Center)
            .with_margin(make_expander_margin(layer_index)),
    )
    .with_background(
        BorderBuilder::new(
            WidgetBuilder::new()
                .with_vertical_alignment(VerticalAlignment::Center)
                .with_min_size(Vector2::new(4.0, 4.0)),
        )
        .with_stroke_thickness(Thickness::zero().into())
        .build(ctx),
    )
    .with_content(
        TextBuilder::new(
            WidgetBuilder::new()
                .with_opt_tooltip(make_tooltip(ctx, &description))
                .with_height(16.0)
                .with_margin(Thickness::left(2.0)),
        )
        .with_vertical_text_alignment(VerticalAlignment::Center)
        .with_text(property_name)
        .build(ctx),
    )
    .checked(Some(true))
    .with_check_mark(make_arrow(ctx, ArrowDirection::Bottom, 8.0))
    .with_uncheck_mark(make_arrow(ctx, ArrowDirection::Right, 8.0))
    .build(ctx);

    // Explicitly state that this expander should **not** be included in the tab navigation.
    ctx[handle].accepts_input = false;

    handle
}

/// Build an [Expander](crate::expander::Expander) widget to contain an editor.
/// * layer_index: How deeply nested is the editor? This controls the width of the left margine.
/// * property_name: The name to use as the label for the expander.
/// * description: The tooltip for the editor.
/// * header: See [Expander](crate::expander::Expander) docs for an explanation of expander headers.
/// * content: The editor widget to be shown or hidden.
/// * ctx: The [BuildContext] to make it possible to create the widget.
pub fn make_expander_container(
    layer_index: usize,
    property_name: &str,
    description: &str,
    header: Handle<UiNode>,
    content: Handle<UiNode>,
    width: f32,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    ExpanderBuilder::new(WidgetBuilder::new())
        .with_checkbox(make_expander_check_box(
            layer_index,
            property_name,
            description,
            ctx,
        ))
        .with_expander_column(Column::strict(width))
        .with_expanded(true)
        .with_header(header)
        .with_content(content)
        .build(ctx)
}

fn create_header(ctx: &mut BuildContext, text: &str, layer_index: usize) -> Handle<UiNode> {
    TextBuilder::new(WidgetBuilder::new().with_margin(make_property_margin(layer_index)))
        .with_text(text)
        .with_vertical_text_alignment(VerticalAlignment::Center)
        .build(ctx)
}

fn make_tooltip(ctx: &mut BuildContext, text: &str) -> Option<RcUiNodeHandle> {
    if text.is_empty() {
        None
    } else {
        Some(make_simple_tooltip(ctx, text))
    }
}

fn make_simple_property_container(
    title: Handle<UiNode>,
    editor: Handle<UiNode>,
    description: &str,
    width: f32,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    ctx[editor].set_row(0).set_column(1);

    let tooltip = make_tooltip(ctx, description);
    ctx[title].set_tooltip(tooltip);

    GridBuilder::new(WidgetBuilder::new().with_child(title).with_child(editor))
        .add_row(Row::auto())
        .add_columns(vec![Column::strict(width), Column::stretch()])
        .build(ctx)
}

/// Filter function for determining which fields of an object should be included in an Inspector.
/// Return true to include a field. If None, then all fields are included.
#[derive(Default, Clone)]
pub struct PropertyFilter(pub Option<Arc<dyn Fn(&dyn Reflect) -> bool + Send + Sync>>);

impl PropertyFilter {
    pub fn new<T>(func: T) -> Self
    where
        T: Fn(&dyn Reflect) -> bool + 'static + Send + Sync,
    {
        Self(Some(Arc::new(func)))
    }

    pub fn pass(&self, value: &dyn Reflect) -> bool {
        match self.0.as_ref() {
            None => true,
            Some(filter) => (filter)(value),
        }
    }
}

fn assign_tab_indices(container: Handle<UiNode>, ui: &mut UserInterface) {
    let mut counter = 0;
    let mut widgets_list = Vec::new();
    for (descendant_handle, descendant_ref) in ui.traverse_iter(container) {
        if descendant_ref.accepts_input {
            widgets_list.push((descendant_handle, counter));
            counter += 1;
        }
    }

    for (descendant, tab_index) in widgets_list {
        ui.node_mut(descendant)
            .tab_index
            .set_value_and_mark_modified(Some(counter - tab_index));
    }
}

impl InspectorContext {
    /// Build the widgets for an Inspector to represent the given object by accessing
    /// the object's fields through reflection.
    /// * object: The object to inspect.
    /// * ctx: The general context for widget creation.
    /// * definition_container: The list of property editor definitions that will create the editors
    /// based on the type of each field.
    /// * environment: Untyped optional generic information about the application using the inspector,
    /// which may be useful to some editors. Often this will be Fyroxed's EditorEnvironment.
    /// * sync_flag: Flag bits to identify messages that are sent by the `sync` method to
    /// update an editor with the current value of the property that it is editing.
    /// This becomes the value of [UiMessage::flags].
    /// * layer_index: Inspectors can be nested within the editors of other inspectors.
    /// The layer_index is the count of how deeply nested this inspector will be.
    /// * generate_property_string_values: Should we use `format!("{:?}", field)` to construct string representations
    /// for each property?
    /// * filter: A filter function that controls whether each field will be included in the inspector.
    pub fn from_object(
        object: &dyn Reflect,
        ctx: &mut BuildContext,
        definition_container: Arc<PropertyEditorDefinitionContainer>,
        environment: Option<Arc<dyn InspectorEnvironment>>,
        sync_flag: u64,
        layer_index: usize,
        generate_property_string_values: bool,
        filter: PropertyFilter,
        name_column_width: f32,
    ) -> Self {
        let mut entries = Vec::new();

        let mut fields_text = Vec::new();
        object.fields_ref(&mut |fields| {
            for field in fields {
                fields_text.push(if generate_property_string_values {
                    format!("{:?}", field.value.field_value_as_reflect())
                } else {
                    Default::default()
                })
            }
        });

        let mut editors = Vec::new();
        object.fields_ref(&mut |fields_ref| {
            for (i, (field_text, info)) in fields_text.iter().zip(fields_ref.iter()).enumerate() {
                if !filter.pass(info.value.field_value_as_reflect()) {
                    continue;
                }

                let description = if info.description.is_empty() {
                    info.display_name.to_string()
                } else {
                    format!("{}\n\n{}", info.display_name, info.description)
                };

                if let Some(definition) = definition_container
                    .definitions()
                    .get(&info.value.type_id())
                {
                    let editor = match definition.property_editor.create_instance(
                        PropertyEditorBuildContext {
                            build_context: ctx,
                            property_info: info,
                            environment: environment.clone(),
                            definition_container: definition_container.clone(),
                            sync_flag,
                            layer_index,
                            generate_property_string_values,
                            filter: filter.clone(),
                            name_column_width,
                        },
                    ) {
                        Ok(instance) => {
                            let (container, editor) = match instance {
                                PropertyEditorInstance::Simple { editor } => (
                                    make_simple_property_container(
                                        create_header(ctx, info.display_name, layer_index),
                                        editor,
                                        &description,
                                        name_column_width,
                                        ctx,
                                    ),
                                    editor,
                                ),
                                PropertyEditorInstance::Custom { container, editor } => {
                                    (container, editor)
                                }
                            };

                            entries.push(ContextEntry {
                                property_editor: editor,
                                property_value_type_id: definition.property_editor.value_type_id(),
                                property_editor_definition_container: definition_container.clone(),
                                property_name: info.name.to_string(),
                                property_display_name: info.display_name.to_string(),
                                property_tag: info.tag.to_string(),
                                property_debug_output: field_text.clone(),
                                property_container: container,
                            });

                            if info.read_only {
                                ctx[editor].set_enabled(false);
                            }

                            container
                        }
                        Err(e) => {
                            Log::err(format!(
                                "Unable to create property editor instance: Reason {e:?}"
                            ));
                            make_simple_property_container(
                                create_header(ctx, info.display_name, layer_index),
                                TextBuilder::new(WidgetBuilder::new().on_row(i).on_column(1))
                                    .with_wrap(WrapMode::Word)
                                    .with_vertical_text_alignment(VerticalAlignment::Center)
                                    .with_text(format!(
                                        "Unable to create property \
                                                    editor instance: Reason {e:?}"
                                    ))
                                    .build(ctx),
                                &description,
                                name_column_width,
                                ctx,
                            )
                        }
                    };

                    editors.push(editor);
                } else {
                    editors.push(make_simple_property_container(
                        create_header(ctx, info.display_name, layer_index),
                        TextBuilder::new(WidgetBuilder::new().on_row(i).on_column(1))
                            .with_wrap(WrapMode::Word)
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .with_text(format!(
                                "Property Editor Is Missing For Type {}!",
                                info.value.type_name()
                            ))
                            .build(ctx),
                        &description,
                        name_column_width,
                        ctx,
                    ));
                }
            }
        });

        let copy_value_as_string;
        let menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new().with_visibility(false)).with_content(
                StackPanelBuilder::new(WidgetBuilder::new().with_child({
                    copy_value_as_string = MenuItemBuilder::new(WidgetBuilder::new())
                        .with_content(MenuItemContent::text("Copy Value as String"))
                        .build(ctx);
                    copy_value_as_string
                }))
                .build(ctx),
            ),
        )
        .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        let stack_panel = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_context_menu(menu.clone())
                .with_children(editors),
        )
        .build(ctx);

        // Assign tab indices for every widget that can accept user input.
        if layer_index == 0 {
            assign_tab_indices(stack_panel, ctx.inner_mut());
        }

        Self {
            stack_panel,
            menu: Menu {
                copy_value_as_string,
                menu: Some(menu),
            },
            entries,
            property_definitions: definition_container,
            sync_flag,
            environment,
            object_type_id: object_type_id(object),
            name_column_width,
        }
    }

    /// Update the widgest to reflect the value of the given object.
    /// We will iterate through the fields and find the appropriate [PropertyEditorDefinition](editors::PropertyEditorDefinition)
    /// for each field. We call [create_message](editors::PropertyEditorDefinition::create_message) to get each property editor
    /// definition to generate the appropriate message to get the editor widget to update itself, and we set the [flags](UiMessage::flags)
    /// of each message to [InspectorContext::sync_flag] before sending the message.
    /// * object: The object to take the property values from.
    /// * ui: The UserInterface to include in the [PropertyEditorMessageContext].
    /// * layer_index: The depth of the nesting of this inspector.
    /// * generator_property_string_values: if any editors within this inspector contain inner inspectors, should those inspectors
    /// generate strings for their properties?
    /// * filter: filter function for the fields of `object` and for any inspectors within the editors of this inspector.
    pub fn sync(
        &self,
        object: &dyn Reflect,
        ui: &mut UserInterface,
        layer_index: usize,
        generate_property_string_values: bool,
        filter: PropertyFilter,
    ) -> Result<(), Vec<InspectorError>> {
        if object_type_id(object) != self.object_type_id {
            return Err(vec![InspectorError::OutOfSync]);
        }

        let mut sync_errors = Vec::new();

        object.fields_ref(&mut |fields_ref| {
            for info in fields_ref {
                if !filter.pass(info.value.field_value_as_reflect()) {
                    continue;
                }

                if let Some(constructor) = self
                    .property_definitions
                    .definitions()
                    .get(&info.value.type_id())
                {
                    if let Some(property_editor) = self.find_property_editor(info.name) {
                        let ctx = PropertyEditorMessageContext {
                            sync_flag: self.sync_flag,
                            instance: property_editor.property_editor,
                            ui,
                            property_info: info,
                            definition_container: self.property_definitions.clone(),
                            layer_index,
                            environment: self.environment.clone(),
                            generate_property_string_values,
                            filter: filter.clone(),
                            name_column_width: self.name_column_width,
                        };

                        match constructor.property_editor.create_message(ctx) {
                            Ok(message) => {
                                if let Some(mut message) = message {
                                    message.flags = self.sync_flag;
                                    ui.send_message(message);
                                }
                            }
                            Err(e) => sync_errors.push(e),
                        }
                    } else {
                        sync_errors.push(InspectorError::OutOfSync);
                    }
                }
            }
        });

        if layer_index == 0 {
            // The stack panel could not exist, if the inspector context was invalidated. This
            // happens when the context is discarded by the inspector widget.
            if ui.is_valid_handle(self.stack_panel) {
                assign_tab_indices(self.stack_panel, ui);
            }
        }

        if sync_errors.is_empty() {
            Ok(())
        } else {
            Err(sync_errors)
        }
    }

    /// Iterates through every property.
    pub fn property_editors(&self) -> impl Iterator<Item = &ContextEntry> + '_ {
        self.entries.iter()
    }

    /// Return the entry for the property with the given name.
    pub fn find_property_editor(&self, name: &str) -> Option<&ContextEntry> {
        self.entries.iter().find(|e| e.property_name == name)
    }

    /// Return the entry for the property with the given tag.
    pub fn find_property_editor_by_tag(&self, tag: &str) -> Option<&ContextEntry> {
        self.entries.iter().find(|e| e.property_tag == tag)
    }

    /// Shortcut for getting the editor widget from the property with the given name.
    /// Returns `Handle::NONE` if there is no property with that name.
    pub fn find_property_editor_widget(&self, name: &str) -> Handle<UiNode> {
        self.find_property_editor(name)
            .map(|e| e.property_editor)
            .unwrap_or_default()
    }
}

uuid_provider!(Inspector = "c599c0f5-f749-4033-afed-1a9949c937a1");

impl Control for Inspector {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle && message.direction() == MessageDirection::ToWidget
        {
            if let Some(InspectorMessage::Context(ctx)) = message.data::<InspectorMessage>() {
                // Remove previous content.
                for child in self.children() {
                    ui.send_message(WidgetMessage::remove(*child, MessageDirection::ToWidget));
                }

                // Link new panel.
                ui.send_message(WidgetMessage::link(
                    ctx.stack_panel,
                    MessageDirection::ToWidget,
                    self.handle,
                ));

                self.context = ctx.clone();
            }
        }

        if let Some(PopupMessage::RelayedMessage(popup_message)) = message.data() {
            if popup_message.destination() == self.context.menu.copy_value_as_string {
                if let Some(MenuItemMessage::Click) = popup_message.data() {
                    // The child that was originally clicked to open the menu was automatically set to be
                    // the owner of the popup, and so event messages have it as the destination.
                    let mut parent_handle = message.destination();

                    // Crawl up from the destination to find the actual editor and do the copy.
                    while let Some(parent) = ui.try_get(parent_handle) {
                        for entry in self.context.entries.iter() {
                            if entry.property_container == parent_handle {
                                let _ = ui
                                    .clipboard_mut()
                                    .unwrap()
                                    .set_contents(entry.property_debug_output.clone());
                                break;
                            }
                        }

                        parent_handle = parent.parent;
                    }
                }
            }
        }

        // Check each message from descendant widget and try to translate it to
        // PropertyChanged message.
        if message.flags != self.context.sync_flag {
            let env = self.context.environment.clone();
            for entry in self.context.entries.iter() {
                if message.destination() == entry.property_editor {
                    if let Some(args) = entry
                        .property_editor_definition_container
                        .definitions()
                        .get(&entry.property_value_type_id)
                        .and_then(|e| {
                            e.property_editor
                                .translate_message(PropertyEditorTranslationContext {
                                    environment: env.clone(),
                                    name: &entry.property_name,
                                    message,
                                    definition_container: self.context.property_definitions.clone(),
                                })
                        })
                    {
                        ui.send_message(InspectorMessage::property_changed(
                            self.handle,
                            MessageDirection::FromWidget,
                            args,
                        ));
                    }
                }
            }
        }
    }
}

/// Build an Inspector from a [WidgetBuilder] and an [InspectorContext].
pub struct InspectorBuilder {
    widget_builder: WidgetBuilder,
    context: InspectorContext,
}

impl InspectorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            context: Default::default(),
        }
    }

    /// Sets the context for the created [Inspector].
    pub fn with_context(mut self, context: InspectorContext) -> Self {
        self.context = context;
        self
    }

    /// If given an inspector context, sets the context for the created inspector.
    /// If given None, does nothing.
    pub fn with_opt_context(mut self, context: Option<InspectorContext>) -> Self {
        if let Some(context) = context {
            self.context = context;
        }
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let canvas = Inspector {
            widget: self
                .widget_builder
                .with_child(self.context.stack_panel)
                .build(ctx),
            context: self.context,
        };
        ctx.add_node(UiNode::new(canvas))
    }
}

#[cfg(test)]
mod test {
    use crate::inspector::InspectorBuilder;
    use crate::{test::test_widget_deletion, widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| InspectorBuilder::new(WidgetBuilder::new()).build(ctx));
    }
}
