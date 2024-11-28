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

use std::{fmt::Display, sync::Arc};

use fyrox::{
    core::algebra::Vector2,
    gui::{grid::SizeMode, stack_panel::StackPanelBuilder, text::TextBuilder, window::Window},
    scene::tilemap::{tileset::TileSet, TileResource, *},
};

use crate::{
    asset::item::AssetItem,
    command::{Command, CommandGroup, SetPropertyCommand},
    fyrox::{
        asset::untyped::UntypedResource,
        core::color::Color,
        core::{parking_lot::Mutex, pool::Handle, TypeUuidProvider, Uuid},
        fxhash::FxHashSet,
        graph::{BaseSceneGraph, SceneGraph, SceneGraphNode},
        gui::{
            border::BorderBuilder,
            brush::Brush,
            button::{Button, ButtonBuilder, ButtonMessage},
            decorator::{DecoratorBuilder, DecoratorMessage},
            dropdown_list::{DropdownListBuilder, DropdownListMessage},
            grid::{Column, GridBuilder, Row},
            image::ImageBuilder,
            message::{MessageDirection, UiMessage},
            utils::make_simple_tooltip,
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            wrap_panel::WrapPanelBuilder,
            BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
            VerticalAlignment,
        },
        scene::{
            node::Node,
            tilemap::{brush::TileMapBrush, tileset::TileSetResource, TileMap},
        },
    },
    gui::make_dropdown_list_option,
    load_image,
    message::MessageSender,
    plugins::tilemap::{
        palette::{PaletteMessage, PaletteWidget, PaletteWidgetBuilder, TileViewMessage},
        DrawingMode, TileMapInteractionMode,
    },
    scene::{commands::GameSceneContext, container::EditorSceneEntry},
    Message,
};

use super::*;

#[derive(Clone)]
enum BrushEntry {
    FromTileMap(TileResource),
    FromOther(TileResource),
}

impl BrushEntry {
    #[inline]
    fn resource(&self) -> &TileResource {
        match self {
            BrushEntry::FromTileMap(r) => r,
            BrushEntry::FromOther(r) => r,
        }
    }
}

impl Display for BrushEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrushEntry::FromTileMap(r) => {
                if let Some(p) = r.path() {
                    write!(f, "{} (from tile map)", p.to_string_lossy())
                } else {
                    write!(f, "Missing Path")
                }
            }
            BrushEntry::FromOther(r) => {
                if let Some(p) = r.path() {
                    p.to_string_lossy().fmt(f)
                } else {
                    write!(f, "Missing Path")
                }
            }
        }
    }
}

pub struct TileMapPanel {
    pub state: TileDrawStateRef,
    pub brush: TileResource,
    pub window: Handle<UiNode>,
    pub tile_set: Option<TileSetResource>,
    brushes: Vec<BrushEntry>,
    tile_set_name: Handle<UiNode>,
    preview: Handle<UiNode>,
    pages: Handle<UiNode>,
    palette: Handle<UiNode>,
    active_brush_selector: Handle<UiNode>,
    drawing_modes_panel: Handle<UiNode>,
    draw_button: Handle<UiNode>,
    erase_button: Handle<UiNode>,
    flood_fill_button: Handle<UiNode>,
    pick_button: Handle<UiNode>,
    rect_fill_button: Handle<UiNode>,
    nine_slice_button: Handle<UiNode>,
    line_button: Handle<UiNode>,
    random_button: Handle<UiNode>,
    left_button: Handle<UiNode>,
    right_button: Handle<UiNode>,
    flip_x_button: Handle<UiNode>,
    flip_y_button: Handle<UiNode>,
}

