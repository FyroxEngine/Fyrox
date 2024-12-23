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
    command::{Command, CommandGroup},
    fyrox::{
        asset::manager::ResourceManager,
        core::{color::Color, log::Log, pool::Handle},
        engine::SerializationContext,
        gui::{
            border::BorderBuilder,
            brush::Brush,
            button::ButtonMessage,
            grid::{Column, GridBuilder, Row},
            message::{MessageDirection, UiMessage},
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, Thickness, UiNode, UserInterface,
        },
        scene::tilemap::TileDefinitionHandle,
    },
    message::MessageSender,
    plugins::inspector::editors::resource::{ResourceFieldBuilder, ResourceFieldMessage},
};
use fyrox::gui::text::TextBuilder;
use fyrox::{
    core::algebra::Vector2,
    gui::{
        color::{ColorFieldBuilder, ColorFieldMessage},
        grid::SizeMode,
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        tab_control::{TabControl, TabControlBuilder, TabControlMessage, TabDefinition},
        text::TextMessage,
    },
    scene::tilemap::{tileset::TileSetRef, TileResource},
};
use palette::{PaletteWidgetBuilder, DEFAULT_MATERIAL_COLOR};
use std::sync::Arc;

use super::*;
use commands::*;

const TAB_MARGIN: Thickness = Thickness {
    left: 10.0,
    top: 2.0,
    right: 10.0,
    bottom: 2.0,
};

const DEFAULT_PAGE: Vector2<i32> = Vector2::new(0, 0);

pub struct TileSetEditor {
    pub window: Handle<UiNode>,
    state: TileDrawStateRef,
    tile_resource: TileResource,
    color_field: Handle<UiNode>,
    cell_position: Handle<UiNode>,
    tab_control: Handle<UiNode>,
    pages_palette: Handle<UiNode>,
    tiles_palette: Handle<UiNode>,
    open_control: Handle<UiNode>,
    remove: Handle<UiNode>,
    all_pages: Handle<UiNode>,
    all_tiles: Handle<UiNode>,
    tile_set_selector: Handle<UiNode>,
    tile_set_field: Handle<UiNode>,
    tile_inspector: TileInspector,
    properties_tab: PropertiesTab,
    colliders_tab: CollidersTab,
}

fn make_tab(name: &str, content: Handle<UiNode>, ctx: &mut BuildContext) -> TabDefinition {
    TabDefinition {
        header: TextBuilder::new(WidgetBuilder::new().with_margin(TAB_MARGIN))
            .with_text(name)
            .build(ctx),
        content,
        can_be_closed: false,
        user_data: None,
    }
}

fn make_button(
    title: &str,
    tooltip: &str,
    row: usize,
    column: usize,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .on_row(row)
            .on_column(column)
            .with_height(24.0)
            .with_margin(Thickness::uniform(1.0))
            .with_tooltip(make_simple_tooltip(ctx, tooltip)),
    )
    .with_text(title)
    .build(ctx)
}

fn make_label(name: &str, ctx: &mut BuildContext) -> Handle<UiNode> {
    TextBuilder::new(WidgetBuilder::new())
        .with_text(name)
        .build(ctx)
}

