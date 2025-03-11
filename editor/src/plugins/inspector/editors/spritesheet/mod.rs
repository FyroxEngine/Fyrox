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

use crate::fyrox::{
    core::{
        pool::Handle, reflect::prelude::*, type_traits::prelude::*, uuid_provider,
        visitor::prelude::*,
    },
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        define_constructor, define_widget_deref,
        grid::{Column, GridBuilder, Row},
        inspector::{
            editors::{
                PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                PropertyEditorMessageContext, PropertyEditorTranslationContext,
            },
            FieldKind, InspectorError, PropertyChanged,
        },
        message::{MessageDirection, UiMessage},
        text::TextBuilder,
        widget::{Widget, WidgetBuilder},
        window::WindowMessage,
        BuildContext, Control, Thickness, UiNode, UserInterface,
    },
    scene::animation::spritesheet::prelude::*,
};
use crate::plugins::inspector::editors::spritesheet::window::SpriteSheetFramesEditorWindow;

use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

mod window;

#[derive(Debug)]
pub struct SpriteSheetFramesContainerEditorDefinition;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpriteSheetFramesPropertyEditorMessage {
    Value(SpriteSheetFramesContainer),
}

impl SpriteSheetFramesPropertyEditorMessage {
    define_constructor!(SpriteSheetFramesPropertyEditorMessage:Value => fn value(SpriteSheetFramesContainer), layout: false);
}

#[derive(Clone, Debug, Reflect, Visit, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct SpriteSheetFramesPropertyEditor {
    widget: Widget,
    edit_button: Handle<UiNode>,
    container: SpriteSheetFramesContainer,
}

define_widget_deref!(SpriteSheetFramesPropertyEditor);

uuid_provider!(SpriteSheetFramesPropertyEditor = "8994228d-6106-4e41-872c-5191840badcc");

impl Control for SpriteSheetFramesPropertyEditor {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.edit_button {
                let window = SpriteSheetFramesEditorWindow::build(
                    &mut ui.build_ctx(),
                    self.container.clone(),
                    self.handle,
                );

                ui.send_message(WindowMessage::open_modal(
                    window,
                    MessageDirection::ToWidget,
                    true,
                    true,
                ));
            }
        } else if let Some(SpriteSheetFramesPropertyEditorMessage::Value(value)) = message.data() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
            {
                self.container = value.clone();
            }
        }
    }
}

impl SpriteSheetFramesPropertyEditor {
    pub fn build(ctx: &mut BuildContext, container: SpriteSheetFramesContainer) -> Handle<UiNode> {
        let edit_button;
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    TextBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .on_column(0),
                    )
                    .with_text(format!("Frames: {}", container.len()))
                    .build(ctx),
                )
                .with_child({
                    edit_button = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_width(60.0)
                            .with_margin(Thickness::uniform(1.0))
                            .on_column(1),
                    )
                    .with_text("Edit...")
                    .build(ctx);
                    edit_button
                }),
        )
        .add_row(Row::auto())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .build(ctx);

        ctx.add_node(UiNode::new(Self {
            widget: WidgetBuilder::new().with_child(grid).build(ctx),
            edit_button,
            container,
        }))
    }
}

impl PropertyEditorDefinition for SpriteSheetFramesContainerEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<SpriteSheetFramesContainer>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx
            .property_info
            .cast_value::<SpriteSheetFramesContainer>()?;

        let editor = SpriteSheetFramesPropertyEditor::build(ctx.build_context, value.clone());

        Ok(PropertyEditorInstance::Simple { editor })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx
            .property_info
            .cast_value::<SpriteSheetFramesContainer>()?;

        Ok(Some(SpriteSheetFramesPropertyEditorMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            value.clone(),
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(SpriteSheetFramesPropertyEditorMessage::Value(container)) =
                ctx.message.data()
            {
                return Some(PropertyChanged {
                    name: ctx.name.to_string(),

                    value: FieldKind::object(container.clone()),
                });
            }
        }
        None
    }
}
