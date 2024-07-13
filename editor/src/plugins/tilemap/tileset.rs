use crate::{
    asset::item::AssetItem,
    command::{make_command, Command, CommandGroup},
    fyrox::{
        asset::{manager::ResourceManager, untyped::ResourceKind, ResourceData},
        core::{color::Color, log::Log, math::Rect, pool::Handle, Uuid},
        engine::SerializationContext,
        graph::{BaseSceneGraph, SceneGraphNode},
        gui::{
            border::BorderBuilder,
            brush::Brush,
            button::ButtonMessage,
            decorator::DecoratorBuilder,
            grid::{Column, GridBuilder, Row},
            image::ImageBuilder,
            inspector::{
                editors::PropertyEditorDefinitionContainer, Inspector, InspectorBuilder,
                InspectorContext, InspectorMessage,
            },
            list_view::{ListView, ListViewBuilder, ListViewMessage},
            message::{MessageDirection, UiMessage},
            stack_panel::StackPanelBuilder,
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            wrap_panel::WrapPanelBuilder,
            BuildContext, Orientation, Thickness, UiNode, UserInterface, VerticalAlignment,
        },
        material::{Material, MaterialResource},
        resource::texture::Texture,
        scene::tilemap::tileset::{TileDefinition, TileSet, TileSetResource},
    },
    inspector::EditorEnvironment,
    message::MessageSender,
    plugins::tilemap::{
        commands::{AddTileCommand, RemoveTileCommand},
        make_button,
        tile_set_import::{ImportResult, TileSetImporter},
    },
    Message,
};
use std::sync::Arc;

pub struct TileSetEditor {
    window: Handle<UiNode>,
    tiles: Handle<UiNode>,
    tile_set: TileSetResource,
    import: Handle<UiNode>,
    remove: Handle<UiNode>,
    remove_all: Handle<UiNode>,
    selection: Option<usize>,
    need_save: bool,
    tile_set_importer: Option<TileSetImporter>,
    inspector: Handle<UiNode>,
}

