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

//! This is the tab of the tile set editor that allows the user to modify the collider
//! layers stored within the tile set. Layers can be created, deleted, renamed
//! and their colors can be modified.

use core::f32;

use brush::{BrushMacroData, TileMapBrushResource};
use fyrox::gui::{
    button::ButtonMessage,
    grid::*,
    list_view::{ListView, ListViewBuilder, ListViewMessage},
    scroll_viewer::ScrollViewerBuilder,
    stack_panel::StackPanelBuilder,
    text::{TextBuilder, TextMessage},
    text_box::{TextBoxBuilder, TextCommitMode},
    utils::{make_arrow, ArrowDirection},
    HorizontalAlignment, VerticalAlignment,
};

use fyrox::scene::tilemap::*;

use crate::{send_sync_message, MSG_SYNC_FLAG};

use super::*;

const MISSING_MACRO: &str = "UNKNOWN MACRO";

/// This is the tab of the tile set editor that allows the user to modify the macros
/// stored within the brush. Macro instances can be created, deleted, renamed
/// and their settings can be modified.
pub struct MacroTab {
    handle: Handle<UiNode>,
    macros: BrushMacroListRef,
    current_macro_id: Option<Uuid>,
    macro_panel: Handle<UiNode>,
    list: Handle<UiNode>,
    up_button: Handle<UiNode>,
    down_button: Handle<UiNode>,
    remove_button: Handle<UiNode>,
    add_buttons: Box<[Handle<UiNode>]>,
    data_panel: Handle<UiNode>,
    name_field: Handle<UiNode>,
}

fn make_arrow_button(
    ctx: &mut BuildContext,
    dir: ArrowDirection,
    column: usize,
    row: usize,
) -> Handle<UiNode> {
    let arrow = make_arrow(ctx, dir, 16.0);
    ButtonBuilder::new(
        WidgetBuilder::new()
            .on_column(column)
            .on_row(row)
            .with_height(24.0)
            .with_margin(Thickness::uniform(1.0)),
    )
    .with_content(arrow)
    .build(ctx)
}

fn make_button(
    title: &str,
    tooltip: &str,
    ctx: &mut BuildContext,
    column: usize,
    row: usize,
) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .on_column(column)
            .on_row(row)
            .with_height(24.0)
            .with_margin(Thickness::uniform(1.0))
            .with_tooltip(make_simple_tooltip(ctx, tooltip)),
    )
    .with_text(title)
    .build(ctx)
}

pub fn make_list_item(
    ctx: &mut BuildContext,
    instance_name: &str,
    macro_name: Option<&str>,
) -> Handle<UiNode> {
    let macro_name = macro_name.unwrap_or(MISSING_MACRO);
    let content = GridBuilder::new(
        WidgetBuilder::new()
            .with_child(
                TextBuilder::new(
                    WidgetBuilder::new()
                        .with_margin(Thickness::right(5.0))
                        .on_column(0),
                )
                .with_vertical_text_alignment(VerticalAlignment::Center)
                .with_horizontal_text_alignment(HorizontalAlignment::Right)
                .with_text(macro_name)
                .build(ctx),
            )
            .with_child(
                TextBuilder::new(
                    WidgetBuilder::new()
                        .with_margin(Thickness::left(5.0))
                        .on_column(1),
                )
                .with_vertical_text_alignment(VerticalAlignment::Center)
                .with_horizontal_text_alignment(HorizontalAlignment::Left)
                .with_text(instance_name)
                .build(ctx),
            ),
    )
    .add_row(Row::auto())
    .add_column(Column::stretch())
    .add_column(Column::stretch())
    .build(ctx);
    DecoratorBuilder::new(
        BorderBuilder::new(WidgetBuilder::new().with_child(content))
            .with_corner_radius(4.0.into())
            .with_pad_by_corner_radius(false),
    )
    .build(ctx)
}

