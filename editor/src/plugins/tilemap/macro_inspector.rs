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

use fyrox::{
    asset::untyped::UntypedResource,
    gui::{
        button::{Button, ButtonMessage},
        grid::{Column, GridBuilder, Row},
        stack_panel::StackPanelBuilder,
        text::{TextBuilder, TextMessage},
        VerticalAlignment,
    },
    scene::tilemap::brush::TileMapBrushResource,
};

use super::*;

const ADD_BUTTON_LABEL: &str = "+";
const REMOVE_BUTTON_LABEL: &str = "-";
const ADD_TOOLTIP: &str = "Add the selected cell to this macro.";
const REMOVE_TOOLTIP: &str = "Remove the selected cell from this macro.";
const CELL_WITH_MACRO_COLOR: Color = Color::DARK_SLATE_BLUE;
const CELL_WITHOUT_MACRO_COLOR: Color = Color::opaque(50, 50, 50);

#[derive(Visit, Reflect)]
pub struct MacroInspector {
    handle: Handle<UiNode>,
    content: Handle<UiNode>,
    #[visit(skip)]
    #[reflect(hidden)]
    instance_list: Vec<Option<UntypedResource>>,
    #[visit(skip)]
    #[reflect(hidden)]
    macro_list: BrushMacroListRef,
    #[visit(skip)]
    #[reflect(hidden)]
    cell_sets: MacroCellSetListRef,
    #[visit(skip)]
    #[reflect(hidden)]
    items: Vec<Item>,
}

impl Debug for MacroInspector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MacroInspector")
            .field("handle", &self.handle)
            .field("content", &self.content)
            .finish()
    }
}

struct Item {
    macro_id: Uuid,
    editor: Option<ItemEditor>,
}

struct ItemEditor {
    handle: Handle<UiNode>,
    header: ItemHeader,
}

struct ItemHeader {
    handle: Handle<UiNode>,
    label: Handle<UiNode>,
    button: Handle<UiNode>,
}

impl ItemEditor {
    fn new(header: ItemHeader, content: Handle<UiNode>, ctx: &mut BuildContext) -> Self {
        let handle = if content.is_some() {
            StackPanelBuilder::new(
                WidgetBuilder::new()
                    .with_child(header.handle)
                    .with_child(content),
            )
            .build(ctx)
        } else {
            header.handle
        };
        Self { handle, header }
    }
    fn button(&self) -> Handle<UiNode> {
        self.header.button
    }
}

fn make_button(title: &str, tooltip: &str, ctx: &mut BuildContext) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .on_column(1)
            .with_height(24.0)
            .with_margin(Thickness::uniform(1.0))
            .with_tooltip(make_simple_tooltip(ctx, tooltip)),
    )
    .with_text(title)
    .build(ctx)
}

fn make_add_button(ctx: &mut BuildContext) -> Handle<UiNode> {
    make_button(ADD_BUTTON_LABEL, ADD_TOOLTIP, ctx)
}

fn make_remove_button(ctx: &mut BuildContext) -> Handle<UiNode> {
    make_button(REMOVE_BUTTON_LABEL, REMOVE_TOOLTIP, ctx)
}

