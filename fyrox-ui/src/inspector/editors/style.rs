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

//! Property editor for [`StyledProperty`]. It acts like a proxy to inner property, but also
//! adds a special "bind" button used to change style binding of the property.

use crate::message::MessageData;
use crate::{
    button::{ButtonBuilder, ButtonMessage},
    core::{
        pool::Handle, reflect::prelude::*, reflect::FieldValue, type_traits::prelude::*,
        visitor::prelude::*, ImmutableString, PhantomDataSendSync,
    },
    define_constructor, define_widget_deref,
    draw::DrawingContext,
    grid::{Column, GridBuilder, Row},
    image::ImageBuilder,
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        FieldKind, InspectorError, PropertyChanged,
    },
    list_view::{ListViewBuilder, ListViewMessage},
    message::{OsEvent, UiMessage},
    resources::BIND_ICON,
    stack_panel::StackPanelBuilder,
    style::{
        resource::{StyleResource, StyleResourceExt},
        Style, StyledProperty,
    },
    utils::{make_dropdown_list_option, make_simple_tooltip},
    widget::WidgetBuilder,
    window::{Window, WindowBuilder, WindowMessage, WindowTitle},
    BuildContext, Control, HorizontalAlignment, MessageDirection, Orientation, Thickness, UiNode,
    UserInterface, VerticalAlignment, Widget,
};
use fyrox_core::algebra::{Matrix3, Vector2};
use fyrox_graph::BaseSceneGraph;
use std::{
    any::{Any, TypeId},
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

#[derive(Debug, Clone, PartialEq)]
pub enum StyledPropertySelectorMessage {
    PropertyName(ImmutableString),
}
impl MessageData for StyledPropertySelectorMessage {}

impl StyledPropertySelectorMessage {
    define_constructor!(StyledPropertySelectorMessage:PropertyName => fn property_name(ImmutableString));
}

#[derive(Debug, Clone, PartialEq)]
pub enum StyledPropertyEditorMessage {
    BindProperty(ImmutableString),
}
impl MessageData for StyledPropertyEditorMessage {}

impl StyledPropertyEditorMessage {
    define_constructor!(StyledPropertyEditorMessage:BindProperty => fn bind_property(ImmutableString));
}

#[derive(Debug, Clone, Visit, Reflect, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "3a863a0f-7414-44f5-a7aa-7a6668a6d406")]
#[reflect(derived_type = "UiNode")]
pub struct StyledPropertySelector {
    window: Window,
    properties: Handle<UiNode>,
    property_list: Vec<ImmutableString>,
    ok: Handle<UiNode>,
    cancel: Handle<UiNode>,
    style_property_name: ImmutableString,
}

impl Deref for StyledPropertySelector {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.window.widget
    }
}

impl DerefMut for StyledPropertySelector {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.window.widget
    }
}

impl Control for StyledPropertySelector {
    fn on_remove(&self, sender: &Sender<UiMessage>) {
        self.window.on_remove(sender)
    }

    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        self.window.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        self.window.arrange_override(ui, final_size)
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        self.window.draw(drawing_context)
    }

    fn on_visual_transform_changed(
        &self,
        old_transform: &Matrix3<f32>,
        new_transform: &Matrix3<f32>,
    ) {
        self.window
            .on_visual_transform_changed(old_transform, new_transform)
    }

    fn post_draw(&self, drawing_context: &mut DrawingContext) {
        self.window.post_draw(drawing_context)
    }

    fn update(&mut self, dt: f32, ui: &mut UserInterface) {
        self.window.update(dt, ui)
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.window.handle_routed_message(ui, message);

        if let Some(ListViewMessage::Selection(selected)) = message.data() {
            if let Some(selected_index) = selected.first() {
                if message.destination() == self.properties {
                    self.style_property_name = self.property_list[*selected_index].clone();
                }
            }
        } else if let Some(ButtonMessage::Click) = message.data_from(self.ok) {
            ui.post(
                self.handle,
                StyledPropertySelectorMessage::PropertyName(self.style_property_name.clone()),
            );
            ui.send(self.handle, WindowMessage::Close);
        } else if let Some(ButtonMessage::Click) = message.data_from(self.cancel) {
            ui.send(self.handle, WindowMessage::Close);
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        self.window.preview_message(ui, message)
    }

    fn handle_os_event(
        &mut self,
        self_handle: Handle<UiNode>,
        ui: &mut UserInterface,
        event: &OsEvent,
    ) {
        self.window.handle_os_event(self_handle, ui, event)
    }

    fn accepts_drop(&self, widget: Handle<UiNode>, ui: &UserInterface) -> bool {
        self.window.accepts_drop(widget, ui)
    }
}

