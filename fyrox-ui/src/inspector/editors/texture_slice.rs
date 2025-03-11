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
    border::BorderBuilder,
    brush::Brush,
    button::{ButtonBuilder, ButtonMessage},
    core::{
        algebra::{Matrix3, Vector2},
        color::Color,
        math::Rect,
        pool::Handle,
        reflect::prelude::*,
        some_or_return,
        type_traits::prelude::*,
        visitor::prelude::*,
    },
    decorator::DecoratorBuilder,
    define_constructor, define_widget_deref,
    draw::{CommandTexture, Draw, DrawingContext},
    grid::{Column, GridBuilder, Row},
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        FieldKind, InspectorError, PropertyChanged,
    },
    message::{CursorIcon, MessageDirection, OsEvent, UiMessage},
    nine_patch::TextureSlice,
    numeric::{NumericUpDownBuilder, NumericUpDownMessage},
    rect::{RectEditorBuilder, RectEditorMessage},
    scroll_viewer::ScrollViewerBuilder,
    stack_panel::StackPanelBuilder,
    text::TextBuilder,
    thumb::{ThumbBuilder, ThumbMessage},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    window::{Window, WindowBuilder, WindowMessage, WindowTitle},
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

impl TextureSliceEditorMessage {
    define_constructor!(TextureSliceEditorMessage:Slice => fn slice(TextureSlice), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
struct DragContext {
    initial_position: Vector2<f32>,
    bottom_margin: u32,
    left_margin: u32,
    right_margin: u32,
    top_margin: u32,
    texture_region: Rect<u32>,
}

#[derive(Clone, Reflect, Visit, TypeUuidProvider, ComponentProvider, Debug)]
#[type_uuid(id = "bd89b59f-13be-4804-bd9c-ed40cfd48b92")]
#[reflect(derived_type = "UiNode")]
pub struct TextureSliceEditor {
    widget: Widget,
    slice: TextureSlice,
    handle_size: f32,
    region_min_thumb: Handle<UiNode>,
    region_max_thumb: Handle<UiNode>,
    slice_min_thumb: Handle<UiNode>,
    slice_max_thumb: Handle<UiNode>,
    #[reflect(hidden)]
    #[visit(skip)]
    drag_context: Option<DragContext>,
    #[reflect(hidden)]
    #[visit(skip)]
    scale: f32,
}

impl TextureSliceEditor {
    fn sync_thumbs(&self, ui: &UserInterface) {
        for (thumb, position) in [
            (self.region_min_thumb, self.slice.texture_region.position),
            (
                self.region_max_thumb,
                self.slice.texture_region.right_bottom_corner(),
            ),
            (self.slice_min_thumb, self.slice.margin_min()),
            (self.slice_max_thumb, self.slice.margin_max()),
        ] {
            ui.send_message(WidgetMessage::desired_position(
                thumb,
                MessageDirection::ToWidget,
                position.cast::<f32>(),
            ))
        }
    }

    fn on_thumb_dragged(&mut self, thumb: Handle<UiNode>, offset: Vector2<f32>) {
        let ctx = some_or_return!(self.drag_context.as_ref());
        let texture = some_or_return!(self.slice.texture_source.clone());
        let texture_state = texture.state();
        let texture_data = some_or_return!(texture_state.data_ref());
        let TextureKind::Rectangle { width, height } = texture_data.kind() else {
            return;
        };

        let offset = Vector2::new(offset.x as i32, offset.y as i32);

        let margin_min = self.slice.margin_min();
        let margin_max = self.slice.margin_max();
        let initial_region = ctx.texture_region;
        let region = self.slice.texture_region.deref_mut();

        if thumb == self.slice_min_thumb {
            let top_margin = ctx.top_margin.saturating_add_signed(offset.y);
            if top_margin + region.position.y <= margin_max.y {
                *self.slice.top_margin = top_margin;
            } else {
                *self.slice.top_margin = margin_max.y - region.position.y;
            }

            let left_margin = ctx.left_margin.saturating_add_signed(offset.x);
            if left_margin + region.position.x <= margin_max.x {
                *self.slice.left_margin = left_margin;
            } else {
                *self.slice.left_margin = margin_max.x - region.position.x;
            }
        } else if thumb == self.slice_max_thumb {
            let bottom_margin = ctx.bottom_margin.saturating_add_signed(-offset.y);
            if (region.position.y + region.size.y).saturating_sub(bottom_margin) >= margin_min.y {
                *self.slice.bottom_margin = bottom_margin;
            } else {
                *self.slice.bottom_margin = region.position.y + region.size.y - margin_min.y;
            }

            let right_margin = ctx.right_margin.saturating_add_signed(-offset.x);
            if (region.position.x + region.size.x).saturating_sub(right_margin) >= margin_min.x {
                *self.slice.right_margin = right_margin;
            } else {
                *self.slice.right_margin = region.position.x + region.size.x - margin_min.x;
            }
        } else if thumb == self.region_min_thumb {
            let x = initial_region.position.x.saturating_add_signed(offset.x);
            let max_x = initial_region.position.x + initial_region.size.x;
            region.position.x = x.min(max_x);

            let y = initial_region.position.y.saturating_add_signed(offset.y);
            let max_y = initial_region.position.y + initial_region.size.y;
            region.position.y = y.min(max_y);

            region.size.x = ctx
                .texture_region
                .size
                .x
                .saturating_add_signed(-offset.x)
                .min(initial_region.position.x + initial_region.size.x);
            region.size.y = ctx
                .texture_region
                .size
                .y
                .saturating_add_signed(-offset.y)
                .min(initial_region.position.y + initial_region.size.y);
        } else if thumb == self.region_max_thumb {
            region.size.x = ctx
                .texture_region
                .size
                .x
                .saturating_add_signed(offset.x)
                .min(width);
            region.size.y = ctx
                .texture_region
                .size
                .y
                .saturating_add_signed(offset.y)
                .min(height);
        }
    }
}

define_widget_deref!(TextureSliceEditor);

impl Control for TextureSliceEditor {
    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        let mut size: Vector2<f32> = self.widget.measure_override(ui, available_size);

        if let Some(texture) = self.slice.texture_source.as_ref() {
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

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        for &child_handle in self.widget.children() {
            let child = ui.nodes.borrow(child_handle);
            ui.arrange_node(
                child_handle,
                &Rect::new(
                    child.desired_local_position().x,
                    child.desired_local_position().y,
                    child.desired_size().x,
                    child.desired_size().y,
                ),
            );
        }

        final_size
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let texture = some_or_return!(self.slice.texture_source.clone());

        let state = texture.state();
        let texture_data = some_or_return!(state.data_ref());

        // Only 2D textures can be used with nine-patch.
        let TextureKind::Rectangle { width, height } = texture_data.kind() else {
            return;
        };

        let texture_width = width as f32;
        let texture_height = height as f32;

        drawing_context.push_rect_filled(&Rect::new(0.0, 0.0, texture_width, texture_height), None);
        drawing_context.commit(
            self.clip_bounds(),
            self.background(),
            CommandTexture::Texture(texture.clone()),
            None,
        );

        let mut bounds = Rect {
            position: self.slice.texture_region.position.cast::<f32>(),
            size: self.slice.texture_region.size.cast::<f32>(),
        };

        if bounds.size.x == 0.0 && bounds.size.y == 0.0 {
            bounds.size.x = texture_width;
            bounds.size.y = texture_height;
        }

        drawing_context.push_rect(&bounds, 1.0);
        drawing_context.commit(
            self.clip_bounds(),
            self.foreground(),
            CommandTexture::Texture(texture.clone()),
            None,
        );

        let left_margin = *self.slice.left_margin as f32;
        let right_margin = *self.slice.right_margin as f32;
        let top_margin = *self.slice.top_margin as f32;
        let bottom_margin = *self.slice.bottom_margin as f32;
        let thickness = 1.0 / self.scale;

        // Draw nine slices.
        drawing_context.push_line(
            Vector2::new(bounds.position.x + left_margin, bounds.position.y),
            Vector2::new(
                bounds.position.x + left_margin,
                bounds.position.y + bounds.size.y,
            ),
            thickness,
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
            thickness,
        );
        drawing_context.push_line(
            Vector2::new(bounds.position.x, bounds.position.y + top_margin),
            Vector2::new(
                bounds.position.x + bounds.size.x,
                bounds.position.y + top_margin,
            ),
            thickness,
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
            thickness,
        );
        drawing_context.commit(
            self.clip_bounds(),
            self.foreground(),
            CommandTexture::None,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(TextureSliceEditorMessage::Slice(slice)) = message.data() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
            {
                self.slice = slice.clone();
                self.sync_thumbs(ui);
            }
        } else if let Some(msg) = message.data::<ThumbMessage>() {
            match msg {
                ThumbMessage::DragStarted { position } => {
                    self.drag_context = Some(DragContext {
                        initial_position: *position,
                        bottom_margin: *self.slice.bottom_margin,
                        left_margin: *self.slice.left_margin,
                        right_margin: *self.slice.right_margin,
                        top_margin: *self.slice.top_margin,
                        texture_region: *self.slice.texture_region,
                    });
                }
                ThumbMessage::DragDelta { offset } => {
                    self.on_thumb_dragged(message.destination(), *offset);
                    self.sync_thumbs(ui);
                }
                ThumbMessage::DragCompleted { .. } => {
                    self.drag_context = None;
                    ui.send_message(TextureSliceEditorMessage::slice(
                        self.handle(),
                        MessageDirection::FromWidget,
                        self.slice.clone(),
                    ));
                }
            }
        } else if let Some(WidgetMessage::MouseWheel { amount, .. }) = message.data() {
            self.scale = (self.scale + 0.1 * *amount).clamp(1.0, 10.0);

            ui.send_message(WidgetMessage::layout_transform(
                self.handle,
                MessageDirection::ToWidget,
                Matrix3::new_scaling(self.scale),
            ));

            for thumb in [
                self.slice_min_thumb,
                self.slice_max_thumb,
                self.region_min_thumb,
                self.region_max_thumb,
            ] {
                ui.send_message(WidgetMessage::width(
                    thumb,
                    MessageDirection::ToWidget,
                    self.handle_size / self.scale,
                ));
                ui.send_message(WidgetMessage::height(
                    thumb,
                    MessageDirection::ToWidget,
                    self.handle_size / self.scale,
                ));
            }
        }
    }
}

pub struct TextureSliceEditorBuilder {
    widget_builder: WidgetBuilder,
    slice: TextureSlice,
    handle_size: f32,
}

fn make_thumb(position: Vector2<u32>, handle_size: f32, ctx: &mut BuildContext) -> Handle<UiNode> {
    ThumbBuilder::new(
        WidgetBuilder::new()
            .with_desired_position(position.cast::<f32>())
            .with_child(
                DecoratorBuilder::new(BorderBuilder::new(
                    WidgetBuilder::new()
                        .with_width(handle_size)
                        .with_height(handle_size)
                        .with_cursor(Some(CursorIcon::Grab))
                        .with_foreground(Brush::Solid(Color::opaque(0, 150, 0)).into()),
                ))
                .with_pressable(false)
                .with_selected(false)
                .with_normal_brush(Brush::Solid(Color::opaque(0, 150, 0)).into())
                .with_hover_brush(Brush::Solid(Color::opaque(0, 255, 0)).into())
                .build(ctx),
            ),
    )
    .build(ctx)
}

impl TextureSliceEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            slice: Default::default(),
            handle_size: 8.0,
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
        let region_min_thumb =
            make_thumb(self.slice.texture_region.position, self.handle_size, ctx);
        let region_max_thumb = make_thumb(
            self.slice.texture_region.right_bottom_corner(),
            self.handle_size,
            ctx,
        );
        let slice_min_thumb = make_thumb(self.slice.margin_min(), self.handle_size, ctx);
        let slice_max_thumb = make_thumb(self.slice.margin_max(), self.handle_size, ctx);

        ctx.add_node(UiNode::new(TextureSliceEditor {
            widget: self
                .widget_builder
                .with_child(region_min_thumb)
                .with_child(region_max_thumb)
                .with_child(slice_min_thumb)
                .with_child(slice_max_thumb)
                .build(ctx),
            slice: self.slice,
            handle_size: self.handle_size,
            region_min_thumb,
            region_max_thumb,
            slice_min_thumb,
            slice_max_thumb,
            drag_context: None,
            scale: 1.0,
        }))
    }
}

