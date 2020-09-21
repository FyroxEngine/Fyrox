use crate::scene::SetSpriteColorCommand;
use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    scene::{
        DeleteBodyCommand, EditorScene, MoveNodeCommand, RotateNodeCommand, ScaleNodeCommand,
        SceneCommand, SetBodyCommand, SetFovCommand, SetLightCastShadowsCommand,
        SetLightColorCommand, SetLightScatterCommand, SetLightScatterEnabledCommand,
        SetNameCommand, SetParticleSystemAccelerationCommand, SetPointLightRadiusCommand,
        SetSpotLightDistanceCommand, SetSpotLightFalloffAngleDeltaCommand,
        SetSpotLightHotspotCommand, SetSpriteRotationCommand, SetSpriteSizeCommand, SetZFarCommand,
        SetZNearCommand,
    },
    GameEngine, Message,
};
use rg3d::gui::color::ColorFieldBuilder;
use rg3d::gui::message::ColorFieldMessage;
use rg3d::{
    core::{
        math::{
            quat::{Quat, RotationOrder},
            vec3::Vec3,
        },
        pool::Handle,
    },
    gui::{
        border::BorderBuilder,
        check_box::CheckBoxBuilder,
        decorator::DecoratorBuilder,
        dropdown_list::DropdownListBuilder,
        grid::{Column, GridBuilder, Row},
        message::{
            CheckBoxMessage, DropdownListMessage, MessageDirection, NumericUpDownMessage,
            TextBoxMessage, UiMessageData, Vec3EditorMessage, WidgetMessage,
        },
        numeric::NumericUpDownBuilder,
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        text_box::TextBoxBuilder,
        vec::Vec3EditorBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        HorizontalAlignment, Thickness, VerticalAlignment,
    },
    physics::{
        convex_shape::{BoxShape, CapsuleShape, ConvexShape, SphereShape},
        rigid_body::RigidBody,
    },
    scene::{light::Light, node::Node},
};
use std::sync::mpsc::Sender;

const ROW_HEIGHT: f32 = 25.0;
const COLUMN_WIDTH: f32 = 120.0;

pub struct SideBar {
    pub window: Handle<UiNode>,
    scroll_viewer: Handle<UiNode>,
    node_name: Handle<UiNode>,
    position: Handle<UiNode>,
    rotation: Handle<UiNode>,
    scale: Handle<UiNode>,
    sender: Sender<Message>,
    body: Handle<UiNode>,
    light_section: LightSection,
    camera_section: CameraSection,
    particle_system_section: ParticleSystemSection,
    sprite_section: SpriteSection,
}

fn make_text_mark(ctx: &mut BuildContext, text: &str, row: usize) -> Handle<UiNode> {
    TextBuilder::new(
        WidgetBuilder::new()
            .with_vertical_alignment(VerticalAlignment::Center)
            .with_margin(Thickness::left(4.0))
            .on_row(row)
            .on_column(0),
    )
    .with_text(text)
    .build(ctx)
}

fn make_vec3_input_field(ctx: &mut BuildContext, row: usize) -> Handle<UiNode> {
    Vec3EditorBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(1.0))
            .on_row(row)
            .on_column(1),
    )
    .build(ctx)
}

fn make_f32_input_field(ctx: &mut BuildContext, row: usize) -> Handle<UiNode> {
    NumericUpDownBuilder::new(
        WidgetBuilder::new()
            .on_row(row)
            .with_margin(Thickness::uniform(1.0))
            .on_column(1),
    )
    .build(ctx)
}

fn make_color_input_field(ctx: &mut BuildContext, row: usize) -> Handle<UiNode> {
    ColorFieldBuilder::new(
        WidgetBuilder::new()
            .on_row(row)
            .with_margin(Thickness::uniform(1.0))
            .on_column(1),
    )
    .build(ctx)
}

fn make_bool_input_field(ctx: &mut BuildContext, row: usize) -> Handle<UiNode> {
    CheckBoxBuilder::new(
        WidgetBuilder::new()
            .on_row(row)
            .with_margin(Thickness::uniform(1.0))
            .on_column(1),
    )
    .build(ctx)
}

fn make_dropdown_list_option(ctx: &mut BuildContext, name: &str) -> Handle<UiNode> {
    DecoratorBuilder::new(BorderBuilder::new(
        WidgetBuilder::new().with_height(26.0).with_child(
            TextBuilder::new(WidgetBuilder::new())
                .with_vertical_text_alignment(VerticalAlignment::Center)
                .with_horizontal_text_alignment(HorizontalAlignment::Center)
                .with_text(name)
                .build(ctx),
        ),
    ))
    .build(ctx)
}

