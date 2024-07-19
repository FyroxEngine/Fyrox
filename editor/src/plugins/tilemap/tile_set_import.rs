use crate::{
    asset::item::AssetItem,
    fyrox::{
        asset::untyped::ResourceKind,
        core::{algebra::Vector2, math::Rect, pool::Handle},
        graph::BaseSceneGraph,
        gui::{
            button::ButtonMessage,
            grid::{Column, GridBuilder, GridMessage, Row},
            image::{ImageBuilder, ImageMessage},
            message::{MessageDirection, UiMessage},
            numeric::{NumericUpDownBuilder, NumericUpDownMessage},
            stack_panel::StackPanelBuilder,
            text::TextBuilder,
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, Orientation, Thickness, UiNode, UserInterface, VerticalAlignment,
        },
        material::{Material, MaterialResource},
        resource::texture::Texture,
        scene::tilemap::tileset::TileDefinition,
    },
    plugins::tilemap::make_button,
};

pub struct TileSetImporter {
    window: Handle<UiNode>,
    import: Handle<UiNode>,
    cancel: Handle<UiNode>,
    image: Handle<UiNode>,
    material: Option<MaterialResource>,
    tiles: Vec<TileDefinition>,
    width_cells: Handle<UiNode>,
    height_cells: Handle<UiNode>,
    grid: Handle<UiNode>,
    size: Vector2<usize>,
}

pub enum ImportResult {
    None(TileSetImporter),
    Closed,
    TileSet(Vec<TileDefinition>),
}

impl TileSetImporter {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let width_cells;
        let height_cells;
        let toolbar = StackPanelBuilder::new(
            WidgetBuilder::new()
                .on_row(0)
                .with_margin(Thickness::uniform(1.0))
                .with_child(
                    TextBuilder::new(
                        WidgetBuilder::new().with_vertical_alignment(VerticalAlignment::Center),
                    )
                    .with_text("Width Cells")
                    .build(ctx),
                )
                .with_child({
                    width_cells = NumericUpDownBuilder::new(WidgetBuilder::new().with_width(120.0))
                        .with_min_value(0usize)
                        .build(ctx);
                    width_cells
                })
                .with_child(
                    TextBuilder::new(
                        WidgetBuilder::new().with_vertical_alignment(VerticalAlignment::Center),
                    )
                    .with_text("Height Cells")
                    .build(ctx),
                )
                .with_child({
                    height_cells =
                        NumericUpDownBuilder::new(WidgetBuilder::new().with_width(120.0))
                            .with_min_value(0usize)
                            .build(ctx);
                    height_cells
                }),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let grid;
        let image = ImageBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .with_allow_drop(true)
                .with_margin(Thickness::uniform(1.0))
                .with_child({
                    grid = GridBuilder::new(WidgetBuilder::new())
                        .draw_border(true)
                        .build(ctx);
                    grid
                }),
        )
        .with_checkerboard_background(true)
        .build(ctx);

        let import;
        let cancel;
        let buttons = StackPanelBuilder::new(
            WidgetBuilder::new()
                .on_row(2)
                .with_child({
                    import = make_button("Import", "Import the tile set.", false, ctx);
                    import
                })
                .with_child({
                    cancel = make_button("Cancel", "Cancel the importing.", true, ctx);
                    cancel
                }),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(toolbar)
                .with_child(image)
                .with_child(buttons),
        )
        .add_row(Row::strict(24.0))
        .add_row(Row::stretch())
        .add_row(Row::auto())
        .add_column(Column::stretch())
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(400.0).with_height(500.0))
            .open(false)
            .with_content(content)
            .with_title(WindowTitle::text("Import Tile Set"))
            .build(ctx);

        ctx.sender()
            .send(WindowMessage::open(
                window,
                MessageDirection::ToWidget,
                true,
                true,
            ))
            .unwrap();

        Self {
            window,
            import,
            cancel,
            image,
            material: None,
            tiles: Default::default(),
            width_cells,
            height_cells,
            grid,
            size: Default::default(),
        }
    }

    fn set_material(&mut self, material: Option<MaterialResource>, ui: &UserInterface) {
        self.material = material;

        ui.send_message(ImageMessage::texture(
            self.image,
            MessageDirection::ToWidget,
            self.material
                .as_ref()
                .and_then(|material| material.data_ref().texture("diffuseTexture"))
                .map(|texture| texture.into()),
        ));

        ui.send_message(WidgetMessage::enabled(
            self.import,
            MessageDirection::ToWidget,
            true,
        ));
    }

    fn update_tiles(&mut self) {
        self.tiles.clear();

        if let Some(material) = self.material.as_ref() {
            for y in 0..self.size.y {
                for x in 0..self.size.x {
                    self.tiles.push(TileDefinition {
                        material: material.clone(),
                        uv_rect: Rect::new(
                            x as f32 / self.size.x as f32,
                            y as f32 / self.size.y as f32,
                            1.0 / self.size.x as f32,
                            1.0 / self.size.y as f32,
                        ),
                        collider: Default::default(),
                        color: Default::default(),
                    });
                }
            }
        }
    }

    pub fn destroy(self, ui: &UserInterface) {
        ui.send_message(WidgetMessage::remove(
            self.window,
            MessageDirection::ToWidget,
        ));
    }

    pub fn handle_ui_message(mut self, message: &UiMessage, ui: &UserInterface) -> ImportResult {
        if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.window {
                self.destroy(ui);
                return ImportResult::Closed;
            }
        } else if let Some(WidgetMessage::Drop(dropped)) = message.data() {
            if message.destination() == self.image {
                if let Some(item) = ui.node(*dropped).cast::<AssetItem>() {
                    if let material @ Some(_) = item.resource::<Material>() {
                        self.set_material(material, ui);
                    }

                    if let Some(texture) = item.resource::<Texture>() {
                        let mut material = Material::standard_2d();

                        material
                            .set_texture(&"diffuseTexture".into(), Some(texture))
                            .unwrap();

                        self.set_material(
                            Some(MaterialResource::new_ok(ResourceKind::Embedded, material)),
                            ui,
                        );
                    }
                }
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.import {
                let tiles = std::mem::take(&mut self.tiles);
                self.destroy(ui);
                return ImportResult::TileSet(tiles);
            } else if message.destination() == self.cancel {
                ui.send_message(WindowMessage::close(
                    self.window,
                    MessageDirection::ToWidget,
                ));
            }
        } else if let Some(NumericUpDownMessage::<usize>::Value(value)) = message.data() {
            if message.direction() == MessageDirection::FromWidget {
                if message.destination() == self.width_cells {
                    ui.send_message(GridMessage::columns(
                        self.grid,
                        MessageDirection::ToWidget,
                        std::iter::repeat(Column::stretch()).take(*value).collect(),
                    ));

                    self.size.x = *value;
                    self.update_tiles();
                } else if message.destination() == self.height_cells {
                    ui.send_message(GridMessage::rows(
                        self.grid,
                        MessageDirection::ToWidget,
                        std::iter::repeat(Row::stretch()).take(*value).collect(),
                    ));

                    self.size.y = *value;
                    self.update_tiles();
                }
            }
        }

        ImportResult::None(self)
    }
}