impl TileSetEditor {
    pub fn new(tile_set: TileSetResource, ctx: &mut BuildContext) -> Self {
        let import;
        let remove;
        let remove_all;
        let buttons = StackPanelBuilder::new(
            WidgetBuilder::new()
                .on_row(0)
                .with_child({
                    import = make_button(
                        "Import...",
                        "Import tile set from a sprite sheet.",
                        true,
                        ctx,
                    );
                    import
                })
                .with_child({
                    remove = make_button("Remove", "Remove selected tile.", false, ctx);
                    remove
                })
                .with_child({
                    remove_all = make_button("Remove All", "Remove all tiles.", true, ctx);
                    remove_all
                }),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let tiles = ListViewBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .with_allow_drop(true),
        )
        .with_items_panel(
            WrapPanelBuilder::new(
                WidgetBuilder::new().with_vertical_alignment(VerticalAlignment::Top),
            )
            .with_orientation(Orientation::Horizontal)
            .build(ctx),
        )
        .build(ctx);

        let inspector = InspectorBuilder::new(
            WidgetBuilder::new()
                .on_column(1)
                .with_width(240.0)
                .with_visibility(false),
        )
        .build(ctx);

        let split_panel = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(tiles)
                .with_child(inspector)
                .on_row(1),
        )
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .add_row(Row::stretch())
        .build(ctx);

        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(buttons)
                .with_child(split_panel),
        )
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(550.0).with_height(400.0))
            .open(false)
            .with_title(WindowTitle::text("Tile Set Editor"))
            .with_content(content)
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
            tiles,
            tile_set,
            import,
            remove,
            remove_all,
            selection: Default::default(),
            need_save: false,
            tile_set_importer: None,
            inspector,
        };

        editor.sync_to_model(ctx.inner_mut());

        editor
    }

    fn destroy(mut self, ui: &UserInterface) {
        ui.send_message(WidgetMessage::remove(
            self.window,
            MessageDirection::ToWidget,
        ));

        if let Some(importer) = self.tile_set_importer.take() {
            importer.destroy(ui);
        }
    }

    pub fn sync_to_model(&mut self, ui: &mut UserInterface) {
        let tile_set = self.tile_set.data_ref();
        let tile_views = ui
            .node(self.tiles)
            .component_ref::<ListView>()
            .unwrap()
            .items
            .clone();

        for tile_view in tile_views.iter() {
            if tile_set
                .tiles
                .iter()
                .all(|tile| tile.id != ui.node(*tile_view).id)
            {
                ui.send_message(ListViewMessage::remove_item(
                    self.tiles,
                    MessageDirection::ToWidget,
                    *tile_view,
                ));
            }
        }

        for tile in tile_set.tiles.iter() {
            if tile_views
                .iter()
                .all(|tile_view| ui.node(*tile_view).id != tile.id)
            {
                let texture = tile.material.data_ref().texture("diffuseTexture");

                let ctx = &mut ui.build_ctx();
                let tile_view = DecoratorBuilder::new(BorderBuilder::new(
                    WidgetBuilder::new().with_id(tile.id).with_child(
                        ImageBuilder::new(
                            WidgetBuilder::new()
                                .with_width(52.0)
                                .with_height(52.0)
                                .with_margin(Thickness::uniform(2.0)),
                        )
                        .with_uv_rect(tile.uv_rect)
                        .with_opt_texture(texture.map(|t| t.into()))
                        .build(ctx),
                    ),
                ))
                .with_selected_brush(Brush::Solid(Color::RED))
                .build(ctx);

                ui.send_message(ListViewMessage::add_item(
                    self.tiles,
                    MessageDirection::ToWidget,
                    tile_view,
                ));
            }
        }

        if let Some(selection) = self.selection {
            if let Some(tile_definition) = tile_set.tiles.get(selection) {
                let ctx = ui
                    .node(self.inspector)
                    .cast::<Inspector>()
                    .unwrap()
                    .context()
                    .clone();

                if let Err(sync_errors) = ctx.sync(tile_definition, ui, 0, true, Default::default())
                {
                    for error in sync_errors {
                        Log::err(format!("Failed to sync property. Reason: {:?}", error))
                    }
                }
            }
        }
    }

    fn try_save(&self) {
        if let ResourceKind::External(path) = self.tile_set.kind() {
            Log::verify(self.tile_set.data_ref().save(&path));
        }
    }

    pub fn handle_ui_message(
        mut self,
        message: &UiMessage,
        ui: &mut UserInterface,
        resource_manager: &ResourceManager,
        sender: &MessageSender,
        property_editors: Arc<PropertyEditorDefinitionContainer>,
        serialization_context: Arc<SerializationContext>,
    ) -> Option<Self> {
        if let Some(importer) = self.tile_set_importer.take() {
            match importer.handle_ui_message(message, ui, resource_manager) {
                ImportResult::None(importer) => {
                    self.tile_set_importer = Some(importer);
                }
                ImportResult::Closed => {}
                ImportResult::TileSet(tiles) => {
                    let commands = tiles
                        .into_iter()
                        .map(|tile| {
                            Command::new(AddTileCommand {
                                tile_set: self.tile_set.clone(),
                                tile,
                            })
                        })
                        .collect::<Vec<_>>();
                    if !commands.is_empty() {
                        sender.do_command(CommandGroup::from(commands));
                        self.need_save = true;
                    }
                }
            }
        }

        if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.window {
                self.try_save();
                self.destroy(ui);
                return None;
            }
        } else if let Some(WidgetMessage::Drop(dropped)) = message.data() {
            if message.destination() == self.tiles {
                if let Some(item) = ui.node(*dropped).cast::<AssetItem>() {
                    if let Some(material) = item.resource::<Material>(resource_manager) {
                        sender.do_command(AddTileCommand {
                            tile_set: self.tile_set.clone(),
                            tile: TileDefinition {
                                material,
                                uv_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
                                collider: Default::default(),
                                color: Default::default(),
                                id: Uuid::new_v4(),
                            },
                        });
                        self.need_save = true;
                    } else if let Some(texture) = item.resource::<Texture>(resource_manager) {
                        let mut material = Material::standard_2d();
                        material
                            .set_texture(&"diffuseTexture".into(), Some(texture))
                            .unwrap();

                        let material = MaterialResource::new_ok(ResourceKind::Embedded, material);

                        sender.do_command(AddTileCommand {
                            tile_set: self.tile_set.clone(),
                            tile: TileDefinition {
                                material,
                                uv_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
                                collider: Default::default(),
                                color: Default::default(),
                                id: Uuid::new_v4(),
                            },
                        });
                        self.need_save = true;
                    }
                }
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.import {
                self.tile_set_importer = Some(TileSetImporter::new(&mut ui.build_ctx()));
            } else if message.destination() == self.remove {
                if let Some(selection) = self.selection {
                    sender.do_command(RemoveTileCommand {
                        tile_set: self.tile_set.clone(),
                        index: selection,
                        tile: None,
                    });
                    self.need_save = true;
                }
            } else if message.destination() == self.remove_all {
                let mut commands = Vec::new();

                let mut tile_index = self.tile_set.data_ref().tiles.len();
                while tile_index > 0 {
                    tile_index -= 1;
                    commands.push(Command::new(RemoveTileCommand {
                        tile_set: self.tile_set.clone(),
                        index: tile_index,
                        tile: None,
                    }));
                }

                if !commands.is_empty() {
                    sender.do_command(CommandGroup::from(commands));
                    self.need_save = true;
                }
            }
        } else if let Some(ListViewMessage::SelectionChanged(selection)) = message.data() {
            if message.destination() == self.tiles
                && message.direction() == MessageDirection::FromWidget
            {
                self.selection = *selection;

                ui.send_message(WidgetMessage::enabled(
                    self.remove,
                    MessageDirection::ToWidget,
                    self.selection.is_some(),
                ));

                ui.send_message(WidgetMessage::visibility(
                    self.inspector,
                    MessageDirection::ToWidget,
                    self.selection.is_some(),
                ));

                if let Some(selection) = selection {
                    let tile_set = self.tile_set.data_ref();
                    if let Some(tile_definition) = tile_set
                        .as_loaded_ref()
                        .and_then(|tile_set| tile_set.tiles.get(*selection))
                    {
                        let env = Arc::new(EditorEnvironment {
                            resource_manager: resource_manager.clone(),
                            serialization_context,
                            available_animations: Default::default(),
                            sender: sender.clone(),
                        });

                        let context = InspectorContext::from_object(
                            tile_definition,
                            &mut ui.build_ctx(),
                            property_editors,
                            Some(env),
                            1,
                            0,
                            true,
                            Default::default(),
                            80.0,
                        );

                        ui.send_message(InspectorMessage::context(
                            self.inspector,
                            MessageDirection::ToWidget,
                            context,
                        ));
                    } else {
                        ui.send_message(InspectorMessage::context(
                            self.inspector,
                            MessageDirection::ToWidget,
                            Default::default(),
                        ));
                    }
                }
            }
        } else if let Some(InspectorMessage::PropertyChanged(args)) = message.data() {
            let tile_set = self.tile_set.clone();
            sender.send(Message::DoCommand(
                make_command(args, move |_| {
                    // FIXME: HACK!
                    unsafe {
                        std::mem::transmute::<&'_ mut TileSet, &'static mut TileSet>(
                            &mut *tile_set.data_ref(),
                        )
                    }
                })
                .unwrap(),
            ));
        }

        Some(self)
    }

    pub fn update(&mut self) {
        if self.need_save {
            self.try_save();
            self.need_save = false;
        }
    }
}
