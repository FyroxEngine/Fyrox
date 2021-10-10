use crate::{
    scene::{EditorScene, Selection},
    send_sync_message,
    sidebar::{
        base::BaseSection, camera::CameraSection, decal::DecalSection, light::LightSection,
        lod::LodGroupEditor, mesh::MeshSection, particle::ParticleSystemSection,
        physics::PhysicsSection, sound::SoundSection, sprite::SpriteSection,
        terrain::TerrainSection,
    },
    GameEngine, Message,
};
use rg3d::gui::message::UiMessage;
use rg3d::gui::{BuildContext, UiNode};
use rg3d::{
    core::{color::Color, pool::Handle, scope_profile},
    gui::{
        border::BorderBuilder,
        brush::Brush,
        check_box::CheckBoxBuilder,
        color::ColorFieldBuilder,
        expander::ExpanderBuilder,
        message::{MessageDirection, WidgetMessage},
        numeric::NumericUpDownBuilder,
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        vec::vec3::Vec3EditorBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        HorizontalAlignment, Thickness, VerticalAlignment,
    },
};
use std::sync::mpsc::Sender;

mod base;
mod camera;
mod decal;
mod light;
mod lod;
mod mesh;
mod particle;
mod physics;
mod sound;
mod sprite;
mod terrain;

const ROW_HEIGHT: f32 = 25.0;
const COLUMN_WIDTH: f32 = 140.0;

pub struct SideBar {
    pub window: Handle<UiNode>,
    scroll_viewer: Handle<UiNode>,
    base_section: BaseSection,
    lod_editor: LodGroupEditor,
    sender: Sender<Message>,
    light_section: LightSection,
    camera_section: CameraSection,
    particle_system_section: ParticleSystemSection,
    sprite_section: SpriteSection,
    mesh_section: MeshSection,
    physics_section: PhysicsSection,
    sound_section: SoundSection,
    decal_section: DecalSection,
    pub terrain_section: TerrainSection,
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

fn make_section(name: &str, content: Handle<UiNode>, ctx: &mut BuildContext) -> Handle<UiNode> {
    BorderBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(1.0))
            .with_child(
                ExpanderBuilder::new(WidgetBuilder::new())
                    .with_header(
                        TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::left(3.0)))
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .with_text(name)
                            .build(ctx),
                    )
                    .with_content(content)
                    .build(ctx),
            )
            .with_foreground(Brush::Solid(Color::opaque(130, 130, 130))),
    )
    .build(ctx)
}

fn make_vec3_input_field(ctx: &mut BuildContext, row: usize) -> Handle<UiNode> {
    Vec3EditorBuilder::<f32>::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(1.0))
            .on_row(row)
            .on_column(1),
    )
    .build(ctx)
}

fn make_f32_input_field(
    ctx: &mut BuildContext,
    row: usize,
    min: f32,
    max: f32,
    step: f32,
) -> Handle<UiNode> {
    NumericUpDownBuilder::new(
        WidgetBuilder::new()
            .with_height(ROW_HEIGHT)
            .on_row(row)
            .with_margin(Thickness::uniform(1.0))
            .on_column(1),
    )
    .with_min_value(min)
    .with_max_value(max)
    .with_step(step)
    .build(ctx)
}

fn make_int_input_field(
    ctx: &mut BuildContext,
    row: usize,
    min: i32,
    max: i32,
    step: i32,
) -> Handle<UiNode> {
    NumericUpDownBuilder::new(
        WidgetBuilder::new()
            .on_row(row)
            .with_margin(Thickness::uniform(1.0))
            .on_column(1),
    )
    .with_min_value(min as f32)
    .with_max_value(max as f32)
    .with_step(step as f32)
    .with_precision(0)
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
            .with_horizontal_alignment(HorizontalAlignment::Left)
            .on_row(row)
            .with_margin(Thickness::uniform(1.0))
            .on_column(1),
    )
    .build(ctx)
}

impl SideBar {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let scroll_viewer;

        let base_section = BaseSection::new(ctx);
        let lod_editor = LodGroupEditor::new(ctx, sender.clone());
        let light_section = LightSection::new(ctx, sender.clone());
        let camera_section = CameraSection::new(ctx, sender.clone());
        let particle_system_section = ParticleSystemSection::new(ctx, sender.clone());
        let sprite_section = SpriteSection::new(ctx, sender.clone());
        let mesh_section = MeshSection::new(ctx, sender.clone());
        let physics_section = PhysicsSection::new(ctx, sender.clone());
        let terrain_section = TerrainSection::new(ctx);
        let sound_section = SoundSection::new(ctx);
        let decal_section = DecalSection::new(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new())
            .can_minimize(false)
            .with_content({
                scroll_viewer =
                    ScrollViewerBuilder::new(WidgetBuilder::new().with_visibility(false))
                        .with_content(
                            StackPanelBuilder::new(WidgetBuilder::new().with_children([
                                base_section.section,
                                light_section.section,
                                camera_section.section,
                                particle_system_section.section,
                                sprite_section.section,
                                mesh_section.section,
                                terrain_section.section,
                                physics_section.section,
                                sound_section.section,
                                decal_section.section,
                            ]))
                            .build(ctx),
                        )
                        .build(ctx);
                scroll_viewer
            })
            .with_title(WindowTitle::text("Properties"))
            .build(ctx);

