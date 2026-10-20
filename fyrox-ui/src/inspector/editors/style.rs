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

//! Property editor for StyledProperty. It acts like a proxy to inner property, but also
//! adds a special "bind" button used to change style binding of the property.

use crate::widget::WidgetMessage;
use crate::{
    BuildContext, Control, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
    VerticalAlignment, Widget,
    border::BorderBuilder,
    brush::Brush,
    button::{Button, ButtonBuilder, ButtonMessage},
    core::{
        ImmutableString,
        algebra::{Matrix3, Vector2},
        color::Color,
        pool::Handle,
        reflect::prelude::*,
        visitor::prelude::*,
    },
    decorator::DecoratorBuilder,
    define_widget_deref,
    draw::DrawingContext,
    grid::{Column, GridBuilder, Row},
    image::ImageBuilder,
    inspector::{
        FieldAction, InspectorError, PropertyChanged,
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
    },
    list_view::{ListView, ListViewBuilder, ListViewMessage},
    message::{MessageData, OsEvent, UiMessage},
    resources::{self, BIND_ICON},
    searchbar::{SearchBar, SearchBarBuilder, SearchBarMessage},
    stack_panel::StackPanelBuilder,
    style::{StyleProperty, StylePropertyContainer, StyledPropertyName, resource::StyleResource},
    text::{Text, TextBuilder, TextMessage},
    utils::{ImageButtonBuilder, adjust_decorator},
    widget::WidgetBuilder,
    window::{Window, WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
};
use rust_fuzzy_search::fuzzy_compare;
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

#[derive(Debug, Clone, PartialEq)]
pub enum StyledPropertySelectorMessage {
    PropertyName(ImmutableString),
}
impl MessageData for StyledPropertySelectorMessage {}

#[derive(Debug, Clone, PartialEq)]
pub enum StyledPropertyEditorMessage {
    BindProperty(ImmutableString),
    PropertyName(ImmutableString),
}
impl MessageData for StyledPropertyEditorMessage {}

#[derive(Debug, Clone, Visit, PartialEq, Reflect)]
#[reflect(type_uuid = "3a863a0f-7414-44f5-a7aa-7a6668a6d406")]
#[reflect(derived_type = "UiNode")]
pub struct StyledPropertySelector {
    window: Window,
    properties: Handle<ListView>,
    property_list: Vec<ImmutableString>,
    ok: Handle<Button>,
    cancel: Handle<Button>,
    search_bar: Handle<SearchBar>,
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
            if let Some(selected_index) = selected.first()
                && message.destination() == self.properties
            {
                self.style_property_name = self.property_list[*selected_index].clone();
            }
        } else if let Some(ButtonMessage::Click) = message.data_from(self.ok) {
            ui.post(
                self.handle,
                StyledPropertySelectorMessage::PropertyName(self.style_property_name.clone()),
            );
            ui.send(self.handle, WindowMessage::Close);
        } else if let Some(ButtonMessage::Click) = message.data_from(self.cancel) {
            ui.send(self.handle, WindowMessage::Close);
        } else if let Some(SearchBarMessage::Text(filter_text)) = message.data_from(self.search_bar)
        {
            let filter_text = filter_text.to_lowercase();
            let list_view_ref = &*ui[self.properties].items;
            for (item, value) in list_view_ref.iter().zip(&self.property_list) {
                let value = value.to_lowercase();
                let visibility = value.contains(&filter_text)
                    || fuzzy_compare(&filter_text, value.as_str()) >= 0.5;
                ui.send(*item, WidgetMessage::Visibility(visibility));
            }
        } else if let Some(WindowMessage::Open { .. }) = message.data_for(self.handle) {
            ui.send(self.search_bar, WidgetMessage::Focus);
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

pub fn make_dropdown_list_option(
    ctx: &mut BuildContext,
    container: &StylePropertyContainer,
) -> Handle<UiNode> {
    let (image, color) = match container.value {
        StyleProperty::Number(_) => (resources::NUMBER.clone(), Color::opaque(0, 162, 232)),
        StyleProperty::Thickness(_) => (resources::THICKNESS.clone(), Color::opaque(34, 177, 76)),
        StyleProperty::Color(_) => (resources::COLOR.clone(), Color::opaque(163, 73, 164)),
        StyleProperty::Brush(_) => (resources::BRUSH.clone(), Color::opaque(255, 127, 39)),
        StyleProperty::Texture(_) => (resources::TEXTURE.clone(), Color::opaque(150, 150, 150)),
    };

    let content = GridBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::left(5.0))
            .with_child(
                ImageBuilder::new(
                    WidgetBuilder::new()
                        .with_background(Brush::Solid(color).into())
                        .with_width(18.0)
                        .with_height(18.0),
                )
                .with_opt_texture(image.into())
                .build(ctx),
            )
            .with_child(
                TextBuilder::new(
                    WidgetBuilder::new()
                        .on_column(1)
                        .with_margin(crate::utils::text_margin())
                        .with_vertical_alignment(VerticalAlignment::Center),
                )
                .with_vertical_text_alignment(VerticalAlignment::Center)
                .with_horizontal_text_alignment(HorizontalAlignment::Left)
                .with_text(container.name.as_str())
                .build(ctx),
            ),
    )
    .add_row(Row::auto())
    .add_column(Column::auto())
    .add_column(Column::stretch())
    .build(ctx);

    adjust_decorator(
        DecoratorBuilder::new(
            BorderBuilder::new(WidgetBuilder::new().with_child(content))
                .with_corner_radius(4.0f32.into())
                .with_pad_by_corner_radius(false),
        ),
        ctx,
    )
    .build(ctx)
    .to_base()
}