struct LightSection {
    section: Handle<UiNode>,
    color: Handle<UiNode>,
    cast_shadows: Handle<UiNode>,
    light_scatter: Handle<UiNode>,
    enable_scatter: Handle<UiNode>,
    point_light_section: PointLightSection,
    spot_light_section: SpotLightSection,
    sender: Sender<Message>,
}

impl LightSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let color;
        let cast_shadows;
        let light_scatter;
        let enable_scatter;
        Self {
            section: GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(make_text_mark(ctx, "Color", 0))
                    .with_child({
                        color = make_vec3_input_field(ctx, 0);
                        color
                    })
                    .with_child(make_text_mark(ctx, "Cast Shadows", 1))
                    .with_child({
                        cast_shadows = make_bool_input_field(ctx, 1);
                        cast_shadows
                    })
                    .with_child(make_text_mark(ctx, "Enable Scatter", 2))
                    .with_child({
                        enable_scatter = make_bool_input_field(ctx, 2);
                        enable_scatter
                    })
                    .with_child(make_text_mark(ctx, "Scatter", 3))
                    .with_child({
                        light_scatter = make_vec3_input_field(ctx, 3);
                        light_scatter
                    }),
            )
            .add_column(Column::strict(COLUMN_WIDTH))
            .add_column(Column::stretch())
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .build(ctx),
            color,
            cast_shadows,
            light_scatter,
            enable_scatter,
            point_light_section: PointLightSection::new(ctx, sender.clone()),
            spot_light_section: SpotLightSection::new(ctx, sender.clone()),
            sender,
        }
    }

    pub fn sync_to_model(&mut self, node: &Node, ui: &mut Ui) {
        if let Node::Light(light) = node {
            ui.send_message(Vec3EditorMessage::value(
                self.light_scatter,
                MessageDirection::ToWidget,
                light.scatter(),
            ));

            ui.send_message(Vec3EditorMessage::value(
                self.color,
                MessageDirection::ToWidget,
                light.color().as_frgba().xyz(),
            ));

            ui.send_message(CheckBoxMessage::checked(
                self.cast_shadows,
                MessageDirection::ToWidget,
                Some(light.is_cast_shadows()),
            ));

            ui.send_message(CheckBoxMessage::checked(
                self.enable_scatter,
                MessageDirection::ToWidget,
                Some(light.is_scatter_enabled()),
            ));
        }
        ui.send_message(WidgetMessage::visibility(
            self.section,
            MessageDirection::ToWidget,
            node.is_light(),
        ));
        self.point_light_section.sync_to_model(node, ui);
        self.spot_light_section.sync_to_model(node, ui);
    }

    pub fn handle_message(&mut self, message: &UiMessage, node: &Node, handle: Handle<Node>) {
        if let Node::Light(light) = node {
            match &message.data() {
                UiMessageData::Vec3Editor(msg) => {
                    if let &Vec3EditorMessage::Value(value) = msg {
                        if message.destination() == self.color
                            && light.color().as_frgba().xyz() != value
                        {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::SetLightColor(
                                    SetLightColorCommand::new(handle, value.into()),
                                )))
                                .unwrap();
                        } else if message.destination() == self.light_scatter
                            && light.scatter() != value
                        {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::SetLightScatter(
                                    SetLightScatterCommand::new(handle, value.into()),
                                )))
                                .unwrap();
                        }
                    }
                }
                UiMessageData::CheckBox(msg) => {
                    if let CheckBoxMessage::Check(value) = msg {
                        let value = value.unwrap_or(false);

                        if message.destination() == self.enable_scatter
                            && light.is_scatter_enabled() != value
                        {
                            self.sender
                                .send(Message::DoSceneCommand(
                                    SceneCommand::SetLightScatterEnabled(
                                        SetLightScatterEnabledCommand::new(handle, value),
                                    ),
                                ))
                                .unwrap();
                        } else if message.destination() == self.cast_shadows
                            && light.is_cast_shadows() != value
                        {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::SetLightCastShadows(
                                    SetLightCastShadowsCommand::new(handle, value),
                                )))
                                .unwrap();
                        }
                    }
                }
                _ => {}
            }
        }
        self.point_light_section
            .handle_message(message, node, handle);
        self.spot_light_section
            .handle_message(message, node, handle);
    }
}