pub struct StyledPropertySelectorBuilder {
    window_builder: WindowBuilder,
}

impl StyledPropertySelectorBuilder {
    pub fn new(window_builder: WindowBuilder) -> Self {
        Self { window_builder }
    }

    pub fn build(
        self,
        target_style: &StyleResource,
        target_type: TypeId,
        style_property_name: ImmutableString,
        ctx: &mut BuildContext,
    ) -> Handle<UiNode> {
        let style_data = target_style.data_ref();
        let (items, property_list): (Vec<_>, Vec<_>) = style_data
            .all_properties()
            .iter()
            .filter_map(|(name, value)| {
                if value.value_type_id() == target_type {
                    Some((make_dropdown_list_option(ctx, name), name.clone()))
                } else {
                    None
                }
            })
            .unzip();
        let selection = property_list
            .iter()
            .position(|name| name == &style_property_name);
        let selected_item = selection.and_then(|i| items.get(i).cloned());
        let properties = ListViewBuilder::new(
            WidgetBuilder::new()
                .on_row(0)
                .on_column(0)
                .with_margin(Thickness::uniform(1.0)),
        )
        .with_selection(selection.map(|i| vec![i]).unwrap_or_default())
        .with_items(items)
        .build(ctx);

        let ok;
        let cancel;
        let buttons = StackPanelBuilder::new(
            WidgetBuilder::new()
                .on_column(0)
                .on_row(1)
                .with_horizontal_alignment(HorizontalAlignment::Right)
                .with_child({
                    ok = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_width(100.0),
                    )
                    .with_text("OK")
                    .build(ctx);
                    ok
                })
                .with_child({
                    cancel = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_width(100.0),
                    )
                    .with_text("Cancel")
                    .build(ctx);
                    cancel
                }),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(properties)
                .with_child(buttons),
        )
        .add_row(Row::stretch())
        .add_row(Row::auto())
        .add_column(Column::stretch())
        .build(ctx);

        let selector = StyledPropertySelector {
            window: self.window_builder.with_content(content).build_window(ctx),
            properties,
            property_list,
            ok,
            cancel,
            style_property_name,
        };

        if let Some(selected_item) = selected_item {
            ctx.inner().send(
                properties,
                ListViewMessage::BringItemIntoView(selected_item),
            );
        }

        ctx.add_node(UiNode::new(selector))
    }

    pub fn build_and_open_window(
        target_style: &StyleResource,
        target_type: TypeId,
        style_property_name: ImmutableString,
        ctx: &mut BuildContext,
    ) -> Handle<UiNode> {
        let window = StyledPropertySelectorBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(200.0))
                .with_title(WindowTitle::text("Select A Style Property"))
                .with_remove_on_close(true)
                .open(false),
        )
        .build(target_style, target_type, style_property_name, ctx);

        ctx.send_message(WindowMessage::open_modal(
            window,
            MessageDirection::ToWidget,
            true,
            true,
        ));

        window
    }
}

#[derive(Debug, Clone, Visit, Reflect, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "1b8fb74a-3911-4b44-bb71-1a0382ebb9a7")]
#[reflect(derived_type = "UiNode")]
pub struct StyledPropertyEditor {
    widget: Widget,
    bind: Handle<UiNode>,
    inner_editor: Handle<UiNode>,
    selector: Handle<UiNode>,
    target_style: Option<StyleResource>,
    style_property_name: ImmutableString,
    #[visit(skip)]
    #[reflect(hidden)]
    target_type_id: TypeId,
}

define_widget_deref!(StyledPropertyEditor);

impl Control for StyledPropertyEditor {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(target_style) = self.target_style.as_ref() {
            if let Some(ButtonMessage::Click) = message.data() {
                if message.destination() == self.bind {
                    self.selector = StyledPropertySelectorBuilder::build_and_open_window(
                        target_style,
                        self.target_type_id,
                        self.style_property_name.clone(),
                        &mut ui.build_ctx(),
                    );
                }
            }
        }

