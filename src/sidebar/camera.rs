use crate::asset::AssetItem;
use crate::{
    gui::make_dropdown_list_option,
    make_relative_path,
    scene::commands::camera::{
        SetCameraPreviewCommand, SetColorGradingEnabledCommand, SetColorGradingLutCommand,
        SetExposureCommand, SetFovCommand, SetZFarCommand, SetZNearCommand,
    },
    send_sync_message,
    sidebar::{
        make_bool_input_field, make_f32_input_field, make_section, make_text_mark, COLUMN_WIDTH,
        ROW_HEIGHT,
    },
    Message,
};
use rg3d::gui::message::UiMessage;
use rg3d::gui::numeric::NumericUpDownMessage;
use rg3d::gui::{BuildContext, UiNode, UserInterface};
use rg3d::{
    core::{futures::executor::block_on, pool::Handle, scope_profile},
    engine::resource_manager::{ResourceManager, TextureImportOptions},
    gui::{
        dropdown_list::DropdownListBuilder,
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        message::{
            CheckBoxMessage, DropdownListMessage, ImageMessage, MessageDirection, UiMessageData,
            WidgetMessage,
        },
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
    },
    resource::texture::CompressionOptions,
    scene::{
        camera::{ColorGradingLut, Exposure},
        node::Node,
    },
    utils::into_gui_texture,
};
use std::sync::mpsc::Sender;

pub struct CameraSection {
    pub section: Handle<UiNode>,
    fov: Handle<UiNode>,
    z_near: Handle<UiNode>,
    z_far: Handle<UiNode>,
    sender: Sender<Message>,
    preview: Handle<UiNode>,
    exposure_kind: Handle<UiNode>,
    exposure_value: Handle<UiNode>,
    key_value: Handle<UiNode>,
    min_luminance: Handle<UiNode>,
    max_luminance: Handle<UiNode>,
    color_grading_lut: Handle<UiNode>,
    use_color_grading: Handle<UiNode>,
    manual_exposure_section: Handle<UiNode>,
    auto_exposure_section: Handle<UiNode>,
}