fn tile_set_to_title(tile_resource: &TileResource) -> String {
    match tile_resource {
        TileResource::Empty => "Missing Resource".into(),
        TileResource::TileSet(_) => {
            let mut result = String::new();
            result.push_str("Tile Set: ");
            result.push_str(
                tile_resource
                    .path()
                    .map(|x| x.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Error".into())
                    .as_ref(),
            );
            result
        }
        TileResource::Brush(_) => {
            let mut result = String::new();
            result.push_str("Tile Map Brush: ");
            result.push_str(
                tile_resource
                    .path()
                    .map(|x| x.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Error".into())
                    .as_ref(),
            );
            result
        }
    }
}

impl TileSetEditor {
    pub fn new(
        tile_resource: TileResource,
        state: TileDrawStateRef,
        sender: MessageSender,
        resource_manager: ResourceManager,
        ctx: &mut BuildContext,
    ) -> Self {
        let tile_set_field =
            ResourceFieldBuilder::<TileSet>::new(WidgetBuilder::new().on_column(1), sender.clone())
                .with_resource(if tile_resource.is_brush() {
                    tile_resource.get_tile_set()
                } else {
                    None
                })
                .build(ctx, resource_manager.clone());
        let tile_set_selector = GridBuilder::new(
            WidgetBuilder::new()
                .with_visibility(tile_resource.is_brush())
                .with_child(make_label("Tile Set", ctx))
                .with_child(tile_set_field),
        )
        .add_column(Column::strict(FIELD_LABEL_WIDTH))
        .add_column(Column::stretch())
        .add_row(Row::auto())
        .build(ctx);
        let remove;
        let all_pages;
        let all_tiles;
        let cell_position = TextBuilder::new(WidgetBuilder::new()).build(ctx);
        let open_control = make_button(
            "Palette",
            "Open the tile palette control window.",
            0,
            0,
            ctx,
        );
        let page_buttons = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(open_control)
                .with_child({
                    all_pages = make_button("All Pages", "Select all pages.", 0, 1, ctx);
                    all_pages
                })
                .with_child({
                    all_tiles = make_button("All Tiles", "Select all tiles.", 0, 2, ctx);
                    all_tiles
                })
                .with_child({
                    remove = make_button("Delete", "Remove selected tile.", 0, 3, ctx);
                    remove
                }),
        )
        .add_row(Row::auto())
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let color_label = make_label("Material Tint", ctx);
        let color_field = ColorFieldBuilder::new(WidgetBuilder::new().on_column(1))
            .with_color(DEFAULT_MATERIAL_COLOR)
            .build(ctx);
        let color_control = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(color_label)
                .with_child(color_field),
        )
        .add_row(Row::auto())
        .add_column(Column::strict(FIELD_LABEL_WIDTH))
        .add_column(Column::stretch())
        .build(ctx);

        let pages_palette = PaletteWidgetBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .with_allow_drop(true),
            sender.clone(),
            state.clone(),
        )
        .with_page(DEFAULT_PAGE)
        .with_resource(tile_resource.clone())
        .with_kind(TilePaletteStage::Pages)
        .with_editable(true)
        .build(ctx);

        let tiles_palette = PaletteWidgetBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .with_allow_drop(true),
            sender.clone(),
            state.clone(),
        )
        .with_page(DEFAULT_PAGE)
        .with_resource(tile_resource.clone())
        .with_kind(TilePaletteStage::Tiles)
        .with_editable(true)
        .build(ctx);

        let tile_inspector = TileInspector::new(
            state.clone(),
            pages_palette,
            tiles_palette,
            tile_resource.clone(),
            resource_manager,
            sender,
            ctx,
        );

        let tile_panel = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    BorderBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(2.0))
                            .on_row(0)
                            .with_child(pages_palette),
                    )
                    .build(ctx),
                )
                .with_child(
                    BorderBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(2.0))
                            .on_row(1)
                            .with_child(tiles_palette),
                    )
                    .build(ctx),
                ),
        )
        .add_row(Row::stretch())
        .add_row(Row::generic(SizeMode::Stretch, 200.0))
        .add_column(Column::stretch())
        .build(ctx);

        let inspector_scroll = ScrollViewerBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness {
                    top: 10.0,
                    bottom: 1.0,
                    left: 2.0,
                    right: 2.0,
                })
                .on_row(1),
        )
        .with_content(tile_inspector.handle())
        .build(ctx);

        let header_fields = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(cell_position)
                .with_child(tile_set_selector)
                .with_child(page_buttons)
                .with_child(color_control),
        )
        .build(ctx);

        let side_panel = BorderBuilder::new(
            WidgetBuilder::new()
                .with_foreground(Brush::Solid(Color::BLACK).into())
                .on_column(1)
                .with_margin(Thickness::uniform(4.0))
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(2.0))
                            .with_child(header_fields)
                            .with_child(inspector_scroll),
                    )
                    .add_row(Row::auto())
                    .add_row(Row::stretch())
                    .add_column(Column::strict(400.0))
                    .build(ctx),
                ),
        )
        .build(ctx);

        let tile_tab = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(tile_panel)
                .with_child(side_panel),
        )
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .add_row(Row::stretch())
        .build(ctx);

        let properties_tab = PropertiesTab::new(tile_resource.clone(), ctx);
        let colliders_tab = CollidersTab::new(tile_resource.clone(), ctx);
        let tab_control = TabControlBuilder::new(WidgetBuilder::new())
            .with_tab(make_tab("Tiles", tile_tab, ctx))
            .with_tab(make_tab("Properties", properties_tab.handle(), ctx))
            .with_tab(make_tab("Collision", colliders_tab.handle(), ctx))
            .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(800.0).with_height(600.0))
            .open(false)
            .with_title(WindowTitle::text(tile_set_to_title(&tile_resource)))
            .with_content(tab_control)
            .build(ctx);

        ctx.sender()
            .send(WindowMessage::open(
                window,
                MessageDirection::ToWidget,
                true,
                true,
            ))
            .unwrap();

        let mut editor = Self {
            window,
            properties_tab,
            colliders_tab,
            state,
            tab_control,
            color_field,
            cell_position,
            pages_palette,
            tiles_palette,
            tile_resource,
            open_control,
            remove,
            all_pages,
            all_tiles,
            tile_inspector,
            tile_set_selector,
            tile_set_field,
        };

        editor.sync_to_model(ctx.inner_mut());

        editor
    }

    pub fn set_tile_resource(&mut self, tile_resource: TileResource, ui: &mut UserInterface) {
        self.try_save();
        self.tile_resource = tile_resource.clone();
        self.tile_inspector
            .set_tile_resource(tile_resource.clone(), ui);
        ui.send_message(WindowMessage::title(
            self.window,
            MessageDirection::ToWidget,
            WindowTitle::text(tile_set_to_title(&tile_resource)),
        ));
        let mut state = self.state.lock_mut("set_tile_resource");
        if state.selection_palette() == self.pages_palette
            || state.selection_palette() == self.tiles_palette
        {
            state.clear_selection();
        }
        drop(state);
        if let TileResource::Brush(brush) = &tile_resource {
            ui.send_message(ResourceFieldMessage::value(
                self.tile_set_field,
                MessageDirection::ToWidget,
                brush.data_ref().tile_set.clone(),
            ));
        }
        self.send_tabs_visible(tile_resource.is_tile_set(), ui);
        ui.send_message(WidgetMessage::visibility(
            self.tile_set_selector,
            MessageDirection::ToWidget,
            tile_resource.is_brush(),
        ));
        ui.send_message(TabControlMessage::active_tab(
            self.tab_control,
            MessageDirection::ToWidget,
            Some(0),
        ));
        ui.send_message(PaletteMessage::set_page(
            self.pages_palette,
            MessageDirection::ToWidget,
            tile_resource.clone(),
            Some(Vector2::new(0, 0)),
        ));
        ui.send_message(PaletteMessage::set_page(
            self.tiles_palette,
            MessageDirection::ToWidget,
            tile_resource,
            Some(Vector2::new(0, 0)),
        ));
    }

    fn send_tabs_visible(&self, visibility: bool, ui: &mut UserInterface) {
        let tab_control = ui.node(self.tab_control).cast::<TabControl>().unwrap();
        let tabs = tab_control.headers_container;
        ui.send_message(WidgetMessage::visibility(
            tabs,
            MessageDirection::ToWidget,
            visibility,
        ));
    }

    pub fn set_position(&self, handle: TileDefinitionHandle, ui: &mut UserInterface) {
        ui.send_message(PaletteMessage::set_page(
            self.pages_palette,
            MessageDirection::ToWidget,
            self.tile_resource.clone(),
            Some(handle.page()),
        ));
        ui.send_message(PaletteMessage::set_page(
            self.tiles_palette,
            MessageDirection::ToWidget,
            self.tile_resource.clone(),
            Some(handle.page()),
        ));
        ui.send_message(PaletteMessage::center(
            self.pages_palette,
            MessageDirection::ToWidget,
            handle.page(),
        ));
        ui.send_message(PaletteMessage::center(
            self.tiles_palette,
            MessageDirection::ToWidget,
            handle.tile(),
        ));
        ui.send_message(PaletteMessage::select_one(
            self.tiles_palette,
            MessageDirection::ToWidget,
            handle.tile(),
        ));
    }

    fn destroy(self, ui: &UserInterface) {
        ui.send_message(WidgetMessage::remove(
            self.window,
            MessageDirection::ToWidget,
        ));
    }

    fn cell_position(&self) -> String {
        let state = self.state.lock();
        if state.selection_palette() == self.pages_palette
            || state.selection_palette() == self.tiles_palette
        {
            state
                .selection_positions()
                .iter()
                .map(|p| format!("({},{})", p.x, p.y))
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            String::default()
        }
    }

    pub fn sync_to_state(&mut self, ui: &mut UserInterface) {
        self.tile_inspector.sync_to_state(ui);
        let cell_position = self.cell_position();
        ui.send_message(TextMessage::text(
            self.cell_position,
            MessageDirection::ToWidget,
            cell_position,
        ));
        ui.send_message(PaletteMessage::sync_to_state(
            self.pages_palette,
            MessageDirection::ToWidget,
        ));
        ui.send_message(PaletteMessage::sync_to_state(
            self.tiles_palette,
            MessageDirection::ToWidget,
        ));
    }

    pub fn sync_to_model(&mut self, ui: &mut UserInterface) {
        if let Some(r) = self.tile_resource.tile_set_ref() {
            let tile_set = TileSetRef::new(r);
            self.properties_tab.sync_to_model(&tile_set, ui);
            self.colliders_tab.sync_to_model(&tile_set, ui);
        }
        self.tile_inspector.sync_to_model(ui);
        if let TileResource::Brush(brush) = &self.tile_resource {
            let brush = brush.data_ref();
            let tile_set = brush.tile_set.clone();
            ui.send_message(ResourceFieldMessage::value(
                self.tile_set_field,
                MessageDirection::ToWidget,
                tile_set,
            ));
        }
    }

    pub fn try_save(&self) {
        Log::verify(self.tile_resource.save());
    }

    pub fn handle_ui_message(
        mut self,
        message: &UiMessage,
        ui: &mut UserInterface,
        _resource_manager: &ResourceManager,
        sender: &MessageSender,
        _serialization_context: Arc<SerializationContext>,
    ) -> Option<Self> {
        self.tile_inspector.handle_ui_message(message, ui, sender);
        if let Some(r) = self.tile_resource.tile_set_ref() {
            self.properties_tab
                .handle_ui_message(r.clone(), message, ui, sender);
            self.colliders_tab
                .handle_ui_message(r.clone(), message, ui, sender);
        }
        if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.window {
                self.try_save();
                self.destroy(ui);
                return None;
            }
        } else if let Some(ResourceFieldMessage::<TileSet>::Value(tile_set)) = message.data() {
            if message.destination() == self.tile_set_field
                && message.direction() == MessageDirection::FromWidget
            {
                if let TileResource::Brush(brush) = &self.tile_resource {
                    sender.do_command(ModifyBrushTileSetCommand {
                        brush: brush.clone(),
                        tile_set: tile_set.clone(),
                    });
                }
            }
        } else if let Some(TileHandleEditorMessage::Goto(handle)) = message.data() {
            if let Some(tile_set) = self.tile_resource.get_tile_set() {
                self.set_tile_resource(TileResource::TileSet(tile_set), ui);
                self.set_position(*handle, ui);
            }
        } else if let Some(TileHandleEditorMessage::OpenPalette(handle)) = message.data() {
            if let Some(tile_set) = self.tile_resource.get_tile_set() {
                ui.send_message(OpenTilePanelMessage::message(
                    TileResource::TileSet(tile_set),
                    Some(*handle),
                ));
            }
        } else if let Some(PaletteMessage::SetPage { .. }) = message.data() {
            if message.destination() == self.pages_palette
                && message.direction() == MessageDirection::FromWidget
            {
                ui.send_message(
                    message
                        .clone()
                        .with_destination(self.tiles_palette)
                        .with_direction(MessageDirection::ToWidget),
                );
                self.sync_to_state(ui);
            }
        } else if let Some(ColorFieldMessage::Color(color)) = message.data() {
            if message.destination() == self.color_field
                && message.direction() == MessageDirection::FromWidget
            {
                ui.send_message(PaletteMessage::material_color(
                    self.tiles_palette,
                    MessageDirection::ToWidget,
                    *color,
                ));
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.open_control {
                ui.send_message(OpenTilePanelMessage::message(
                    self.tile_resource.clone(),
                    None,
                ));
            } else if message.destination() == self.remove {
                self.do_delete_command(ui, sender);
            } else if message.destination() == self.all_tiles {
                ui.send_message(PaletteMessage::select_all(
                    self.tiles_palette,
                    MessageDirection::ToWidget,
                ));
            } else if message.destination() == self.all_pages {
                ui.send_message(PaletteMessage::select_all(
                    self.pages_palette,
                    MessageDirection::ToWidget,
                ));
            }
        }
        Some(self)
    }

    fn do_delete_command(&mut self, ui: &mut UserInterface, sender: &MessageSender) {
        let state = self.state.lock();
        let palette = state.selection_palette();
        if palette == self.pages_palette {
            let sel = state.selection_positions().clone();
            drop(state);
            let commands = sel
                .iter()
                .filter_map(|p| self.delete_page(*p))
                .collect::<Vec<_>>();
            sender.do_command(CommandGroup::from(commands).with_custom_name("Delete Pages"));
        } else if palette == self.tiles_palette {
            ui.send_message(PaletteMessage::delete(
                self.tiles_palette,
                MessageDirection::ToWidget,
            ));
        }
    }

    fn delete_page(&mut self, position: Vector2<i32>) -> Option<Command> {
        match &self.tile_resource {
            TileResource::Empty => None,
            TileResource::TileSet(tile_set) => Some(Command::new(SetTileSetPageCommand {
                tile_set: tile_set.clone(),
                position,
                page: None,
            })),
            TileResource::Brush(brush) => Some(Command::new(SetBrushPageCommand {
                brush: brush.clone(),
                position,
                page: None,
            })),
        }
    }

    // TODO: Is this needed?
    pub fn update(&mut self) {}
}
