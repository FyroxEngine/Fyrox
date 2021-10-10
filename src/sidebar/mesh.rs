use crate::{
    gui::make_dropdown_list_option,
    scene::commands::mesh::{
        SetMeshCastShadowsCommand, SetMeshDecalLayerIndexCommand, SetMeshRenderPathCommand,
    },
    send_sync_message,
    sidebar::{
        make_bool_input_field, make_int_input_field, make_section, make_text_mark, COLUMN_WIDTH,
        ROW_HEIGHT,
    },
    Message,
};
use rg3d::gui::dropdown_list::DropdownList;
use rg3d::gui::message::UiMessage;
use rg3d::gui::numeric::NumericUpDownMessage;
use rg3d::gui::{BuildContext, UiNode, UserInterface};
use rg3d::{
    core::{pool::Handle, scope_profile},
    gui::{
        button::ButtonBuilder,
        dropdown_list::DropdownListBuilder,
        grid::{Column, GridBuilder, Row},
        message::{
            ButtonMessage, CheckBoxMessage, DropdownListMessage, MessageDirection, UiMessageData,
            WidgetMessage,
        },
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
        Thickness,
    },
    scene::{mesh::RenderPath, node::Node},
};
use std::sync::mpsc::Sender;

pub struct MeshSection {
    pub section: Handle<UiNode>,
    cast_shadows: Handle<UiNode>,
    render_path: Handle<UiNode>,
    decal_layer_index: Handle<UiNode>,
    sender: Sender<Message>,
    surfaces_list: Handle<UiNode>,
    current_surface: Option<usize>,
    surface_section: Handle<UiNode>,
    edit_material: Handle<UiNode>,
}

