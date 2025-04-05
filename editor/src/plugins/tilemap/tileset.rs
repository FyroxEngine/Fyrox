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

//! The Tile Set Editor window that opens whenever a tile set or brush is double-clicked.
//! It serves to edit both tile sets and brushes, since they are conceptually similiar,
//! with only minor modifications needed when switching between the two modes.

use super::{commands::*, *};
use crate::{
    command::{Command, CommandGroup},
    fyrox::{
        asset::manager::ResourceManager,
        core::{algebra::Vector2, color::Color, log::Log, pool::Handle},
        gui::{
            border::BorderBuilder,
            brush::Brush,
            button::Button,
            button::ButtonMessage,
            color::{ColorFieldBuilder, ColorFieldMessage},
            decorator::DecoratorMessage,
            grid::SizeMode,
            grid::{Column, GridBuilder, Row},
            message::{MessageDirection, UiMessage},
            scroll_viewer::ScrollViewerBuilder,
            stack_panel::StackPanelBuilder,
            tab_control::{TabControl, TabControlBuilder, TabControlMessage, TabDefinition},
            text::TextBuilder,
            text::TextMessage,
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, Thickness, UiNode, UserInterface,
        },
        scene::tilemap::{tileset::TileSetRef, TileBook, TileDefinitionHandle},
    },
    message::MessageSender,
    plugins::inspector::editors::resource::{ResourceFieldBuilder, ResourceFieldMessage},
};
use fyrox::scene::tilemap::brush::TileMapBrushResource;
use macro_tab::MacroTab;
use palette::{PaletteWidgetBuilder, DEFAULT_MATERIAL_COLOR};

const TAB_MARGIN: Thickness = Thickness {
    left: 10.0,
    top: 2.0,
    right: 10.0,
    bottom: 2.0,
};

const DEFAULT_PAGE: Vector2<i32> = Vector2::new(0, 0);