fn make_instance_items(
    ctx: &mut BuildContext,
    brush: TileMapBrushResource,
    macros: &BrushMacroList,
) -> Vec<Handle<UiNode>> {
    brush
        .data_ref()
        .macros
        .iter()
        .map(|d| {
            make_list_item(
                ctx,
                d.name.as_str(),
                macros.get_by_uuid(&d.macro_id).map(|m| m.name()),
            )
        })
        .collect()
}

fn make_add_button(title: &str, ctx: &mut BuildContext) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_height(24.0)
            .with_margin(Thickness::uniform(1.0)),
    )
    .with_text(title)
    .build(ctx)
}

fn make_add_buttons(ctx: &mut BuildContext, macros: &BrushMacroList) -> Box<[Handle<UiNode>]> {
    macros
        .iter()
        .map(|m| make_add_button(m.name(), ctx))
        .collect()
}

fn make_macro_instance_editor(
    ctx: &mut BuildContext,
    brush: TileMapBrushResource,
    macros: &mut BrushMacroList,
    instance: Option<BrushMacroData>,
) -> Option<Handle<UiNode>> {
    let instance = instance?;
    let brush_macro = macros.get_by_uuid_mut(&instance.macro_id)?;
    brush_macro.build_instance_editor(
        &BrushMacroInstance {
            brush,
            settings: instance.settings.clone(),
        },
        ctx,
    )
}

