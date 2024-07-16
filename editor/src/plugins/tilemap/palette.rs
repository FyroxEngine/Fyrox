use crate::plugins::tilemap::tileset::TileSetTileView;
use crate::{
    absm::selectable::{Selectable, SelectableMessage},
    fyrox::{
        core::{
            algebra::{Matrix3, Point2, Vector2},
            color::Color,
            math::{OptionRect, Rect},
            pool::Handle,
            reflect::prelude::*,
            type_traits::prelude::*,
            visitor::prelude::*,
        },
        graph::{BaseSceneGraph, SceneGraph},
        gui::{
            brush::Brush,
            define_constructor, define_widget_deref,
            draw::{CommandTexture, Draw, DrawingContext},
            message::{KeyCode, MessageDirection, MouseButton, UiMessage},
            widget::{Widget, WidgetBuilder, WidgetMessage},
            BuildContext, Control, UiNode, UserInterface,
        },
        scene::tilemap::{
            brush::{BrushTile, TileMapBrush},
            tileset::TileSetResource,
        },
    },
};
use fyrox::scene::tilemap::tileset::TileDefinitionHandle;
use std::ops::{Deref, DerefMut};

#[derive(Debug, PartialEq, Clone)]
pub enum PaletteMessage {
    // Direction: FromWidget
    ActiveBrush(TileMapBrush),
    // Direction: ToWidget
    AddTile(Handle<UiNode>),
    // Direction: ToWidget
    RemoveTile(Handle<UiNode>),
    // Direction: FromWidget
    MoveTiles(Vec<(Uuid, Vector2<i32>)>),
    // Direction: FromWidget
    DeleteTiles(Vec<Uuid>),
    // Direction: FromWidget
    DuplicateTiles(Vec<(Uuid, Vector2<i32>)>),
    InsertTile {
        definition_id: TileDefinitionHandle,
        position: Vector2<i32>,
    },
}

impl PaletteMessage {
    define_constructor!(PaletteMessage:ActiveBrush => fn active_brush(TileMapBrush), layout: false);
    define_constructor!(PaletteMessage:AddTile => fn add_tile(Handle<UiNode>), layout: false);
    define_constructor!(PaletteMessage:RemoveTile => fn remove_tile(Handle<UiNode>), layout: false);
    define_constructor!(PaletteMessage:MoveTiles => fn move_tiles(Vec<(Uuid, Vector2<i32>)>), layout: false);
    define_constructor!(PaletteMessage:DeleteTiles => fn delete_tiles(Vec<Uuid>), layout: false);
    define_constructor!(PaletteMessage:DuplicateTiles => fn duplicate_tiles(Vec<(Uuid, Vector2<i32>)>), layout: false);
    define_constructor!(PaletteMessage:InsertTile  => fn insert_tile(definition_id: TileDefinitionHandle, position: Vector2<i32>), layout: false);
}

#[derive(Debug, Clone, PartialEq, Visit, Reflect, Default)]
pub(super) struct Entry {
    pub node: Handle<UiNode>,
    pub initial_position: Vector2<f32>,
}

#[derive(Debug, Clone, PartialEq, Visit, Reflect, Default)]
pub(super) struct DragContext {
    initial_cursor_position: Vector2<f32>,
    entries: Vec<Entry>,
}

#[derive(Clone, Debug, Visit, Reflect, PartialEq)]
enum Mode {
    None,
    Panning {
        initial_view_position: Vector2<f32>,
        click_position: Vector2<f32>,
    },
    Selecting {
        click_position: Vector2<f32>,
    },
    Dragging(DragContext),
}

#[derive(Clone, Debug, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "5356a864-c026-4bd7-a4b1-30bacf77d8fa")]
pub struct PaletteWidget {
    widget: Widget,
    pub tiles: Vec<Handle<UiNode>>,
    view_position: Vector2<f32>,
    zoom: f32,
    tile_size: Vector2<f32>,
    selection: Vec<Handle<UiNode>>,
    mode: Mode,
}

define_widget_deref!(PaletteWidget);

impl PaletteWidget {
    pub fn point_to_local_space(&self, point: Vector2<f32>) -> Vector2<f32> {
        self.visual_transform()
            .try_inverse()
            .unwrap_or_default()
            .transform_point(&Point2::from(point))
            .coords
    }

    pub fn update_transform(&self, ui: &UserInterface) {
        let transform =
            Matrix3::new_translation(&-self.view_position) * Matrix3::new_scaling(self.zoom);

        ui.send_message(WidgetMessage::layout_transform(
            self.handle(),
            MessageDirection::ToWidget,
            transform,
        ));
    }

