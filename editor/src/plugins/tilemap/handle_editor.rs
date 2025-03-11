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

//! Give the Fyrox Inspector the ability to edit [`TileDefinitionHandle`] properties.

use std::any::TypeId;
use std::ops::{Deref, DerefMut};

use crate::{send_sync_message, MSG_SYNC_FLAG};

use fyrox::gui::inspector::FieldKind;
use fyrox::{
    core::{
        color::Color, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        visitor::prelude::*,
    },
    gui::{
        brush::Brush,
        button::{ButtonBuilder, ButtonMessage},
        define_constructor, define_widget_deref,
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        inspector::{
            editors::{
                PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                PropertyEditorMessageContext, PropertyEditorTranslationContext,
            },
            InspectorError, PropertyChanged,
        },
        message::{MessageDirection, UiMessage},
        text::TextMessage,
        text_box::{TextBoxBuilder, TextCommitMode},
        widget::{Widget, WidgetBuilder},
        BuildContext, Control, Thickness, UiNode, UserInterface,
    },
    scene::tilemap::TileDefinitionHandle,
};

use super::*;

/// A message for events related to [`TileDefinitionHandleEditor`].
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TileDefinitionHandleEditorMessage {
    /// The value of the handle has changed.
    Value(Option<TileDefinitionHandle>),
    /// The user has clicked the go-to button beside the handle.
    Goto(TileDefinitionHandle),
}

impl TileDefinitionHandleEditorMessage {
    define_constructor!(
        /// The value of the handle has changed.
        TileDefinitionHandleEditorMessage:Value => fn value(Option<TileDefinitionHandle>), layout: false);
    define_constructor!(
        /// The user has clicked the go-to button beside the handle.
        TileDefinitionHandleEditorMessage:Goto => fn goto(TileDefinitionHandle), layout: false);
}

/// The widget for editing a [`TileDefinitionHandle`].
/// It has a button that can be used to focus the tile map control panel on the tile
/// represented by this handle, assuming that the control panel has the correct
/// tile set. There is no way to ensure this, since a TileDefinitionHandle includes
/// no information about which tile set it refers to.
///
/// The lack of tile set informaiton also means that this widget cannot show an image
/// of the tile that the TileDefinitionHandle refers to, but a potential future
/// improvement might be to borrow the tile set from the tile map control panel and
/// use that to display a tile for this widget.
///
/// The value is displayed in a text box in the form "(x,y):(x,y)" where the first
/// pair is the page coordinates and the second pair is the tile coordinates.
/// When editing the handle, one need merely type four integers. Whatever
/// characters separate the integers are ignored, so "1 2 3 4" would be accepted.
#[derive(Clone, Debug, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
#[type_uuid(id = "60146cf0-33e3-4757-8e66-e7196324271f")]
pub struct TileDefinitionHandleEditor {
    widget: Widget,
    field: Handle<UiNode>,
    button: Handle<UiNode>,
    value: Option<TileDefinitionHandle>,
    allow_none: bool,
}

define_widget_deref!(TileDefinitionHandleEditor);

fn value_to_string(value: Option<TileDefinitionHandle>) -> String {
    value
        .map(|handle| handle.to_string())
        .unwrap_or_else(|| "None".into())
}

impl TileDefinitionHandleEditor {
    fn text(&self) -> String {
        value_to_string(self.value)
    }
}

impl Control for TileDefinitionHandleEditor {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        if message.flags == MSG_SYNC_FLAG {
            return;
        }
        if let Some(&TileDefinitionHandleEditorMessage::Value(handle)) = message.data() {
            if message.direction() == MessageDirection::ToWidget
                && message.destination() == self.handle()
            {
                self.value = handle;
                send_sync_message(
                    ui,
                    TextMessage::text(self.field, MessageDirection::ToWidget, self.text()),
                );
                ui.send_message(message.reverse());
            }
        } else if let Some(TextMessage::Text(text)) = message.data() {
            if message.direction() == MessageDirection::FromWidget
                && message.destination() == self.field
            {
                let value = TileDefinitionHandle::parse(text);
                if self.allow_none || value.is_some() {
                    self.value = value;
                }
                ui.send_message(TileDefinitionHandleEditorMessage::value(
                    self.handle(),
                    MessageDirection::FromWidget,
                    self.value,
                ));
                send_sync_message(
                    ui,
                    TextMessage::text(self.field, MessageDirection::ToWidget, self.text()),
                );
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.direction() == MessageDirection::FromWidget
                && message.destination() == self.button
            {
                if let Some(handle) = self.value {
                    ui.send_message(TileDefinitionHandleEditorMessage::goto(
                        self.handle(),
                        MessageDirection::FromWidget,
                        handle,
                    ));
                }
            }
        }
    }
}