impl MacroTab {
    pub fn new(macros: BrushMacroListRef, tile_book: TileBook, ctx: &mut BuildContext) -> Self {
        let macros_guard = macros.lock();
        let items;
        if let TileBook::Brush(brush) = &tile_book {
            items = make_instance_items(ctx, brush.clone(), &macros_guard);
        } else {
            items = Vec::default();
        }
        let add_buttons = make_add_buttons(ctx, &macros_guard);
        let properties_scroll = ScrollViewerBuilder::new(WidgetBuilder::new()).build(ctx);
        let list = ListViewBuilder::new(WidgetBuilder::new().on_row(1))
            .with_items(items)
            .with_scroll_viewer(properties_scroll)
            .build(ctx);
        let up_button = make_arrow_button(ctx, ArrowDirection::Top, 0, 0);
        let down_button = make_arrow_button(ctx, ArrowDirection::Bottom, 1, 0);
        let up_down = GridBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .on_row(2)
                .with_child(up_button)
                .with_child(down_button),
        )
        .add_row(Row::auto())
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .build(ctx);
        let add_button_panel = ScrollViewerBuilder::new(
            WidgetBuilder::new()
                .with_max_size(Vector2::new(f32::INFINITY, 100.0))
                .with_margin(Thickness::uniform(2.0))
                .on_row(3),
        )
        .with_horizontal_scroll_allowed(false)
        .with_content(
            StackPanelBuilder::new(WidgetBuilder::new().with_children(add_buttons.iter().copied()))
                .build(ctx),
        )
        .build(ctx);
        let left_label = TextBuilder::new(
            WidgetBuilder::new()
                .with_horizontal_alignment(HorizontalAlignment::Center)
                .with_margin(Thickness::uniform(2.0)),
        )
        .with_text("Macros:")
        .build(ctx);
        let left_side = GridBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .with_child(left_label)
                .with_child(list)
                .with_child(up_down)
                .with_child(add_button_panel),
        )
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_row(Row::strict(30.0))
        .add_row(Row::auto())
        .add_column(Column::stretch())
        .build(ctx);
        let name_text = TextBuilder::new(
            WidgetBuilder::new()
                .with_vertical_alignment(VerticalAlignment::Center)
                .with_margin(Thickness::right(4.0)),
        )
        .with_text("Name:")
        .build(ctx);
        let name_field = TextBoxBuilder::new(WidgetBuilder::new().with_height(20.0).on_column(1))
            .with_text_commit_mode(TextCommitMode::Changed)
            .with_vertical_text_alignment(VerticalAlignment::Center)
            .build(ctx);
        let remove_button = make_button(
            "Delete",
            "Delete this collider from every tile in the tile set.",
            ctx,
            2,
            0,
        );
        let header = GridBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .with_child(name_text)
                .with_child(name_field)
                .with_child(remove_button),
        )
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_column(Column::strict(100.0))
        .add_row(Row::auto())
        .build(ctx);
        let macro_panel =
            BorderBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(2.0)))
                .with_stroke_thickness(Thickness::uniform(1.0).into())
                .build(ctx);
        let data_panel = GridBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .on_column(1)
                .with_child(header)
                .with_child(
                    ScrollViewerBuilder::new(WidgetBuilder::new().on_row(1))
                        .with_content(macro_panel)
                        .with_horizontal_scroll_allowed(false)
                        .build(ctx),
                ),
        )
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);
        drop(macros_guard);
        Self {
            handle: GridBuilder::new(
                WidgetBuilder::new()
                    .with_margin(Thickness::uniform(2.0))
                    .with_child(left_side)
                    .with_child(data_panel),
            )
            .add_row(Row::stretch())
            .add_column(Column::stretch())
            .add_column(Column::stretch())
            .build(ctx),
            macros,
            current_macro_id: None,
            macro_panel,
            list,
            add_buttons,
            up_button,
            down_button,
            remove_button,
            data_panel,
            name_field,
        }
    }
    pub fn handle(&self) -> Handle<UiNode> {
        self.handle
    }
    pub fn sync_to_model(&mut self, brush: TileMapBrushResource, ui: &mut UserInterface) {
        let items = make_instance_items(&mut ui.build_ctx(), brush.clone(), &self.macros.lock());
        ui.send_message(ListViewMessage::items(
            self.list,
            MessageDirection::ToWidget,
            items,
        ));
        self.sync_data(brush, ui);
    }
    fn sync_data(&mut self, brush: TileMapBrushResource, ui: &mut UserInterface) {
        let sel_index = self.selection_index(ui);
        let brush_guard = brush.data_ref();
        let brush_macro = sel_index.and_then(|i| brush_guard.macros.get(i));
        let name = brush_macro.map(|m| m.name.clone()).unwrap_or_default();
        let macro_id = brush_macro.map(|m| m.macro_id);
        ui.send_message(WidgetMessage::enabled(
            self.data_panel,
            MessageDirection::ToWidget,
            brush_macro.is_some(),
        ));
        send_sync_message(
            ui,
            TextMessage::text(self.name_field, MessageDirection::ToWidget, name),
        );
        if macro_id == self.current_macro_id {
            if let Some(brush_macro) = brush_macro {
                let macro_id = brush_macro.macro_id;
                let settings = brush_macro.settings.clone();
                drop(brush_guard);
                let mut macro_list = self.macros.lock();
                if let Some(m) = macro_list.get_by_uuid_mut(&macro_id) {
                    m.sync_instance_editor(
                        &BrushMacroInstance {
                            brush: brush.clone(),
                            settings,
                        },
                        ui,
                    );
                }
            }
        } else {
            let instance = brush_macro.cloned();
            drop(brush_guard);
            self.current_macro_id = macro_id;
            let mut macro_list = self.macros.lock();
            let editor = make_macro_instance_editor(
                &mut ui.build_ctx(),
                brush.clone(),
                &mut macro_list,
                instance,
            );
            ui.send_message(WidgetMessage::replace_children(
                self.macro_panel,
                MessageDirection::ToWidget,
                editor.into_iter().collect(),
            ));
        }
    }
    pub fn handle_ui_message(
        &mut self,
        brush: &TileMapBrushResource,
        message: &UiMessage,
        editor: &mut Editor,
    ) {
        if message.direction() == MessageDirection::ToWidget || message.flags == MSG_SYNC_FLAG {
            return;
        }
        if let Some(sel_index) = self.selection_index(editor.engine.user_interfaces.first_mut()) {
            let brush_guard = brush.data_ref();
            let instance = brush_guard.macros.get(sel_index);
            if let Some(instance) = instance {
                let mut macro_list = self.macros.lock();
                if let Some(m) = macro_list.get_by_uuid_mut(&instance.macro_id) {
                    let settings = instance.settings.clone();
                    drop(brush_guard);
                    m.on_instance_ui_message(
                        &BrushMacroInstance {
                            brush: brush.clone(),
                            settings,
                        },
                        message,
                        editor,
                    );
                }
            }
        }
        let ui = editor.engine.user_interfaces.first_mut();
        let sender = &editor.message_sender;
        if let Some(ListViewMessage::SelectionChanged(_)) = message.data() {
            if message.destination() == self.list {
                self.sync_data(brush.clone(), ui);
            }
        } else if let Some(TextMessage::Text(value)) = message.data() {
            if message.destination() == self.name_field {
                self.update_name(brush, value, ui, sender);
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.up_button {
                self.move_instance(brush, -1, ui, sender);
            } else if message.destination() == self.down_button {
                self.move_instance(brush, 1, ui, sender);
            } else if message.destination() == self.remove_button {
                self.remove_instance(brush, ui, sender);
            } else if let Some(index) = self
                .add_buttons
                .iter()
                .position(|&h| h == message.destination())
            {
                self.add_instance(brush, index, ui, sender);
            }
        }
    }
    fn selection_index(&self, ui: &UserInterface) -> Option<usize> {
        ui.node(self.list)
            .cast::<ListView>()?
            .selection
            .last()
            .copied()
    }
    fn update_name(
        &self,
        resource: &TileMapBrushResource,
        name: &str,
        ui: &UserInterface,
        sender: &MessageSender,
    ) {
        let brush = resource.data_ref();
        let macros = &brush.macros;
        let Some(sel_index) = self
            .selection_index(ui)
            .map(|i| i.clamp(0, macros.len().saturating_sub(1)))
        else {
            return;
        };
        ui.send_message(WidgetMessage::focus(
            self.name_field,
            MessageDirection::ToWidget,
        ));
        sender.do_command(SetMacroNameCommand {
            brush: resource.clone(),
            index: sel_index,
            name: name.into(),
        });
    }
    fn move_instance(
        &self,
        resource: &TileMapBrushResource,
        amount: isize,
        ui: &UserInterface,
        sender: &MessageSender,
    ) {
        let brush = resource.data_ref();
        let Some(sel_index) = self.selection_index(ui) else {
            return;
        };
        let new_index = sel_index
            .saturating_add_signed(amount)
            .clamp(0, brush.macros.len().saturating_sub(1));
        if sel_index == new_index {
            return;
        }
        ui.send_message(ListViewMessage::selection(
            self.list,
            MessageDirection::ToWidget,
            vec![new_index],
        ));
        sender.do_command(MoveMacroCommand {
            brush: resource.clone(),
            start: sel_index,
            end: new_index,
        });
    }
    fn add_instance(
        &self,
        resource: &TileMapBrushResource,
        macro_index: usize,
        ui: &UserInterface,
        sender: &MessageSender,
    ) {
        let brush = resource.data_ref();
        let index = self
            .selection_index(ui)
            .map(|i| i + 1)
            .unwrap_or(0)
            .clamp(0, brush.macros.len());
        let macros = self.macros.lock();
        let Some(brush_macro) = macros.get_by_index(macro_index) else {
            return;
        };
        let data = brush_macro.create_instance(resource);
        let data = Some(BrushMacroData {
            macro_id: *brush_macro.uuid(),
            name: String::default(),
            settings: data,
        });
        ui.send_message(ListViewMessage::selection(
            self.list,
            MessageDirection::ToWidget,
            vec![index],
        ));
        sender.do_command(AddMacroCommand {
            brush: resource.clone(),
            index,
            data,
        });
    }
    fn remove_instance(
        &self,
        resource: &TileMapBrushResource,
        ui: &UserInterface,
        sender: &MessageSender,
    ) {
        let brush = resource.data_ref();
        let Some(index) = self
            .selection_index(ui)
            .map(|i| i.clamp(0, brush.macros.len().saturating_sub(1)))
        else {
            return;
        };
        sender.do_command(RemoveMacroCommand {
            brush: resource.clone(),
            index,
            data: None,
        });
    }
}
