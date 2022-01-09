use crate::{
    gui::make_dropdown_list_option_with_height, load_image, DropdownListBuilder, GameEngine,
    InteractionModeKind, Message, SettingsSectionKind,
};
use fyrox::{
    core::{algebra::Vector2, color::Color, math::Rect, pool::Handle},
    gui::{
        border::BorderBuilder,
        brush::Brush,
        button::{ButtonBuilder, ButtonMessage},
        canvas::CanvasBuilder,
        dropdown_list::DropdownListMessage,
        formatted_text::WrapMode,
        grid::{Column, GridBuilder, Row},
        image::{ImageBuilder, ImageMessage},
        message::{MessageDirection, MouseButton, UiMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
    },
    resource::texture::Texture,
    scene::camera::Projection,
    utils::into_gui_texture,
};
use std::sync::mpsc::Sender;

pub struct SceneViewer {
    frame: Handle<UiNode>,
    window: Handle<UiNode>,
    pub last_mouse_pos: Option<Vector2<f32>>,
    pub click_mouse_pos: Option<Vector2<f32>>,
    selection_frame: Handle<UiNode>,
    // Side bar stuff
    select_mode: Handle<UiNode>,
    move_mode: Handle<UiNode>,
    rotate_mode: Handle<UiNode>,
    scale_mode: Handle<UiNode>,
    navmesh_mode: Handle<UiNode>,
    terrain_mode: Handle<UiNode>,
    camera_projection: Handle<UiNode>,
    sender: Sender<Message>,
}

fn make_interaction_mode_button(
    ctx: &mut BuildContext,
    image: &[u8],
    tooltip: &str,
) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_tooltip(
                BorderBuilder::new(
                    WidgetBuilder::new()
                        .with_max_size(Vector2::new(300.0, f32::MAX))
                        .with_child(
                            TextBuilder::new(
                                WidgetBuilder::new().with_margin(Thickness::uniform(2.0)),
                            )
                            .with_wrap(WrapMode::Word)
                            .with_text(tooltip)
                            .build(ctx),
                        ),
                )
                .build(ctx),
            )
            .with_margin(Thickness::uniform(1.0)),
    )
    .with_content(
        ImageBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .with_width(32.0)
                .with_height(32.0),
        )
        .with_opt_texture(load_image(image))
        .build(ctx),
    )
    .build(ctx)
}

impl SceneViewer {
    pub fn new(engine: &mut GameEngine, sender: Sender<Message>) -> Self {
        let ctx = &mut engine.user_interface.build_ctx();

        let select_mode_tooltip = "Select Object(s) - Shortcut: [1]\n\nSelection interaction mode \
        allows you to select an object by a single left mouse button click or multiple objects using either \
        frame selection (click and drag) or by holding Ctrl+Click";

        let move_mode_tooltip =
            "Move Object(s) - Shortcut: [2]\n\nMovement interaction mode allows you to move selected \
        objects. Keep in mind that movement always works in local coordinates!\n\n\
        This also allows you to select an object or add an object to current selection using Ctrl+Click";

        let rotate_mode_tooltip =
            "Rotate Object(s) - Shortcut: [3]\n\nRotation interaction mode allows you to rotate selected \
        objects. Keep in mind that rotation always works in local coordinates!\n\n\
        This also allows you to select an object or add an object to current selection using Ctrl+Click";

        let scale_mode_tooltip =
            "Scale Object(s) - Shortcut: [4]\n\nScaling interaction mode allows you to scale selected \
        objects. Keep in mind that scaling always works in local coordinates!\n\n\
        This also allows you to select an object or add an object to current selection using Ctrl+Click";

        let navmesh_mode_tooltip =
            "Edit Navmesh\n\nNavmesh edit mode allows you to modify selected \
        navigational mesh.";

        let terrain_mode_tooltip =
            "Edit Terrain\n\nTerrain edit mode allows you to modify selected \
        terrain.";

        let frame;
        let select_mode;
        let move_mode;
        let rotate_mode;
        let scale_mode;
        let navmesh_mode;
        let terrain_mode;
        let selection_frame;
        let camera_projection;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .can_close(false)
            .can_minimize(false)
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .on_row(0)
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .with_margin(Thickness::uniform(1.0))
                                    .with_horizontal_alignment(HorizontalAlignment::Right)
                                    .with_child({
                                        camera_projection = DropdownListBuilder::new(
                                            WidgetBuilder::new().with_width(150.0),
                                        )
                                        .with_items(vec![
                                            make_dropdown_list_option_with_height(
                                                ctx,
                                                "Perspective (3D)",
                                                22.0,
                                            ),
                                            make_dropdown_list_option_with_height(
                                                ctx,
                                                "Orthographic (2D)",
                                                22.0,
                                            ),
                                        ])
                                        .with_selected(0)
                                        .build(ctx);
                                        camera_projection
                                    }),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        )
                        .with_child(
                            GridBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(1)
                                    .with_child({
                                        frame = ImageBuilder::new(
                                            WidgetBuilder::new()
                                                .on_row(0)
                                                .on_column(1)
                                                .with_allow_drop(true),
                                        )
                                        .with_flip(true)
                                        .build(ctx);
                                        frame
                                    })
                                    .with_child(
                                        CanvasBuilder::new(
                                            WidgetBuilder::new().on_column(1).with_child({
                                                selection_frame = BorderBuilder::new(
                                                    WidgetBuilder::new()
                                                        .with_visibility(false)
                                                        .with_background(Brush::Solid(
                                                            Color::from_rgba(255, 255, 255, 40),
                                                        ))
                                                        .with_foreground(Brush::Solid(
                                                            Color::opaque(0, 255, 0),
                                                        )),
                                                )
                                                .with_stroke_thickness(Thickness::uniform(1.0))
                                                .build(ctx);
                                                selection_frame
                                            }),
                                        )
                                        .build(ctx),
                                    )
                                    .with_child(
                                        StackPanelBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0))
                                                .on_row(0)
                                                .on_column(0)
                                                .with_child({
                                                    select_mode = make_interaction_mode_button(
                                                        ctx,
                                                        include_bytes!(
                                                            "../resources/embed/select.png"
                                                        ),
                                                        select_mode_tooltip,
                                                    );
                                                    select_mode
                                                })
                                                .with_child({
                                                    move_mode = make_interaction_mode_button(
                                                        ctx,
                                                        include_bytes!(
                                                            "../resources/embed/move_arrow.png"
                                                        ),
                                                        move_mode_tooltip,
                                                    );
                                                    move_mode
                                                })
                                                .with_child({
                                                    rotate_mode = make_interaction_mode_button(
                                                        ctx,
                                                        include_bytes!(
                                                            "../resources/embed/rotate_arrow.png"
                                                        ),
                                                        rotate_mode_tooltip,
                                                    );
                                                    rotate_mode
                                                })
                                                .with_child({
                                                    scale_mode = make_interaction_mode_button(
                                                        ctx,
                                                        include_bytes!(
                                                            "../resources/embed/scale_arrow.png"
                                                        ),
                                                        scale_mode_tooltip,
                                                    );
                                                    scale_mode
                                                })
                                                .with_child({
                                                    navmesh_mode = make_interaction_mode_button(
                                                        ctx,
                                                        include_bytes!(
                                                            "../resources/embed/navmesh.png"
                                                        ),
                                                        navmesh_mode_tooltip,
                                                    );
                                                    navmesh_mode
                                                })
                                                .with_child({
                                                    terrain_mode = make_interaction_mode_button(
                                                        ctx,
                                                        include_bytes!(
                                                            "../resources/embed/terrain.png"
                                                        ),
                                                        terrain_mode_tooltip,
                                                    );
                                                    terrain_mode
                                                }),
                                        )
                                        .build(ctx),
                                    ),
                            )
                            .add_row(Row::stretch())
                            .add_column(Column::auto())
                            .add_column(Column::stretch())
                            .build(ctx),
                        ),
                )
                .add_row(Row::strict(25.0))
                .add_row(Row::stretch())
                .add_column(Column::stretch())
                .build(ctx),
            )
            .with_title(WindowTitle::text("Scene Preview"))
            .build(ctx);

        Self {
            sender,
            window,
            frame,
            last_mouse_pos: None,
            move_mode,
            rotate_mode,
            scale_mode,
            selection_frame,
            select_mode,
            navmesh_mode,
            terrain_mode,
            camera_projection,
            click_mouse_pos: None,
        }
    }
}