impl StyledPropertySelectorBuilder {
    pub fn new(window_builder: WindowBuilder) -> Self {
        Self { window_builder }
    }

    pub fn build(
        self,
        target_style: &StyleResource,
        style_property_name: ImmutableString,
        ctx: &mut BuildContext,
    ) -> Handle<StyledPropertySelector> {
        let style_data = target_style.data_ref();
        let mut sorted_properties = style_data.all_properties().inner().clone();
        sorted_properties.sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
        let (items, property_list): (Vec<_>, Vec<_>) = sorted_properties
            .iter()
            .map(|container| {
                (
                    make_dropdown_list_option(ctx, container),
                    container.name.clone(),
                )
            })
            .unzip();
        let selection = property_list
            .iter()
            .position(|name| name == &style_property_name);
        let selected_item = selection.and_then(|i| items.get(i).cloned());
        let properties = ListViewBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
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
                .on_row(2)
                .with_height(28.0)
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

        let search_bar = SearchBarBuilder::new(
            WidgetBuilder::new()
                .with_height(24.0)
                .with_margin(Thickness::uniform(1.0)),
        )
        .build(ctx);

        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(search_bar)
                .with_child(properties)
                .with_child(buttons),
        )
        .add_row(Row::auto())
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
            search_bar,
            style_property_name,
        };

        if let Some(selected_item) = selected_item {
            ctx.inner().send(
                properties,
                ListViewMessage::BringItemIntoView(selected_item),
            );
        }

        ctx.add(selector)
    }

    pub fn build_and_open_window(
        target_style: &StyleResource,
        style_property_name: ImmutableString,
        ctx: &mut BuildContext,
    ) -> Handle<StyledPropertySelector> {
        let window = StyledPropertySelectorBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(600.0))
                .with_title(WindowTitle::text("Select A Style Property"))
                .with_remove_on_close(true)
                .open(false),
        )
        .build(target_style, style_property_name, ctx);

        ctx.inner().send(
            window,
            WindowMessage::Open {
                alignment: WindowAlignment::Center,
                modal: true,
                focus_content: true,
            },
        );

        window
    }
}