        // Re-cast messages from inner editor as message from this editor.
        // If anything is listening to messages from this editor, let them hear the messages from the inner
        // editor as if they were coming from this editor, but *do not* re-cast messages to the inner editor
        // to this editor. Particularly, when the inner editor is made invisible, that does not mean that
        // this editor should be invisible.
        if message.is_from(self.inner_editor) {
            let mut clone = message.clone();
            clone.destination = self.handle;
            ui.send_message(clone);
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if let Some(StyledPropertySelectorMessage::PropertyName(name)) =
            message.data_from(self.selector)
        {
            ui.post(
                self.handle,
                StyledPropertyEditorMessage::BindProperty(name.clone()),
            )
        }
    }
}

struct StyledPropertyEditorBuilder {
    widget_builder: WidgetBuilder,
    inner_editor: Handle<UiNode>,
    container: Handle<UiNode>,
    style_property_name: ImmutableString,
    target_style: Option<StyleResource>,
    target_type_id: TypeId,
}

impl StyledPropertyEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            inner_editor: Handle::NONE,
            container: Handle::NONE,
            style_property_name: Default::default(),
            target_style: None,
            target_type_id: ().type_id(),
        }
    }

    pub fn with_inner_editor(mut self, inner_editor: Handle<UiNode>) -> Self {
        self.inner_editor = inner_editor;
        self
    }

    pub fn with_container(mut self, container: Handle<UiNode>) -> Self {
        self.container = container;
        self
    }

    pub fn with_style_property_name(mut self, style_property_name: ImmutableString) -> Self {
        self.style_property_name = style_property_name;
        self
    }

    pub fn with_style_resource(mut self, target_style: Option<StyleResource>) -> Self {
        self.target_style = target_style;
        self
    }

    pub fn with_target_type_id(mut self, type_id: TypeId) -> Self {
        self.target_type_id = type_id;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let is_bound = !self.style_property_name.is_empty();
        let brush = if is_bound {
            ctx.style.property(Style::BRUSH_BRIGHT_BLUE)
        } else {
            ctx.style.property(Style::BRUSH_BRIGHTEST)
        };

        let tooltip = if is_bound {
            &format!("Bound To `{}` Property", self.style_property_name)
        } else {
            "Bind To Style Property"
        };

        let bind;
        let grid = GridBuilder::new(WidgetBuilder::new().with_child(self.container).with_child({
            bind = ButtonBuilder::new(
                WidgetBuilder::new()
                    .on_column(1)
                    .with_tooltip(make_simple_tooltip(ctx, tooltip))
                    .with_margin(Thickness::uniform(1.0))
                    .with_vertical_alignment(VerticalAlignment::Top)
                    .with_width(18.0)
                    .with_height(18.0),
            )
            .with_content(
                ImageBuilder::new(
                    WidgetBuilder::new()
                        .with_background(brush)
                        .with_margin(Thickness::uniform(2.0))
                        .with_width(16.0)
                        .with_height(16.0),
                )
                .with_opt_texture(BIND_ICON.clone())
                .build(ctx),
            )
            .build(ctx);
            bind
        }))
        .add_row(Row::auto())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .build(ctx);

        ctx.add_node(UiNode::new(StyledPropertyEditor {
            widget: self
                .widget_builder
                .with_preview_messages(true)
                .with_child(grid)
                .build(ctx),
            bind,
            inner_editor: self.inner_editor,
            selector: Default::default(),
            target_style: self.target_style,
            style_property_name: self.style_property_name,
            target_type_id: self.target_type_id,
        }))
    }
}

pub struct StyledPropertyEditorDefinition<T>
where
    T: FieldValue,
{
    #[allow(dead_code)]
    phantom: PhantomDataSendSync<T>,
}

impl<T> StyledPropertyEditorDefinition<T>
where
    T: FieldValue,
{
    pub fn new() -> Self {
        Self {
            phantom: Default::default(),
        }
    }
}

impl<T> Debug for StyledPropertyEditorDefinition<T>
where
    T: Reflect + FieldValue,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "StyledPropertyEditorDefinition")
    }
}