impl ItemHeader {
    fn new(name: String, has_cell: bool, ctx: &mut BuildContext) -> Self {
        let label = TextBuilder::new(
            WidgetBuilder::new().with_vertical_alignment(VerticalAlignment::Center),
        )
        .with_vertical_text_alignment(VerticalAlignment::Center)
        .with_text(name)
        .build(ctx);
        let button = if has_cell {
            make_remove_button(ctx)
        } else {
            make_add_button(ctx)
        };
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(label)
                .with_child(button)
                .with_margin(Thickness::uniform(2.0)),
        )
        .add_row(Row::auto())
        .add_column(Column::stretch())
        .add_column(Column::strict(20.0))
        .build(ctx);
        let back = Brush::Solid(if has_cell {
            CELL_WITH_MACRO_COLOR
        } else {
            CELL_WITHOUT_MACRO_COLOR
        });
        Self {
            handle: BorderBuilder::new(
                WidgetBuilder::new()
                    .with_background(back.into())
                    .with_foreground(Brush::Solid(Color::BLACK).into())
                    .with_child(grid),
            )
            .build(ctx),
            label,
            button,
        }
    }
    fn sync(&self, name: String, has_cell: bool, ui: &mut UserInterface) {
        ui.send_message(TextMessage::text(
            self.label,
            MessageDirection::ToWidget,
            name,
        ));
        let button_text = if has_cell {
            REMOVE_BUTTON_LABEL
        } else {
            ADD_BUTTON_LABEL
        };
        let button = ui.node(self.button).cast::<Button>().unwrap();
        ui.send_message(TextMessage::text(
            *button.content,
            MessageDirection::ToWidget,
            button_text.into(),
        ));
        let tooltip = if has_cell {
            REMOVE_TOOLTIP
        } else {
            ADD_TOOLTIP
        };
        let tooltip = make_simple_tooltip(&mut ui.build_ctx(), tooltip);
        ui.send_message(WidgetMessage::tooltip(
            self.button,
            MessageDirection::ToWidget,
            Some(tooltip),
        ));
        let color = if has_cell {
            CELL_WITH_MACRO_COLOR
        } else {
            CELL_WITHOUT_MACRO_COLOR
        };
        ui.send_message(WidgetMessage::background(
            self.handle,
            MessageDirection::ToWidget,
            Brush::Solid(color).into(),
        ));
    }
}

fn make_items(
    macro_list: &mut BrushMacroList,
    cell_sets: &MacroCellSetList,
    brush: Option<TileMapBrushResource>,
    cell: Option<TileDefinitionHandle>,
    ctx: &mut BuildContext,
    items: &mut Vec<Item>,
) {
    items.clear();
    let Some(brush) = brush else {
        return;
    };
    let mut brush_macro_list = brush.data_ref().macros.clone();
    for (cell_set, instance) in cell_sets.iter().zip(brush_macro_list.iter_mut()) {
        let macro_id = instance.macro_id;
        let Some(m) = macro_list.get_by_uuid_mut(&macro_id) else {
            items.push(Item {
                macro_id,
                editor: None,
            });
            continue;
        };
        if !m.can_create_cell() {
            items.push(Item {
                macro_id,
                editor: None,
            });
            continue;
        }
        let brush_macro_cell = BrushMacroCellContext {
            brush: brush.clone(),
            settings: instance.settings.clone(),
            cell,
        };
        let has_cell = cell
            .as_ref()
            .map(|c| cell_set.contains(c))
            .unwrap_or_default();
        let header = ItemHeader::new(std::mem::take(&mut instance.name), has_cell, ctx);
        let content = m
            .build_cell_editor(&brush_macro_cell, ctx)
            .unwrap_or_default();
        items.push(Item {
            macro_id,
            editor: Some(ItemEditor::new(header, content, ctx)),
        });
    }
}

fn fill_instance_list(
    brush: Option<&TileMapBrushResource>,
    instance_list: &mut Vec<Option<UntypedResource>>,
) {
    instance_list.clear();
    let Some(brush) = brush else {
        return;
    };
    instance_list.extend(
        brush
            .data_ref()
            .macros
            .iter()
            .map(|inst| inst.settings.clone()),
    );
}