#[derive(Clone, Reflect, Visit, TypeUuidProvider, ComponentProvider, Debug)]
#[type_uuid(id = "0293081d-55fd-4aa2-a06e-d53fba1a2617")]
#[reflect(derived_type = "UiNode")]
pub struct TextureSliceEditorWindow {
    window: Window,
    parent_editor: Handle<UiNode>,
    slice_editor: Handle<UiNode>,
    texture_slice: TextureSlice,
    left_margin: Handle<UiNode>,
    right_margin: Handle<UiNode>,
    top_margin: Handle<UiNode>,
    bottom_margin: Handle<UiNode>,
    region: Handle<UiNode>,
}

impl TextureSliceEditorWindow {
    fn on_slice_changed(&self, ui: &UserInterface) {
        ui.send_message(RectEditorMessage::value(
            self.region,
            MessageDirection::ToWidget,
            *self.texture_slice.texture_region,
        ));

        for (widget, value) in [
            (self.left_margin, &self.texture_slice.left_margin),
            (self.right_margin, &self.texture_slice.right_margin),
            (self.top_margin, &self.texture_slice.top_margin),
            (self.bottom_margin, &self.texture_slice.bottom_margin),
        ] {
            ui.send_message(NumericUpDownMessage::value(
                widget,
                MessageDirection::ToWidget,
                **value,
            ));
        }

        // Send the slice to the parent editor.
        ui.send_message(TextureSliceEditorMessage::slice(
            self.parent_editor,
            MessageDirection::ToWidget,
            self.texture_slice.clone(),
        ));
    }
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
        if let Some(TextureSliceEditorMessage::Slice(slice)) = message.data() {
            if message.direction() == MessageDirection::FromWidget
                && message.destination() == self.slice_editor
            {
                self.texture_slice = slice.clone();
                self.on_slice_changed(ui);
            }

            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
                && &self.texture_slice != slice
            {
                self.texture_slice = slice.clone();

                ui.send_message(TextureSliceEditorMessage::slice(
                    self.slice_editor,
                    MessageDirection::ToWidget,
                    self.texture_slice.clone(),
                ));

                self.on_slice_changed(ui);
            }
        } else if let Some(NumericUpDownMessage::Value(value)) =
            message.data::<NumericUpDownMessage<u32>>()
        {
            if message.direction() == MessageDirection::FromWidget {
                let mut slice = self.texture_slice.clone();
                let mut target = None;
                for (widget, margin) in [
                    (self.left_margin, &mut slice.left_margin),
                    (self.right_margin, &mut slice.right_margin),
                    (self.top_margin, &mut slice.top_margin),
                    (self.bottom_margin, &mut slice.bottom_margin),
                ] {
                    if message.destination() == widget {
                        margin.set_value_and_mark_modified(*value);
                        target = Some(widget);
                        break;
                    }
                }
                if target.is_some() {
                    ui.send_message(TextureSliceEditorMessage::slice(
                        self.handle,
                        MessageDirection::ToWidget,
                        slice,
                    ));
                }
            }
        } else if let Some(RectEditorMessage::Value(value)) =
            message.data::<RectEditorMessage<u32>>()
        {
            if message.direction() == MessageDirection::FromWidget
                && message.destination() == self.region
            {
                let mut slice = self.texture_slice.clone();
                slice.texture_region.set_value_and_mark_modified(*value);
                ui.send_message(TextureSliceEditorMessage::slice(
                    self.handle,
                    MessageDirection::ToWidget,
                    slice,
                ));
            }
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
    texture_slice: TextureSlice,
}

impl TextureSliceEditorWindowBuilder {
    pub fn new(window_builder: WindowBuilder) -> Self {
        Self {
            window_builder,
            texture_slice: Default::default(),
        }
    }