    fn make_drag_context(&self, ui: &UserInterface) -> DragContext {
        for selected in self.selection.iter() {
            ui.send_message(WidgetMessage::topmost(
                *selected,
                MessageDirection::ToWidget,
            ));
        }

        DragContext {
            initial_cursor_position: self.point_to_local_space(ui.cursor_position()),
            entries: self
                .selection
                .iter()
                .map(|n| Entry {
                    node: *n,
                    initial_position: ui.node(*n).actual_local_position(),
                })
                .collect(),
        }
    }

    fn tiles_to_brush(&self, tiles: &[Handle<UiNode>], ui: &UserInterface) -> TileMapBrush {
        let mut tiles = tiles
            .iter()
            .filter_map(|h| ui.try_get_of_type::<BrushTileView>(*h))
            .map(|view| BrushTile {
                definition_handle: view.definition_handle,
                local_position: view.local_position,
                id: view.id,
            })
            .collect::<Vec<_>>();

        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        for tile in tiles.iter() {
            if tile.local_position.x < min_x {
                min_x = tile.local_position.x;
            }
            if tile.local_position.y < min_y {
                min_y = tile.local_position.y;
            }
        }
        let origin = Vector2::new(min_x, min_y);

        for tile in tiles.iter_mut() {
            tile.local_position -= origin;
            // Flip the position, because world's coordinate system is X-left Y-up, but palette has
            // X-right Y-down.
            tile.local_position = -tile.local_position;
        }

        TileMapBrush { tiles }
    }

    fn set_selection(&mut self, new_selection: &[Handle<UiNode>], ui: &UserInterface) {
        if self.selection != new_selection {
            for &child in self
                .children()
                .iter()
                .filter(|n| ui.node(**n).query_component::<Selectable>().is_some())
            {
                ui.send_message(
                    SelectableMessage::select(
                        child,
                        MessageDirection::ToWidget,
                        new_selection.contains(&child),
                    )
                    .with_handled(true),
                );
            }

            self.selection = new_selection.to_vec();

            ui.send_message(PaletteMessage::active_brush(
                self.handle(),
                MessageDirection::FromWidget,
                self.tiles_to_brush(&self.selection, ui),
            ));

            // Make sure to update dragging context if we're in Drag mode.
            if let Mode::Dragging(_) = self.mode {
                self.mode = Mode::Dragging(self.make_drag_context(ui));
            }
        }
    }

    fn local_to_grid_pos(&self, pos: Vector2<f32>) -> Vector2<i32> {
        Vector2::new(
            (pos.x / self.tile_size.x) as i32,
            (pos.y / self.tile_size.y) as i32,
        )
    }
}

impl Control for PaletteWidget {
    fn measure_override(&self, ui: &UserInterface, _available_size: Vector2<f32>) -> Vector2<f32> {
        for child_handle in self.widget.children() {
            ui.measure_node(*child_handle, self.tile_size);
        }

        Vector2::default()
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        for &child_handle in self.widget.children() {
            if let Some(tile) = ui.try_get_of_type::<BrushTileView>(child_handle) {
                ui.arrange_node(
                    child_handle,
                    &Rect::new(
                        tile.local_position.x as f32 * self.tile_size.x,
                        tile.local_position.y as f32 * self.tile_size.y,
                        self.tile_size.x,
                        self.tile_size.y,
                    ),
                );
            }
        }

        final_size
    }