impl SceneViewer {
    pub fn window(&self) -> Handle<UiNode> {
        self.window
    }

    pub fn frame(&self) -> Handle<UiNode> {
        self.frame
    }

    pub fn selection_frame(&self) -> Handle<UiNode> {
        self.selection_frame
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, ui: &UserInterface) {
        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.scale_mode {
                self.sender
                    .send(Message::SetInteractionMode(InteractionModeKind::Scale))
                    .unwrap();
            } else if message.destination() == self.rotate_mode {
                self.sender
                    .send(Message::SetInteractionMode(InteractionModeKind::Rotate))
                    .unwrap();
            } else if message.destination() == self.move_mode {
                self.sender
                    .send(Message::SetInteractionMode(InteractionModeKind::Move))
                    .unwrap();
            } else if message.destination() == self.select_mode {
                self.sender
                    .send(Message::SetInteractionMode(InteractionModeKind::Select))
                    .unwrap();
            } else if message.destination() == self.navmesh_mode {
                self.sender
                    .send(Message::SetInteractionMode(InteractionModeKind::Navmesh))
                    .unwrap();
            } else if message.destination() == self.terrain_mode {
                self.sender
                    .send(Message::SetInteractionMode(InteractionModeKind::Terrain))
                    .unwrap();
            }
        } else if let Some(WidgetMessage::MouseDown { button, .. }) =
            message.data::<WidgetMessage>()
        {
            if ui.is_node_child_of(message.destination(), self.move_mode)
                && *button == MouseButton::Right
            {
                self.sender
                    .send(Message::OpenSettings(SettingsSectionKind::MoveModeSettings))
                    .unwrap();
            }
        } else if let Some(DropdownListMessage::SelectionChanged(Some(index))) = message.data() {
            if message.destination() == self.camera_projection {
                if *index == 0 {
                    self.sender
                        .send(Message::SetEditorCameraProjection(Projection::Perspective(
                            Default::default(),
                        )))
                        .unwrap()
                } else {
                    self.sender
                        .send(Message::SetEditorCameraProjection(
                            Projection::Orthographic(Default::default()),
                        ))
                        .unwrap()
                }
            }
        }
    }

    pub fn set_render_target(&self, ui: &UserInterface, render_target: Option<Texture>) {
        ui.send_message(ImageMessage::texture(
            self.frame,
            MessageDirection::ToWidget,
            render_target.map(into_gui_texture),
        ));
    }

    pub fn set_title(&self, ui: &UserInterface, title: String) {
        ui.send_message(WindowMessage::title(
            self.window,
            MessageDirection::ToWidget,
            WindowTitle::Text(title),
        ));
    }

    pub fn frame_bounds(&self, ui: &UserInterface) -> Rect<f32> {
        ui.node(self.frame).screen_bounds()
    }
}
