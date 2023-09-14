use crate::inspector::editors::spritesheet::window::SpriteSheetFramesEditorWindow;
use fyrox::{
    animation::spritesheet::SpriteSheetFramesContainer,
    core::pool::Handle,
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
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

mod window;

#[derive(Debug)]
pub struct SpriteSheetFramesContainerEditorDefinition;

#[derive(Debug, PartialEq, Eq)]
pub enum SpriteSheetFramesPropertyEditorMessage {
    Value(SpriteSheetFramesContainer),
}

impl SpriteSheetFramesPropertyEditorMessage {
    define_constructor!(Self:Value => fn value(SpriteSheetFramesContainer), layout: false);
}

#[derive(Clone)]
pub struct SpriteSheetFramesPropertyEditor {
    widget: Widget,
    edit_button: Handle<UiNode>,
    container: SpriteSheetFramesContainer,
}

define_widget_deref!(SpriteSheetFramesPropertyEditor);

impl Control for SpriteSheetFramesPropertyEditor {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

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
            widget: WidgetBuilder::new().with_child(grid).build(),
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
                    owner_type_id: ctx.owner_type_id,
                    value: FieldKind::object(container.clone()),
                });
            }
        }
        None
    }
}