struct PointLightSection {
    section: Handle<UiNode>,
    radius: Handle<UiNode>,
    sender: Sender<Message>,
}

impl PointLightSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let radius;

        Self {
            section: GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(make_text_mark(ctx, "Radius", 0))
                    .with_child({
                        radius = make_vec3_input_field(ctx, 0);
                        radius
                    }),
            )
            .add_column(Column::strict(COLUMN_WIDTH))
            .add_column(Column::stretch())
            .add_row(Row::strict(ROW_HEIGHT))
            .build(ctx),

            radius,
            sender,
        }
    }

    pub fn sync_to_model(&mut self, node: &Node, ui: &mut Ui) {
        let visible = if let Node::Light(light) = node {
            if let Light::Point(point) = light {
                ui.send_message(NumericUpDownMessage::value(
                    self.radius,
                    MessageDirection::ToWidget,
                    point.radius(),
                ));

                true
            } else {
                false
            }
        } else {
            false
        };
        ui.send_message(WidgetMessage::visibility(
            self.section,
            MessageDirection::ToWidget,
            visible,
        ));
    }

    pub fn handle_message(&mut self, message: &UiMessage, node: &Node, handle: Handle<Node>) {
        if let Node::Light(light) = node {
            if let Light::Point(point) = light {
                if let UiMessageData::NumericUpDown(msg) = &message.data() {
                    if let &NumericUpDownMessage::Value(value) = msg {
                        if message.destination() == self.radius && point.radius() != value {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::SetPointLightRadius(
                                    SetPointLightRadiusCommand::new(handle, value),
                                )))
                                .unwrap();
                        }
                    }
                }
            }
        }
    }
}

struct SpotLightSection {
    section: Handle<UiNode>,
    hotspot: Handle<UiNode>,
    falloff_delta: Handle<UiNode>,
    distance: Handle<UiNode>,
    sender: Sender<Message>,
}

impl SpotLightSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let hotspot;
        let falloff_delta;
        let distance;
        Self {
            section: GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(make_text_mark(ctx, "Hotspot", 0))
                    .with_child({
                        hotspot = make_f32_input_field(ctx, 0);
                        hotspot
                    })
                    .with_child(make_text_mark(ctx, "Falloff Delta", 1))
                    .with_child({
                        falloff_delta = make_f32_input_field(ctx, 1);
                        falloff_delta
                    })
                    .with_child(make_text_mark(ctx, "Radius", 2))
                    .with_child({
                        distance = make_f32_input_field(ctx, 2);
                        distance
                    }),
            )
            .add_column(Column::strict(COLUMN_WIDTH))
            .add_column(Column::stretch())
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .build(ctx),
            hotspot,
            falloff_delta,
            distance,
            sender,
        }
    }

    pub fn sync_to_model(&mut self, node: &Node, ui: &mut Ui) {
        let visible = if let Node::Light(light) = node {
            if let Light::Spot(spot) = light {
                ui.send_message(NumericUpDownMessage::value(
                    self.hotspot,
                    MessageDirection::ToWidget,
                    spot.hotspot_cone_angle(),
                ));

                ui.send_message(NumericUpDownMessage::value(
                    self.falloff_delta,
                    MessageDirection::ToWidget,
                    spot.falloff_angle_delta(),
                ));

                ui.send_message(NumericUpDownMessage::value(
                    self.distance,
                    MessageDirection::ToWidget,
                    spot.distance(),
                ));

                true
            } else {
                false
            }
        } else {
            false
        };
        ui.send_message(WidgetMessage::visibility(
            self.section,
            MessageDirection::ToWidget,
            visible,
        ));
    }

    pub fn handle_message(&mut self, message: &UiMessage, node: &Node, handle: Handle<Node>) {
        if let Node::Light(light) = node {
            if let Light::Spot(spot) = light {
                if let UiMessageData::NumericUpDown(msg) = &message.data() {
                    if let &NumericUpDownMessage::Value(value) = msg {
                        if message.destination() == self.hotspot
                            && spot.hotspot_cone_angle() != value
                        {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::SetSpotLightHotspot(
                                    SetSpotLightHotspotCommand::new(handle, value),
                                )))
                                .unwrap();
                        } else if message.destination() == self.falloff_delta
                            && spot.falloff_angle_delta() != value
                        {
                            self.sender
                                .send(Message::DoSceneCommand(
                                    SceneCommand::SetSpotLightFalloffAngleDelta(
                                        SetSpotLightFalloffAngleDeltaCommand::new(handle, value),
                                    ),
                                ))
                                .unwrap();
                        } else if message.destination() == self.distance && spot.distance() != value
                        {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::SetSpotLightDistance(
                                    SetSpotLightDistanceCommand::new(handle, value),
                                )))
                                .unwrap();
                        }
                    }
                }
            }
        }
    }
}

