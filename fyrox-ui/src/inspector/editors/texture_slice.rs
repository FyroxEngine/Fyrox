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
    core::{
        algebra::Vector2, math::Rect, pool::Handle, reflect::prelude::*, some_or_return,
        type_traits::prelude::*, visitor::prelude::*,
    },
    define_constructor, define_widget_deref,
    draw::{CommandTexture, Draw, DrawingContext},
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        InspectorError, PropertyChanged,
    },
    message::{MessageDirection, OsEvent, UiMessage},
    nine_patch::TextureSlice,
    scroll_viewer::ScrollViewerBuilder,
    widget::{Widget, WidgetBuilder},
    window::{Window, WindowBuilder},
    BuildContext, Control, Thickness, UiNode, UserInterface, VerticalAlignment,
};
use fyrox_texture::TextureKind;
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

#[derive(Debug, Clone, PartialEq)]
pub enum TextureSliceEditorMessage {
    Slice(TextureSlice),
}

impl crate::button::ButtonMessage {
    define_constructor!(TextureSliceEditorMessage:Slice => fn slice(TextureSlice), layout: false);
}

#[derive(Clone, Reflect, Visit, TypeUuidProvider, ComponentProvider, Debug)]
#[type_uuid(id = "bd89b59f-13be-4804-bd9c-ed40cfd48b92")]
pub struct TextureSliceEditor {
    widget: Widget,
    slice: TextureSlice,
    handle_size: f32,
}

define_widget_deref!(TextureSliceEditor);

impl Control for TextureSliceEditor {
    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        let mut size: Vector2<f32> = self.widget.measure_override(ui, available_size);

        if let Some(texture) = self.slice.texture.as_ref() {
            let state = texture.state();
            if let Some(data) = state.data_ref() {
                if let TextureKind::Rectangle { width, height } = data.kind() {
                    let width = width as f32;
                    let height = height as f32;
                    if size.x < width {
                        size.x = width;
                    }
                    if size.y < height {
                        size.y = height;
                    }
                }
            }
        }

        size
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let texture = some_or_return!(self.slice.texture.deref().clone());
        drawing_context.commit(
            self.clip_bounds(),
            self.background(),
            CommandTexture::Texture(texture.clone()),
            None,
        );

        let state = texture.state();
        let texture_data = some_or_return!(state.data_ref());

        // Only 2D textures can be used with nine-patch.
        let TextureKind::Rectangle { width, height } = texture_data.kind() else {
            return;
        };

        let texture_width = width as f32;
        let texture_height = height as f32;

        let bounds = self
            .slice
            .texture_region
            .map(|region| Rect {
                position: region.position.cast::<f32>(),
                size: region.size.cast::<f32>(),
            })
            .unwrap_or_else(|| Rect::new(0.0, 0.0, texture_width, texture_height));

        let left_margin = *self.slice.left_margin as f32;
        let right_margin = *self.slice.right_margin as f32;
        let top_margin = *self.slice.top_margin as f32;
        let bottom_margin = *self.slice.bottom_margin as f32;

        // Draw nine slices.
        drawing_context.push_line(
            Vector2::new(bounds.position.x + left_margin, bounds.position.y),
            Vector2::new(
                bounds.position.x + left_margin,
                bounds.position.y + bounds.size.y,
            ),
            1.0,
        );
        drawing_context.push_line(
            Vector2::new(
                bounds.position.x + bounds.size.x - right_margin,
                bounds.position.y,
            ),
            Vector2::new(
                bounds.position.x + bounds.size.x - right_margin,
                bounds.position.y + bounds.size.y,
            ),
            1.0,
        );
        drawing_context.push_line(
            Vector2::new(bounds.position.x, bounds.position.y + top_margin),
            Vector2::new(
                bounds.position.x + bounds.size.x,
                bounds.position.y + top_margin,
            ),
            1.0,
        );
        drawing_context.push_line(
            Vector2::new(
                bounds.position.x,
                bounds.position.y + bounds.size.y - bottom_margin,
            ),
            Vector2::new(
                bounds.position.x + bounds.size.x,
                bounds.position.y + bounds.size.y - bottom_margin,
            ),
            1.0,
        );
        drawing_context.commit(
            self.clip_bounds(),
            self.foreground(),
            CommandTexture::None,
            None,
        );