    pub fn with_texture_slice(mut self, slice: TextureSlice) -> Self {
        self.texture_slice = slice;
        self
    }

    pub fn build(self, parent_editor: Handle<UiNode>, ctx: &mut BuildContext) -> Handle<UiNode> {
        let region_text = TextBuilder::new(WidgetBuilder::new())
            .with_text("Texture Region")
            .build(ctx);
        let region =
            RectEditorBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                .with_value(*self.texture_slice.texture_region)
                .build(ctx);
        let left_margin_text = TextBuilder::new(WidgetBuilder::new())
            .with_text("Left Margin")
            .build(ctx);
        let left_margin =
            NumericUpDownBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                .with_value(*self.texture_slice.left_margin)
                .build(ctx);
        let right_margin_text = TextBuilder::new(WidgetBuilder::new())
            .with_text("Right Margin")
            .build(ctx);
        let right_margin =
            NumericUpDownBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                .with_value(*self.texture_slice.right_margin)
                .build(ctx);
        let top_margin_text = TextBuilder::new(WidgetBuilder::new())
            .with_text("Top Margin")
            .build(ctx);
        let top_margin =
            NumericUpDownBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                .with_value(*self.texture_slice.top_margin)
                .build(ctx);
        let bottom_margin_text = TextBuilder::new(WidgetBuilder::new())
            .with_text("Bottom Margin")
            .build(ctx);
        let bottom_margin =
            NumericUpDownBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                .with_value(*self.texture_slice.bottom_margin)
                .build(ctx);

        let toolbar = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(region_text)
                .with_child(region)
                .with_child(left_margin_text)
                .with_child(left_margin)
                .with_child(right_margin_text)
                .with_child(right_margin)
                .with_child(top_margin_text)
                .with_child(top_margin)
                .with_child(bottom_margin_text)
                .with_child(bottom_margin)
                .on_column(0),
        )
        .build(ctx);