/// A window for editing tile sets and tile map brushes.
pub struct TileSetEditor {
    /// The window that contains the tile set editor.
    pub window: Handle<UiNode>,
    /// The state that is shared by many tile editing objects,
    /// such as palette widgets that display the tiles,
    /// the tile map control panel that allows the user to switch
    /// tools and select stamps, and others.
    state: TileDrawStateRef,
    macro_list: BrushMacroListRef,
    /// The resource to be edited. It can either be a tile set or a brush.
    tile_book: TileBook,
    /// The field that controls the tint of the background material on tile atlas pages.
    /// This tint allows the background material to be visually distinguished from actual tiles.
    color_field: Handle<UiNode>,
    /// A text widget showing the coordinates of the currently selected cells.
    cell_position: Handle<UiNode>,
    /// The control that allows the editor to switch between the tiles tab,
    /// the properties tab, and the colliders tabl.
    tab_control: Handle<UiNode>,
    /// The palette widget for the page icons. It is used to select which page to edit.
    pages_palette: Handle<UiNode>,
    /// The palette widget for the actual tiles. This is the main work area of the editor.
    tiles_palette: Handle<UiNode>,
    /// A button to switch to the pick tool.
    pick_button: Handle<UiNode>,
    /// A button to open the tile map control panel.
    /// It can sometimes be useful while editing a tile set or brush.
    open_control: Handle<UiNode>,
    /// A button to deleted the selected tiles or pages.
    remove: Handle<UiNode>,
    /// A button to select all the pages of the current resource.
    all_pages: Handle<UiNode>,
    /// A button to select all the tiles of the current page.
    all_tiles: Handle<UiNode>,
    /// When editing a brush, this is the area that allows the user to choose
    /// the tile set for the brush.
    tile_set_selector: Handle<UiNode>,
    /// This is the resource field that lets the user select the tile set for a brush.
    tile_set_field: Handle<UiNode>,
    /// This is the area that shows the data for the currently selected tiles.
    tile_inspector: TileInspector,
    /// The tab that allows users to add, remove, and edit property layers.
    properties_tab: PropertiesTab,
    /// The tab that allows users to add, remove, and edit collider layers.
    colliders_tab: CollidersTab,
    /// The tab that allows users to add, remove, and edit brush macros.
    macros_tab: MacroTab,
    /// The set of cells associated with each macro instance in order.
    /// The length of this Vec matches the length of the brush's [`TileMapBrush::macros`]
    /// list, and each element of this Vec is the set of cells for the macro
    /// at the same index in the brush's macro list.
    brush_macro_cell_sets: MacroCellSetListRef,
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

fn tile_set_to_title(resource_manager: &ResourceManager, tile_book: &TileBook) -> String {
    match tile_book {
        TileBook::Empty => "Missing Resource".into(),
        TileBook::TileSet(_) => format!("Tile Set: {}", tile_book.name(resource_manager)),
        TileBook::Brush(_) => format!("Tile Map Brush: {}", tile_book.name(resource_manager)),
    }
}

fn build_brush_macro_cell_sets(
    macro_list: &BrushMacroList,
    brush: TileMapBrushResource,
    brush_macro_cell_sets: &mut MacroCellSetList,
) {
    let brush_guard = brush.data_ref();
    let instances = &brush_guard.macros;
    brush_macro_cell_sets.resize_with(instances.len(), FxHashSet::default);
    for (instance, set) in instances.iter().zip(brush_macro_cell_sets.iter_mut()) {
        set.clear();
        let Some(m) = macro_list.get_by_uuid(&instance.macro_id) else {
            continue;
        };
        m.fill_cell_set(
            &BrushMacroInstance {
                brush: brush.clone(),
                settings: instance.settings.clone(),
            },
            set,
        );
    }
    brush_macro_cell_sets.finalize();
}

impl TileSetEditor {
    /// Create a new tile set editor window.
    /// The `state` a reference to a shared [`TileDrawState`] which allows the `TileSetEditor` to know
    /// what is happening in the [`TileMapPanel`] and react appropriately to the current tool and stamp.
    pub fn new(
        tile_book: TileBook,
        state: TileDrawStateRef,
        macro_list: BrushMacroListRef,
        sender: MessageSender,
        resource_manager: ResourceManager,
        ctx: &mut BuildContext,
    ) -> Self {
        let mut brush_macro_cell_sets = MacroCellSetList::default();
        if let Some(brush) = tile_book.brush_ref() {
            build_brush_macro_cell_sets(
                &macro_list.lock(),
                brush.clone(),
                &mut brush_macro_cell_sets,
            );
        }
        let brush_macro_cell_sets = MacroCellSetListRef::new(brush_macro_cell_sets);
        let tile_set_field =
            ResourceFieldBuilder::<TileSet>::new(WidgetBuilder::new().on_column(1), sender.clone())
                .with_resource(tile_book.get_tile_set())
                .build(ctx, resource_manager.clone());
        let tile_set_selector = GridBuilder::new(
            WidgetBuilder::new()
                .with_visibility(tile_book.is_brush())
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
        let pick_button = make_drawing_mode_button(
            ctx,
            20.0,
            20.0,
            PICK_IMAGE.clone(),
            "Pick tiles for drawing from the tile map.",
            None,
        );
        let open_control = make_button(
            "Palette",
            "Open the tile palette control window.",
            0,
            1,
            ctx,
        );
        let page_buttons = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(pick_button)
                .with_child(open_control)
                .with_child({
                    all_pages = make_button("All Pages", "Select all pages.", 0, 2, ctx);
                    all_pages
                })
                .with_child({
                    all_tiles = make_button("All Tiles", "Select all tiles.", 0, 3, ctx);
                    all_tiles
                })
                .with_child({
                    remove = make_button("Delete", "Remove selected tile.", 0, 4, ctx);
                    remove
                }),
        )
        .add_row(Row::auto())
        .add_column(Column::auto())
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
        .with_resource(tile_book.clone())
        .with_kind(TilePaletteStage::Pages)
        .with_editable(true)
        .with_macro_list(macro_list.clone())
        .build(ctx);

        let tiles_palette = PaletteWidgetBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .with_allow_drop(true),
            sender.clone(),
            state.clone(),
        )
        .with_page(DEFAULT_PAGE)
        .with_resource(tile_book.clone())
        .with_kind(TilePaletteStage::Tiles)
        .with_editable(true)
        .with_macro_list(macro_list.clone())
        .with_macro_cells(brush_macro_cell_sets.clone())
        .build(ctx);

