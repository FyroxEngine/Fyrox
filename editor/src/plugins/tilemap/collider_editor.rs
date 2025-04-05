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

//! The [`TileEditor`] for a tile's collider layer. This allows the tile set editor
//! to display a dropdown list for the various [`TileCollider`] options, and displays
//! a text box for editing custom colliders when appropriate.
//! See [`TileColliderEditor`] for more information.

use commands::SetTileSetTilesCommand;
use fyrox::{
    asset::Resource,
    core::{algebra::Vector2, color::Color, pool::Handle, type_traits::prelude::*},
    gui::{
        border::BorderBuilder,
        brush::Brush,
        button::{Button, ButtonBuilder, ButtonMessage},
        decorator::{DecoratorBuilder, DecoratorMessage},
        dropdown_list::{DropdownListBuilder, DropdownListMessage},
        formatted_text::WrapMode,
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        message::UiMessage,
        stack_panel::StackPanelBuilder,
        text::{TextBuilder, TextMessage},
        text_box::{TextBoxBuilder, TextCommitMode},
        utils::make_simple_tooltip,
        widget::WidgetBuilder,
        BuildContext, HorizontalAlignment, Thickness, UiNode, UserInterface, VerticalAlignment,
    },
    scene::tilemap::{tileset::*, CustomTileCollider, CustomTileColliderResource, TileSetUpdate},
};
use std::str::FromStr;

use crate::{send_sync_message, MSG_SYNC_FLAG};

use super::*;

const COLLIDER_NAMES: &[&str] = &["None", "Full", "Custom"];

fn collider_to_index(tile_collider: &TileCollider) -> Option<usize> {
    match tile_collider {
        TileCollider::None => Some(0),
        TileCollider::Rectangle => Some(1),
        TileCollider::Custom(_) => Some(2),
        TileCollider::Mesh => None,
    }
}

fn send_visibility(ui: &UserInterface, destination: Handle<UiNode>, visible: bool) {
    ui.send_message(WidgetMessage::visibility(
        destination,
        MessageDirection::ToWidget,
        visible,
    ));
}

fn highlight_tool_button(button: Handle<UiNode>, highlight: bool, ui: &UserInterface) {
    let decorator = *ui.try_get_of_type::<Button>(button).unwrap().decorator;
    ui.send_message(DecoratorMessage::select(
        decorator,
        MessageDirection::ToWidget,
        highlight,
    ));
}

pub struct TileColliderEditor {
    /// The handle for the overall editor.
    handle: Handle<UiNode>,
    /// The UUID of the collider layer that we are editing.
    collider_id: Uuid,
    /// The current value being edited.
    value: TileCollider,
    /// The button which actives this editor as a draw tool
    /// and allows the user to apply this value to other tiles.
    draw_button: Handle<UiNode>,
    /// The button which toggles the visibility of the collider layer.
    show_button: Handle<UiNode>,
    /// The widget that shows the color of the collider layer.
    color_icon: Handle<UiNode>,
    /// The widget for the name of the collider layer.
    name_field: Handle<UiNode>,
    /// The dropdown list of collider types.
    list: Handle<UiNode>,
    /// The textbox for editing a custom collider.
    custom_field: Handle<UiNode>,
    /// A text widget for showing an error in the custom collider text.
    error_field: Handle<UiNode>,
    /// True if the custom collider text actually has an error.
    has_error: bool,
}

pub fn make_list_option(ctx: &mut BuildContext, name: &str) -> Handle<UiNode> {
    let text = TextBuilder::new(WidgetBuilder::new())
        .with_vertical_text_alignment(VerticalAlignment::Center)
        .with_horizontal_text_alignment(HorizontalAlignment::Left)
        .with_text(name)
        .build(ctx);
    DecoratorBuilder::new(
        BorderBuilder::new(WidgetBuilder::new().with_child(text))
            .with_corner_radius((4.0).into())
            .with_pad_by_corner_radius(false),
    )
    .build(ctx)
}

fn build_list(ctx: &mut BuildContext) -> Vec<Handle<UiNode>> {
    COLLIDER_NAMES
        .iter()
        .map(|name| make_list_option(ctx, name))
        .collect()
}