    fn draw(&self, ctx: &mut DrawingContext) {
        let grid_size = 9999.0;

        let grid_bounds = self
            .widget
            .bounding_rect()
            .inflate(grid_size, grid_size)
            .translate(Vector2::new(grid_size * 0.5, grid_size * 0.5));
        ctx.push_rect_filled(&grid_bounds, None);
        ctx.commit(
            self.clip_bounds(),
            self.widget.background(),
            CommandTexture::None,
            None,
        );

        ctx.push_grid(self.zoom, self.tile_size, grid_bounds);
        ctx.commit(
            self.clip_bounds(),
            Brush::Solid(Color::repeat_opaque(60)),
            CommandTexture::None,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(SelectableMessage::Select(true)) = message.data() {
            if message.direction() == MessageDirection::FromWidget && !message.handled() {
                let selected_node = message.destination();

                let new_selection = if ui.keyboard_modifiers().control {
                    let mut selection = self.selection.clone();
                    selection.push(selected_node);
                    selection
                } else {
                    vec![selected_node]
                };

                self.set_selection(&new_selection, ui);
            }
        } else if let Some(WidgetMessage::MouseDown { pos, button }) = message.data() {
            if *button == MouseButton::Middle {
                self.mode = Mode::Panning {
                    initial_view_position: self.view_position,
                    click_position: *pos,
                };
                ui.capture_mouse(self.handle());
            } else if *button == MouseButton::Left && !message.handled() {
                if message.destination() != self.handle {
                    if ui.keyboard_modifiers().alt {
                        self.mode = Mode::Dragging(self.make_drag_context(ui));
                    } else {
                        self.mode = Mode::Selecting {
                            click_position: *pos,
                        };
                    }
                } else {
                    self.set_selection(&[], ui);
                }
            }
        } else if let Some(WidgetMessage::MouseUp { button, .. }) = message.data() {
            if *button == MouseButton::Middle {
                if matches!(self.mode, Mode::Panning { .. }) {
                    self.mode = Mode::None;
                    ui.release_mouse_capture();
                }
            } else if *button == MouseButton::Left {
                match self.mode {
                    Mode::Selecting { .. } => {
                        self.mode = Mode::None;
                    }
                    Mode::Dragging(ref drag_context) => {
                        ui.send_message(PaletteMessage::move_tiles(
                            self.handle,
                            MessageDirection::FromWidget,
                            drag_context
                                .entries
                                .iter()
                                .map(|entry| {
                                    let tile_view =
                                        ui.try_get_of_type::<BrushTileView>(entry.node).unwrap();

                                    (tile_view.id, tile_view.local_position)
                                })
                                .collect::<Vec<_>>(),
                        ));

                        self.mode = Mode::None;
                    }
                    _ => (),
                }
            }
        } else if let Some(WidgetMessage::MouseMove { pos, .. }) = message.data() {
            let local_cursor_pos = self.point_to_local_space(*pos);

            match self.mode {
                Mode::None => {}
                Mode::Selecting { click_position } => {
                    let local_click_position = self.point_to_local_space(click_position);
                    let grid_click_position = self.local_to_grid_pos(local_click_position);
                    let current_grid_position = self.local_to_grid_pos(local_cursor_pos);

                    let mut rect = OptionRect::default();

                    rect.push(grid_click_position);
                    rect.push(current_grid_position);

                    let selection_bounds = rect.unwrap();

                    let mut selection = Vec::new();
                    for tile in self.tiles.iter() {
                        let tile_ref = ui.try_get_of_type::<BrushTileView>(*tile).unwrap();
                        if selection_bounds.contains(tile_ref.local_position) {
                            selection.push(*tile);
                        }
                    }
                    self.set_selection(&selection, ui);
                }
                Mode::Dragging(ref drag_context) => {
                    for entry in drag_context.entries.iter() {
                        let new_position = entry.initial_position
                            + (local_cursor_pos - drag_context.initial_cursor_position);

                        let grid_position = self.local_to_grid_pos(new_position);

                        ui.send_message(TileViewMessage::local_position(
                            entry.node,
                            MessageDirection::ToWidget,
                            grid_position,
                        ));
                    }
                }
                Mode::Panning {
                    initial_view_position,
                    click_position,
                } => {
                    self.view_position = initial_view_position + (*pos - click_position);
                    self.update_transform(ui);
                }
            }
        } else if let Some(WidgetMessage::MouseWheel { amount, pos }) = message.data() {
            let cursor_pos = (*pos - self.screen_position()).scale(self.zoom);

            self.zoom = (self.zoom + 0.1 * amount).clamp(0.2, 2.0);

            let new_cursor_pos = (*pos - self.screen_position()).scale(self.zoom);

            self.view_position -= (new_cursor_pos - cursor_pos).scale(self.zoom);

            self.update_transform(ui);
        } else if let Some(msg) = message.data::<PaletteMessage>() {
            if message.direction() == MessageDirection::ToWidget {
                match msg {
                    PaletteMessage::AddTile(tile) => {
                        if !self.tiles.contains(tile) {
                            ui.send_message(WidgetMessage::link(
                                *tile,
                                MessageDirection::ToWidget,
                                self.handle,
                            ));
                            self.tiles.push(*tile);
                        }
                    }
                    PaletteMessage::RemoveTile(tile) => {
                        if let Some(position) = self
                            .tiles
                            .iter()
                            .position(|existing_tile| existing_tile == tile)
                        {
                            ui.send_message(WidgetMessage::remove(
                                *tile,
                                MessageDirection::ToWidget,
                            ));

                            self.tiles.remove(position);

                            if let Some(pos_in_selection) = self
                                .selection
                                .iter()
                                .position(|selected| *selected == *tile)
                            {
                                self.selection.remove(pos_in_selection);
                            }
                        }
                    }
                    _ => (),
                }
            }
        } else if let Some(WidgetMessage::KeyDown(key)) = message.data() {
            if *key == KeyCode::Delete && !message.handled() {
                let tiles = self
                    .tiles_to_brush(&self.selection, ui)
                    .tiles
                    .into_iter()
                    .map(|tile| tile.id)
                    .collect::<Vec<_>>();

                if !tiles.is_empty() {
                    ui.send_message(PaletteMessage::delete_tiles(
                        self.handle,
                        MessageDirection::FromWidget,
                        tiles,
                    ));

                    message.set_handled(true);
                }
            };
        } else if let Some(WidgetMessage::Drop(widget)) = message.data() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::FromWidget
            {
                if let Some(tile_set_tile) = ui.try_get_of_type::<TileSetTileView>(*widget) {
                    let local_cursor_position = self.point_to_local_space(ui.cursor_position());
                    let grid_cursor_position = self.local_to_grid_pos(local_cursor_position);

                    ui.send_message(PaletteMessage::insert_tile(
                        self.handle,
                        MessageDirection::FromWidget,
                        tile_set_tile.definition_handle,
                        grid_cursor_position,
                    ));
                }
            }
        }
    }
}

