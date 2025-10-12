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

use crate::{
    asset::preview::cache::IconRequest,
    fyrox::{
        asset::manager::ResourceManager,
        core::{
            algebra::{Matrix2, Matrix3, Matrix4, Vector2, Vector3, Vector4},
            color::Color,
            parking_lot::Mutex,
            pool::Handle,
            some_or_continue, some_or_return,
            sstorage::ImmutableString,
        },
        fxhash::FxHashMap,
        graph::SceneGraph,
        graphics::gpu_program::{ShaderProperty, ShaderPropertyKind},
        gui::{
            border::BorderBuilder,
            check_box::{CheckBoxBuilder, CheckBoxMessage},
            color::{ColorFieldBuilder, ColorFieldMessage},
            dock::DockingManagerMessage,
            grid::{Column, GridBuilder, Row},
            inspector::editors::{
                inherit::InheritablePropertyEditorDefinition,
                inspectable::InspectablePropertyEditorDefinition,
            },
            list_view::{ListView, ListViewBuilder, ListViewMessage},
            matrix::{MatrixEditorBuilder, MatrixEditorMessage},
            message::{MessageDirection, UiMessage},
            numeric::{NumericUpDownBuilder, NumericUpDownMessage},
            scroll_viewer::ScrollViewerBuilder,
            stack_panel::StackPanelBuilder,
            text::TextBuilder,
            utils::make_simple_tooltip,
            vec::{
                Vec2EditorMessage, Vec3EditorMessage, Vec4EditorMessage, VecEditorBuilder,
                VecEditorMessage,
            },
            widget::{WidgetBuilder, WidgetMaterial, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, Thickness, UiNode, UserInterface, VerticalAlignment,
        },
        material::{
            shader::{Shader, ShaderResourceKind},
            MaterialProperty, MaterialResource, MaterialResourceBinding, MaterialTextureBinding,
        },
        scene::{
            base::BaseBuilder,
            mesh::{
                surface::{SurfaceBuilder, SurfaceData, SurfaceResource},
                MeshBuilder,
            },
        },
    },
    message::MessageSender,
    plugin::EditorPlugin,
    plugins::{
        inspector::{
            editors::{
                resource::{ResourceFieldBuilder, ResourceFieldMessage},
                texture::{TextureEditorBuilder, TextureEditorMessage},
            },
            InspectorPlugin,
        },
        material::editor::MaterialPropertyEditorDefinition,
    },
    preview::PreviewPanel,
    scene::commands::material::{
        SetMaterialBindingCommand, SetMaterialPropertyGroupPropertyValueCommand,
        SetMaterialShaderCommand,
    },
    send_sync_message, Editor, Engine, Message,
};
use fyrox::asset::event::ResourceEvent;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::{mpsc::Sender, Arc};

pub mod editor;

enum ResourceViewKind {
    Sampler,
    PropertyGroup {
        property_views: FxHashMap<ImmutableString, Handle<UiNode>>,
    },
}

struct ResourceView {
    name: ImmutableString,
    kind: ResourceViewKind,
    container: Handle<UiNode>,
    editor: Handle<UiNode>,
}

pub struct MaterialEditor {
    pub window: Handle<UiNode>,
    properties_panel: Handle<UiNode>,
    resource_views: Vec<ResourceView>,
    preview: PreviewPanel,
    material: Option<MaterialResource>,
    shader: Handle<UiNode>,
}

fn make_item_container(ctx: &mut BuildContext, name: &str, item: Handle<UiNode>) -> Handle<UiNode> {
    ctx[item].set_column(1);

    GridBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(1.0))
            .with_child(
                TextBuilder::new(WidgetBuilder::new())
                    .with_text(name)
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .build(ctx),
            )
            .with_child(item),
    )
    .add_row(Row::auto())
    .add_column(Column::strict(150.0))
    .add_column(Column::stretch())
    .build(ctx)
}