struct CameraSection {
    section: Handle<UiNode>,
    fov: Handle<UiNode>,
    z_near: Handle<UiNode>,
    z_far: Handle<UiNode>,
    sender: Sender<Message>,
}

impl CameraSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let fov;
        let z_near;
        let z_far;
        Self {
            section: GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(make_text_mark(ctx, "FOV", 0))
                    .with_child({
                        fov = make_f32_input_field(ctx, 0);
                        fov
                    })
                    .with_child(make_text_mark(ctx, "Z Near", 1))
                    .with_child({
                        z_near = make_f32_input_field(ctx, 1);
                        z_near
                    })
                    .with_child(make_text_mark(ctx, "Z Far", 2))
                    .with_child({
                        z_far = make_vec3_input_field(ctx, 2);
                        z_far
                    }),
            )
            .add_column(Column::strict(COLUMN_WIDTH))
            .add_column(Column::stretch())
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .build(ctx),
            fov,
            z_near,
            z_far,
            sender,
        }
    }

    pub fn sync_to_model(&mut self, node: &Node, ui: &mut Ui) {
        ui.send_message(WidgetMessage::visibility(
            self.section,
            MessageDirection::ToWidget,
            node.is_camera(),
        ));

        if let Node::Camera(camera) = node {
            ui.send_message(NumericUpDownMessage::value(
                self.fov,
                MessageDirection::ToWidget,
                camera.fov(),
            ));

            ui.send_message(NumericUpDownMessage::value(
                self.z_near,
                MessageDirection::ToWidget,
                camera.z_near(),
            ));

            ui.send_message(NumericUpDownMessage::value(
                self.z_far,
                MessageDirection::ToWidget,
                camera.z_far(),
            ));
        }
    }

    pub fn handle_message(&mut self, message: &UiMessage, node: &Node, handle: Handle<Node>) {
        if let Node::Camera(camera) = node {
            if let UiMessageData::NumericUpDown(msg) = &message.data() {
                if let &NumericUpDownMessage::Value(value) = msg {
                    if message.destination() == self.fov && camera.fov() != value {
                        self.sender
                            .send(Message::DoSceneCommand(SceneCommand::SetFov(
                                SetFovCommand::new(handle, value),
                            )))
                            .unwrap();
                    } else if message.destination() == self.z_far && camera.z_far() != value {
                        self.sender
                            .send(Message::DoSceneCommand(SceneCommand::SetZFar(
                                SetZFarCommand::new(handle, value),
                            )))
                            .unwrap();
                    } else if message.destination() == self.z_near && camera.z_near() != value {
                        self.sender
                            .send(Message::DoSceneCommand(SceneCommand::SetZNear(
                                SetZNearCommand::new(handle, value),
                            )))
                            .unwrap();
                    }
                }
            }
        }
    }
}

struct ParticleSystemSection {
    section: Handle<UiNode>,
    acceleration: Handle<UiNode>,
    sender: Sender<Message>,
}

impl ParticleSystemSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let acceleration;
        Self {
            section: GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(make_text_mark(ctx, "Acceleration", 0))
                    .with_child({
                        acceleration = make_vec3_input_field(ctx, 0);
                        acceleration
                    }),
            )
            .add_column(Column::strict(COLUMN_WIDTH))
            .add_column(Column::stretch())
            .add_row(Row::strict(ROW_HEIGHT))
            .build(ctx),
            acceleration,
            sender,
        }
    }

    pub fn sync_to_model(&mut self, node: &Node, ui: &mut Ui) {
        ui.send_message(WidgetMessage::visibility(
            self.section,
            MessageDirection::ToWidget,
            node.is_particle_system(),
        ));

        if let Node::ParticleSystem(particle_system) = node {
            ui.send_message(Vec3EditorMessage::value(
                self.acceleration,
                MessageDirection::ToWidget,
                particle_system.acceleration(),
            ));
        }
    }

    pub fn handle_message(&mut self, message: &UiMessage, node: &Node, handle: Handle<Node>) {
        if let Node::ParticleSystem(particle_system) = node {
            if let UiMessageData::Vec3Editor(msg) = &message.data() {
                if let &Vec3EditorMessage::Value(value) = msg {
                    if particle_system.acceleration() != value {
                        if message.destination() == self.acceleration {
                            self.sender
                                .send(Message::DoSceneCommand(
                                    SceneCommand::SetParticleSystemAcceleration(
                                        SetParticleSystemAccelerationCommand::new(handle, value),
                                    ),
                                ))
                                .unwrap();
                        }
                    }
                }
            }
        }
    }
}

