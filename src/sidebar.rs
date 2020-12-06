use crate::scene::{DeleteColliderCommand, SetColliderCommand};
use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    physics::{Collider, RigidBody},
    scene::{
        CommandGroup, DeleteBodyCommand, EditorScene, MoveNodeCommand, RotateNodeCommand,
        ScaleNodeCommand, SceneCommand, SetBodyCommand, SetFovCommand, SetLightCastShadowsCommand,
        SetLightColorCommand, SetLightScatterCommand, SetLightScatterEnabledCommand,
        SetNameCommand, SetParticleSystemAccelerationCommand, SetPointLightRadiusCommand,
        SetSpotLightDistanceCommand, SetSpotLightFalloffAngleDeltaCommand,
        SetSpotLightHotspotCommand, SetSpriteColorCommand, SetSpriteRotationCommand,
        SetSpriteSizeCommand, SetZFarCommand, SetZNearCommand,
    },
    GameEngine, Message,
};
use rg3d::scene::physics::{BodyStatusDesc, CapsuleDesc};
use rg3d::{
    core::{
        algebra::Vector3,
        math::{quat_from_euler, Matrix4Ext, RotationOrder, UnitQuaternionExt},
        pool::Handle,
    },
    gui::{
        border::BorderBuilder,
        check_box::CheckBoxBuilder,
        color::ColorFieldBuilder,
        decorator::DecoratorBuilder,
        dropdown_list::DropdownListBuilder,
        grid::{Column, GridBuilder, Row},
        message::{
            CheckBoxMessage, ColorFieldMessage, DropdownListMessage, MessageDirection,
            NumericUpDownMessage, TextBoxMessage, UiMessageData, Vec3EditorMessage, WidgetMessage,
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
    physics::na::UnitQuaternion,
    scene::physics::{
        BallDesc, ColliderShapeDesc, ConeDesc, CuboidDesc, CylinderDesc, HeightfieldDesc,
        RoundCylinderDesc, SegmentDesc, TriangleDesc, TrimeshDesc,
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
    collider: Handle<UiNode>,
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
        let section = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(make_text_mark(ctx, "Color", 0))
                .with_child({
                    color = ColorFieldBuilder::new(WidgetBuilder::new().on_column(1)).build(ctx);
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
        .build(ctx);

        Self {
            section,
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

            ui.send_message(ColorFieldMessage::color(
                self.color,
                MessageDirection::ToWidget,
                light.color(),
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
                    if let Vec3EditorMessage::Value(value) = *msg {
                        if message.destination() == self.light_scatter && light.scatter() != value {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::SetLightScatter(
                                    SetLightScatterCommand::new(handle, value),
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
                UiMessageData::ColorField(msg) => {
                    if let ColorFieldMessage::Color(color) = *msg {
                        if message.destination() == self.color && light.color() != color {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::SetLightColor(
                                    SetLightColorCommand::new(handle, color),
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
        let section = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(make_text_mark(ctx, "Radius", 0))
                .with_child({
                    radius = make_f32_input_field(ctx, 0);
                    radius
                }),
        )
        .add_column(Column::strict(COLUMN_WIDTH))
        .add_column(Column::stretch())
        .add_row(Row::strict(ROW_HEIGHT))
        .build(ctx);

        Self {
            section,
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
                    if let NumericUpDownMessage::Value(value) = *msg {
                        if message.destination() == self.radius && point.radius().ne(&value) {
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

        let section = GridBuilder::new(
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
        .build(ctx);

        Self {
            section,
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
                    if let NumericUpDownMessage::Value(value) = *msg {
                        if message.destination() == self.hotspot
                            && spot.hotspot_cone_angle().ne(&value)
                        {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::SetSpotLightHotspot(
                                    SetSpotLightHotspotCommand::new(handle, value),
                                )))
                                .unwrap();
                        } else if message.destination() == self.falloff_delta
                            && spot.falloff_angle_delta().ne(&value)
                        {
                            self.sender
                                .send(Message::DoSceneCommand(
                                    SceneCommand::SetSpotLightFalloffAngleDelta(
                                        SetSpotLightFalloffAngleDeltaCommand::new(handle, value),
                                    ),
                                ))
                                .unwrap();
                        } else if message.destination() == self.distance
                            && spot.distance().ne(&value)
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
        let section = GridBuilder::new(
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
        .build(ctx);

        Self {
            section,
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
                if let NumericUpDownMessage::Value(value) = *msg {
                    if message.destination() == self.fov && camera.fov().ne(&value) {
                        self.sender
                            .send(Message::DoSceneCommand(SceneCommand::SetFov(
                                SetFovCommand::new(handle, value),
                            )))
                            .unwrap();
                    } else if message.destination() == self.z_far && camera.z_far().ne(&value) {
                        self.sender
                            .send(Message::DoSceneCommand(SceneCommand::SetZFar(
                                SetZFarCommand::new(handle, value),
                            )))
                            .unwrap();
                    } else if message.destination() == self.z_near && camera.z_near().ne(&value) {
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
        let section = GridBuilder::new(
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
        .build(ctx);

        Self {
            section,
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
                if let Vec3EditorMessage::Value(value) = *msg {
                    if particle_system.acceleration() != value
                        && message.destination() == self.acceleration
                    {
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
        let section = GridBuilder::new(
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
        .build(ctx);

        Self {
            section,
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
                    if let NumericUpDownMessage::Value(value) = *msg {
                        if message.destination() == self.size && sprite.size().ne(&value) {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::SetSpriteSize(
                                    SetSpriteSizeCommand::new(handle, value),
                                )))
                                .unwrap();
                        } else if message.destination() == self.rotation
                            && sprite.rotation().ne(&value)
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
                    if let ColorFieldMessage::Color(color) = *msg {
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
        let collider;

        let light_section = LightSection::new(ctx, sender.clone());
        let camera_section = CameraSection::new(ctx, sender.clone());
        let particle_system_section = ParticleSystemSection::new(ctx, sender.clone());
        let sprite_section = SpriteSection::new(ctx, sender.clone());

        let window = WindowBuilder::new(WidgetBuilder::new())
            .can_minimize(false)
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
                                                    make_dropdown_list_option(ctx, "Dynamic"),
                                                    make_dropdown_list_option(ctx, "Static"),
                                                    make_dropdown_list_option(ctx, "Kinematic"),
                                                ])
                                                .build(ctx);
                                                body
                                            })
                                            .with_child(make_text_mark(ctx, "Collider", 5))
                                            .with_child({
                                                collider = DropdownListBuilder::new(
                                                    WidgetBuilder::new()
                                                        .on_row(5)
                                                        .on_column(1)
                                                        .with_margin(Thickness::uniform(1.0)),
                                                )
                                                .with_items(vec![
                                                    make_dropdown_list_option(ctx, "Ball"),
                                                    make_dropdown_list_option(ctx, "Cylinder"),
                                                    make_dropdown_list_option(
                                                        ctx,
                                                        "Round Cylinder",
                                                    ),
                                                    make_dropdown_list_option(ctx, "Cone"),
                                                    make_dropdown_list_option(ctx, "Cuboid"),
                                                    make_dropdown_list_option(ctx, "Capsule"),
                                                    make_dropdown_list_option(ctx, "Segment"),
                                                    make_dropdown_list_option(ctx, "Triangle"),
                                                    make_dropdown_list_option(ctx, "Trimesh"),
                                                    make_dropdown_list_option(ctx, "Heightfield"),
                                                ])
                                                .build(ctx);
                                                collider
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
            collider,
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
                let euler_degrees = Vector3::new(
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
                let body_index =
                    if let Some(&body_handle) = editor_scene.physics.binder.get(&node_handle) {
                        let body = &editor_scene.physics.bodies[body_handle];
                        match body.status {
                            BodyStatusDesc::Dynamic => 1,
                            BodyStatusDesc::Static => 2,
                            BodyStatusDesc::Kinematic => 3,
                        }
                    } else {
                        0
                    };

                ui.send_message(DropdownListMessage::selection(
                    self.body,
                    MessageDirection::ToWidget,
                    Some(body_index),
                ));

                if let Some(&body_handle) = editor_scene.physics.binder.get(&node_handle) {
                    let body = &editor_scene.physics.bodies[body_handle];

                    if let Some(&collider) = body.colliders.get(0) {
                        let collider_index =
                            match editor_scene.physics.colliders[collider.into()].shape {
                                ColliderShapeDesc::Ball(_) => 0,
                                ColliderShapeDesc::Cylinder(_) => 1,
                                ColliderShapeDesc::RoundCylinder(_) => 2,
                                ColliderShapeDesc::Cone(_) => 3,
                                ColliderShapeDesc::Cuboid(_) => 4,
                                ColliderShapeDesc::Capsule(_) => 5,
                                ColliderShapeDesc::Segment(_) => 6,
                                ColliderShapeDesc::Triangle(_) => 7,
                                ColliderShapeDesc::Trimesh(_) => 8,
                                ColliderShapeDesc::Heightfield(_) => 9,
                            };
                        dbg!(collider_index);
                        ui.send_message(DropdownListMessage::selection(
                            self.collider,
                            MessageDirection::ToWidget,
                            Some(collider_index),
                        ));
                    }
                }

                self.light_section.sync_to_model(node, ui);
                self.camera_section.sync_to_model(node, ui);
                self.particle_system_section.sync_to_model(node, ui);
                self.sprite_section.sync_to_model(node, ui);
            }
        }
    }

    pub fn handle_ui_message(
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
                    if let Vec3EditorMessage::Value(value) = *msg {
                        let transform = graph[node_handle].local_transform();
                        if message.destination() == self.rotation {
                            let old_rotation = transform.rotation();
                            let euler = Vector3::new(
                                value.x.to_radians(),
                                value.y.to_radians(),
                                value.z.to_radians(),
                            );
                            let new_rotation = quat_from_euler(euler, RotationOrder::XYZ);
                            if !old_rotation.approx_eq(&new_rotation, 0.001) {
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
                    if let DropdownListMessage::SelectionChanged(index) = msg {
                        if let Some(index) = index {
                            if message.destination() == self.body {
                                match index {
                                    0 => {
                                        // Remove body.
                                        if let Some(&body_handle) =
                                            editor_scene.physics.binder.get(&node_handle)
                                        {
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
                                        let mut current_status = 0;
                                        if let Some(&body) =
                                            editor_scene.physics.binder.get(&node_handle)
                                        {
                                            current_status =
                                                match editor_scene.physics.bodies[body].status {
                                                    BodyStatusDesc::Dynamic => 1,
                                                    BodyStatusDesc::Static => 2,
                                                    BodyStatusDesc::Kinematic => 3,
                                                };
                                        }

                                        if *index != current_status {
                                            // Create body.
                                            let node = &graph[node_handle];
                                            let body = RigidBody {
                                                position: node.global_position(),
                                                rotation: UnitQuaternion::from_matrix(
                                                    &node.global_transform().basis(),
                                                ),
                                                status: match index {
                                                    1 => BodyStatusDesc::Dynamic,
                                                    2 => BodyStatusDesc::Static,
                                                    3 => BodyStatusDesc::Kinematic,
                                                    _ => unreachable!(),
                                                },
                                                ..Default::default()
                                            };

                                            let mut commands = Vec::new();

                                            if let Some(&body) =
                                                editor_scene.physics.binder.get(&node_handle)
                                            {
                                                for &collider in editor_scene.physics.bodies[body]
                                                    .colliders
                                                    .iter()
                                                {
                                                    commands.push(SceneCommand::DeleteCollider(
                                                        DeleteColliderCommand::new(collider.into()),
                                                    ))
                                                }

                                                commands.push(SceneCommand::DeleteBody(
                                                    DeleteBodyCommand::new(body),
                                                ));
                                            }

                                            commands.push(SceneCommand::SetBody(
                                                SetBodyCommand::new(node_handle, body),
                                            ));

                                            self.sender
                                                .send(Message::DoSceneCommand(
                                                    SceneCommand::CommandGroup(CommandGroup::from(
                                                        commands,
                                                    )),
                                                ))
                                                .unwrap();
                                        }
                                    }
                                    _ => unreachable!(),
                                };
                            } else if message.destination() == self.collider {
                                if let Some(&body) = editor_scene.physics.binder.get(&node_handle) {
                                    let mut current_index = 0;
                                    if let Some(&first_collider) =
                                        editor_scene.physics.bodies[body].colliders.first()
                                    {
                                        current_index = editor_scene.physics.colliders
                                            [first_collider.into()]
                                        .shape
                                        .id();
                                    }

                                    if current_index != *index as u32 {
                                        let collider = match index {
                                            0 => Collider {
                                                shape: ColliderShapeDesc::Ball(BallDesc {
                                                    radius: 0.5,
                                                }),
                                                ..Default::default()
                                            },
                                            1 => Collider {
                                                shape: ColliderShapeDesc::Cylinder(CylinderDesc {
                                                    half_height: 0.5,
                                                    radius: 0.5,
                                                }),
                                                ..Default::default()
                                            },
                                            2 => Collider {
                                                shape: ColliderShapeDesc::RoundCylinder(
                                                    RoundCylinderDesc {
                                                        half_height: 0.5,
                                                        radius: 0.5,
                                                        border_radius: 0.1,
                                                    },
                                                ),
                                                ..Default::default()
                                            },
                                            3 => Collider {
                                                shape: ColliderShapeDesc::Cone(ConeDesc {
                                                    half_height: 0.5,
                                                    radius: 0.5,
                                                }),
                                                ..Default::default()
                                            },
                                            4 => Collider {
                                                shape: ColliderShapeDesc::Cuboid(CuboidDesc {
                                                    half_extents: Vector3::new(0.5, 0.5, 0.5),
                                                }),
                                                ..Default::default()
                                            },
                                            5 => Collider {
                                                shape: ColliderShapeDesc::Capsule(CapsuleDesc {
                                                    begin: Vector3::new(0.0, 0.0, 0.0),
                                                    end: Vector3::new(0.0, 1.0, 0.0),
                                                    radius: 0.5,
                                                }),
                                                ..Default::default()
                                            },
                                            6 => Collider {
                                                shape: ColliderShapeDesc::Segment(SegmentDesc {
                                                    begin: Vector3::new(0.0, 0.0, 0.0),
                                                    end: Vector3::new(1.0, 0.0, 0.0),
                                                }),
                                                ..Default::default()
                                            },
                                            7 => Collider {
                                                shape: ColliderShapeDesc::Triangle(TriangleDesc {
                                                    a: Vector3::new(0.0, 0.0, 0.0),
                                                    b: Vector3::new(1.0, 0.0, 0.0),
                                                    c: Vector3::new(1.0, 0.0, 1.0),
                                                }),
                                                ..Default::default()
                                            },
                                            8 => Collider {
                                                shape: ColliderShapeDesc::Trimesh(TrimeshDesc),
                                                ..Default::default()
                                            },
                                            9 => Collider {
                                                shape: ColliderShapeDesc::Heightfield(
                                                    HeightfieldDesc,
                                                ),
                                                ..Default::default()
                                            },
                                            _ => unreachable!(),
                                        };
                                        let mut commands = Vec::new();
                                        // For now only one collider per body is supported.
                                        // It is easy to add more.
                                        if let Some(&first_collider) =
                                            editor_scene.physics.bodies[body].colliders.first()
                                        {
                                            commands.push(SceneCommand::DeleteCollider(
                                                DeleteColliderCommand::new(first_collider.into()),
                                            ))
                                        }
                                        commands.push(SceneCommand::SetCollider(
                                            SetColliderCommand::new(body, collider),
                                        ));
                                        self.sender
                                            .send(Message::DoSceneCommand(
                                                SceneCommand::CommandGroup(CommandGroup::from(
                                                    commands,
                                                )),
                                            ))
                                            .unwrap();
                                    }
                                }
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