#[derive(Debug, Clone, Visit, PartialEq, Reflect)]
#[reflect(type_uuid = "1b8fb74a-3911-4b44-bb71-1a0382ebb9a7")]
#[reflect(derived_type = "UiNode")]
pub struct StyledPropertyNameEditor {
    widget: Widget,
    bind: Handle<Button>,
    name_text: Handle<Text>,
    selector: Handle<StyledPropertySelector>,
    target_style: Option<StyleResource>,
    style_property_name: ImmutableString,
}

define_widget_deref!(StyledPropertyNameEditor);

impl Control for StyledPropertyNameEditor {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(target_style) = self.target_style.as_ref()
            && let Some(ButtonMessage::Click) = message.data()
            && message.destination() == self.bind
        {
            self.selector = StyledPropertySelectorBuilder::build_and_open_window(
                target_style,
                self.style_property_name.clone(),
                &mut ui.build_ctx(),
            );
        }

        if let Some(StyledPropertyEditorMessage::PropertyName(name)) = message.data_for(self.handle)
        {
            ui.send(self.name_text, TextMessage::Text(name.to_string()));
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

struct StyledPropertyNameEditorBuilder {
    widget_builder: WidgetBuilder,
    style_property_name: ImmutableString,
    target_style: Option<StyleResource>,
}

impl StyledPropertyNameEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            style_property_name: Default::default(),
            target_style: None,
        }
    }

    pub fn with_style_property_name(mut self, style_property_name: ImmutableString) -> Self {
        self.style_property_name = style_property_name;
        self
    }

    pub fn with_style_resource(mut self, target_style: Option<StyleResource>) -> Self {
        self.target_style = target_style;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<StyledPropertyNameEditor> {
        let bind = ImageButtonBuilder::default()
            .on_column(1)
            .with_tooltip("Bind")
            .with_margin(Thickness::uniform(1.0))
            .with_image_color(Color::opaque(0, 200, 0))
            .with_vertical_alignment(VerticalAlignment::Top)
            .with_width(20.0)
            .with_height(20.0)
            .with_margin(Thickness::uniform(1.0))
            .with_vertical_alignment(VerticalAlignment::Top)
            .with_image_size(14.0)
            .with_image(BIND_ICON.clone().into())
            .build_button(ctx);

        let name_text = TextBuilder::new(WidgetBuilder::new())
            .with_vertical_text_alignment(VerticalAlignment::Center)
            .with_text(self.style_property_name.clone())
            .build(ctx);

        let grid = GridBuilder::new(WidgetBuilder::new().with_child(name_text).with_child(bind))
            .add_row(Row::auto())
            .add_column(Column::stretch())
            .add_column(Column::auto())
            .build(ctx);

        ctx.add(StyledPropertyNameEditor {
            widget: self
                .widget_builder
                .with_preview_messages(true)
                .with_child(grid)
                .build(ctx),
            bind,
            name_text,
            selector: Default::default(),
            target_style: self.target_style,
            style_property_name: self.style_property_name,
        })
    }
}

#[derive(Debug)]
pub struct StyledPropertyNameEditorDefinition;

impl PropertyEditorDefinition for StyledPropertyNameEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<StyledPropertyName>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<StyledPropertyName>()?;

        let editor = StyledPropertyNameEditorBuilder::new(WidgetBuilder::new())
            .with_style_property_name(value.0.clone())
            .with_style_resource(ctx.environment.as_ref().and_then(|env| {
                (&**env as &dyn Reflect)
                    .first_field_ref::<Option<StyleResource>>()
                    .cloned()
                    .flatten()
            }))
            .build(ctx.build_context);

        Ok(PropertyEditorInstance::simple(editor))
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<StyledPropertyName>()?;
        Ok(Some(UiMessage::for_widget(
            ctx.instance,
            StyledPropertyEditorMessage::PropertyName(value.0.clone()),
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if let Some(StyledPropertyEditorMessage::BindProperty(name)) = ctx.message.data() {
            return Some(PropertyChanged {
                name: ctx.name.to_string(),
                action: FieldAction::object(StyledPropertyName(name.clone())),
            });
        }

        None
    }
}
