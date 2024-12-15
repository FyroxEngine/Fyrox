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

use crate::plugins::inspector::EditorEnvironment;
use crate::{
    asset::item::AssetItem,
    command::{make_command, Command, CommandGroup, CommandTrait},
    fyrox::{
        asset::{manager::ResourceManager, untyped::ResourceKind, ResourceData},
        core::{
            color::Color, log::Log, math::Rect, pool::Handle, reflect::prelude::*,
            type_traits::prelude::*, visitor::prelude::*, Uuid,
        },
        engine::SerializationContext,
        graph::{BaseSceneGraph, SceneGraphNode},
        gui::{
            border::BorderBuilder,
            brush::Brush,
            button::ButtonMessage,
            decorator::DecoratorBuilder,
            define_widget_deref,
            grid::{Column, GridBuilder, Row},
            image::{ImageBuilder, ImageMessage},
            inspector::{
                editors::PropertyEditorDefinitionContainer, Inspector, InspectorBuilder,
                InspectorContext, InspectorMessage,
            },
            list_view::{ListView, ListViewBuilder, ListViewMessage},
            message::{MessageDirection, UiMessage},
            stack_panel::StackPanelBuilder,
            widget::{Widget, WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            wrap_panel::WrapPanelBuilder,
            BuildContext, Control, Orientation, Thickness, UiNode, UserInterface,
            VerticalAlignment,
        },
        material::{Material, MaterialResource},
        resource::texture::Texture,
        scene::tilemap::{
            tileset::{TileDefinition, TileSet, TileSetResource},
            TileDefinitionHandle,
        },
    },
    message::MessageSender,
    plugins::tilemap::tile_set_import::{ImportResult, TileSetImporter},
    Message,
};
use fyrox::{
    core::algebra::Vector2,
    gui::{
        color::{ColorFieldBuilder, ColorFieldMessage},
        grid::SizeMode,
        scroll_viewer::ScrollViewerBuilder,
        tab_control::{TabControlBuilder, TabDefinition},
        text::TextMessage,
        vec::Vec2EditorBuilder,
    },
    resource::texture::TextureKind,
    scene::tilemap::{
        brush::TileMapBrushResource, tileset::TileSetPageSource, TileDataUpdate, TileResource,
        TileSetUpdate,
    },
};
use fyrox::{graph::SceneGraph, gui::text::TextBuilder};
use palette::{PaletteWidget, PaletteWidgetBuilder, DEFAULT_MATERIAL_COLOR};
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use super::*;
use commands::*;

const TAB_MARGIN: Thickness = Thickness {
    left: 10.0,
    top: 2.0,
    right: 10.0,
    bottom: 2.0,
};

pub struct TileSetEditor {
    pub window: Handle<UiNode>,
    state: TileDrawStateRef,
    page: Option<Vector2<i32>>,
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

fn send_visibility(ui: &UserInterface, destination: Handle<UiNode>, visible: bool) {
    ui.send_message(WidgetMessage::visibility(
        destination,
        MessageDirection::ToWidget,
        visible,
    ));
}

fn tile_set_to_title(tile_set: &TileResource) -> String {
    match tile_set {
        TileResource::Empty => "Missing Resource".into(),
        TileResource::TileSet(_) => {
            let mut result = String::new();
            result.push_str("Tile Set: ");
            result.push_str(
                tile_set
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
                tile_set
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
        let state_guard = state.lock();
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
                .on_row(1)
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

        let color_label = TextBuilder::new(WidgetBuilder::new())
            .with_text("Material Tint")
            .build(ctx);
        let color_field = ColorFieldBuilder::new(WidgetBuilder::new().on_column(1))
            .with_color(DEFAULT_MATERIAL_COLOR)
            .build(ctx);
        let color_control = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(2)
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
                .on_row(3),
        )
        .with_content(tile_inspector.handle())
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
                            .with_child(cell_position)
                            .with_child(page_buttons)
                            .with_child(color_control)
                            .with_child(inspector_scroll),
                    )
                    .add_row(Row::auto())
                    .add_row(Row::auto())
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

        drop(state_guard);
        let mut editor = Self {
            window,
            page: None,
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
        };

        editor.sync_to_model(ctx.inner_mut());

        editor
    }

    pub fn set_tile_resource(&mut self, tile_set: TileResource, ui: &mut UserInterface) {
        ui.send_message(WindowMessage::title(
            self.window,
            MessageDirection::ToWidget,
            WindowTitle::text(tile_set_to_title(&tile_set)),
        ));
        let mut state = self.state.lock_mut();
        if state.selection_palette() == self.pages_palette
            || state.selection_palette() == self.tiles_palette
        {
            state.clear_selection();
        }
        drop(state);
        match &tile_set {
            TileResource::Empty => self.init_empty(ui),
            TileResource::TileSet(resource) => self.init_tile_set(resource, ui),
            TileResource::Brush(resource) => self.init_brush(resource, ui),
        }
        ui.send_message(PaletteMessage::set_page(
            self.pages_palette,
            MessageDirection::ToWidget,
            tile_set.clone(),
            None,
        ));
        ui.send_message(PaletteMessage::set_page(
            self.tiles_palette,
            MessageDirection::ToWidget,
            tile_set,
            None,
        ));
    }

    fn init_empty(&mut self, ui: &mut UserInterface) {
        // TODO
    }

    fn init_tile_set(&mut self, resource: &TileSetResource, ui: &mut UserInterface) {
        // TODO
    }

    fn init_brush(&mut self, resource: &TileMapBrushResource, ui: &mut UserInterface) {
        // TODO
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
        if let TileResource::TileSet(r) = &self.tile_resource {
            let tile_set = r.data_ref();
            self.properties_tab.sync_to_model(&tile_set, ui);
            self.colliders_tab.sync_to_model(&tile_set, ui);
        }
        self.tile_inspector.sync_to_model(ui);
    }

    fn try_save(&self) {
        Log::verify(self.tile_resource.save());
    }

    pub fn handle_ui_message(
        mut self,
        message: &UiMessage,
        ui: &mut UserInterface,
        resource_manager: &ResourceManager,
        sender: &MessageSender,
        serialization_context: Arc<SerializationContext>,
    ) -> Option<Self> {
        self.tile_inspector.handle_ui_message(message, ui, sender);
        if let TileResource::TileSet(r) = &self.tile_resource {
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