fn pad_vec<T: UiView>(v: &[T], max_len: usize) -> Vec<T> {
    let mut vec = v.to_vec();
    for _ in v.len()..max_len {
        vec.push(T::default());
    }
    vec
}

fn make_array_view(
    ctx: &mut BuildContext,
    value: &[impl UiView],
    max_len: usize,
) -> Handle<UiNode> {
    let value = pad_vec(value, max_len);
    ListViewBuilder::new(WidgetBuilder::new())
        .with_items(value.into_iter().map(|v| v.make_view(ctx)).collect())
        .build(ctx)
}

fn sync_array(ui: &UserInterface, handle: Handle<UiNode>, array: &[impl UiView]) {
    let views = &**ui.try_get_of_type::<ListView>(handle).unwrap().items;
    for (item, view) in array.iter().zip(views) {
        send_sync_message(ui, item.into_message(*view))
    }
}

trait UiView: Default + Copy {
    fn into_message(self, item: Handle<UiNode>) -> UiMessage;
    fn make_view(self, ctx: &mut BuildContext) -> Handle<UiNode>;
    fn send(self, ui: &UserInterface, item: Handle<UiNode>) {
        send_sync_message(ui, self.into_message(item))
    }
}

macro_rules! numeric_ui_view {
    ($($ty:ty),*) => {
         $(impl UiView for $ty {
            fn into_message(self, item: Handle<UiNode>) -> UiMessage {
                NumericUpDownMessage::value(item, MessageDirection::ToWidget, self)
            }
            fn make_view(self, ctx: &mut BuildContext) -> Handle<UiNode> {
                NumericUpDownBuilder::new(WidgetBuilder::new().with_height(24.0))
                    .with_value(self)
                    .build(ctx)
            }
        })*
    };
}
numeric_ui_view!(f32, u32, i32);

macro_rules! vec_ui_view {
    ($($ty:ty),*) => {
        $(impl UiView for $ty {
            fn into_message(self, item: Handle<UiNode>) -> UiMessage {
                VecEditorMessage::value(item, MessageDirection::ToWidget, self)
            }
            fn make_view(self, ctx: &mut BuildContext) -> Handle<UiNode> {
                VecEditorBuilder::new(WidgetBuilder::new().with_height(24.0))
                    .with_value(self)
                    .build(ctx)
            }
        })*
    };
}
vec_ui_view!(Vector2<f32>, Vector3<f32>, Vector4<f32>);

macro_rules! mat_ui_view {
    ($($ty:ty),*) => {
        $(impl UiView for $ty {
            fn into_message(self, item: Handle<UiNode>) -> UiMessage {
                MatrixEditorMessage::value(item, MessageDirection::ToWidget, self)
            }
            fn make_view(self, ctx: &mut BuildContext) -> Handle<UiNode> {
                MatrixEditorBuilder::new(WidgetBuilder::new())
                    .with_value(self)
                    .build(ctx)
            }
        })*
    };
}
mat_ui_view!(Matrix2<f32>, Matrix3<f32>, Matrix4<f32>);

impl UiView for bool {
    fn into_message(self, item: Handle<UiNode>) -> UiMessage {
        CheckBoxMessage::checked(item, MessageDirection::ToWidget, Some(self))
    }
    fn make_view(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        CheckBoxBuilder::new(WidgetBuilder::new())
            .checked(Some(self))
            .build(ctx)
    }
}

impl UiView for Color {
    fn into_message(self, item: Handle<UiNode>) -> UiMessage {
        ColorFieldMessage::color(item, MessageDirection::ToWidget, self)
    }
    fn make_view(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        ColorFieldBuilder::new(WidgetBuilder::new())
            .with_color(self)
            .build(ctx)
    }
}

impl MaterialEditor {
    const TITLE: &'static str = "Material Editor";