        Self {
            scroll_viewer,
            window,
            base_section,
            sender,
            lod_editor,
            light_section,
            camera_section,
            particle_system_section,
            sprite_section,
            mesh_section,
            physics_section,
            terrain_section,
            sound_section,
            decal_section,
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        scope_profile!();

        send_sync_message(
            &engine.user_interface,
            WidgetMessage::visibility(
                self.scroll_viewer,
                MessageDirection::ToWidget,
                editor_scene.selection.is_single_selection(),
            ),
        );

        let scene = &engine.scenes[editor_scene.scene];
        let ui = &engine.user_interface;

        match &editor_scene.selection {
            Selection::Graph(selection) => {
                if selection.is_single_selection() {
                    let node_handle = selection.nodes()[0];
                    if scene.graph.is_valid_handle(node_handle) {
                        let node = &scene.graph[node_handle];

                        let ui = &mut engine.user_interface;

                        send_sync_message(
                            ui,
                            WidgetMessage::visibility(
                                self.base_section.section,
                                MessageDirection::ToWidget,
                                true,
                            ),
                        );
                        send_sync_message(
                            ui,
                            WidgetMessage::visibility(
                                self.sound_section.section,
                                MessageDirection::ToWidget,
                                false,
                            ),
                        );

                        self.base_section.sync_to_model(node, ui);
                        self.lod_editor.sync_to_model(node, scene, ui);
                        self.light_section.sync_to_model(node, ui);
                        self.camera_section.sync_to_model(node, ui);
                        self.particle_system_section.sync_to_model(node, ui);
                        self.sprite_section.sync_to_model(node, ui);
                        self.decal_section.sync_to_model(node, ui);
                        self.mesh_section.sync_to_model(node, ui);
                        self.terrain_section.sync_to_model(node, ui);
                        self.physics_section.sync_to_model(editor_scene, engine);
                    }
                }
            }
            Selection::Sound(selection) => {
                for &section in &[
                    self.base_section.section,
                    self.sprite_section.section,
                    self.decal_section.section,
                    self.light_section.section,
                    self.camera_section.section,
                    self.particle_system_section.section,
                    self.mesh_section.section,
                    self.terrain_section.section,
                    self.physics_section.section,
                ] {
                    send_sync_message(
                        ui,
                        WidgetMessage::visibility(section, MessageDirection::ToWidget, false),
                    );
                }

                send_sync_message(
                    ui,
                    WidgetMessage::visibility(
                        self.sound_section.section,
                        MessageDirection::ToWidget,
                        true,
                    ),
                );

                if selection.is_single_selection() {
                    if let Some(first) = selection.first() {
                        let state = scene.sound_context.state();

                        if state.is_valid_handle(first) {
                            self.sound_section
                                .sync_to_model(state.source(first), &mut engine.user_interface);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
    ) {
        scope_profile!();

        match &editor_scene.selection {
            Selection::Graph(selection) => {
                if selection.is_single_selection() {
                    self.physics_section
                        .handle_ui_message(message, editor_scene, engine);

                    let scene = &mut engine.scenes[editor_scene.scene];
                    let graph = &mut scene.graph;
                    let node_handle = selection.nodes()[0];
                    let node = &mut graph[node_handle];

                    if message.direction() == MessageDirection::FromWidget {
                        self.light_section
                            .handle_ui_message(message, node, node_handle);
                        self.camera_section.handle_ui_message(
                            message,
                            node,
                            node_handle,
                            &engine.user_interface,
                            engine.resource_manager.clone(),
                        );
                        self.particle_system_section.handle_ui_message(
                            message,
                            node,
                            node_handle,
                            &engine.user_interface,
                        );
                        self.sprite_section
                            .handle_ui_message(message, node, node_handle);
                        self.decal_section.handle_ui_message(
                            message,
                            &mut engine.user_interface,
                            engine.resource_manager.clone(),
                            node_handle,
                            &self.sender,
                        );
                        self.mesh_section
                            .handle_ui_message(message, node, node_handle);
                        self.base_section.handle_ui_message(
                            message,
                            &self.sender,
                            node,
                            node_handle,
                            &mut engine.user_interface,
                            &mut self.lod_editor,
                        );

                        self.terrain_section.handle_ui_message(
                            message,
                            &mut engine.user_interface,
                            graph,
                            node_handle,
                            &self.sender,
                        );

                        self.lod_editor.handle_ui_message(
                            message,
                            node_handle,
                            scene,
                            &mut engine.user_interface,
                        );
                    }
                }
            }
            Selection::Sound(selection) => {
                if selection.is_single_selection() {
                    if let Some(first) = selection.first() {
                        let scene = &mut engine.scenes[editor_scene.scene];
                        let state = scene.sound_context.state();
                        if state.is_valid_handle(first) {
                            self.sound_section.handle_message(
                                message,
                                &self.sender,
                                state.source(first),
                                first,
                                &engine.user_interface,
                                engine.resource_manager.clone(),
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
