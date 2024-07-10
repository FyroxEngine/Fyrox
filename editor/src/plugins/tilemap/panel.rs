use crate::{
    asset::item::AssetItem,
    command::{Command, CommandGroup, SetPropertyCommand},
    fyrox::{
        asset::manager::ResourceManager,
        core::{algebra::Vector2, pool::Handle, Uuid},
        graph::{BaseSceneGraph, SceneGraphNode},
        gui::{
            border::BorderBuilder,
            dropdown_list::{DropdownListBuilder, DropdownListMessage},
            grid::{Column, GridBuilder, Row},
            message::{MessageDirection, UiMessage},
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            wrap_panel::WrapPanelBuilder,
            BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
            VerticalAlignment,
        },
        scene::{
            node::Node,
            tilemap::{
                brush::{BrushTile, TileMapBrush},
                tileset::{TileSet, TileSetResource},
                TileMap,
            },
        },
    },
    gui::make_dropdown_list_option,
    message::MessageSender,
    plugins::tilemap::{
        commands::{MoveBrushTilesCommand, RemoveBrushTileCommand, SetBrushTilesCommand},
        palette::{
            PaletteMessage, PaletteWidget, PaletteWidgetBuilder, TileViewBuilder, TileViewMessage,
        },
    },
    scene::commands::GameSceneContext,
};

pub struct TileMapPanel {
    pub window: Handle<UiNode>,
    pub palette: Handle<UiNode>,
    active_brush_selector: Handle<UiNode>,
}

fn generate_tiles(
    tile_set_resource: &TileSetResource,
    tile_map_brush: &TileMapBrush,
    ctx: &mut BuildContext,
) -> Vec<Handle<UiNode>> {
    tile_map_brush
        .tiles
        .iter()
        .map(|tile| {
            TileViewBuilder::new(
                tile_set_resource.clone(),
                WidgetBuilder::new().with_id(tile.id),
            )
            .with_definition_index(tile.definition_index)
            .with_position(tile.local_position)
            .build(ctx)
        })
        .collect::<Vec<_>>()
}

fn make_brush_entries(tile_map: &TileMap, ctx: &mut BuildContext) -> Vec<Handle<UiNode>> {
    tile_map
        .brushes()
        .iter()
        .flatten()
        .map(|brush| make_dropdown_list_option(ctx, &brush.kind().to_string()))
        .collect::<Vec<_>>()
}

fn selected_brush_index(tile_map: &TileMap) -> Option<usize> {
    tile_map
        .brushes()
        .iter()
        .position(|brush| brush == &tile_map.active_brush())
}

impl TileMapPanel {
    pub fn new(ctx: &mut BuildContext, scene_frame: Handle<UiNode>, tile_map: &TileMap) -> Self {
        let tiles = tile_map
            .tile_set()
            .and_then(|tile_set| {
                tile_map
                    .active_brush()
                    .map(|brush| generate_tiles(tile_set, &brush.data_ref(), ctx))
            })
            .unwrap_or_default();

        let palette = PaletteWidgetBuilder::new(WidgetBuilder::new())
            .with_tiles(tiles)
            .build(ctx);

        let active_brush_selector =
            DropdownListBuilder::new(WidgetBuilder::new().with_width(250.0))
                .with_opt_selected(selected_brush_index(tile_map))
                .with_items(make_brush_entries(tile_map, ctx))
                .build(ctx);

        let toolbar = WrapPanelBuilder::new(
            WidgetBuilder::new()
                .on_row(0)
                .with_child(active_brush_selector),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let content = GridBuilder::new(WidgetBuilder::new().with_child(toolbar).with_child(
            BorderBuilder::new(WidgetBuilder::new().on_row(1).with_child(palette)).build(ctx),
        ))
        .add_row(Row::strict(23.0))
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(250.0).with_height(350.0))
            .open(false)
            .with_title(WindowTitle::text("Tile Map Control Panel"))
            .with_content(content)
            .build(ctx);

        ctx.sender()
            .send(WindowMessage::open_and_align(
                window,
                MessageDirection::ToWidget,
                scene_frame,
                HorizontalAlignment::Right,
                VerticalAlignment::Top,
                Thickness::uniform(2.0),
                false,
                true,
            ))
            .unwrap();

        Self {
            window,
            palette,
            active_brush_selector,
        }
    }

    pub fn destroy(self, ui: &UserInterface) {
        ui.send_message(WidgetMessage::remove(
            self.window,
            MessageDirection::ToWidget,
        ));
    }