struct SpriteSection {
    section: Handle<UiNode>,
    size: Handle<UiNode>,
    rotation: Handle<UiNode>,
    color: Handle<UiNode>,
    sender: Sender<Message>,
}

impl SpriteSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let size;
        let rotation;
        let color;
        Self {
            section: GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(make_text_mark(ctx, "Size", 0))
                    .with_child({
                        size = make_f32_input_field(ctx, 0);
                        size
                    })
                    .with_child(make_text_mark(ctx, "Rotation", 1))
                    .with_child({
                        rotation = make_f32_input_field(ctx, 1);
                        rotation
                    })
                    .with_child(make_text_mark(ctx, "Color", 2))
                    .with_child({
                        color = make_color_input_field(ctx, 2);
                        color
                    }),
            )
            .add_column(Column::strict(COLUMN_WIDTH))
            .add_column(Column::stretch())
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .build(ctx),
            size,
            rotation,
            sender,
            color,
        }
    }

    pub fn sync_to_model(&mut self, node: &Node, ui: &mut Ui) {
        ui.send_message(WidgetMessage::visibility(
            self.section,
            MessageDirection::ToWidget,
            node.is_sprite(),
        ));

        if let Node::Sprite(sprite) = node {
            ui.send_message(NumericUpDownMessage::value(
                self.size,
                MessageDirection::ToWidget,
                sprite.size(),
            ));

            ui.send_message(NumericUpDownMessage::value(
                self.rotation,
                MessageDirection::ToWidget,
                sprite.rotation(),
            ));

            ui.send_message(ColorFieldMessage::color(
                self.color,
                MessageDirection::ToWidget,
                sprite.color(),
            ));
        }
    }

    pub fn handle_message(&mut self, message: &UiMessage, node: &Node, handle: Handle<Node>) {
        if let Node::Sprite(sprite) = node {
            match &message.data() {
                UiMessageData::NumericUpDown(msg) => {
                    if let &NumericUpDownMessage::Value(value) = msg {
                        if message.destination() == self.size && sprite.size() != value {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::SetSpriteSize(
                                    SetSpriteSizeCommand::new(handle, value),
                                )))
                                .unwrap();
                        } else if message.destination() == self.rotation
                            && sprite.rotation() != value
                        {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::SetSpriteRotation(
                                    SetSpriteRotationCommand::new(handle, value),
                                )))
                                .unwrap();
                        }
                    }
                }
                UiMessageData::ColorField(msg) => {
                    if let &ColorFieldMessage::Color(color) = msg {
                        dbg!(color);
                        if message.destination() == self.color && sprite.color() != color {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::SetSpriteColor(
                                    SetSpriteColorCommand::new(handle, color),
                                )))
                                .unwrap();
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

impl SideBar {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let scroll_viewer;
        let node_name;
        let position;
        let rotation;
        let scale;
        let body;

        let light_section = LightSection::new(ctx, sender.clone());
        let camera_section = CameraSection::new(ctx, sender.clone());
        let particle_system_section = ParticleSystemSection::new(ctx, sender.clone());
        let sprite_section = SpriteSection::new(ctx, sender.clone());

        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_content({
                scroll_viewer =
                    ScrollViewerBuilder::new(WidgetBuilder::new().with_visibility(false))
                        .with_content(
                            StackPanelBuilder::new(
                                WidgetBuilder::new().with_children(&[
                                    GridBuilder::new(
                                        WidgetBuilder::new()
                                            .with_child(make_text_mark(ctx, "Name", 0))
                                            .with_child({
                                                node_name = TextBoxBuilder::new(
                                                    WidgetBuilder::new()
                                                        .on_row(0)
                                                        .on_column(1)
                                                        .with_margin(Thickness::uniform(1.0)),
                                                )
                                                .build(ctx);
                                                node_name
                                            })
                                            .with_child(make_text_mark(ctx, "Position", 1))
                                            .with_child({
                                                position = make_vec3_input_field(ctx, 1);
                                                position
                                            })
                                            .with_child(make_text_mark(ctx, "Rotation", 2))
                                            .with_child({
                                                rotation = make_vec3_input_field(ctx, 2);
                                                rotation
                                            })
                                            .with_child(make_text_mark(ctx, "Scale", 3))
                                            .with_child({
                                                scale = make_vec3_input_field(ctx, 3);
                                                scale
                                            })
                                            .with_child(make_text_mark(ctx, "Body", 4))
                                            .with_child({
                                                body = DropdownListBuilder::new(
                                                    WidgetBuilder::new()
                                                        .on_row(4)
                                                        .on_column(1)
                                                        .with_margin(Thickness::uniform(1.0)),
                                                )
                                                .with_items(vec![
                                                    make_dropdown_list_option(ctx, "None"),
                                                    make_dropdown_list_option(ctx, "Sphere"),
                                                    make_dropdown_list_option(ctx, "Cube"),
                                                    make_dropdown_list_option(ctx, "Capsule"),
                                                    make_dropdown_list_option(ctx, "Static Mesh"),
                                                ])
                                                .build(ctx);
                                                body
                                            }),
                                    )
                                    .add_column(Column::strict(COLUMN_WIDTH))
                                    .add_column(Column::stretch())
                                    .add_row(Row::strict(ROW_HEIGHT))
                                    .add_row(Row::strict(ROW_HEIGHT))
                                    .add_row(Row::strict(ROW_HEIGHT))
                                    .add_row(Row::strict(ROW_HEIGHT))
                                    .add_row(Row::strict(ROW_HEIGHT))
                                    .add_row(Row::stretch())
                                    .build(ctx),
                                    light_section.section,
                                    light_section.point_light_section.section,
                                    light_section.spot_light_section.section,
                                    camera_section.section,
                                    particle_system_section.section,
                                    sprite_section.section,
                                ]),
                            )
                            .build(ctx),
                        )
                        .build(ctx);
                scroll_viewer
            })
            .with_title(WindowTitle::text("Node Properties"))
            .build(ctx);

        Self {
            scroll_viewer,
            window,
            node_name,
            position,
            rotation,
            sender,
            scale,
            body,
            light_section,
            camera_section,
            particle_system_section,
            sprite_section,
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let scene = &engine.scenes[editor_scene.scene];
        engine
            .user_interface
            .send_message(WidgetMessage::visibility(
                self.scroll_viewer,
                MessageDirection::ToWidget,
                editor_scene.selection.is_single_selection(),
            ));
        if editor_scene.selection.is_single_selection() {
            let node_handle = editor_scene.selection.nodes()[0];
            if scene.graph.is_valid_handle(node_handle) {
                let node = &scene.graph[node_handle];

                let ui = &mut engine.user_interface;

                ui.send_message(TextBoxMessage::text(
                    self.node_name,
                    MessageDirection::ToWidget,
                    node.name().to_owned(),
                ));

                ui.send_message(Vec3EditorMessage::value(
                    self.position,
                    MessageDirection::ToWidget,
                    node.local_transform().position(),
                ));

                let euler = node.local_transform().rotation().to_euler();
                let euler_degrees = Vec3::new(
                    euler.x.to_degrees(),
                    euler.y.to_degrees(),
                    euler.z.to_degrees(),
                );
                ui.send_message(Vec3EditorMessage::value(
                    self.rotation,
                    MessageDirection::ToWidget,
                    euler_degrees,
                ));

                ui.send_message(Vec3EditorMessage::value(
                    self.scale,
                    MessageDirection::ToWidget,
                    node.local_transform().scale(),
                ));

                // Sync physical body info.
                let body_handle = scene.physics_binder.body_of(node_handle);
                let index = if body_handle.is_some() {
                    let body = scene.physics.borrow_body(body_handle);
                    match body.get_shape() {
                        ConvexShape::Sphere(_) => 1,
                        ConvexShape::Box(_) => 2,
                        ConvexShape::Capsule(_) => 3,
                        _ => 0,
                    }
                } else {
                    0
                };

                ui.send_message(DropdownListMessage::selection(
                    self.body,
                    MessageDirection::ToWidget,
                    Some(index),
                ));
                self.light_section.sync_to_model(node, ui);
                self.camera_section.sync_to_model(node, ui);
                self.particle_system_section.sync_to_model(node, ui);
                self.sprite_section.sync_to_model(node, ui);
            }
        }
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        engine: &GameEngine,
    ) {
        let scene = &engine.scenes[editor_scene.scene];
        let graph = &scene.graph;

        if editor_scene.selection.is_single_selection()
            && message.direction() == MessageDirection::FromWidget
        {
            let node_handle = editor_scene.selection.nodes()[0];
            let node = &graph[node_handle];

            self.light_section
                .handle_message(message, node, node_handle);
            self.camera_section
                .handle_message(message, node, node_handle);
            self.particle_system_section
                .handle_message(message, node, node_handle);
            self.sprite_section
                .handle_message(message, node, node_handle);

            match &message.data() {
                UiMessageData::Vec3Editor(msg) => {
                    if let &Vec3EditorMessage::Value(value) = msg {
                        let transform = graph[node_handle].local_transform();
                        if message.destination() == self.rotation {
                            let old_rotation = transform.rotation();
                            let euler = Vec3::new(
                                value.x.to_radians(),
                                value.y.to_radians(),
                                value.z.to_radians(),
                            );
                            let new_rotation = Quat::from_euler(euler, RotationOrder::XYZ);
                            if !old_rotation.approx_eq(new_rotation, 0.001) {
                                self.sender
                                    .send(Message::DoSceneCommand(SceneCommand::RotateNode(
                                        RotateNodeCommand::new(
                                            node_handle,
                                            old_rotation,
                                            new_rotation,
                                        ),
                                    )))
                                    .unwrap();
                            }
                        } else if message.destination() == self.position {
                            let old_position = transform.position();
                            if old_position != value {
                                self.sender
                                    .send(Message::DoSceneCommand(SceneCommand::MoveNode(
                                        MoveNodeCommand::new(node_handle, old_position, value),
                                    )))
                                    .unwrap();
                            }
                        } else if message.destination() == self.scale {
                            let old_scale = transform.scale();
                            if old_scale != value {
                                self.sender
                                    .send(Message::DoSceneCommand(SceneCommand::ScaleNode(
                                        ScaleNodeCommand::new(node_handle, old_scale, value),
                                    )))
                                    .unwrap();
                            }
                        }
                    }
                }
                UiMessageData::DropdownList(msg) => {
                    if message.destination() == self.body {
                        if let DropdownListMessage::SelectionChanged(index) = msg {
                            if let Some(index) = index {
                                match index {
                                    0 => {
                                        let body_handle = scene.physics_binder.body_of(node_handle);
                                        if body_handle.is_some() {
                                            self.sender
                                                .send(Message::DoSceneCommand(
                                                    SceneCommand::DeleteBody(
                                                        DeleteBodyCommand::new(body_handle),
                                                    ),
                                                ))
                                                .unwrap();
                                        }
                                    }
                                    1 | 2 | 3 => {
                                        let mut body = match index {
                                            1 => RigidBody::new(ConvexShape::Sphere(
                                                SphereShape::default(),
                                            )),
                                            2 => RigidBody::new(ConvexShape::Box(
                                                BoxShape::default(),
                                            )),
                                            3 => RigidBody::new(ConvexShape::Capsule(
                                                CapsuleShape::default(),
                                            )),
                                            _ => unreachable!(),
                                        };
                                        body.set_position(graph[node_handle].global_position());
                                        self.sender
                                            .send(Message::DoSceneCommand(SceneCommand::SetBody(
                                                SetBodyCommand::new(node_handle, body),
                                            )))
                                            .unwrap();
                                    }
                                    4 => {
                                        println!("implement me!");
                                    }
                                    _ => unreachable!(),
                                };
                            }
                        }
                    }
                }
                UiMessageData::TextBox(msg) => {
                    if message.destination() == self.node_name {
                        if let TextBoxMessage::Text(new_name) = msg {
                            let old_name = graph[node_handle].name();
                            if old_name != new_name {
                                self.sender
                                    .send(Message::DoSceneCommand(SceneCommand::SetName(
                                        SetNameCommand::new(node_handle, new_name.to_owned()),
                                    )))
                                    .unwrap();
                            }
                        }
                    }
                }
                _ => (),
            }
        }
    }
}