        let slice_editor = TextureSliceEditorBuilder::new(
            WidgetBuilder::new()
                .with_clip_to_bounds(false)
                .with_background(Brush::Solid(Color::WHITE).into())
                .with_foreground(Brush::Solid(Color::GREEN).into())
                .with_margin(Thickness::uniform(3.0)),
        )
        .with_texture_slice(self.texture_slice.clone())
        .build(ctx);
        let scroll_viewer = ScrollViewerBuilder::new(WidgetBuilder::new().on_column(1))
            .with_horizontal_scroll_allowed(true)
            .with_vertical_scroll_allowed(true)
            .with_content(slice_editor)
            // Disable scrolling via mouse wheel. Mouse wheel is used to change zoom.
            .with_h_scroll_speed(0.0)
            .with_v_scroll_speed(0.0)
            .build(ctx);
        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(toolbar)
                .with_child(scroll_viewer),
        )
        .add_column(Column::strict(200.0))
        .add_column(Column::stretch())
        .add_row(Row::stretch())
        .build(ctx);

        let node = UiNode::new(TextureSliceEditorWindow {
            window: self.window_builder.with_content(content).build_window(ctx),
            parent_editor,
            slice_editor,
            texture_slice: self.texture_slice,
            left_margin,
            right_margin,
            top_margin,
            bottom_margin,
            region,
        });