    pub fn new(
        engine: &mut Engine,
        sender: MessageSender,
        icon_request_sender: Sender<IconRequest>,
    ) -> Self {
        let mut preview = PreviewPanel::new(engine, 350, 400);

        let graph = &mut engine.scenes[preview.scene()].graph;
        let sphere = MeshBuilder::new(BaseBuilder::new())
            .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_embedded(
                SurfaceData::make_sphere(30, 30, 1.0, &Matrix4::identity()),
            ))
            .build()])
            .build(graph);
        preview.set_model(sphere, engine);

        let ctx = &mut engine.user_interfaces.first_mut().build_ctx();

        let shader_tooltip = make_simple_tooltip(
            ctx,
            "Drag and drop a shader from the asset browser \
        to assign it here.",
        );
        let panel;
        let properties_panel;
        let shader;
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(500.0).with_height(800.0))
            .open(false)
            .with_title(WindowTitle::text(Self::TITLE))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            GridBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(0)
                                    .with_child(
                                        TextBuilder::new(
                                            WidgetBuilder::new().on_row(0).on_column(0),
                                        )
                                        .with_vertical_text_alignment(VerticalAlignment::Center)
                                        .with_text("Shader")
                                        .build(ctx),
                                    )
                                    .with_child({
                                        shader = ResourceFieldBuilder::<Shader>::new(
                                            WidgetBuilder::new()
                                                .on_column(1)
                                                .with_tooltip(shader_tooltip),
                                            sender,
                                        )
                                        .build(
                                            ctx,
                                            icon_request_sender,
                                            engine.resource_manager.clone(),
                                        );
                                        shader
                                    }),
                            )
                            .add_column(Column::strict(150.0))
                            .add_column(Column::stretch())
                            .add_row(Row::strict(25.0))
                            .build(ctx),
                        )
                        .with_child(
                            ScrollViewerBuilder::new(WidgetBuilder::new().on_row(1))
                                .with_content({
                                    properties_panel = StackPanelBuilder::new(
                                        WidgetBuilder::new().with_margin(Thickness::uniform(2.0)),
                                    )
                                    .build(ctx);
                                    properties_panel
                                })
                                .build(ctx),
                        )
                        .with_child({
                            panel = BorderBuilder::new(WidgetBuilder::new().on_row(2).on_column(0))
                                .build(ctx);
                            panel
                        }),
                )
                .add_row(Row::strict(26.0))
                .add_row(Row::stretch())
                .add_row(Row::strict(300.0))
                .add_column(Column::stretch())
                .build(ctx),
            )
            .build(ctx);

        ctx.link(preview.root, panel);

        Self {
            window,
            preview,
            properties_panel,
            resource_views: Default::default(),
            material: None,
            shader,
        }
    }

    pub fn destroy(self, docking_manager: Handle<UiNode>, engine: &mut Engine) {
        self.preview.destroy(engine);
        let ui = engine.user_interfaces.first();
        ui.send_message(DockingManagerMessage::remove_floating_window(
            docking_manager,
            MessageDirection::ToWidget,
            self.window,
        ));
        ui.send_message(WidgetMessage::remove(
            self.window,
            MessageDirection::ToWidget,
        ));
    }

    pub fn set_material(
        &mut self,
        material: Option<MaterialResource>,
        sender: &MessageSender,
        icon_request_sender: &Sender<IconRequest>,
        engine: &mut Engine,
    ) {
        self.material = material;

        if let Some(material) = self.material.clone() {
            let material_name = engine
                .resource_manager
                .resource_path(&material)
                .unwrap_or(PathBuf::from("Embedded"))
                .to_string_lossy()
                .to_string();
            engine
                .user_interfaces
                .first()
                .send_message(WindowMessage::title(
                    self.window,
                    MessageDirection::ToWidget,
                    WindowTitle::text(format!("{} - {}", Self::TITLE, material_name)),
                ));

            engine.scenes[self.preview.scene()].graph[self.preview.model()]
                .as_mesh_mut()
                .surfaces_mut()
                .first_mut()
                .unwrap()
                .set_material(material);
        }

        let ui = engine.user_interfaces.first_mut();
        self.create_property_editors(ui, sender, icon_request_sender, &engine.resource_manager);
        self.sync_to_model(ui);
    }

    /// Creates property editors for each resource descriptor used by material's shader. Fills
    /// the views with default values from the shader.
    fn create_property_editors(
        &mut self,
        ui: &mut UserInterface,
        sender: &MessageSender,
        icon_request_sender: &Sender<IconRequest>,
        resource_manager: &ResourceManager,
    ) {
        for resource_view in self.resource_views.drain(..) {
            send_sync_message(
                ui,
                WidgetMessage::remove(resource_view.container, MessageDirection::ToWidget),
            );
        }

        let material = some_or_return!(self.material.clone());

        let mut material_state = material.state();
        let material = some_or_return!(material_state.data());

        let mut shader_state = material.shader().state();
        let shader = some_or_return!(shader_state.data());

        for resource in shader.definition.resources.iter() {
            if resource.is_built_in() {
                continue;
            }

            let view = match resource.kind {
                ShaderResourceKind::Texture { fallback, .. } => {
                    let texture = material
                        .texture_ref(resource.name.clone())
                        .and_then(|d| d.value.clone());
                    let path = texture
                        .as_ref()
                        .and_then(|tex| resource_manager.resource_path(tex.as_ref()))
                        .map(|path| path.to_string_lossy().to_string())
                        .unwrap_or_else(|| fallback.as_ref().to_string());
                    let ctx = &mut ui.build_ctx();
                    let editor = TextureEditorBuilder::new(
                        WidgetBuilder::new()
                            .with_height(28.0)
                            .with_user_data(Arc::new(Mutex::new(resource.name.clone())))
                            .with_allow_drop(true)
                            .with_tooltip(make_simple_tooltip(ctx, &path)),
                    )
                    .with_texture(texture)
                    .build(
                        ctx,
                        sender.clone(),
                        icon_request_sender.clone(),
                        resource_manager.clone(),
                    );
                    ResourceView {
                        name: resource.name.clone(),
                        container: make_item_container(ctx, resource.name.as_str(), editor),
                        kind: ResourceViewKind::Sampler,
                        editor,
                    }
                }
                ShaderResourceKind::PropertyGroup(ref group) => self.create_property_group_view(
                    resource.name.clone(),
                    group,
                    &mut ui.build_ctx(),
                ),
            };

            send_sync_message(
                ui,
                WidgetMessage::link(
                    view.container,
                    MessageDirection::ToWidget,
                    self.properties_panel,
                ),
            );

            self.resource_views.push(view);
        }
    }

    fn create_property_group_view(
        &mut self,
        name: ImmutableString,
        group: &[ShaderProperty],
        ctx: &mut BuildContext,
    ) -> ResourceView {
        let mut property_views = Vec::new();
        let property_containers = group
            .iter()
            .map(|property| {
                use ShaderPropertyKind as Kind;
                let item = match &property.kind {
                    Kind::Float { value } => value.make_view(ctx),
                    Kind::FloatArray { value, max_len } => make_array_view(ctx, value, *max_len),
                    Kind::Int { value } => value.make_view(ctx),
                    Kind::IntArray { value, max_len } => make_array_view(ctx, value, *max_len),
                    Kind::UInt { value } => value.make_view(ctx),
                    Kind::UIntArray { value, max_len } => make_array_view(ctx, value, *max_len),
                    Kind::Vector2 { value } => value.make_view(ctx),
                    Kind::Vector2Array { value, max_len } => make_array_view(ctx, value, *max_len),
                    Kind::Vector3 { value } => value.make_view(ctx),
                    Kind::Vector3Array { value, max_len } => make_array_view(ctx, value, *max_len),
                    Kind::Vector4 { value } => value.make_view(ctx),
                    Kind::Vector4Array { value, max_len } => make_array_view(ctx, value, *max_len),
                    Kind::Matrix2 { value } => value.make_view(ctx),
                    Kind::Matrix2Array { value, max_len } => make_array_view(ctx, value, *max_len),
                    Kind::Matrix3 { value } => value.make_view(ctx),
                    Kind::Matrix3Array { value, max_len } => make_array_view(ctx, value, *max_len),
                    Kind::Matrix4 { value } => value.make_view(ctx),
                    Kind::Matrix4Array { value, max_len } => make_array_view(ctx, value, *max_len),
                    Kind::Bool { value } => value.make_view(ctx),
                    Kind::Color { r, g, b, a } => ColorFieldBuilder::new(WidgetBuilder::new())
                        .with_color(Color::from_rgba(*r, *g, *b, *a))
                        .build(ctx),
                };

                property_views.push(item);
                make_item_container(ctx, &property.name, item)
            })
            .collect::<Vec<_>>();

        let panel = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .with_children(property_containers.iter().cloned()),
        )
        .build(ctx);

        ResourceView {
            container: make_item_container(ctx, name.as_str(), panel),
            name,
            kind: ResourceViewKind::PropertyGroup {
                property_views: group
                    .iter()
                    .zip(property_views.iter())
                    .map(|(property, view)| (property.name.clone(), *view))
                    .collect::<FxHashMap<_, _>>(),
            },
            editor: panel,
        }
    }

    fn find_resource_view(&self, name: &str) -> Option<&ResourceView> {
        self.resource_views
            .iter()
            .find(|view| view.name.as_str() == name)
    }

    pub fn sync_to_model(&mut self, ui: &mut UserInterface) {
        let Some(material) = self.material.as_ref() else {
            send_sync_message(
                ui,
                ListViewMessage::items(self.properties_panel, MessageDirection::ToWidget, vec![]),
            );
            return;
        };

        let mut material_state = material.state();
        let material = some_or_return!(material_state.data());

        for (binding_name, binding_value) in material.bindings() {
            let view = some_or_continue!(self.find_resource_view(binding_name));
            match binding_value {
                MaterialResourceBinding::Texture(ref binding) => send_sync_message(
                    ui,
                    TextureEditorMessage::texture(
                        view.editor,
                        MessageDirection::ToWidget,
                        binding.value.clone(),
                    ),
                ),
                MaterialResourceBinding::PropertyGroup(ref group) => {
                    let ResourceViewKind::PropertyGroup { ref property_views } = view.kind else {
                        continue;
                    };

                    for (property_name, property_value) in group.properties() {
                        let item = *some_or_continue!(property_views.get(property_name));
                        match property_value {
                            MaterialProperty::Float(value) => value.send(ui, item),
                            MaterialProperty::FloatArray(value) => sync_array(ui, item, value),
                            MaterialProperty::Int(value) => value.send(ui, item),
                            MaterialProperty::IntArray(value) => sync_array(ui, item, value),
                            MaterialProperty::UInt(value) => value.send(ui, item),
                            MaterialProperty::UIntArray(value) => sync_array(ui, item, value),
                            MaterialProperty::Vector2(value) => value.send(ui, item),
                            MaterialProperty::Vector2Array(value) => sync_array(ui, item, value),
                            MaterialProperty::Vector3(value) => value.send(ui, item),
                            MaterialProperty::Vector3Array(value) => sync_array(ui, item, value),
                            MaterialProperty::Vector4(value) => value.send(ui, item),
                            MaterialProperty::Vector4Array(value) => sync_array(ui, item, value),
                            MaterialProperty::Matrix2(value) => value.send(ui, item),
                            MaterialProperty::Matrix2Array(value) => sync_array(ui, item, value),
                            MaterialProperty::Matrix3(value) => value.send(ui, item),
                            MaterialProperty::Matrix3Array(value) => sync_array(ui, item, value),
                            MaterialProperty::Matrix4(value) => value.send(ui, item),
                            MaterialProperty::Matrix4Array(value) => sync_array(ui, item, value),
                            MaterialProperty::Bool(value) => value.send(ui, item),
                            MaterialProperty::Color(value) => value.send(ui, item),
                        }
                    }
                }
            }
        }

        send_sync_message(
            ui,
            ResourceFieldMessage::value(
                self.shader,
                MessageDirection::ToWidget,
                Some(material.shader().clone()),
            ),
        );
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        engine: &mut Engine,
        sender: &MessageSender,
    ) {
        let material = some_or_return!(self.material.clone());

        self.preview.handle_message(message, engine);

        if let Some(msg) = message.data::<ResourceFieldMessage<Shader>>() {
            if message.destination() == self.shader
                && message.direction() == MessageDirection::FromWidget
            {
                if let ResourceFieldMessage::Value(Some(value)) = msg {
                    sender.do_command(SetMaterialShaderCommand::new(
                        material.clone(),
                        value.clone(),
                        engine.resource_manager.resource_path(material.as_ref()),
                    ));
                }
            }
        }

        for resource_view in self.resource_views.iter() {
            match resource_view.kind {
                ResourceViewKind::Sampler => {
                    if let Some(TextureEditorMessage::Texture(texture)) = message.data() {
                        if resource_view.editor == message.destination() {
                            sender.do_command(SetMaterialBindingCommand::new(
                                material.clone(),
                                resource_view.name.clone(),
                                MaterialResourceBinding::Texture(MaterialTextureBinding {
                                    value: texture.clone(),
                                }),
                                engine.resource_manager.resource_path(material.as_ref()),
                            ));
                        }
                    }
                }
                ResourceViewKind::PropertyGroup { ref property_views } => {
                    for (property_name, property_view) in property_views {
                        if *property_view == message.destination()
                            && message.direction() == MessageDirection::FromWidget
                        {
                            let property_value = try_extract_message_value(message);

                            if let Some(property_value) = property_value {
                                sender.do_command(
                                    SetMaterialPropertyGroupPropertyValueCommand::new(
                                        material.clone(),
                                        resource_view.name.clone(),
                                        property_name.clone(),
                                        property_value,
                                        engine.resource_manager.resource_path(material.as_ref()),
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn update(
        &mut self,
        engine: &mut Engine,
        receiver: &Receiver<ResourceEvent>,
        sender: &MessageSender,
        icon_request_sender: &Sender<IconRequest>,
    ) {
        for event in receiver.try_iter() {
            if let ResourceEvent::Reloaded(resource) = event {
                let shader = some_or_continue!(resource.try_cast::<Shader>());
                let material_resource = some_or_continue!(self.material.as_ref());
                let material_data = material_resource.data_ref();
                let material = some_or_continue!(material_data.as_loaded_ref());
                if material.shader() == &shader {
                    drop(material_data);
                    self.set_material(self.material.clone(), sender, icon_request_sender, engine);
                }
            }
        }

        self.preview.update(engine)
    }
}

fn try_extract_message_value(message: &UiMessage) -> Option<MaterialProperty> {
    if let Some(NumericUpDownMessage::<f32>::Value(value)) = message.data() {
        Some(MaterialProperty::Float(*value))
    } else if let Some(NumericUpDownMessage::<i32>::Value(value)) = message.data() {
        Some(MaterialProperty::Int(*value))
    } else if let Some(NumericUpDownMessage::<u32>::Value(value)) = message.data() {
        Some(MaterialProperty::UInt(*value))
    } else if let Some(Vec2EditorMessage::Value(value)) = message.data() {
        Some(MaterialProperty::Vector2(*value))
    } else if let Some(Vec3EditorMessage::Value(value)) = message.data() {
        Some(MaterialProperty::Vector3(*value))
    } else if let Some(Vec4EditorMessage::Value(value)) = message.data() {
        Some(MaterialProperty::Vector4(*value))
    } else if let Some(ColorFieldMessage::Color(color)) = message.data() {
        Some(MaterialProperty::Color(*color))
    } else if let Some(MatrixEditorMessage::<2, 2, f32>::Value(value)) = message.data() {
        Some(MaterialProperty::Matrix2(*value))
    } else if let Some(MatrixEditorMessage::<3, 3, f32>::Value(value)) = message.data() {
        Some(MaterialProperty::Matrix3(*value))
    } else if let Some(MatrixEditorMessage::<4, 4, f32>::Value(value)) = message.data() {
        Some(MaterialProperty::Matrix4(*value))
    } else if let Some(CheckBoxMessage::Check(Some(value))) = message.data() {
        Some(MaterialProperty::Bool(*value))
    } else {
        None
    }
}

#[derive(Default)]
pub struct MaterialPlugin {
    material_editor: Option<MaterialEditor>,
    receiver: Option<Receiver<ResourceEvent>>,
}

impl EditorPlugin for MaterialPlugin {
    fn on_start(&mut self, editor: &mut Editor) {
        let container = &editor.plugins.get_mut::<InspectorPlugin>().property_editors;
        container.insert(MaterialPropertyEditorDefinition {
            sender: Mutex::new(editor.message_sender.clone()),
        });
        let (sender, receiver) = std::sync::mpsc::channel();
        editor
            .engine
            .resource_manager
            .state()
            .event_broadcaster
            .add(sender);
        self.receiver = Some(receiver);
        container.insert(InheritablePropertyEditorDefinition::<MaterialResource>::new());
        container.insert(InheritablePropertyEditorDefinition::<WidgetMaterial>::new());
        container.insert(InspectablePropertyEditorDefinition::<WidgetMaterial>::new());
    }

    fn on_sync_to_model(&mut self, editor: &mut Editor) {
        let material_editor = some_or_return!(self.material_editor.as_mut());
        material_editor.sync_to_model(editor.engine.user_interfaces.first_mut());
    }

    fn on_ui_message(&mut self, message: &mut UiMessage, editor: &mut Editor) {
        let mut material_editor = some_or_return!(self.material_editor.take());

        material_editor.handle_ui_message(message, &mut editor.engine, &editor.message_sender);

        if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == material_editor.window {
                material_editor.destroy(editor.docking_manager, &mut editor.engine);
                return;
            }
        }

        self.material_editor = Some(material_editor);
    }

    fn on_update(&mut self, editor: &mut Editor) {
        let material_editor = some_or_return!(self.material_editor.as_mut());
        if let Some(receiver) = self.receiver.as_ref() {
            material_editor.update(
                &mut editor.engine,
                receiver,
                &editor.message_sender,
                &editor.asset_browser.preview_sender,
            );
        }
    }

    fn on_message(&mut self, message: &Message, editor: &mut Editor) {
        let Message::OpenMaterialEditor(material) = message else {
            return;
        };

        let engine = &mut editor.engine;

        let material_editor = self.material_editor.get_or_insert_with(|| {
            MaterialEditor::new(
                engine,
                editor.message_sender.clone(),
                editor.asset_browser.preview_sender.clone(),
            )
        });

        material_editor.set_material(
            Some(material.clone()),
            &editor.message_sender,
            &editor.asset_browser.preview_sender,
            engine,
        );

        let ui = engine.user_interfaces.first_mut();
        ui.send_message(WindowMessage::open(
            material_editor.window,
            MessageDirection::ToWidget,
            true,
            true,
        ));
        ui.send_message(DockingManagerMessage::add_floating_window(
            editor.docking_manager,
            MessageDirection::ToWidget,
            material_editor.window,
        ));
    }
}