/// A builder for creating instances of [`TileDefinitionHandleEditor`].
pub struct TileDefinitionHandleEditorBuilder {
    widget_builder: WidgetBuilder,
    value: Option<TileDefinitionHandle>,
    allow_none: bool,
}

impl TileDefinitionHandleEditorBuilder {
    /// Begin building a new [`TileDefinitionHandleEditor`] with a widget from the given builder.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            value: None,
            allow_none: true,
        }
    }
    /// Control whether None is an acceptable value for the handle. This defaults to `true`.
    pub fn with_allow_none(mut self, allow_none: bool) -> Self {
        self.allow_none = allow_none;
        self
    }
    /// Set the initial value of the handle.
    pub fn with_value(mut self, value: Option<TileDefinitionHandle>) -> Self {
        self.value = value;
        self
    }
    /// Build the widgets for the [`TileDefinitionHandleEditor`].
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let text = value_to_string(self.value);
        let field = TextBoxBuilder::new(WidgetBuilder::new())
            .with_text(text)
            .with_text_commit_mode(TextCommitMode::LostFocusPlusEnter)
            .build(ctx);
        let button = ButtonBuilder::new(WidgetBuilder::new().on_column(1))
            .with_content(
                ImageBuilder::new(
                    WidgetBuilder::new()
                        .with_background(Brush::Solid(Color::opaque(180, 180, 180)).into())
                        .with_margin(Thickness::uniform(1.0))
                        .with_width(16.0)
                        .with_height(16.0),
                )
                .with_opt_texture(PALETTE_IMAGE.clone())
                .build(ctx),
            )
            .build(ctx);
        let grid = GridBuilder::new(WidgetBuilder::new().with_child(field).with_child(button))
            .add_column(Column::stretch())
            .add_column(Column::auto())
            .add_row(Row::auto())
            .build(ctx);
        ctx.add_node(UiNode::new(TileDefinitionHandleEditor {
            widget: self.widget_builder.with_child(grid).build(ctx),
            field,
            button,
            allow_none: self.allow_none,
            value: self.value,
        }))
    }
}

/// [`PropertyEditorDefinition`] for [`TileDefinitionHandleEditor`].
#[derive(Debug)]
pub struct TileDefinitionHandlePropertyEditorDefinition;

impl PropertyEditorDefinition for TileDefinitionHandlePropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<TileDefinitionHandle>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = *ctx.property_info.cast_value::<TileDefinitionHandle>()?;
        Ok(PropertyEditorInstance::Simple {
            editor: TileDefinitionHandleEditorBuilder::new(
                WidgetBuilder::new()
                    .with_min_size(Vector2::new(0.0, 17.0))
                    .with_margin(Thickness::uniform(1.0)),
            )
            .with_allow_none(false)
            .with_value(Some(value))
            .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = *ctx.property_info.cast_value::<TileDefinitionHandle>()?;
        Ok(Some(TileDefinitionHandleEditorMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            Some(value),
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(&TileDefinitionHandleEditorMessage::Value(Some(value))) = ctx.message.data()
            {
                return Some(PropertyChanged {
                    name: ctx.name.to_string(),
                    value: FieldKind::object(value),
                });
            }
        }
        None
    }
}

/// [`PropertyEditorDefinition`] for optional [`TileDefinitionHandleEditor`].
#[derive(Debug)]
pub struct OptionTileDefinitionHandlePropertyEditorDefinition;

impl PropertyEditorDefinition for OptionTileDefinitionHandlePropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Option<TileDefinitionHandle>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = *ctx
            .property_info
            .cast_value::<Option<TileDefinitionHandle>>()?;
        Ok(PropertyEditorInstance::Simple {
            editor: TileDefinitionHandleEditorBuilder::new(
                WidgetBuilder::new()
                    .with_min_size(Vector2::new(0.0, 17.0))
                    .with_margin(Thickness::uniform(1.0)),
            )
            .with_value(value)
            .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = *ctx
            .property_info
            .cast_value::<Option<TileDefinitionHandle>>()?;
        Ok(Some(TileDefinitionHandleEditorMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            value,
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(&TileDefinitionHandleEditorMessage::Value(value)) = ctx.message.data() {
                return Some(PropertyChanged {
                    name: ctx.name.to_string(),
                    value: FieldKind::object(value),
                });
            }
        }
        None
    }
}