pub struct PaletteWidgetBuilder {
    widget_builder: WidgetBuilder,
    tiles: Vec<Handle<UiNode>>,
}

impl PaletteWidgetBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            tiles: Default::default(),
        }
    }

    pub fn with_tiles(mut self, tiles: Vec<Handle<UiNode>>) -> Self {
        self.tiles = tiles;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        ctx.add_node(UiNode::new(PaletteWidget {
            widget: self
                .widget_builder
                .with_allow_drop(true)
                .with_clip_to_bounds(false)
                .with_children(self.tiles.iter().cloned())
                .build(),
            tiles: self.tiles,
            view_position: Default::default(),
            zoom: 1.0,
            tile_size: Vector2::repeat(32.0),
            selection: Default::default(),
            mode: Mode::None,
        }))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum TileViewMessage {
    LocalPosition(Vector2<i32>),
}

impl TileViewMessage {
    define_constructor!(TileViewMessage:LocalPosition => fn local_position(Vector2<i32>), layout: false);
}

#[derive(Clone, Debug, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "c8ff0080-fb29-480a-8a88-59ee4c58d60d")]
pub struct BrushTileView {
    widget: Widget,
    #[component(include)]
    selectable: Selectable,
    definition_handle: TileDefinitionHandle,
    local_position: Vector2<i32>,
    tile_set: TileSetResource,
}

define_widget_deref!(BrushTileView);

impl Control for BrushTileView {
    fn draw(&self, drawing_context: &mut DrawingContext) {
        let tile_set = self.tile_set.data_ref();
        if let Some(tile_definition) = tile_set.tiles.try_borrow(self.definition_handle) {
            if let Some(texture) = tile_definition
                .material
                .data_ref()
                .texture("diffuseTexture")
            {
                drawing_context.push_rect_filled(
                    &self.bounding_rect(),
                    Some(&[
                        Vector2::new(
                            tile_definition.uv_rect.position.x,
                            tile_definition.uv_rect.position.y,
                        ),
                        Vector2::new(
                            tile_definition.uv_rect.position.x + tile_definition.uv_rect.size.x,
                            tile_definition.uv_rect.position.y,
                        ),
                        Vector2::new(
                            tile_definition.uv_rect.position.x + tile_definition.uv_rect.size.x,
                            tile_definition.uv_rect.position.y + tile_definition.uv_rect.size.y,
                        ),
                        Vector2::new(
                            tile_definition.uv_rect.position.x,
                            tile_definition.uv_rect.position.y + tile_definition.uv_rect.size.y,
                        ),
                    ]),
                );
                drawing_context.commit(
                    self.clip_bounds(),
                    Brush::Solid(Color::WHITE),
                    CommandTexture::Texture(texture.into()),
                    None,
                );
            }
        }

        if self.selectable.selected {
            drawing_context.push_rect(&self.bounding_rect(), 1.0);
            drawing_context.commit(
                self.clip_bounds(),
                (*self.foreground).clone(),
                CommandTexture::None,
                None,
            );
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);
        self.selectable
            .handle_routed_message(self.handle, ui, message);
        if let Some(TileViewMessage::LocalPosition(position)) = message.data() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
            {
                self.local_position = *position;
                self.invalidate_layout();
            }
        }
    }
}

pub struct BrushTileViewBuilder {
    widget_builder: WidgetBuilder,
    definition_id: TileDefinitionHandle,
    local_position: Vector2<i32>,
    tile_set: TileSetResource,
}

impl BrushTileViewBuilder {
    pub fn new(tile_set: TileSetResource, widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            definition_id: Default::default(),
            local_position: Default::default(),
            tile_set,
        }
    }

    pub fn with_position(mut self, position: Vector2<i32>) -> Self {
        self.local_position = position;
        self
    }

    pub fn with_definition_id(mut self, id: TileDefinitionHandle) -> Self {
        self.definition_id = id;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        ctx.add_node(UiNode::new(BrushTileView {
            widget: self.widget_builder.build(),
            selectable: Default::default(),
            definition_handle: self.definition_id,
            local_position: self.local_position,
            tile_set: self.tile_set,
        }))
    }
}
