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

use crate::{
    asset::item::AssetItem,
    fyrox::{
        core::{
            algebra::Vector2, color::Color, pool::Handle, reflect::prelude::*,
            type_traits::prelude::*, uuid_provider, visitor::prelude::*,
        },
        graph::BaseSceneGraph,
        gui::{
            brush::Brush,
            define_constructor,
            draw::{CommandTexture, Draw, DrawingContext},
            font::{Font, FontResource, BUILT_IN_FONT},
            formatted_text::WrapMode,
            inspector::{
                editors::{
                    PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                    PropertyEditorMessageContext, PropertyEditorTranslationContext,
                },
                FieldKind, InspectorError, PropertyChanged,
            },
            message::{MessageDirection, UiMessage},
            text::{TextBuilder, TextMessage},
            widget::{Widget, WidgetBuilder, WidgetMessage},
            BuildContext, Control, UiNode, UserInterface,
        },
    },
};

use fyrox::asset::manager::ResourceManager;
use std::{
    any::TypeId,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
};

#[derive(Clone, Visit, Reflect, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct FontField {
    widget: Widget,
    text_preview: Handle<UiNode>,
    font: FontResource,
    #[reflect(hidden)]
    #[visit(skip)]
    resource_manager: ResourceManager,
}

impl Debug for FontField {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TextureEditor")
    }
}

impl Deref for FontField {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl DerefMut for FontField {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

#[derive(Debug, PartialEq, Clone, Eq)]
pub enum FontFieldMessage {
    Font(FontResource),
}

impl FontFieldMessage {
    define_constructor!(FontFieldMessage:Font => fn font(FontResource), layout: false);
}

uuid_provider!(FontField = "5db49479-ff89-49b8-a038-0766253d6493");

impl Control for FontField {
    fn draw(&self, drawing_context: &mut DrawingContext) {
        // Emit transparent geometry for the field to be able to catch mouse events without precise pointing at the
        // node name letters.
        drawing_context.push_rect_filled(&self.bounding_rect(), None);
        drawing_context.commit(
            self.clip_bounds(),
            Brush::Solid(Color::TRANSPARENT),
            CommandTexture::None,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(WidgetMessage::Drop(dropped)) = message.data::<WidgetMessage>() {
            if message.destination() == self.handle {
                if let Some(item) = ui.node(*dropped).cast::<AssetItem>() {
                    if let Some(font) = item.resource::<Font>() {
                        ui.send_message(FontFieldMessage::font(
                            self.handle(),
                            MessageDirection::ToWidget,
                            font,
                        ));
                    }
                }
            }
        } else if let Some(FontFieldMessage::Font(font)) = message.data::<FontFieldMessage>() {
            if &self.font != font && message.direction() == MessageDirection::ToWidget {
                self.font = font.clone();

                ui.send_message(TextMessage::font(
                    self.text_preview,
                    MessageDirection::ToWidget,
                    font.clone(),
                ));
                ui.send_message(TextMessage::text(
                    self.text_preview,
                    MessageDirection::ToWidget,
                    make_name(&self.resource_manager, &self.font),
                ));

                ui.send_message(message.reverse());
            }
        }
    }
}

pub struct FontFieldBuilder {
    widget_builder: WidgetBuilder,
    font: FontResource,
}

fn make_name(resource_manager: &ResourceManager, font: &FontResource) -> String {
    match resource_manager.resource_path(font.as_ref()) {
        Some(path) => {
            if font == &BUILT_IN_FONT.resource.clone() {
                "BuiltIn - AaBbCcDd1234567890".to_string()
            } else {
                format!("{} - AaBbCcDd1234567890", path.display())
            }
        }
        None => "Embedded - AaBbCcDd1234567890".to_string(),
    }
}

impl FontFieldBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            font: BUILT_IN_FONT.resource(),
        }
    }

    pub fn with_font(mut self, font: FontResource) -> Self {
        self.font = font;
        self
    }

    pub fn build(
        self,
        resource_manager: ResourceManager,
        ctx: &mut BuildContext,
    ) -> Handle<UiNode> {
        let text_preview;
        let widget = self
            .widget_builder
            .with_allow_drop(true)
            .with_child({
                text_preview = TextBuilder::new(WidgetBuilder::new())
                    .with_wrap(WrapMode::Word)
                    .with_text(make_name(&resource_manager, &self.font))
                    .with_font(self.font.clone())
                    .build(ctx);
                text_preview
            })
            .build(ctx);

        let editor = FontField {
            widget,
            text_preview,
            font: self.font,
            resource_manager,
        };

        ctx.add_node(UiNode::new(editor))
    }
}

#[derive(Debug)]
pub struct FontPropertyEditorDefinition {
    pub resource_manager: ResourceManager,
}

impl PropertyEditorDefinition for FontPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<FontResource>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<FontResource>()?;

        Ok(PropertyEditorInstance::Simple {
            editor: FontFieldBuilder::new(
                WidgetBuilder::new().with_min_size(Vector2::new(0.0, 17.0)),
            )
            .with_font(value.clone())
            .build(self.resource_manager.clone(), ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<FontResource>()?;

        Ok(Some(FontFieldMessage::font(
            ctx.instance,
            MessageDirection::ToWidget,
            value.clone(),
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(FontFieldMessage::Font(value)) = ctx.message.data() {
                return Some(PropertyChanged {
                    name: ctx.name.to_string(),
                    value: FieldKind::object(value.clone()),
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod test {
    use crate::plugins::inspector::editors::font::FontFieldBuilder;
    use fyrox::asset::manager::ResourceManager;
    use fyrox::core::task::TaskPool;
    use fyrox::{gui::test::test_widget_deletion, gui::widget::WidgetBuilder};
    use std::sync::Arc;

    #[test]
    fn test_deletion() {
        let resource_manager = ResourceManager::new(Arc::new(TaskPool::new()));
        test_widget_deletion(|ctx| {
            FontFieldBuilder::new(WidgetBuilder::new()).build(resource_manager, ctx)
        });
    }
}