        ctx.add_node(node)
    }
}

#[derive(Clone, Reflect, Visit, TypeUuidProvider, ComponentProvider, Debug)]
#[type_uuid(id = "024f3a3a-6784-4675-bd99-a4c6c19a8d91")]
#[reflect(derived_type = "UiNode")]
pub struct TextureSliceFieldEditor {
    widget: Widget,
    texture_slice: TextureSlice,
    edit: Handle<UiNode>,
    editor: Handle<UiNode>,
}

define_widget_deref!(TextureSliceFieldEditor);

impl Control for TextureSliceFieldEditor {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.edit {
                self.editor = TextureSliceEditorWindowBuilder::new(
                    WindowBuilder::new(WidgetBuilder::new().with_width(700.0).with_height(500.0))
                        .with_title(WindowTitle::text("Texture Slice Editor"))
                        .open(false)
                        .with_remove_on_close(true),
                )
                .with_texture_slice(self.texture_slice.clone())
                .build(self.handle, &mut ui.build_ctx());

                ui.send_message(WindowMessage::open_modal(
                    self.editor,
                    MessageDirection::ToWidget,
                    true,
                    true,
                ));
            }
        } else if let Some(TextureSliceEditorMessage::Slice(slice)) = message.data() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
                && &self.texture_slice != slice
            {
                self.texture_slice = slice.clone();
                ui.send_message(message.reverse());
                ui.send_message(TextureSliceEditorMessage::slice(
                    self.editor,
                    MessageDirection::ToWidget,
                    self.texture_slice.clone(),
                ));
            }
        }
    }
}

pub struct TextureSliceFieldEditorBuilder {
    widget_builder: WidgetBuilder,
    texture_slice: TextureSlice,
}

impl TextureSliceFieldEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            texture_slice: Default::default(),
        }
    }

    pub fn with_texture_slice(mut self, slice: TextureSlice) -> Self {
        self.texture_slice = slice;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let edit = ButtonBuilder::new(WidgetBuilder::new())
            .with_text("Edit...")
            .build(ctx);

        let node = UiNode::new(TextureSliceFieldEditor {
            widget: self.widget_builder.with_child(edit).build(ctx),
            texture_slice: self.texture_slice,
            edit,
            editor: Default::default(),
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
            editor: TextureSliceFieldEditorBuilder::new(
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
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<TextureSlice>()?;
        Ok(Some(TextureSliceEditorMessage::slice(
            ctx.instance,
            MessageDirection::ToWidget,
            value.clone(),
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(TextureSliceEditorMessage::Slice(value)) = ctx.message.data() {
                return Some(PropertyChanged {
                    name: ctx.name.to_string(),

                    value: FieldKind::object(value.clone()),
                });
            }
        }
        None
    }
}