impl MacroInspector {
    pub fn new(
        macro_list: BrushMacroListRef,
        cell_sets: MacroCellSetListRef,
        brush: Option<TileMapBrushResource>,
        cell: Option<TileDefinitionHandle>,
        ctx: &mut BuildContext,
    ) -> Self {
        let mut instance_list = Vec::default();
        fill_instance_list(brush.as_ref(), &mut instance_list);
        let mut items = Vec::default();
        make_items(
            &mut macro_list.lock(),
            &cell_sets.lock(),
            brush,
            cell,
            ctx,
            &mut items,
        );
        let content = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .with_children(
                    items
                        .iter()
                        .filter_map(|item| item.editor.as_ref())
                        .map(|editor| editor.handle),
                ),
        )
        .build(ctx);
        Self {
            handle: BorderBuilder::new(WidgetBuilder::new().with_child(content)).build(ctx),
            content,
            macro_list,
            cell_sets,
            items,
            instance_list,
        }
    }
    pub fn handle(&self) -> Handle<UiNode> {
        self.handle
    }
    pub fn sync_to_cell(
        &mut self,
        brush: Option<TileMapBrushResource>,
        cell: Option<TileDefinitionHandle>,
        ui: &mut UserInterface,
    ) {
        let Some(brush) = brush else {
            return;
        };
        let brush_guard = brush.data_ref();
        let brush_instances = brush_guard.macros.iter().map(|m| m.settings.as_ref());
        let inspector_instances = self.instance_list.iter().map(|inst| inst.as_ref());
        let needs_rebuild = !brush_instances.eq(inspector_instances);
        drop(brush_guard);
        if !needs_rebuild {
            self.sync_to_cell_inner(brush, cell, ui);
        } else {
            fill_instance_list(Some(&brush), &mut self.instance_list);
            make_items(
                &mut self.macro_list.lock(),
                &self.cell_sets.lock(),
                Some(brush),
                cell,
                &mut ui.build_ctx(),
                &mut self.items,
            );
            ui.send_message(WidgetMessage::replace_children(
                self.content,
                MessageDirection::ToWidget,
                self.items
                    .iter()
                    .filter_map(|item| item.editor.as_ref())
                    .map(|e| e.handle)
                    .collect(),
            ));
        }
    }
    fn sync_to_cell_inner(
        &self,
        brush: TileMapBrushResource,
        cell: Option<TileDefinitionHandle>,
        ui: &mut UserInterface,
    ) {
        let context = MacroMessageContext {
            brush: brush.clone(),
            cell,
        };
        for m in self.macro_list.lock().iter_mut() {
            m.sync_cell_editors(&context, ui);
        }
        let cell_sets = self.cell_sets.lock();
        let brush_guard = brush.data_ref();
        for (i, item) in self.items.iter().enumerate() {
            let Some(instance) = brush_guard.macros.get(i) else {
                continue;
            };
            let Some(editor) = item.editor.as_ref() else {
                continue;
            };
            let has_cell = cell
                .map(|c| cell_sets.cell_has_macro(c, i))
                .unwrap_or_default();
            editor.header.sync(instance.name.clone(), has_cell, ui);
        }
    }
    pub fn handle_ui_message(
        &mut self,
        brush: TileMapBrushResource,
        cell: Option<TileDefinitionHandle>,
        message: &UiMessage,
        editor: &mut Editor,
    ) {
        let context = MacroMessageContext {
            brush: brush.clone(),
            cell,
        };
        let mut macro_list = self.macro_list.lock();
        for brush_macro in macro_list.iter_mut() {
            brush_macro.on_cell_ui_message(&context, message, editor);
        }
        let Some(cell) = cell else {
            return;
        };
        if let Some(ButtonMessage::Click) = message.data() {
            let cell_sets = self.cell_sets.lock();
            for (i, item) in self.items.iter().enumerate() {
                let Some(cell_editor) = item.editor.as_ref() else {
                    continue;
                };
                if cell_editor.button() == message.destination() {
                    let Some(brush_macro) = macro_list.get_by_uuid_mut(&item.macro_id) else {
                        continue;
                    };
                    let brush_guard = brush.data_ref();
                    let Some(instance) = brush_guard.macros.get(i) else {
                        continue;
                    };
                    let settings = instance.settings.clone();
                    drop(brush_guard);
                    let command = if cell_sets.cell_has_macro(cell, i) {
                        brush_macro.copy_cell(None, cell, &BrushMacroInstance { brush, settings })
                    } else {
                        let cell = Some(cell);
                        brush_macro.create_cell(&BrushMacroCellContext {
                            brush,
                            settings,
                            cell,
                        })
                    };
                    if let Some(command) = command {
                        editor.message_sender.send(Message::DoCommand(command));
                    }
                    break;
                }
            }
        }
    }
}