impl MeshSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let cast_shadows;
        let render_path;
        let decal_layer_index;
        let surfaces_list;
        let surface_section;
        let edit_material;
        let section = make_section(
            "Mesh Properties",
            StackPanelBuilder::new(
                WidgetBuilder::new()
                    .with_child(
                        GridBuilder::new(
                            WidgetBuilder::new()
                                .with_child(make_text_mark(ctx, "Cast Shadows", 0))
                                .with_child({
                                    cast_shadows = make_bool_input_field(ctx, 0);
                                    cast_shadows
                                })
                                .with_child(make_text_mark(ctx, "Render Path", 1))
                                .with_child({
                                    render_path = DropdownListBuilder::new(
                                        WidgetBuilder::new()
                                            .on_row(1)
                                            .on_column(1)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_close_on_selection(true)
                                    .with_items(vec![
                                        make_dropdown_list_option(ctx, "Deferred"),
                                        make_dropdown_list_option(ctx, "Forward"),
                                    ])
                                    .build(ctx);
                                    render_path
                                })
                                .with_child(make_text_mark(ctx, "Decal Layer Index", 2))
                                .with_child({
                                    decal_layer_index = make_int_input_field(ctx, 2, 0, 255, 1);
                                    decal_layer_index
                                })
                                .with_child(make_text_mark(ctx, "Surfaces", 3))
                                .with_child({
                                    surfaces_list = DropdownListBuilder::new(
                                        WidgetBuilder::new().on_row(3).on_column(1),
                                    )
                                    .with_close_on_selection(true)
                                    .build(ctx);
                                    surfaces_list
                                }),
                        )
                        .add_column(Column::strict(COLUMN_WIDTH))
                        .add_column(Column::stretch())
                        .add_row(Row::strict(ROW_HEIGHT))
                        .add_row(Row::strict(ROW_HEIGHT))
                        .add_row(Row::strict(ROW_HEIGHT))
                        .add_row(Row::strict(ROW_HEIGHT))
                        .build(ctx),
                    )
                    .with_child({
                        surface_section = make_section(
                            "Surface Properties",
                            GridBuilder::new(
                                WidgetBuilder::new()
                                    .with_child(make_text_mark(ctx, "Material", 0))
                                    .with_child({
                                        edit_material = ButtonBuilder::new(
                                            WidgetBuilder::new().on_row(0).on_column(1),
                                        )
                                        .with_text("...")
                                        .build(ctx);
                                        edit_material
                                    }),
                            )
                            .add_column(Column::strict(COLUMN_WIDTH))
                            .add_column(Column::stretch())
                            .add_row(Row::strict(ROW_HEIGHT))
                            .build(ctx),
                            ctx,
                        );
                        surface_section
                    }),
            )
            .build(ctx),
            ctx,
        );

        Self {
            section,
            cast_shadows,
            render_path,
            sender,
            decal_layer_index,
            surfaces_list,
            edit_material,
            surface_section,
            current_surface: None,
        }
    }

    pub fn sync_to_model(&mut self, node: &Node, ui: &mut UserInterface) {
        send_sync_message(
            ui,
            WidgetMessage::visibility(self.section, MessageDirection::ToWidget, node.is_mesh()),
        );

        if let Node::Mesh(mesh) = node {
            send_sync_message(
                ui,
                CheckBoxMessage::checked(
                    self.cast_shadows,
                    MessageDirection::ToWidget,
                    Some(mesh.cast_shadows()),
                ),
            );

            let variant = match mesh.render_path() {
                RenderPath::Deferred => 0,
                RenderPath::Forward => 1,
            };

            send_sync_message(
                ui,
                WidgetMessage::visibility(
                    self.surface_section,
                    MessageDirection::ToWidget,
                    self.current_surface.is_some(),
                ),
            );

            send_sync_message(
                ui,
                DropdownListMessage::selection(
                    self.render_path,
                    MessageDirection::ToWidget,
                    Some(variant),
                ),
            );

            send_sync_message(
                ui,
                NumericUpDownMessage::value(
                    self.decal_layer_index,
                    MessageDirection::ToWidget,
                    mesh.decal_layer_index() as f32,
                ),
            );

            if mesh.surfaces().len()
                != ui
                    .node(self.surfaces_list)
                    .cast::<DropdownList>()
                    .unwrap()
                    .items()
                    .len()
            {
                let items = mesh
                    .surfaces()
                    .iter()
                    .enumerate()
                    .map(|(n, _)| {
                        make_dropdown_list_option(&mut ui.build_ctx(), &format!("Surface {}", n))
                    })
                    .collect::<Vec<_>>();

                let selection = if items.is_empty() { None } else { Some(0) };

                send_sync_message(
                    ui,
                    DropdownListMessage::items(
                        self.surfaces_list,
                        MessageDirection::ToWidget,
                        items,
                    ),
                );

                // This has to be sent without `send_sync_message` because we need to get response message
                // in `handle_ui_message`.
                ui.send_message(DropdownListMessage::selection(
                    self.surfaces_list,
                    MessageDirection::ToWidget,
                    selection,
                ));
            }
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, node: &Node, handle: Handle<Node>) {
        scope_profile!();

        if let Node::Mesh(mesh) = node {
            match message.data() {
                UiMessageData::CheckBox(CheckBoxMessage::Check(value)) => {
                    let value = value.unwrap_or(false);
                    if message.destination() == self.cast_shadows && mesh.cast_shadows().ne(&value)
                    {
                        self.sender
                            .send(Message::do_scene_command(SetMeshCastShadowsCommand::new(
                                handle, value,
                            )))
                            .unwrap();
                    }
                }
                &UiMessageData::DropdownList(DropdownListMessage::SelectionChanged(selection)) => {
                    if message.destination() == self.render_path {
                        if let Some(selection) = selection {
                            let new_render_path = match selection {
                                0 => RenderPath::Deferred,
                                1 => RenderPath::Forward,
                                _ => unreachable!(),
                            };
                            if new_render_path != mesh.render_path() {
                                self.sender
                                    .send(Message::do_scene_command(SetMeshRenderPathCommand::new(
                                        handle,
                                        new_render_path,
                                    )))
                                    .unwrap();
                            }
                        }
                    } else if message.destination() == self.surfaces_list {
                        self.current_surface = selection;

                        self.sender.send(Message::SyncToModel).unwrap();
                    }
                }
                UiMessageData::Button(ButtonMessage::Click) => {
                    if message.destination() == self.edit_material {
                        if let Some(current_surface) = self.current_surface {
                            if let Some(surface) = mesh.surfaces().get(current_surface) {
                                self.sender
                                    .send(Message::OpenMaterialEditor(surface.material().clone()))
                                    .unwrap();
                            }
                        }
                    }
                }
                UiMessageData::User(msg) => {
                    if let Some(&NumericUpDownMessage::Value(value)) =
                        msg.cast::<NumericUpDownMessage<f32>>()
                    {
                        if message.destination() == self.decal_layer_index {
                            let index = value.clamp(0.0, 255.0) as u8;

                            if index != mesh.decal_layer_index() {
                                self.sender
                                    .send(Message::do_scene_command(
                                        SetMeshDecalLayerIndexCommand::new(handle, index),
                                    ))
                                    .unwrap();
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