        // Draw handles.
        let half_handle_size = self.handle_size / 2.0;
        drawing_context.push_rect_filled(
            &Rect::new(
                bounds.position.x + left_margin - half_handle_size,
                bounds.position.y + top_margin - half_handle_size,
                self.handle_size,
                self.handle_size,
            ),
            None,
        );
        drawing_context.push_rect_filled(
            &Rect::new(
                bounds.position.x + bounds.size.x - right_margin - half_handle_size,
                bounds.position.y + bounds.size.y - bottom_margin - half_handle_size,
                self.handle_size,
                self.handle_size,
            ),
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(TextureSliceEditorMessage::Slice(slice)) = message.data() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::FromWidget
            {
                self.slice = slice.clone();
            }
        }
    }
}

pub struct TextureSliceEditorBuilder {
    widget_builder: WidgetBuilder,
    slice: TextureSlice,
    handle_size: f32,
}

impl TextureSliceEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            slice: Default::default(),
            handle_size: 4.0,
        }
    }

    pub fn with_texture_slice(mut self, slice: TextureSlice) -> Self {
        self.slice = slice;
        self
    }

    pub fn with_handle_size(mut self, size: f32) -> Self {
        self.handle_size = size;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        ctx.add_node(UiNode::new(TextureSliceEditor {
            widget: self.widget_builder.build(ctx),
            slice: self.slice,
            handle_size: self.handle_size,
        }))
    }
}

#[derive(Clone, Reflect, Visit, TypeUuidProvider, ComponentProvider, Debug)]
#[type_uuid(id = "0293081d-55fd-4aa2-a06e-d53fba1a2617")]
pub struct TextureSliceEditorWindow {
    window: Window,
    slice_editor: Handle<UiNode>,
}

impl Deref for TextureSliceEditorWindow {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.window.widget
    }
}

impl DerefMut for TextureSliceEditorWindow {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.window.widget
    }
}

impl Control for TextureSliceEditorWindow {
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

    fn on_visual_transform_changed(&self) {
        self.window.on_visual_transform_changed()
    }

    fn post_draw(&self, drawing_context: &mut DrawingContext) {
        self.window.post_draw(drawing_context)
    }

    fn update(&mut self, dt: f32, ui: &mut UserInterface) {
        self.window.update(dt, ui);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.window.handle_routed_message(ui, message);
        if message.data::<TextureSliceEditorMessage>().is_some()
            && message.direction() == MessageDirection::FromWidget
            && message.destination() == self.slice_editor
        {
            // Re-cast the message.
            ui.send_message(
                message
                    .clone()
                    .with_destination(self.handle)
                    .with_direction(MessageDirection::FromWidget),
            );
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        self.window.preview_message(ui, message);
    }

    fn handle_os_event(
        &mut self,
        self_handle: Handle<UiNode>,
        ui: &mut UserInterface,
        event: &OsEvent,
    ) {
        self.window.handle_os_event(self_handle, ui, event);
    }
}

pub struct TextureSliceEditorWindowBuilder {
    window_builder: WindowBuilder,
}

impl TextureSliceEditorWindowBuilder {
    pub fn new(window_builder: WindowBuilder) -> Self {
        Self { window_builder }
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let slice_editor = TextureSliceEditorBuilder::new(WidgetBuilder::new()).build(ctx);
        let scroll_viewer = ScrollViewerBuilder::new(WidgetBuilder::new())
            .with_content(slice_editor)
            .build(ctx);

        let node = UiNode::new(TextureSliceEditorWindow {
            window: self
                .window_builder
                .with_content(scroll_viewer)
                .build_window(ctx),
            slice_editor,
        });

        ctx.add_node(node)
    }
}

#[derive(Debug)]
pub struct TextureSlicePropertyEditorDefinition;

impl PropertyEditorDefinition for TextureSlicePropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<TextureSlice>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<TextureSlice>()?;
        Ok(PropertyEditorInstance::Simple {
            editor: TextureSliceEditorBuilder::new(
                WidgetBuilder::new()
                    .with_margin(Thickness::top_bottom(1.0))
                    .with_vertical_alignment(VerticalAlignment::Center),
            )
            .with_texture_slice(value.clone())
            .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        _ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        todo!()
    }

    fn translate_message(&self, _ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        todo!()
    }
}