impl CameraSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let fov;
        let z_near;
        let z_far;
        let preview;
        let exposure_kind;
        let exposure_value;
        let key_value;
        let min_luminance;
        let max_luminance;
        let manual_exposure_section;
        let auto_exposure_section;
        let color_grading_lut;
        let use_color_grading;
        let section = make_section(
            "Camera Properties",
            StackPanelBuilder::new(
                WidgetBuilder::new()
                    .with_child(
                        GridBuilder::new(
                            WidgetBuilder::new()
                                .with_child(make_text_mark(ctx, "FOV", 0))
                                .with_child({
                                    fov = make_f32_input_field(
                                        ctx,
                                        0,
                                        0.0,
                                        std::f32::consts::PI,
                                        0.01,
                                    );
                                    fov
                                })
                                .with_child(make_text_mark(ctx, "Z Near", 1))
                                .with_child({
                                    z_near = make_f32_input_field(ctx, 1, 0.0, f32::MAX, 0.01);
                                    z_near
                                })
                                .with_child(make_text_mark(ctx, "Z Far", 2))
                                .with_child({
                                    z_far = make_f32_input_field(ctx, 2, 0.0, f32::MAX, 1.0);
                                    z_far
                                })
                                .with_child(make_text_mark(ctx, "Preview", 3))
                                .with_child({
                                    preview = make_bool_input_field(ctx, 3);
                                    preview
                                })
                                .with_child(make_text_mark(ctx, "Use Color Grading", 4))
                                .with_child({
                                    use_color_grading = make_bool_input_field(ctx, 4);
                                    use_color_grading
                                })
                                .with_child(make_text_mark(ctx, "Color Grading LUT", 5))
                                .with_child({
                                    color_grading_lut = ImageBuilder::new(
                                        WidgetBuilder::new()
                                            .on_row(5)
                                            .on_column(1)
                                            .with_allow_drop(true),
                                    )
                                    .build(ctx);
                                    color_grading_lut
                                })
                                .with_child(make_text_mark(ctx, "Exposure Kind", 6))
                                .with_child({
                                    exposure_kind = DropdownListBuilder::new(
                                        WidgetBuilder::new().on_row(6).on_column(1),
                                    )
                                    .with_close_on_selection(true)
                                    .with_items(vec![
                                        make_dropdown_list_option(ctx, "Auto"),
                                        make_dropdown_list_option(ctx, "Manual"),
                                    ])
                                    .build(ctx);
                                    exposure_kind
                                }),
                        )
                        .add_column(Column::strict(COLUMN_WIDTH))
                        .add_column(Column::stretch())
                        .add_row(Row::strict(ROW_HEIGHT))
                        .add_row(Row::strict(ROW_HEIGHT))
                        .add_row(Row::strict(ROW_HEIGHT))
                        .add_row(Row::strict(ROW_HEIGHT))
                        .add_row(Row::strict(ROW_HEIGHT))
                        .add_row(Row::strict(ROW_HEIGHT))
                        .add_row(Row::strict(ROW_HEIGHT))
                        .build(ctx),
                    )
                    .with_child(
                        StackPanelBuilder::new(
                            WidgetBuilder::new()
                                .with_child({
                                    manual_exposure_section = make_section(
                                        "Manual Exposure",
                                        GridBuilder::new(
                                            WidgetBuilder::new()
                                                .with_child(make_text_mark(
                                                    ctx,
                                                    "Exposure Value",
                                                    0,
                                                ))
                                                .with_child({
                                                    exposure_value = make_f32_input_field(
                                                        ctx,
                                                        0,
                                                        0.0,
                                                        f32::MAX,
                                                        1.0,
                                                    );
                                                    exposure_value
                                                }),
                                        )
                                        .add_column(Column::strict(COLUMN_WIDTH))
                                        .add_column(Column::stretch())
                                        .add_row(Row::strict(ROW_HEIGHT))
                                        .build(ctx),
                                        ctx,
                                    );
                                    manual_exposure_section
                                })
                                .with_child({
                                    auto_exposure_section = make_section(
                                        "Auto Exposure",
                                        GridBuilder::new(
                                            WidgetBuilder::new()
                                                .with_child(make_text_mark(ctx, "Key Value", 0))
                                                .with_child({
                                                    key_value = make_f32_input_field(
                                                        ctx,
                                                        0,
                                                        0.001,
                                                        f32::MAX,
                                                        1.0,
                                                    );
                                                    key_value
                                                })
                                                .with_child(make_text_mark(ctx, "Min Luminance", 1))
                                                .with_child({
                                                    min_luminance = make_f32_input_field(
                                                        ctx,
                                                        1,
                                                        0.001,
                                                        f32::MAX,
                                                        1.0,
                                                    );
                                                    min_luminance
                                                })
                                                .with_child(make_text_mark(ctx, "Max Luminance", 2))
                                                .with_child({
                                                    max_luminance = make_f32_input_field(
                                                        ctx,
                                                        2,
                                                        0.0,
                                                        f32::MAX,
                                                        1.0,
                                                    );
                                                    max_luminance
                                                }),
                                        )
                                        .add_column(Column::strict(COLUMN_WIDTH))
                                        .add_column(Column::stretch())
                                        .add_row(Row::strict(ROW_HEIGHT))
                                        .add_row(Row::strict(ROW_HEIGHT))
                                        .add_row(Row::strict(ROW_HEIGHT))
                                        .build(ctx),
                                        ctx,
                                    );
                                    auto_exposure_section
                                }),
                        )
                        .build(ctx),
                    ),
            )
            .build(ctx),
            ctx,
        );

        Self {
            section,
            fov,
            z_near,
            z_far,
            sender,
            preview,
            exposure_kind,
            exposure_value,
            key_value,
            min_luminance,
            max_luminance,
            auto_exposure_section,
            manual_exposure_section,
            color_grading_lut,
            use_color_grading,
        }
    }

    pub fn sync_to_model(&mut self, node: &Node, ui: &mut UserInterface) {
        send_sync_message(
            ui,
            WidgetMessage::visibility(self.section, MessageDirection::ToWidget, node.is_camera()),
        );

        if let Node::Camera(camera) = node {
            send_sync_message(
                ui,
                NumericUpDownMessage::value(self.fov, MessageDirection::ToWidget, camera.fov()),
            );

            send_sync_message(
                ui,
                NumericUpDownMessage::value(
                    self.z_near,
                    MessageDirection::ToWidget,
                    camera.z_near(),
                ),
            );

            send_sync_message(
                ui,
                NumericUpDownMessage::value(self.z_far, MessageDirection::ToWidget, camera.z_far()),
            );

            send_sync_message(
                ui,
                CheckBoxMessage::checked(
                    self.preview,
                    MessageDirection::ToWidget,
                    Some(camera.is_enabled()),
                ),
            );

            send_sync_message(
                ui,
                CheckBoxMessage::checked(
                    self.use_color_grading,
                    MessageDirection::ToWidget,
                    Some(camera.color_grading_enabled()),
                ),
            );

            send_sync_message(
                ui,
                ImageMessage::texture(
                    self.color_grading_lut,
                    MessageDirection::ToWidget,
                    camera
                        .color_grading_lut_ref()
                        .map(|lut| into_gui_texture(lut.unwrapped_lut().clone())),
                ),
            );

            match camera.exposure() {
                Exposure::Auto {
                    key_value,
                    min_luminance,
                    max_luminance,
                } => {
                    send_sync_message(
                        ui,
                        NumericUpDownMessage::value(
                            self.key_value,
                            MessageDirection::ToWidget,
                            key_value,
                        ),
                    );
                    send_sync_message(
                        ui,
                        NumericUpDownMessage::value(
                            self.min_luminance,
                            MessageDirection::ToWidget,
                            min_luminance,
                        ),
                    );
                    send_sync_message(
                        ui,
                        NumericUpDownMessage::value(
                            self.max_luminance,
                            MessageDirection::ToWidget,
                            max_luminance,
                        ),
                    );

                    send_sync_message(
                        ui,
                        DropdownListMessage::selection(
                            self.exposure_kind,
                            MessageDirection::ToWidget,
                            Some(0),
                        ),
                    );

                    send_sync_message(
                        ui,
                        WidgetMessage::visibility(
                            self.auto_exposure_section,
                            MessageDirection::ToWidget,
                            true,
                        ),
                    );
                    send_sync_message(
                        ui,
                        WidgetMessage::visibility(
                            self.manual_exposure_section,
                            MessageDirection::ToWidget,
                            false,
                        ),
                    );
                }
                Exposure::Manual(value) => {
                    send_sync_message(
                        ui,
                        NumericUpDownMessage::value(
                            self.exposure_value,
                            MessageDirection::ToWidget,
                            value,
                        ),
                    );

                    send_sync_message(
                        ui,
                        DropdownListMessage::selection(
                            self.exposure_kind,
                            MessageDirection::ToWidget,
                            Some(1),
                        ),
                    );

                    send_sync_message(
                        ui,
                        WidgetMessage::visibility(
                            self.auto_exposure_section,
                            MessageDirection::ToWidget,
                            false,
                        ),
                    );
                    send_sync_message(
                        ui,
                        WidgetMessage::visibility(
                            self.manual_exposure_section,
                            MessageDirection::ToWidget,
                            true,
                        ),
                    );
                }
            }
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        node: &Node,
        handle: Handle<Node>,
        ui: &UserInterface,
        resource_manager: ResourceManager,
    ) {
        scope_profile!();

        if let Node::Camera(camera) = node {
            match message.data() {
                UiMessageData::User(msg) if message.direction() == MessageDirection::FromWidget => {
                    if let Some(&NumericUpDownMessage::Value(value)) =
                        msg.cast::<NumericUpDownMessage<f32>>()
                    {
                        if message.destination() == self.fov && camera.fov().ne(&value) {
                            self.sender
                                .send(Message::do_scene_command(SetFovCommand::new(handle, value)))
                                .unwrap();
                        } else if message.destination() == self.z_far && camera.z_far().ne(&value) {
                            self.sender
                                .send(Message::do_scene_command(SetZFarCommand::new(
                                    handle, value,
                                )))
                                .unwrap();
                        } else if message.destination() == self.z_near && camera.z_near().ne(&value)
                        {
                            self.sender
                                .send(Message::do_scene_command(SetZNearCommand::new(
                                    handle, value,
                                )))
                                .unwrap();
                        } else if message.destination() == self.exposure_value {
                            self.sender
                                .send(Message::do_scene_command(SetExposureCommand::new(
                                    handle,
                                    Exposure::Manual(value),
                                )))
                                .unwrap();
                        } else if message.destination() == self.key_value {
                            let mut current_auto_exposure = camera.exposure().clone();
                            if let Exposure::Auto {
                                ref mut key_value, ..
                            } = current_auto_exposure
                            {
                                *key_value = value;
                            }

                            self.sender
                                .send(Message::do_scene_command(SetExposureCommand::new(
                                    handle,
                                    current_auto_exposure,
                                )))
                                .unwrap();
                        } else if message.destination() == self.min_luminance {
                            let mut current_auto_exposure = camera.exposure().clone();
                            if let Exposure::Auto {
                                ref mut min_luminance,
                                ..
                            } = current_auto_exposure
                            {
                                *min_luminance = value;
                            }

                            self.sender
                                .send(Message::do_scene_command(SetExposureCommand::new(
                                    handle,
                                    current_auto_exposure,
                                )))
                                .unwrap();
                        } else if message.destination() == self.min_luminance {
                            let mut current_auto_exposure = camera.exposure().clone();
                            if let Exposure::Auto {
                                ref mut max_luminance,
                                ..
                            } = current_auto_exposure
                            {
                                *max_luminance = value;
                            }

                            self.sender
                                .send(Message::do_scene_command(SetExposureCommand::new(
                                    handle,
                                    current_auto_exposure,
                                )))
                                .unwrap();
                        }
                    }
                }
                UiMessageData::CheckBox(CheckBoxMessage::Check(value)) => {
                    if message.destination() == self.preview
                        && camera.is_enabled().ne(&value.unwrap())
                    {
                        self.sender
                            .send(Message::do_scene_command(SetCameraPreviewCommand::new(
                                handle,
                                value.unwrap_or(false),
                            )))
                            .unwrap();
                    } else if message.destination() == self.use_color_grading {
                        self.sender
                            .send(Message::do_scene_command(
                                SetColorGradingEnabledCommand::new(
                                    handle,
                                    value.unwrap_or_default(),
                                ),
                            ))
                            .unwrap();
                    }
                }
                UiMessageData::DropdownList(DropdownListMessage::SelectionChanged(Some(index))) => {
                    if message.destination() == self.exposure_kind {
                        let exposure = match index {
                            0 => Exposure::default(),
                            1 => Exposure::Manual(1.0),
                            _ => unreachable!(),
                        };

                        self.sender
                            .send(Message::do_scene_command(SetExposureCommand::new(
                                handle, exposure,
                            )))
                            .unwrap();
                    }
                }
                &UiMessageData::Widget(WidgetMessage::Drop(dropped)) => {
                    if message.destination() == self.color_grading_lut {
                        if let Some(item) = ui.node(dropped).cast::<AssetItem>() {
                            let relative_path = make_relative_path(&item.path);

                            match block_on(ColorGradingLut::new(
                                resource_manager.request_texture(
                                    relative_path,
                                    Some(
                                        TextureImportOptions::default()
                                            .with_compression(CompressionOptions::NoCompression),
                                    ),
                                ),
                            )) {
                                Ok(lut) => {
                                    self.sender
                                        .send(Message::do_scene_command(
                                            SetColorGradingLutCommand::new(handle, Some(lut)),
                                        ))
                                        .unwrap();
                                }
                                Err(e) => self
                                    .sender
                                    .send(Message::Log(format!(
                                        "Failed to load color grading look-up texture. Reason: {}",
                                        e
                                    )))
                                    .unwrap(),
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
