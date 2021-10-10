use crate::{
    scene::EditorScene,
    settings::{make_bool_input_field, make_text_mark},
    GameEngine,
};
use rg3d::gui::message::UiMessage;
use rg3d::gui::numeric::NumericUpDownMessage;
use rg3d::gui::{BuildContext, UiNode, UserInterface};
use rg3d::{
    core::pool::Handle,
    gui::{
        color::ColorFieldBuilder,
        grid::{Column, GridBuilder, Row},
        message::{CheckBoxMessage, ColorFieldMessage, MessageDirection, UiMessageData},
        numeric::NumericUpDownBuilder,
        widget::WidgetBuilder,
        Thickness,
    },
    renderer::QualitySettings,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub struct GraphicsSettings {
    pub quality: QualitySettings,
    pub z_near: f32,
    pub z_far: f32,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            quality: Default::default(),
            z_near: 0.025,
            z_far: 128.0,
        }
    }
}

pub struct GraphicsSection {
    pub section: Handle<UiNode>,
    ssao: Handle<UiNode>,
    point_shadows: Handle<UiNode>,
    spot_shadows: Handle<UiNode>,
    ambient_color: Handle<UiNode>,
    light_scatter: Handle<UiNode>,
    near_plane: Handle<UiNode>,
    far_plane: Handle<UiNode>,
    parallax_mapping: Handle<UiNode>,
}

impl GraphicsSection {
    pub fn new(ctx: &mut BuildContext, settings: &GraphicsSettings) -> Self {
        let ssao;
        let ambient_color;
        let point_shadows;
        let spot_shadows;
        let light_scatter;
        let near_plane;
        let far_plane;
        let parallax_mapping;

        let section = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(make_text_mark(ctx, "SSAO", 0))
                .with_child({
                    ssao = make_bool_input_field(ctx, 0, settings.quality.use_ssao);
                    ssao
                })
                .with_child(make_text_mark(ctx, "Ambient Color", 1))
                .with_child({
                    ambient_color =
                        ColorFieldBuilder::new(WidgetBuilder::new().on_column(1).on_row(1))
                            .build(ctx);
                    ambient_color
                })
                .with_child(make_text_mark(ctx, "Point Shadows", 2))
                .with_child({
                    point_shadows =
                        make_bool_input_field(ctx, 2, settings.quality.point_shadows_enabled);
                    point_shadows
                })
                .with_child(make_text_mark(ctx, "Spot Shadows", 3))
                .with_child({
                    spot_shadows =
                        make_bool_input_field(ctx, 3, settings.quality.spot_shadows_enabled);
                    spot_shadows
                })
                .with_child(make_text_mark(ctx, "Light Scatter", 4))
                .with_child({
                    light_scatter =
                        make_bool_input_field(ctx, 4, settings.quality.light_scatter_enabled);
                    light_scatter
                })
                .with_child(make_text_mark(ctx, "Near Plane", 5))
                .with_child({
                    near_plane = NumericUpDownBuilder::new(
                        WidgetBuilder::new()
                            .on_column(1)
                            .on_row(5)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .with_value(settings.z_near)
                    .with_min_value(0.001)
                    .build(ctx);
                    near_plane
                })
                .with_child(make_text_mark(ctx, "Far Plane", 6))
                .with_child({
                    far_plane = NumericUpDownBuilder::new(
                        WidgetBuilder::new()
                            .on_column(1)
                            .on_row(6)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .with_min_value(1.0)
                    .with_value(settings.z_far)
                    .build(ctx);
                    far_plane
                })
                .with_child(make_text_mark(ctx, "Parallax Mapping", 7))
                .with_child({
                    parallax_mapping =
                        make_bool_input_field(ctx, 7, settings.quality.use_parallax_mapping);
                    parallax_mapping
                }),
        )
        .add_row(Row::strict(25.0))
        .add_row(Row::strict(25.0))
        .add_row(Row::strict(25.0))
        .add_row(Row::strict(25.0))
        .add_row(Row::strict(25.0))
        .add_row(Row::strict(25.0))
        .add_row(Row::strict(25.0))
        .add_row(Row::strict(25.0))
        .add_row(Row::stretch())
        .add_row(Row::stretch())
        .add_column(Column::strict(120.0))
        .add_column(Column::stretch())
        .build(ctx);

        Self {
            section,
            ssao,
            ambient_color,
            point_shadows,
            spot_shadows,
            light_scatter,
            near_plane,
            far_plane,
            parallax_mapping,
        }
    }

    pub fn sync_to_model(&self, ui: &UserInterface, settings: &GraphicsSettings) {
        ui.send_message(NumericUpDownMessage::value(
            self.near_plane,
            MessageDirection::ToWidget,
            settings.z_near,
        ));
        ui.send_message(NumericUpDownMessage::value(
            self.far_plane,
            MessageDirection::ToWidget,
            settings.z_far,
        ));

        let sync_check_box = |handle: Handle<UiNode>, value: bool| {
            ui.send_message(CheckBoxMessage::checked(
                handle,
                MessageDirection::ToWidget,
                Some(value),
            ));
        };

        sync_check_box(self.ssao, settings.quality.use_ssao);
        sync_check_box(self.point_shadows, settings.quality.point_shadows_enabled);
        sync_check_box(self.spot_shadows, settings.quality.spot_shadows_enabled);
        sync_check_box(self.light_scatter, settings.quality.light_scatter_enabled);
        sync_check_box(self.parallax_mapping, settings.quality.use_parallax_mapping);
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
        settings: &mut GraphicsSettings,
    ) {
        match message.data() {
            UiMessageData::CheckBox(CheckBoxMessage::Check(check)) => {
                let value = check.unwrap_or(false);
                if message.destination() == self.ssao {
                    settings.quality.use_ssao = value;
                } else if message.destination() == self.point_shadows {
                    settings.quality.point_shadows_enabled = value;
                } else if message.destination() == self.spot_shadows {
                    settings.quality.spot_shadows_enabled = value;
                } else if message.destination() == self.light_scatter {
                    settings.quality.light_scatter_enabled = value;
                } else if message.destination() == self.parallax_mapping {
                    settings.quality.use_parallax_mapping = value;
                }
            }
            UiMessageData::ColorField(msg)
                if message.direction() == MessageDirection::FromWidget =>
            {
                // TODO: Should not be here!
                if message.destination() == self.ambient_color {
                    if let ColorFieldMessage::Color(color) = *msg {
                        engine.scenes[editor_scene.scene].ambient_lighting_color = color;
                    }
                }
            }
            UiMessageData::User(msg) if message.direction() == MessageDirection::FromWidget => {
                if let Some(&NumericUpDownMessage::Value(value)) =
                    msg.cast::<NumericUpDownMessage<f32>>()
                {
                    if message.destination() == self.near_plane {
                        settings.z_near = value;
                    } else if message.destination() == self.far_plane {
                        settings.z_far = value;
                    }
                }
            }
            _ => {}
        }
    }
}