        let tile_inspector = TileInspector::new(
            state.clone(),
            macro_list.clone(),
            brush_macro_cell_sets.clone(),
            pages_palette,
            tiles_palette,
            tile_book.clone(),
            sender,
            resource_manager.clone(),
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

        let properties_tab = PropertiesTab::new(tile_book.clone(), ctx);
        let colliders_tab = CollidersTab::new(tile_book.clone(), ctx);
        let macros_tab = MacroTab::new(macro_list.clone(), tile_book.clone(), ctx);
        let tab_control = TabControlBuilder::new(WidgetBuilder::new())
            .with_tab(make_tab("Tiles", tile_tab, ctx))
            .with_tab(make_tab("Properties", properties_tab.handle(), ctx))
            .with_tab(make_tab("Collision", colliders_tab.handle(), ctx))
            .with_tab(make_tab("Macros", macros_tab.handle(), ctx))
            .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(800.0).with_height(600.0))
            .open(false)
            .with_title(WindowTitle::text(tile_set_to_title(
                &resource_manager,
                &tile_book,
            )))
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
            macros_tab,
            state,
            macro_list,
            tab_control,
            color_field,
            cell_position,
            pages_palette,
            tiles_palette,
            tile_book,
            pick_button,
            open_control,
            remove,
            all_pages,
            all_tiles,
            tile_inspector,
            tile_set_selector,
            tile_set_field,
            brush_macro_cell_sets,
        };

        editor.sync_to_model(ctx.inner_mut());