    pub fn handle_ui_message(
        self,
        message: &UiMessage,
        ui: &UserInterface,
        resource_manager: &ResourceManager,
        tile_map_handle: Handle<Node>,
        tile_map: Option<&TileMap>,
        sender: &MessageSender,
    ) -> Option<Self> {
        if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.window {
                self.destroy(ui);
                return None;
            }
        } else if let Some(WidgetMessage::Drop(dropped)) = message.data() {
            if let Some(tile_map) = tile_map {
                if message.destination() == self.palette {
                    if let Some(item) = ui.node(*dropped).cast::<AssetItem>() {
                        if let Some(tile_set) = item.resource::<TileSet>(resource_manager) {
                            if let Some(active_brush) = tile_map.active_brush().as_ref() {
                                let tile_set = tile_set.data_ref();
                                let tiles = tile_set
                                    .tiles
                                    .iter()
                                    .enumerate()
                                    .map(|(index, _tile)| {
                                        let side_size = 11;

                                        BrushTile {
                                            definition_index: index,
                                            local_position: Vector2::new(
                                                index as i32 % side_size,
                                                index as i32 / side_size,
                                            ),
                                            id: Uuid::new_v4(),
                                        }
                                    })
                                    .collect::<Vec<_>>();

                                sender.do_command(SetBrushTilesCommand {
                                    brush: active_brush.clone(),
                                    tiles,
                                });
                            }
                        }
                    }
                }
            }
        } else if let Some(PaletteMessage::MoveTiles(move_data)) = message.data() {
            if let Some(tile_map) = tile_map {
                if let Some(active_brush_resource) = tile_map.active_brush().as_ref() {
                    if message.destination() == self.palette
                        && message.direction == MessageDirection::FromWidget
                    {
                        let mut commands = vec![Command::new(MoveBrushTilesCommand {
                            brush: active_brush_resource.clone(),
                            positions: move_data.clone(),
                        })];

                        let active_brush = active_brush_resource.data_ref();
                        for (id, new_tile_position) in move_data.iter() {
                            if let Some(tile) = active_brush.find_tile(id) {
                                for other_tile in active_brush.tiles.iter() {
                                    if !std::ptr::eq(tile, other_tile)
                                        && other_tile.local_position == *new_tile_position
                                    {
                                        commands.push(Command::new(RemoveBrushTileCommand {
                                            brush: active_brush_resource.clone(),
                                            id: other_tile.id,
                                            tile: None,
                                        }));
                                    }
                                }
                            }
                        }

                        sender.do_command(CommandGroup::from(commands));
                    }
                }
            }
        } else if let Some(DropdownListMessage::SelectionChanged(Some(index))) = message.data() {
            if message.destination() == self.active_brush_selector
                && message.direction() == MessageDirection::FromWidget
            {
                if let Some(tile_map) = tile_map {
                    if let Some(brush) = tile_map.brushes().get(*index) {
                        sender.do_command(SetPropertyCommand::new(
                            "active_brush".into(),
                            Box::new(brush.clone()),
                            move |ctx| {
                                ctx.get_mut::<GameSceneContext>()
                                    .scene
                                    .graph
                                    .node_mut(tile_map_handle)
                            },
                        ));
                    }
                }
            }
        }

        Some(self)
    }

    pub fn sync_to_model(&self, ui: &mut UserInterface, tile_map: &TileMap) {
        let items = make_brush_entries(tile_map, &mut ui.build_ctx());
        ui.send_message(DropdownListMessage::items(
            self.active_brush_selector,
            MessageDirection::ToWidget,
            items,
        ));

        let Some(active_brush) = tile_map.active_brush() else {
            return;
        };

        let active_brush = active_brush.data_ref();

        let Some(tile_set) = tile_map.tile_set() else {
            return;
        };

        let tile_views = ui
            .node(self.palette)
            .component_ref::<PaletteWidget>()
            .unwrap()
            .tiles
            .clone();

        for tile_view in tile_views.iter() {
            if active_brush
                .tiles
                .iter()
                .all(|tile| tile.id != ui.node(*tile_view).id)
            {
                ui.send_message(PaletteMessage::remove_tile(
                    self.palette,
                    MessageDirection::ToWidget,
                    *tile_view,
                ));
            }
        }

        for tile in active_brush.tiles.iter() {
            if let Some(tile_view) = tile_views
                .iter()
                .find(|tile_view| ui.node(**tile_view).id == tile.id)
            {
                ui.send_message(TileViewMessage::local_position(
                    *tile_view,
                    MessageDirection::ToWidget,
                    tile.local_position,
                ));
            } else {
                let ctx = &mut ui.build_ctx();
                let tile_view =
                    TileViewBuilder::new(tile_set.clone(), WidgetBuilder::new().with_id(tile.id))
                        .with_definition_index(tile.definition_index)
                        .with_position(tile.local_position)
                        .build(ctx);

                ui.send_message(PaletteMessage::add_tile(
                    self.palette,
                    MessageDirection::ToWidget,
                    tile_view,
                ));
            }
        }
    }
}