impl TileMapPanel {
    pub fn new(ctx: &mut BuildContext, state: TileDrawStateRef, sender: MessageSender) -> Self {
        let tile_set_name = TextBuilder::new(WidgetBuilder::new().on_row(0)).build(ctx);
        let active_brush_selector = DropdownListBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .with_min_size(Vector2::new(250.0, 20.0))
                .with_height(20.0),
        )
        .build(ctx);
        let preview = PanelPreviewBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .with_min_size(Vector2::new(80.0, 100.0)),
            state.clone(),
        )
        .build(ctx);
        let pages = PaletteWidgetBuilder::new(
            WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
            sender.clone(),
            state.clone(),
        )
        .with_kind(TilePaletteStage::Pages)
        .build(ctx);
        let palette = PaletteWidgetBuilder::new(
            WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
            sender.clone(),
            state.clone(),
        )
        .with_kind(TilePaletteStage::Tiles)
        .build(ctx);
        let preview_frame = BorderBuilder::new(
            WidgetBuilder::new()
                .on_column(1)
                .with_foreground(Brush::Solid(Color::BLACK))
                .with_child(preview),
        )
        .build(ctx);
        let pages_frame = BorderBuilder::new(
            WidgetBuilder::new()
                .on_row(3)
                .with_margin(Thickness::uniform(2.0))
                .with_foreground(Brush::Solid(Color::BLACK))
                .with_child(pages),
        )
        .build(ctx);
        let palette_frame = BorderBuilder::new(
            WidgetBuilder::new()
                .on_row(4)
                .with_margin(Thickness::uniform(2.0))
                .with_foreground(Brush::Solid(Color::BLACK))
                .with_child(palette),
        )
        .build(ctx);

        let width = 20.0;
        let height = 20.0;
        let draw_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            BRUSH_IMAGE.clone(),
            "Draw with active brush.",
            Some(0),
        );
        let erase_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            ERASER_IMAGE.clone(),
            "Erase with active brush.",
            Some(1),
        );
        let flood_fill_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            FILL_IMAGE.clone(),
            "Flood fill with tiles from current brush.",
            Some(2),
        );
        let pick_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            PICK_IMAGE.clone(),
            "Pick tiles for drawing from the tile map.",
            Some(3),
        );
        let rect_fill_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            RECT_FILL_IMAGE.clone(),
            "Fill the rectangle using the current brush.",
            Some(4),
        );
        let nine_slice_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            NINE_SLICE_IMAGE.clone(),
            "Draw rectangles with fixed corners, but stretchable sides.",
            Some(5),
        );
        let line_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            LINE_IMAGE.clone(),
            "Draw a line using tiles from the given brush.",
            Some(6),
        );
        let left_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            TURN_LEFT_IMAGE.clone(),
            "Rotate left 90 degrees.",
            Some(7),
        );
        let right_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            TURN_RIGHT_IMAGE.clone(),
            "Rotate right 90 degrees.",
            Some(8),
        );
        let flip_x_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            FLIP_X_IMAGE.clone(),
            "Flip along x axis.",
            Some(9),
        );
        let flip_y_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            FLIP_Y_IMAGE.clone(),
            "Flip along y axis.",
            Some(10),
        );
        let random_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            RANDOM_IMAGE.clone(),
            "Toggle random fill mode.",
            Some(11),
        );

        let drawing_modes_panel = WrapPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(draw_button)
                .with_child(erase_button)
                .with_child(flood_fill_button)
                .with_child(pick_button)
                .with_child(rect_fill_button)
                .with_child(nine_slice_button)
                .with_child(line_button),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);
        let modifiers_panel = WrapPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(left_button)
                .with_child(right_button)
                .with_child(flip_x_button)
                .with_child(flip_y_button)
                .with_child(random_button),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let toolbar = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(drawing_modes_panel)
                .with_child(modifiers_panel)
                .with_child(active_brush_selector),
        )
        .build(ctx);

        let header = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(2)
                .with_child(toolbar)
                .with_child(preview_frame),
        )
        .add_row(Row::auto())
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(tile_set_name)
                .with_child(active_brush_selector)
                .with_child(header)
                .with_child(pages_frame)
                .with_child(palette_frame),
        )
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_row(Row::generic(SizeMode::Stretch, 200.0))
        .add_column(Column::stretch())
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(400.0).with_height(600.0))
            .open(false)
            .with_title(WindowTitle::text("Tile Map Control Panel"))
            .with_content(content)
            .build(ctx);

        Self {
            state,
            tile_set: None,
            brush: TileResource::Empty,
            brushes: Vec::new(),
            window,
            tile_set_name,
            preview,
            pages,
            palette,
            active_brush_selector,
            drawing_modes_panel,
            draw_button,
            erase_button,
            flood_fill_button,
            pick_button,
            rect_fill_button,
            nine_slice_button,
            line_button,
            left_button,
            right_button,
            flip_x_button,
            flip_y_button,
            random_button,
        }
    }

    pub fn align(&self, relative_to: Handle<UiNode>, ui: &UserInterface) {
        if ui.node(self.window).visibility() {
            ui.send_message(WidgetMessage::align(
                self.window,
                MessageDirection::ToWidget,
                relative_to,
                HorizontalAlignment::Right,
                VerticalAlignment::Top,
                Thickness::uniform(2.0),
            ));
            ui.send_message(WidgetMessage::topmost(
                self.window,
                MessageDirection::ToWidget,
            ));
            ui.send_message(WidgetMessage::focus(
                ui.node(self.window).cast::<Window>().unwrap().content,
                MessageDirection::ToWidget,
            ));
        } else {
            ui.send_message(WindowMessage::open_and_align(
                self.window,
                MessageDirection::ToWidget,
                relative_to,
                HorizontalAlignment::Right,
                VerticalAlignment::Top,
                Thickness::uniform(2.0),
                false,
                true,
            ));
        }
    }

    pub fn to_top(&self, ui: &UserInterface) {
        ui.send_message(WidgetMessage::topmost(
            self.window,
            MessageDirection::ToWidget,
        ));
        ui.send_message(WidgetMessage::focus(
            ui.node(self.window).cast::<Window>().unwrap().content,
            MessageDirection::ToWidget,
        ));
        ui.send_message(WidgetMessage::visibility(
            self.window,
            MessageDirection::ToWidget,
            true,
        ));
    }

    pub fn destroy(self, ui: &UserInterface) {
        ui.send_message(WidgetMessage::remove(
            self.window,
            MessageDirection::ToWidget,
        ));
    }

    pub fn add_brush(&mut self, brush: TileResource, _ui: &mut UserInterface) {
        self.brushes.push(BrushEntry::FromOther(brush));
    }

    pub fn set_tile_map(&mut self, tile_map: &TileMap, ui: &mut UserInterface) {
        // TODO
    }

    pub fn set_resource(&mut self, resource: TileResource, ui: &mut UserInterface) {
        self.brush = resource;
        ui.send_message(PaletteMessage::set_page(
            self.pages,
            MessageDirection::ToWidget,
            self.brush.clone(),
            None,
        ));
        ui.send_message(PaletteMessage::set_page(
            self.palette,
            MessageDirection::ToWidget,
            self.brush.clone(),
            None,
        ));
    }

    pub fn set_focus(&mut self, handle: TileDefinitionHandle, ui: &mut UserInterface) {
        let mut state = self.state.lock_mut();
        state.selection.source = SelectionSource::Widget(self.palette);
        let tiles = state.selection_tiles_mut();
        tiles.clear();
        if let Some(handle) =
            self.brush
                .get_tile_handle(TilePaletteStage::Tiles, handle.page(), handle.tile())
        {
            tiles.insert(handle.tile(), handle);
        }
        ui.send_message(PaletteMessage::set_page(
            self.pages,
            MessageDirection::ToWidget,
            self.brush.clone(),
            Some(handle.page()),
        ));
        ui.send_message(PaletteMessage::set_page(
            self.palette,
            MessageDirection::ToWidget,
            self.brush.clone(),
            Some(handle.page()),
        ));
        ui.send_message(PaletteMessage::center(
            self.pages,
            MessageDirection::ToWidget,
            handle.page(),
        ));
        ui.send_message(PaletteMessage::center(
            self.palette,
            MessageDirection::ToWidget,
            handle.tile(),
        ));
    }

    pub fn set_visibility(&self, visible: bool, ui: &mut UserInterface) {
        ui.send_message(WidgetMessage::visibility(
            self.window,
            MessageDirection::ToWidget,
            visible,
        ));
    }

    fn make_brush_entries(&self, ctx: &mut BuildContext) -> Vec<Handle<UiNode>> {
        let Some(tile_set) = &self.tile_set else {
            return Vec::default();
        };
        let make = make_dropdown_list_option;
        let mut tile_set_name: String = "tile set: ".into();
        tile_set_name.push_str(tile_set.kind().to_string().as_str());
        std::iter::once(make(ctx, tile_set_name.as_str()))
            .chain(
                self.brushes
                    .iter()
                    .map(|brush| make(ctx, &brush.to_string())),
            )
            .collect()
    }

    fn find_brush_index(&self, brush: &TileResource) -> Option<usize> {
        self.brushes
            .iter()
            .position(|b| b.resource() == brush)
            .map(|i| i + 1)
    }

    fn get_brush_at_index(&self, index: usize) -> Option<TileResource> {
        if index == 0 {
            Some(TileResource::TileSet(self.tile_set.clone()?))
        } else {
            self.brushes
                .get(index - 1)
                .map(BrushEntry::resource)
                .cloned()
        }
    }

    fn handle_button(&mut self, button: Handle<UiNode>, ui: &mut UserInterface) {
        if button == self.draw_button {
            self.state.lock_mut().drawing_mode = DrawingMode::Draw;
        } else if button == self.erase_button {
            self.state.lock_mut().drawing_mode = DrawingMode::Erase;
        } else if button == self.flood_fill_button {
            self.state.lock_mut().drawing_mode = DrawingMode::FloodFill;
        } else if button == self.pick_button {
            self.state.lock_mut().drawing_mode = DrawingMode::Pick;
        } else if button == self.rect_fill_button {
            self.state.lock_mut().drawing_mode = DrawingMode::RectFill;
        } else if button == self.nine_slice_button {
            self.state.lock_mut().drawing_mode = DrawingMode::NineSlice;
        } else if button == self.line_button {
            self.state.lock_mut().drawing_mode = DrawingMode::Line;
        } else if button == self.random_button {
            let mut state = self.state.lock_mut();
            state.random_mode = !state.random_mode;
        } else if button == self.left_button {
            self.state.lock_mut().stamp.rotate(1);
        } else if button == self.right_button {
            self.state.lock_mut().stamp.rotate(-1);
        } else if button == self.flip_x_button {
            self.state.lock_mut().stamp.x_flip();
        } else if button == self.flip_y_button {
            self.state.lock_mut().stamp.y_flip();
        }
    }

    pub fn handle_ui_message(
        mut self,
        message: &UiMessage,
        ui: &mut UserInterface,
        tile_map_handle: Handle<Node>,
        tile_map: Option<&TileMap>,
        sender: &MessageSender,
        editor_scene: Option<&mut EditorSceneEntry>,
    ) -> Option<Self> {
        if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.window {
                self.destroy(ui);
                return None;
            }
        }
        if let Some(ButtonMessage::Click) = message.data() {
            self.handle_button(message.destination(), ui);
        } else if let Some(PaletteMessage::SetPage { .. }) = message.data() {
            if message.destination() == self.pages
                && message.direction() == MessageDirection::FromWidget
            {
                ui.send_message(
                    message
                        .clone()
                        .with_destination(self.palette)
                        .with_direction(MessageDirection::ToWidget),
                );
            }
        } else if let Some(WidgetMessage::Drop(dropped)) = message.data() {
            if ui.is_node_child_of(message.destination(), self.window) {
                let tile_res = if let Some(item) = ui.node(*dropped).cast::<AssetItem>() {
                    if let Some(brush) = item.resource::<TileMapBrush>() {
                        Some(TileResource::Brush(brush))
                    } else {
                        item.resource::<TileSet>().map(TileResource::TileSet)
                    }
                } else {
                    None
                };
                if let Some(tile_res) = tile_res {
                    self.add_brush(tile_res, ui);
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

    pub fn sync_to_state(&self, ui: &mut UserInterface) {
        fn highlight_tool_button(button: Handle<UiNode>, highlight: bool, ui: &UserInterface) {
            let decorator = *ui.try_get_of_type::<Button>(button).unwrap().decorator;
            ui.send_message(DecoratorMessage::select(
                decorator,
                MessageDirection::ToWidget,
                highlight,
            ));
        }
        fn highlight_all_except(
            button: Handle<UiNode>,
            buttons: &[Handle<UiNode>],
            highlight: bool,
            ui: &UserInterface,
        ) {
            for other_button in buttons {
                if *other_button == button {
                    highlight_tool_button(*other_button, highlight, ui);
                } else {
                    highlight_tool_button(*other_button, !highlight, ui);
                }
            }
        }
        fn highlight_all(buttons: &[Handle<UiNode>], highlight: bool, ui: &UserInterface) {
            for button in buttons {
                highlight_tool_button(*button, highlight, ui);
            }
        }
        let buttons = [
            self.pick_button,
            self.draw_button,
            self.erase_button,
            self.flood_fill_button,
            self.rect_fill_button,
            self.nine_slice_button,
            self.line_button,
        ];
        let state = self.state.lock();
        highlight_tool_button(self.random_button, state.random_mode, ui);
        match state.drawing_mode {
            DrawingMode::Draw => {
                highlight_all_except(self.draw_button, &buttons, true, ui);
            }
            DrawingMode::Erase => {
                highlight_all_except(self.erase_button, &buttons, true, ui);
            }
            DrawingMode::FloodFill => {
                highlight_all_except(self.flood_fill_button, &buttons, true, ui);
            }
            DrawingMode::Pick { .. } => {
                highlight_all_except(self.pick_button, &buttons, true, ui);
            }
            DrawingMode::RectFill { .. } => {
                highlight_all_except(self.rect_fill_button, &buttons, true, ui);
            }
            DrawingMode::NineSlice { .. } => {
                highlight_all_except(self.nine_slice_button, &buttons, true, ui);
            }
            DrawingMode::Line { .. } => {
                highlight_all_except(self.line_button, &buttons, true, ui);
            }
            _ => {
                highlight_all(&buttons, false, ui);
            }
        }
        ui.send_message(PaletteMessage::sync_to_state(
            self.preview,
            MessageDirection::ToWidget,
        ));
        ui.send_message(PaletteMessage::sync_to_state(
            self.pages,
            MessageDirection::ToWidget,
        ));
        ui.send_message(PaletteMessage::sync_to_state(
            self.palette,
            MessageDirection::ToWidget,
        ));
    }

    pub fn sync_to_model(&self, ui: &mut UserInterface, tile_map: &TileMap) {
        let items = self.make_brush_entries(&mut ui.build_ctx());
        ui.send_message(DropdownListMessage::items(
            self.active_brush_selector,
            MessageDirection::ToWidget,
            items,
        ));
    }
}