        editor
    }

    /// Change the resource being edited by this window.
    pub fn set_tile_resource(
        &mut self,
        resource_manager: &ResourceManager,
        tile_book: TileBook,
        ui: &mut UserInterface,
    ) {
        self.try_save(resource_manager);
        self.tile_book = tile_book.clone();
        if let Some(brush) = tile_book.brush_ref() {
            build_brush_macro_cell_sets(
                &self.macro_list.lock(),
                brush.clone(),
                &mut self.brush_macro_cell_sets.lock(),
            );
        } else {
            self.brush_macro_cell_sets.lock().clear();
        }
        self.tile_inspector.set_tile_resource(tile_book.clone(), ui);
        ui.send_message(WindowMessage::title(
            self.window,
            MessageDirection::ToWidget,
            WindowTitle::text(tile_set_to_title(resource_manager, &tile_book)),
        ));
        let mut state = self.state.lock_mut("set_tile_resource");
        if state.selection_palette() == self.pages_palette
            || state.selection_palette() == self.tiles_palette
        {
            state.clear_selection();
        }
        drop(state);
        if let TileBook::Brush(brush) = &tile_book {
            ui.send_message(ResourceFieldMessage::value(
                self.tile_set_field,
                MessageDirection::ToWidget,
                brush.data_ref().tile_set(),
            ));
        }
        self.send_tabs_visible(tile_book.is_tile_set(), ui);
        ui.send_message(WidgetMessage::visibility(
            self.tile_set_selector,
            MessageDirection::ToWidget,
            tile_book.is_brush(),
        ));
        ui.send_message(TabControlMessage::active_tab(
            self.tab_control,
            MessageDirection::ToWidget,
            Some(0),
        ));
        for palette in [self.pages_palette, self.tiles_palette] {
            ui.send_message(PaletteMessage::set_page(
                palette,
                MessageDirection::ToWidget,
                tile_book.clone(),
                Some(Vector2::new(0, 0)),
            ));
        }
        self.sync_to_model(ui);
    }

    fn send_tabs_visible(&self, is_tile_set: bool, ui: &mut UserInterface) {
        let tab_control = ui.node(self.tab_control).cast::<TabControl>().unwrap();
        let tabs = tab_control.headers_container;
        let children = ui.node(tabs).children();
        for &tab in &children[1..3] {
            ui.send_message(WidgetMessage::visibility(
                tab,
                MessageDirection::ToWidget,
                is_tile_set,
            ));
        }
        ui.send_message(WidgetMessage::visibility(
            children[3],
            MessageDirection::ToWidget,
            !is_tile_set,
        ));
    }

    /// Focus the editor on a particular tile and select that tile.
    pub fn set_position(&self, handle: TileDefinitionHandle, ui: &mut UserInterface) {
        for palette in [self.pages_palette, self.tiles_palette] {
            ui.send_message(PaletteMessage::set_page(
                palette,
                MessageDirection::ToWidget,
                self.tile_book.clone(),
                Some(handle.page()),
            ));
        }
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

    /// Update the widgets of this editor after the shared [`TileDrawState`] may have changed.
    pub fn sync_to_state(&mut self, ui: &mut UserInterface) {
        self.tile_inspector.sync_to_state(ui);
        let decorator = *ui
            .try_get_of_type::<Button>(self.pick_button)
            .unwrap()
            .decorator;
        ui.send_message(DecoratorMessage::select(
            decorator,
            MessageDirection::ToWidget,
            self.state.lock().drawing_mode == DrawingMode::Pick,
        ));
        let cell_position = self.cell_position();
        ui.send_message(TextMessage::text(
            self.cell_position,
            MessageDirection::ToWidget,
            cell_position,
        ));
        for palette in [self.pages_palette, self.tiles_palette] {
            ui.send_message(PaletteMessage::sync_to_state(
                palette,
                MessageDirection::ToWidget,
            ));
        }
    }

    /// Update the widgets of this editor after the edited resource may have changed.
    pub fn sync_to_model(&mut self, ui: &mut UserInterface) {
        if let Some(r) = self.tile_book.tile_set_ref() {
            let mut tile_set = TileSetRef::new(r);
            let tile_set = tile_set.as_loaded();
            self.properties_tab.sync_to_model(&tile_set, ui);
            self.colliders_tab.sync_to_model(&tile_set, ui);
            self.brush_macro_cell_sets.lock().clear();
        } else if let Some(brush) = self.tile_book.brush_ref() {
            build_brush_macro_cell_sets(
                &self.macro_list.lock(),
                brush.clone(),
                &mut self.brush_macro_cell_sets.lock(),
            );
            self.macros_tab.sync_to_model(brush.clone(), ui);
        }
        self.tile_inspector.sync_to_model(ui);
        if let TileBook::Brush(brush) = &self.tile_book {
            let brush = brush.data_ref();
            let tile_set = brush.tile_set();
            ui.send_message(ResourceFieldMessage::value(
                self.tile_set_field,
                MessageDirection::ToWidget,
                tile_set,
            ));
        }
    }

    /// Attempt to save the resource being edited, if there is a resource and it
    /// has been changed, otherwise do nothing.
    pub fn try_save(&self, resource_manager: &ResourceManager) {
        Log::verify(self.tile_book.save(resource_manager));
    }

    /// React appropriately to any UI message that may involve the widgets of this editor.
    pub fn handle_ui_message(mut self, message: &UiMessage, editor: &mut Editor) -> Option<Self> {
        self.tile_inspector.handle_ui_message(message, editor);
        if let Some(r) = self.tile_book.tile_set_ref() {
            let ui = editor.engine.user_interfaces.first_mut();
            let sender = &editor.message_sender;
            self.properties_tab
                .handle_ui_message(r.clone(), message, ui, sender);
            self.colliders_tab
                .handle_ui_message(r.clone(), message, ui, sender);
        } else if let Some(r) = self.tile_book.brush_ref() {
            self.macros_tab.handle_ui_message(r, message, editor);
        }
        let ui = editor.engine.user_interfaces.first_mut();
        let sender = &editor.message_sender;
        if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.window {
                self.try_save(&editor.engine.resource_manager);
                self.destroy(ui);
                return None;
            }
        } else if let Some(ResourceFieldMessage::<TileSet>::Value(tile_set)) = message.data() {
            if message.destination() == self.tile_set_field
                && message.direction() == MessageDirection::FromWidget
            {
                if let TileBook::Brush(brush) = &self.tile_book {
                    sender.do_command(ModifyBrushTileSetCommand {
                        brush: brush.clone(),
                        tile_set: tile_set.clone(),
                    });
                }
            }
        } else if let Some(TileHandleEditorMessage::Goto(handle)) = message.data() {
            if let Some(tile_set) = self.tile_book.get_tile_set() {
                self.set_tile_resource(
                    &editor.engine.resource_manager,
                    TileBook::TileSet(tile_set),
                    ui,
                );
                self.set_position(*handle, ui);
            }
        } else if let Some(TileHandleEditorMessage::OpenPalette(handle)) = message.data() {
            if let Some(tile_set) = self.tile_book.get_tile_set() {
                ui.send_message(OpenTilePanelMessage::message(
                    TileBook::TileSet(tile_set),
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
            if message.destination() == self.pick_button {
                self.state.lock_mut("Pick mode").drawing_mode = DrawingMode::Pick;
            } else if message.destination() == self.open_control {
                ui.send_message(OpenTilePanelMessage::message(self.tile_book.clone(), None));
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
        match &self.tile_book {
            TileBook::Empty => None,
            TileBook::TileSet(tile_set) => Some(Command::new(SetTileSetPageCommand {
                tile_set: tile_set.clone(),
                position,
                page: None,
            })),
            TileBook::Brush(brush) => Some(Command::new(SetBrushPageCommand {
                brush: brush.clone(),
                position,
                page: None,
            })),
        }
    }
}