impl TileColliderEditor {
    pub fn new(
        collider_layer: &TileSetColliderLayer,
        value: TileCollider,
        ctx: &mut BuildContext,
    ) -> Self {
        let color_icon = BorderBuilder::new(
            WidgetBuilder::new()
                .on_column(2)
                .with_background(Brush::Solid(collider_layer.color.to_opaque()).into()),
        )
        .build(ctx);
        let draw_button = make_draw_button(Some(0), ctx);
        let show_button = make_show_button(Some(1), ctx);
        let name_field = TextBuilder::new(WidgetBuilder::new().on_column(3))
            .with_text(collider_layer.name.clone())
            .build(ctx);
        let custom_field = TextBoxBuilder::new(
            WidgetBuilder::new()
                .with_visibility(value.is_custom())
                .with_min_size(Vector2::new(0.0, 100.0)),
        )
        .with_multiline(true)
        .with_wrap(WrapMode::Word)
        .with_text_commit_mode(TextCommitMode::Changed)
        .build(ctx);
        let error_field = TextBuilder::new(WidgetBuilder::new().with_visibility(false))
            .with_wrap(WrapMode::Word)
            .with_horizontal_text_alignment(HorizontalAlignment::Center)
            .build(ctx);
        let index = collider_to_index(&value).unwrap_or_default();
        let list = DropdownListBuilder::new(WidgetBuilder::new().on_column(4))
            .with_items(build_list(ctx))
            .with_selected(index)
            .build(ctx);
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(draw_button)
                .with_child(show_button)
                .with_child(color_icon)
                .with_child(name_field)
                .with_child(list),
        )
        .add_column(Column::auto())
        .add_column(Column::auto())
        .add_column(Column::strict(24.0))
        .add_column(Column::strict(120.0))
        .add_column(Column::stretch())
        .add_row(Row::auto())
        .build(ctx);
        let handle = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(grid)
                .with_child(custom_field)
                .with_child(error_field),
        )
        .build(ctx);
        Self {
            handle,
            collider_id: collider_layer.uuid,
            value,
            draw_button,
            show_button,
            color_icon,
            name_field,
            custom_field,
            error_field,
            list,
            has_error: false,
        }
    }
    fn apply_collider_update(&mut self, state: &TileEditorState, ui: &mut UserInterface) {
        let layer = state.find_collider(self.collider_id).unwrap();
        ui.send_message(TextMessage::text(
            self.name_field,
            MessageDirection::ToWidget,
            layer.name.to_string(),
        ));
        ui.send_message(WidgetMessage::background(
            self.color_icon,
            MessageDirection::ToWidget,
            Brush::Solid(layer.color.to_opaque()).into(),
        ));
    }
    fn find_value(&self, state: &TileEditorState) -> Option<TileCollider> {
        let mut iter = state.tile_data().map(|(_, d)| {
            d.colliders
                .get(&self.collider_id)
                .cloned()
                .unwrap_or_default()
        });
        let value = iter.next()?;
        if iter.all(|v| v == value) {
            Some(value)
        } else {
            None
        }
    }
    fn sync_value_to_list(&self, ui: &mut UserInterface) {
        let index = collider_to_index(&self.value);
        send_sync_message(
            ui,
            DropdownListMessage::selection(self.list, MessageDirection::ToWidget, index),
        );
        send_visibility(ui, self.custom_field, self.value.is_custom());
        send_visibility(
            ui,
            self.error_field,
            self.value.is_custom() && self.has_error,
        );
    }
    fn build_custom_collider(
        &mut self,
        source: &str,
        ui: &mut UserInterface,
    ) -> Option<CustomTileColliderResource> {
        match CustomTileCollider::from_str(source) {
            Ok(collider) => {
                self.has_error = false;
                send_visibility(ui, self.error_field, false);
                Some(Resource::new_embedded(collider))
            }
            Err(e) => {
                self.has_error = true;
                send_visibility(ui, self.error_field, true);
                ui.send_message(TextMessage::text(
                    self.error_field,
                    MessageDirection::ToWidget,
                    e.to_string(),
                ));
                None
            }
        }
    }
    fn build_empty_collider(&mut self, ui: &mut UserInterface) -> CustomTileColliderResource {
        send_visibility(ui, self.error_field, false);
        send_sync_message(
            ui,
            TextMessage::text(self.custom_field, MessageDirection::ToWidget, "".into()),
        );
        Resource::new_embedded(CustomTileCollider::default())
    }
    fn send_value(&self, state: &TileEditorState, sender: &MessageSender, tile_book: &TileBook) {
        let Some(tile_set) = tile_book.tile_set_ref().cloned() else {
            return;
        };
        let Some(page) = state.page() else {
            return;
        };
        let mut update = TileSetUpdate::default();
        for position in state.selected_positions() {
            update.set_collider(
                page,
                position,
                std::iter::once(self.collider_id),
                &self.value,
            );
        }
        sender.do_command(SetTileSetTilesCommand {
            tile_set,
            tiles: update,
        });
    }
}

impl TileEditor for TileColliderEditor {
    fn handle(&self) -> Handle<UiNode> {
        self.handle
    }

    fn draw_button(&self) -> Handle<UiNode> {
        self.draw_button
    }