impl<T> PropertyEditorDefinition for StyledPropertyEditorDefinition<T>
where
    T: Reflect + FieldValue,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<StyledProperty<T>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<StyledProperty<T>>()?;
        if let Some(definition) = ctx
            .definition_container
            .definitions()
            .get(&TypeId::of::<T>())
        {
            let property_info = ctx.property_info;

            let proxy_property_info = FieldRef {
                metadata: &FieldMetadata {
                    name: property_info.name,
                    display_name: property_info.display_name,
                    read_only: property_info.read_only,
                    immutable_collection: property_info.immutable_collection,
                    min_value: property_info.min_value,
                    max_value: property_info.max_value,
                    step: property_info.step,
                    precision: property_info.precision,
                    tag: property_info.tag,
                    doc: property_info.doc,
                },
                value: &**value,
            };

            let instance =
                definition
                    .property_editor
                    .create_instance(PropertyEditorBuildContext {
                        build_context: ctx.build_context,
                        property_info: &proxy_property_info,
                        environment: ctx.environment.clone(),
                        definition_container: ctx.definition_container.clone(),
                        sync_flag: ctx.sync_flag,
                        layer_index: ctx.layer_index,
                        generate_property_string_values: ctx.generate_property_string_values,
                        filter: ctx.filter,
                        name_column_width: ctx.name_column_width,
                        base_path: ctx.base_path.clone(),
                        has_parent_object: ctx.has_parent_object,
                    })?;

            let wrapper = StyledPropertyEditorBuilder::new(WidgetBuilder::new())
                .with_container(match instance {
                    PropertyEditorInstance::Simple { editor } => editor,
                    PropertyEditorInstance::Custom { container, .. } => container,
                })
                .with_inner_editor(match instance {
                    PropertyEditorInstance::Simple { editor } => editor,
                    PropertyEditorInstance::Custom { editor, .. } => editor,
                })
                .with_target_type_id(TypeId::of::<T>())
                .with_style_resource(ctx.environment.as_ref().and_then(|env| {
                    (&**env as &dyn ComponentProvider)
                        .component_ref::<Option<StyleResource>>()
                        .cloned()
                        .flatten()
                }))
                .with_style_property_name(value.name.clone())
                .build(ctx.build_context);

            Ok(match instance {
                PropertyEditorInstance::Simple { .. } => {
                    PropertyEditorInstance::Simple { editor: wrapper }
                }
                PropertyEditorInstance::Custom { .. } => PropertyEditorInstance::Custom {
                    container: wrapper,
                    editor: wrapper,
                },
            })
        } else {
            Err(InspectorError::Custom("No editor!".to_string()))
        }
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        if let Some(definition) = ctx
            .definition_container
            .definitions()
            .get(&TypeId::of::<T>())
        {
            let instance = ctx
                .ui
                .node(ctx.instance)
                .cast::<StyledPropertyEditor>()
                .unwrap();

            let property_info = ctx.property_info;

            let value = property_info.cast_value::<StyledProperty<T>>()?;

            let proxy_property_info = FieldRef {
                metadata: &FieldMetadata {
                    name: property_info.name,
                    display_name: property_info.display_name,
                    read_only: property_info.read_only,
                    immutable_collection: property_info.immutable_collection,
                    min_value: property_info.min_value,
                    max_value: property_info.max_value,
                    step: property_info.step,
                    precision: property_info.precision,
                    tag: property_info.tag,
                    doc: property_info.doc,
                },
                value: &**value,
            };

            return definition
                .property_editor
                .create_message(PropertyEditorMessageContext {
                    property_info: &proxy_property_info,
                    environment: ctx.environment.clone(),
                    definition_container: ctx.definition_container.clone(),
                    sync_flag: ctx.sync_flag,
                    instance: instance.inner_editor,
                    layer_index: ctx.layer_index,
                    ui: ctx.ui,
                    generate_property_string_values: ctx.generate_property_string_values,
                    filter: ctx.filter,
                    name_column_width: ctx.name_column_width,
                    base_path: ctx.base_path.clone(),
                    has_parent_object: ctx.has_parent_object,
                });
        }

        Err(InspectorError::Custom("No editor!".to_string()))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if let Some(StyledPropertyEditorMessage::BindProperty(name)) = ctx.message.data() {
            return Some(PropertyChanged {
                name: format!("{}.{}", ctx.name, StyledProperty::<T>::NAME),
                value: FieldKind::object(name.clone()),
            });
        }

        // Try to translate other messages using inner property editor.
        if let Some(definition) = ctx
            .definition_container
            .definitions()
            .get(&TypeId::of::<T>())
        {
            let mut property_change =
                definition
                    .property_editor
                    .translate_message(PropertyEditorTranslationContext {
                        environment: ctx.environment.clone(),
                        name: ctx.name,
                        message: ctx.message,
                        definition_container: ctx.definition_container.clone(),
                    })?;

            property_change.name += ".";
            property_change.name += StyledProperty::<T>::PROPERTY;

            return Some(property_change);
        }

        None
    }
}
