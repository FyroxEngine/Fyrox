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

use fyrox::core::log::{Log, MessageKind};
use fyrox::core::pool::Handle;
use fyrox::gui::inspector::editors::PropertyEditorDefinitionContainer;
use fyrox::gui::inspector::editors::enumeration::EnumPropertyEditorDefinition;
use fyrox::gui::inspector::{
    Inspector, InspectorBuilder, InspectorContext, InspectorContextArgs, InspectorMessage,
    PropertyAction,
};
use fyrox::gui::message::UiMessage;
use fyrox::gui::widget::{WidgetBuilder, WidgetMessage};
use fyrox::gui::window::{Window, WindowBuilder, WindowTitle};
use fyrox::gui::{BuildContext, UserInterface};
use fyrox::scene::terrain::{Brush, BrushMode, BrushShape, BrushTarget};
use std::sync::Arc;

#[derive(PartialEq)]
pub struct BrushPanel {
    pub window: Handle<Window>,
    pub inspector: Handle<Inspector>,
}

fn make_brush_mode_enum_property_editor_definition() -> EnumPropertyEditorDefinition<BrushMode> {
    EnumPropertyEditorDefinition {
        variant_generator: |i| match i {
            0 => BrushMode::Raise { amount: 0.1 },
            1 => BrushMode::Assign { value: 0.0 },
            2 => BrushMode::Flatten,
            3 => BrushMode::Smooth { kernel_radius: 5 },
            _ => unreachable!(),
        },
        index_generator: |v| match v {
            BrushMode::Raise { .. } => 0,
            BrushMode::Assign { .. } => 1,
            BrushMode::Flatten => 2,
            BrushMode::Smooth { .. } => 3,
        },
        names_generator: || {
            vec![
                "Raise or Lower".to_string(),
                "Assign Value".to_string(),
                "Flatten".to_string(),
                "Smooth".to_string(),
            ]
        },
    }
}

fn make_brush_target_enum_property_editor_definition() -> EnumPropertyEditorDefinition<BrushTarget>
{
    EnumPropertyEditorDefinition {
        variant_generator: |i| match i {
            0 => BrushTarget::HeightMap,
            1 => BrushTarget::LayerMask { layer: 0 },
            2 => BrushTarget::HoleMask,
            _ => unreachable!(),
        },
        index_generator: |v| match v {
            BrushTarget::HeightMap => 0,
            BrushTarget::LayerMask { .. } => 1,
            BrushTarget::HoleMask => 2,
        },
        names_generator: || {
            vec![
                "Height Map".to_string(),
                "Layer Mask".to_string(),
                "Holes".to_string(),
            ]
        },
    }
}

fn make_brush_shape_enum_property_editor_definition() -> EnumPropertyEditorDefinition<BrushShape> {
    EnumPropertyEditorDefinition {
        variant_generator: |i| match i {
            0 => BrushShape::Circle { radius: 0.5 },
            1 => BrushShape::Rectangle {
                width: 0.5,
                length: 0.5,
            },
            _ => unreachable!(),
        },
        index_generator: |v| match v {
            BrushShape::Circle { .. } => 0,
            BrushShape::Rectangle { .. } => 1,
        },
        names_generator: || vec!["Circle".to_string(), "Rectangle".to_string()],
    }
}

impl BrushPanel {
    pub fn new(ctx: &mut BuildContext, brush: &Brush) -> Self {
        let property_editors = PropertyEditorDefinitionContainer::with_default_editors();
        property_editors.insert(make_brush_mode_enum_property_editor_definition());
        property_editors.insert(make_brush_target_enum_property_editor_definition());
        property_editors.insert(make_brush_shape_enum_property_editor_definition());

        let context = InspectorContext::from_object(InspectorContextArgs {
            root_object: brush,
            object: brush,
            ctx,
            definition_container: Arc::new(property_editors),
            environment: None,
            layer_index: 0,
            generate_property_string_values: true,
            filter: Default::default(),
            name_column_width: 150.0,
            hide_name_column: false,
            base_path: Default::default(),
            has_parent_object: false,
        });

        let inspector;
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(250.0))
            .can_minimize(false)
            .can_maximize(false)
            .with_content({
                inspector = InspectorBuilder::new(WidgetBuilder::new())
                    .with_context(context)
                    .build(ctx);
                inspector
            })
            .open(false)
            .with_title(WindowTitle::text("Brush Options"))
            .build(ctx);

        Self { window, inspector }
    }

    pub fn sync_to_model(&self, ui: &mut UserInterface, brush: &Brush) {
        let ctx = ui[self.inspector].context().clone();
        if let Err(e) = ctx.sync(
            brush,
            ui,
            brush,
            0,
            true,
            Default::default(),
            Default::default(),
        ) {
            Log::writeln(
                MessageKind::Error,
                format!("Failed to sync BrushPanel's inspector. Reason: {e:?}"),
            )
        }
    }

    pub fn handle_ui_message(&self, message: &UiMessage, brush: &mut Brush) -> Option<()> {
        if let Some(InspectorMessage::PropertyChanged(msg)) =
            message.data_from::<InspectorMessage>(self.inspector)
        {
            PropertyAction::from_field_action(&msg.action).apply(
                &msg.path(),
                brush,
                &mut |result| {
                    Log::verify(result);
                },
            );
        }
        Some(())
    }

    pub fn destroy(self, ui: &UserInterface) {
        ui.send(self.window, WidgetMessage::Remove);
    }
}