    fn sync_to_model(&mut self, state: &TileEditorState, ui: &mut UserInterface) {
        self.apply_collider_update(state, ui);
    }

    fn sync_to_state(&mut self, state: &TileEditorState, ui: &mut UserInterface) {
        self.has_error = false;
        self.value = self.find_value(state).unwrap_or_default();
        self.sync_value_to_list(ui);
        send_visibility(ui, self.custom_field, self.value.is_custom());
        send_visibility(ui, self.error_field, false);
        if let TileCollider::Custom(custom) = &self.value {
            let text = custom.data_ref().to_string();
            send_sync_message(
                ui,
                TextMessage::text(self.custom_field, MessageDirection::ToWidget, text),
            );
        }
        highlight_tool_button(
            self.show_button,
            state.is_visible_collider(self.collider_id),
            ui,
        );
    }

    fn draw_tile(
        &self,
        handle: TileDefinitionHandle,
        _subposition: Vector2<usize>,
        state: &TileDrawState,
        update: &mut TileSetUpdate,
        _tile_resource: &TileBook,
    ) {
        update.set_collider(
            handle.page(),
            handle.tile(),
            state.visible_colliders.iter().copied(),
            &self.value,
        );
    }

    fn handle_ui_message(
        &mut self,
        state: &mut TileEditorState,
        message: &UiMessage,
        ui: &mut UserInterface,
        tile_book: &TileBook,
        sender: &MessageSender,
    ) {
        if message.flags == MSG_SYNC_FLAG || message.direction() == MessageDirection::ToWidget {
            return;
        }
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.show_button {
                let visible = state.is_visible_collider(self.collider_id);
                state.set_visible_collider(self.collider_id, !visible);
            }
        } else if let Some(DropdownListMessage::SelectionChanged(Some(index))) = message.data() {
            if message.destination() == self.list {
                self.value = match *index {
                    1 => TileCollider::Rectangle,
                    2 => TileCollider::Custom(self.build_empty_collider(ui)),
                    _ => TileCollider::None,
                };
                send_visibility(ui, self.custom_field, self.value.is_custom());
                self.send_value(state, sender, tile_book);
            }
        } else if let Some(TextMessage::Text(text)) = message.data() {
            if message.destination() == self.custom_field {
                if let Some(value) = self.build_custom_collider(text, ui) {
                    self.value = TileCollider::Custom(value);
                    self.send_value(state, sender, tile_book);
                }
            }
        }
    }
}

const DRAW_BUTTON_WIDTH: f32 = 14.0;
const DRAW_BUTTON_HEIGHT: f32 = 14.0;

fn make_button(
    tab_index: Option<usize>,
    column: usize,
    tooltip: &str,
    icon: Option<TextureResource>,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_tab_index(tab_index)
            .on_column(column)
            .with_tooltip(make_simple_tooltip(ctx, tooltip))
            .with_margin(Thickness::uniform(1.0)),
    )
    .with_back(
        DecoratorBuilder::new(
            BorderBuilder::new(
                WidgetBuilder::new().with_foreground(ctx.style.property(Style::BRUSH_DARKER)),
            )
            .with_pad_by_corner_radius(false)
            .with_corner_radius((4.0).into())
            .with_stroke_thickness(Thickness::uniform(1.0).into()),
        )
        .with_selected_brush(ctx.style.property(Style::BRUSH_BRIGHT_BLUE))
        .with_normal_brush(ctx.style.property(Style::BRUSH_LIGHT))
        .with_hover_brush(ctx.style.property(Style::BRUSH_LIGHTER))
        .with_pressed_brush(ctx.style.property(Style::BRUSH_LIGHTEST))
        .build(ctx),
    )
    .with_content(
        ImageBuilder::new(
            WidgetBuilder::new()
                .with_background(Brush::Solid(Color::opaque(180, 180, 180)).into())
                .with_margin(Thickness::uniform(2.0))
                .with_width(DRAW_BUTTON_WIDTH)
                .with_height(DRAW_BUTTON_HEIGHT),
        )
        .with_opt_texture(icon)
        .build(ctx),
    )
    .build(ctx)
}

fn make_draw_button(tab_index: Option<usize>, ctx: &mut BuildContext) -> Handle<UiNode> {
    make_button(
        tab_index,
        0,
        "Apply collider to tiles",
        BRUSH_IMAGE.clone(),
        ctx,
    )
}

fn make_show_button(tab_index: Option<usize>, ctx: &mut BuildContext) -> Handle<UiNode> {
    make_button(
        tab_index,
        1,
        "Make collider visible",
        VISIBLE_IMAGE.clone(),
        ctx,
    )
}
