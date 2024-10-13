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
    asset::item::AssetItem,
    fyrox::{
        asset::{manager::ResourceManager, untyped::ResourceKind},
        core::{
            algebra::{Matrix4, SMatrix, Vector2, Vector3, Vector4},
            color::Color,
            parking_lot::Mutex,
            pool::Handle,
            sstorage::ImmutableString,
        },
        fxhash::FxHashMap,
        graph::{BaseSceneGraph, SceneGraph},
        gui::{
            border::BorderBuilder,
            check_box::{CheckBoxBuilder, CheckBoxMessage},
            color::{ColorFieldBuilder, ColorFieldMessage},
            grid::{Column, GridBuilder, Row},
            image::{Image, ImageBuilder, ImageMessage},
            list_view::{ListView, ListViewBuilder, ListViewMessage},
            matrix::{MatrixEditorBuilder, MatrixEditorMessage},
            menu::{ContextMenuBuilder, MenuItemBuilder, MenuItemContent, MenuItemMessage},
            message::{MessageDirection, UiMessage},
            numeric::{NumericUpDownBuilder, NumericUpDownMessage},
            popup::{Placement, PopupBuilder, PopupMessage},
            scroll_viewer::ScrollViewerBuilder,
            stack_panel::StackPanelBuilder,
            text::TextBuilder,
            vec::{
                Vec2EditorBuilder, Vec2EditorMessage, Vec3EditorBuilder, Vec3EditorMessage,
                Vec4EditorBuilder, Vec4EditorMessage,
            },
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowTitle},
            BuildContext, RcUiNodeHandle, Thickness, UiNode, UserInterface, VerticalAlignment,
        },
        material::{
            shader::{Shader, ShaderResourceKind},
            MaterialPropertyValue, MaterialResource, MaterialResourceBindingValue,
        },
        renderer::framework::gpu_program::{ShaderProperty, ShaderPropertyKind},
        resource::texture::Texture,
        scene::{
            base::BaseBuilder,
            mesh::{
                surface::{SurfaceBuilder, SurfaceData, SurfaceResource},
                MeshBuilder,
            },
        },
    },
    inspector::editors::resource::{ResourceFieldBuilder, ResourceFieldMessage},
    message::MessageSender,
    preview::PreviewPanel,
    scene::commands::material::{
        SetMaterialBindingCommand, SetMaterialPropertyGroupPropertyValueCommand,
        SetMaterialShaderCommand,
    },
    send_sync_message, Engine, Message,
};
use std::sync::Arc;

struct TextureContextMenu {
    popup: RcUiNodeHandle,
    show_in_asset_browser: Handle<UiNode>,
    unassign: Handle<UiNode>,
    target: Handle<UiNode>,
}

impl TextureContextMenu {
    fn new(ctx: &mut BuildContext) -> Self {
        let show_in_asset_browser;
        let unassign;
        let popup = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new().with_visibility(false)).with_content(
                StackPanelBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            show_in_asset_browser = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text("Show In Asset Browser"))
                                .build(ctx);
                            show_in_asset_browser
                        })
                        .with_child({
                            unassign = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text("Unassign"))
                                .build(ctx);
                            unassign
                        }),
                )
                .build(ctx),
            ),
        )
        .build(ctx);
        let popup = RcUiNodeHandle::new(popup, ctx.sender());

        Self {
            popup,
            show_in_asset_browser,
            unassign,
            target: Default::default(),
        }
    }
}

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
    texture_context_menu: TextureContextMenu,
}

fn create_item_container(
    ctx: &mut BuildContext,
    name: &str,
    item: Handle<UiNode>,
) -> Handle<UiNode> {
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

fn create_array_view<T, B>(
    ctx: &mut BuildContext,
    value: &[T],
    mut item_builder: B,
) -> Handle<UiNode>
where
    T: Clone,
    B: FnMut(&mut BuildContext, T) -> Handle<UiNode>,
{
    ListViewBuilder::new(WidgetBuilder::new())
        .with_items(value.iter().map(|v| item_builder(ctx, v.clone())).collect())
        .build(ctx)
}

fn create_float_view(ctx: &mut BuildContext, value: f32) -> Handle<UiNode> {
    NumericUpDownBuilder::<f32>::new(WidgetBuilder::new().with_height(24.0))
        .with_value(value)
        .build(ctx)
}

fn create_int_view(ctx: &mut BuildContext, value: i32) -> Handle<UiNode> {
    NumericUpDownBuilder::<i32>::new(WidgetBuilder::new().with_height(24.0))
        .with_value(value)
        .with_precision(0)
        .build(ctx)
}

fn create_uint_view(ctx: &mut BuildContext, value: u32) -> Handle<UiNode> {
    NumericUpDownBuilder::<u32>::new(WidgetBuilder::new().with_height(24.0))
        .with_value(value)
        .with_precision(0)
        .build(ctx)
}

fn create_vec2_view(ctx: &mut BuildContext, value: Vector2<f32>) -> Handle<UiNode> {
    Vec2EditorBuilder::new(WidgetBuilder::new().with_height(24.0))
        .with_value(value)
        .build(ctx)
}

fn create_vec3_view(ctx: &mut BuildContext, value: Vector3<f32>) -> Handle<UiNode> {
    Vec3EditorBuilder::new(WidgetBuilder::new().with_height(24.0))
        .with_value(value)
        .build(ctx)
}

fn create_vec4_view(ctx: &mut BuildContext, value: Vector4<f32>) -> Handle<UiNode> {
    Vec4EditorBuilder::new(WidgetBuilder::new().with_height(24.0))
        .with_value(value)
        .build(ctx)
}

fn create_mat_view<const R: usize, const C: usize>(
    ctx: &mut BuildContext,
    value: SMatrix<f32, R, C>,
) -> Handle<UiNode> {
    MatrixEditorBuilder::<R, C, f32>::new(WidgetBuilder::new())
        .with_value(value)
        .build(ctx)
}

fn sync_array<T, B>(ui: &UserInterface, handle: Handle<UiNode>, array: &[T], mut message_builder: B)
where
    T: Clone,
    B: FnMut(&T, Handle<UiNode>) -> UiMessage,
{
    let views = &**ui.try_get_of_type::<ListView>(handle).unwrap().items;
    for (item, view) in array.iter().zip(views) {
        send_sync_message(ui, message_builder(item, *view))
    }
}

fn pad_vec<T: Default + Clone>(v: &[T], max_len: usize) -> Vec<T> {
    let mut vec = v.to_vec();
    for _ in v.len()..max_len {
        vec.push(T::default());
    }
    vec
}

impl MaterialEditor {
    pub fn new(engine: &mut Engine, sender: MessageSender) -> Self {
        let mut preview = PreviewPanel::new(engine, 350, 400);

        let graph = &mut engine.scenes[preview.scene()].graph;
        let sphere = MeshBuilder::new(BaseBuilder::new())
            .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_ok(
                ResourceKind::Embedded,
                SurfaceData::make_sphere(30, 30, 1.0, &Matrix4::identity()),
            ))
            .build()])
            .build(graph);
        preview.set_model(sphere, engine);

        let ctx = &mut engine.user_interfaces.first_mut().build_ctx();

        let panel;
        let properties_panel;
        let shader;
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(500.0).with_height(800.0))
            .open(false)
            .with_title(WindowTitle::text("Material Editor"))
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
                                            WidgetBuilder::new().on_column(1),
                                            sender,
                                        )
                                        .build(ctx, engine.resource_manager.clone());
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
                                    properties_panel =
                                        StackPanelBuilder::new(WidgetBuilder::new()).build(ctx);
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
            texture_context_menu: TextureContextMenu::new(ctx),
            window,
            preview,
            properties_panel,
            resource_views: Default::default(),
            material: None,
            shader,
        }
    }

    pub fn set_material(&mut self, material: Option<MaterialResource>, engine: &mut Engine) {
        self.material = material;

        if let Some(material) = self.material.clone() {
            engine.scenes[self.preview.scene()].graph[self.preview.model()]
                .as_mesh_mut()
                .surfaces_mut()
                .first_mut()
                .unwrap()
                .set_material(material);
        }

        let ui = engine.user_interfaces.first_mut();
        self.create_property_editors(ui, &engine.resource_manager);
        self.sync_to_model(ui);
    }

    /// Creates property editors for each resource descriptor used by material's shader. Fills
    /// the views with default values from the shader.
    fn create_property_editors(
        &mut self,
        ui: &mut UserInterface,
        resource_manager: &ResourceManager,
    ) {
        for resource_view in self.resource_views.drain(..) {
            send_sync_message(
                ui,
                WidgetMessage::remove(resource_view.container, MessageDirection::ToWidget),
            );
        }

        let Some(material) = self.material.clone() else {
            return;
        };

        let mut material_state = material.state();
        let Some(material) = material_state.data() else {
            return;
        };

        let mut shader_state = material.shader().state();
        let Some(shader) = shader_state.data() else {
            return;
        };

        for resource in shader.definition.resources.iter() {
            let view = match resource.kind {
                ShaderResourceKind::Sampler { ref default, .. } => {
                    let value = default
                        .as_ref()
                        .and_then(|default| resource_manager.try_request::<Texture>(&default));
                    let editor = ImageBuilder::new(
                        WidgetBuilder::new()
                            .with_height(28.0)
                            .with_user_data(Arc::new(Mutex::new(resource.name.clone())))
                            .with_allow_drop(true)
                            .with_context_menu(self.texture_context_menu.popup.clone()),
                    )
                    .with_opt_texture(value.map(Into::into))
                    .build(&mut ui.build_ctx());
                    ResourceView {
                        name: resource.name.clone(),
                        container: create_item_container(
                            &mut ui.build_ctx(),
                            resource.name.as_str(),
                            editor,
                        ),
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
                let item = match &property.kind {
                    ShaderPropertyKind::Float(value) => create_float_view(ctx, *value),
                    ShaderPropertyKind::FloatArray { value, max_len } => {
                        create_array_view(ctx, &pad_vec(value, *max_len), create_float_view)
                    }
                    ShaderPropertyKind::Int(value) => create_int_view(ctx, *value),
                    ShaderPropertyKind::IntArray { value, max_len } => {
                        create_array_view(ctx, &pad_vec(value, *max_len), create_int_view)
                    }
                    ShaderPropertyKind::UInt(value) => create_uint_view(ctx, *value),
                    ShaderPropertyKind::UIntArray { value, max_len } => {
                        create_array_view(ctx, &pad_vec(value, *max_len), create_uint_view)
                    }
                    ShaderPropertyKind::Vector2(value) => create_vec2_view(ctx, *value),
                    ShaderPropertyKind::Vector2Array { value, max_len } => {
                        create_array_view(ctx, &pad_vec(value, *max_len), create_vec2_view)
                    }
                    ShaderPropertyKind::Vector3(value) => create_vec3_view(ctx, *value),
                    ShaderPropertyKind::Vector3Array { value, max_len } => {
                        create_array_view(ctx, &pad_vec(value, *max_len), create_vec3_view)
                    }
                    ShaderPropertyKind::Vector4(value) => create_vec4_view(ctx, *value),
                    ShaderPropertyKind::Vector4Array { value, max_len } => {
                        create_array_view(ctx, &pad_vec(value, *max_len), create_vec4_view)
                    }
                    ShaderPropertyKind::Matrix2(value) => create_mat_view(ctx, *value),
                    ShaderPropertyKind::Matrix2Array { value, max_len } => {
                        create_array_view(ctx, &pad_vec(value, *max_len), create_mat_view)
                    }
                    ShaderPropertyKind::Matrix3(value) => create_mat_view(ctx, *value),
                    ShaderPropertyKind::Matrix3Array { value, max_len } => {
                        create_array_view(ctx, &pad_vec(value, *max_len), create_mat_view)
                    }
                    ShaderPropertyKind::Matrix4(value) => create_mat_view(ctx, *value),
                    ShaderPropertyKind::Matrix4Array { value, max_len } => {
                        create_array_view(ctx, &pad_vec(value, *max_len), create_mat_view)
                    }
                    ShaderPropertyKind::Bool(value) => CheckBoxBuilder::new(WidgetBuilder::new())
                        .checked(Some(*value))
                        .build(ctx),
                    ShaderPropertyKind::Color { r, g, b, a } => {
                        ColorFieldBuilder::new(WidgetBuilder::new())
                            .with_color(Color::from_rgba(*r, *g, *b, *a))
                            .build(ctx)
                    }
                };

                property_views.push(item);
                create_item_container(ctx, &property.name, item)
            })
            .collect::<Vec<_>>();

        let panel = StackPanelBuilder::new(
            WidgetBuilder::new().with_children(property_containers.iter().cloned()),
        )
        .build(ctx);

        ResourceView {
            container: create_item_container(ctx, name.as_str(), panel),
            name,
            kind: ResourceViewKind::PropertyGroup {
                property_views: group
                    .iter()
                    .zip(property_views.iter())
                    .map(|(property, view)| ((&property.name).into(), *view))
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
        let Some(material) = material_state.data() else {
            return;
        };

        for binding in material.bindings() {
            let Some(view) = self.find_resource_view(&binding.name) else {
                continue;
            };
            match binding.value {
                MaterialResourceBindingValue::Sampler { ref value, .. } => send_sync_message(
                    ui,
                    ImageMessage::texture(
                        view.editor,
                        MessageDirection::ToWidget,
                        value.clone().map(Into::into),
                    ),
                ),
                MaterialResourceBindingValue::PropertyGroup(ref group) => {
                    let ResourceViewKind::PropertyGroup { ref property_views } = view.kind else {
                        continue;
                    };

                    for property in group.properties() {
                        let item = *property_views
                            .get(&property.name)
                            .unwrap_or_else(|| panic!("Property not found {}", property.name));

                        match &property.value {
                            MaterialPropertyValue::Float(value) => {
                                send_sync_message(
                                    ui,
                                    NumericUpDownMessage::value(
                                        item,
                                        MessageDirection::ToWidget,
                                        *value,
                                    ),
                                );
                            }
                            MaterialPropertyValue::FloatArray(value) => {
                                sync_array(ui, item, value, |value, item| {
                                    NumericUpDownMessage::value(
                                        item,
                                        MessageDirection::ToWidget,
                                        *value,
                                    )
                                })
                            }
                            MaterialPropertyValue::Int(value) => {
                                send_sync_message(
                                    ui,
                                    NumericUpDownMessage::value(
                                        item,
                                        MessageDirection::ToWidget,
                                        *value as f32,
                                    ),
                                );
                            }
                            MaterialPropertyValue::IntArray(value) => {
                                sync_array(ui, item, value, |value, item| {
                                    NumericUpDownMessage::value(
                                        item,
                                        MessageDirection::ToWidget,
                                        *value as f32,
                                    )
                                })
                            }
                            MaterialPropertyValue::UInt(value) => {
                                send_sync_message(
                                    ui,
                                    NumericUpDownMessage::value(
                                        item,
                                        MessageDirection::ToWidget,
                                        *value as f32,
                                    ),
                                );
                            }
                            MaterialPropertyValue::UIntArray(value) => {
                                sync_array(ui, item, value, |value, item| {
                                    NumericUpDownMessage::value(
                                        item,
                                        MessageDirection::ToWidget,
                                        *value as f32,
                                    )
                                })
                            }
                            MaterialPropertyValue::Vector2(value) => send_sync_message(
                                ui,
                                Vec2EditorMessage::value(item, MessageDirection::ToWidget, *value),
                            ),
                            MaterialPropertyValue::Vector2Array(value) => {
                                sync_array(ui, item, value, |value, item| {
                                    Vec2EditorMessage::value(
                                        item,
                                        MessageDirection::ToWidget,
                                        *value,
                                    )
                                })
                            }
                            MaterialPropertyValue::Vector3(value) => send_sync_message(
                                ui,
                                Vec3EditorMessage::value(item, MessageDirection::ToWidget, *value),
                            ),
                            MaterialPropertyValue::Vector3Array(value) => {
                                sync_array(ui, item, value, |value, item| {
                                    Vec3EditorMessage::value(
                                        item,
                                        MessageDirection::ToWidget,
                                        *value,
                                    )
                                })
                            }
                            MaterialPropertyValue::Vector4(value) => send_sync_message(
                                ui,
                                Vec4EditorMessage::value(item, MessageDirection::ToWidget, *value),
                            ),
                            MaterialPropertyValue::Vector4Array(value) => {
                                sync_array(ui, item, value, |value, item| {
                                    Vec4EditorMessage::value(
                                        item,
                                        MessageDirection::ToWidget,
                                        *value,
                                    )
                                })
                            }
                            MaterialPropertyValue::Matrix2(value) => send_sync_message(
                                ui,
                                MatrixEditorMessage::value(
                                    item,
                                    MessageDirection::ToWidget,
                                    *value,
                                ),
                            ),
                            MaterialPropertyValue::Matrix2Array(value) => {
                                sync_array(ui, item, value, |value, item| {
                                    MatrixEditorMessage::value(
                                        item,
                                        MessageDirection::ToWidget,
                                        *value,
                                    )
                                })
                            }
                            MaterialPropertyValue::Matrix3(value) => send_sync_message(
                                ui,
                                MatrixEditorMessage::value(
                                    item,
                                    MessageDirection::ToWidget,
                                    *value,
                                ),
                            ),
                            MaterialPropertyValue::Matrix3Array(value) => {
                                sync_array(ui, item, value, |value, item| {
                                    MatrixEditorMessage::value(
                                        item,
                                        MessageDirection::ToWidget,
                                        *value,
                                    )
                                })
                            }
                            MaterialPropertyValue::Matrix4(value) => send_sync_message(
                                ui,
                                MatrixEditorMessage::value(
                                    item,
                                    MessageDirection::ToWidget,
                                    *value,
                                ),
                            ),
                            MaterialPropertyValue::Matrix4Array(value) => {
                                sync_array(ui, item, value, |value, item| {
                                    MatrixEditorMessage::value(
                                        item,
                                        MessageDirection::ToWidget,
                                        *value,
                                    )
                                })
                            }
                            MaterialPropertyValue::Bool(value) => {
                                send_sync_message(
                                    ui,
                                    CheckBoxMessage::checked(
                                        item,
                                        MessageDirection::ToWidget,
                                        Some(*value),
                                    ),
                                );
                            }
                            MaterialPropertyValue::Color(value) => {
                                send_sync_message(
                                    ui,
                                    ColorFieldMessage::color(
                                        item,
                                        MessageDirection::ToWidget,
                                        *value,
                                    ),
                                );
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
            }
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        engine: &mut Engine,
        sender: &MessageSender,
    ) {
        let Some(material) = self.material.clone() else {
            return;
        };

        self.preview.handle_message(message, engine);

        if let Some(msg) = message.data::<ResourceFieldMessage<Shader>>() {
            if message.destination() == self.shader
                && message.direction() == MessageDirection::FromWidget
            {
                if let ResourceFieldMessage::Value(Some(value)) = msg {
                    sender.do_command(SetMaterialShaderCommand::new(
                        material.clone(),
                        value.clone(),
                    ));
                }
            }
        } else if let Some(PopupMessage::Placement(Placement::Cursor(target))) =
            message.data::<PopupMessage>()
        {
            if message.destination() == self.texture_context_menu.popup.handle() {
                self.texture_context_menu.target = *target;
            }
        } else if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
            if message.destination() == self.texture_context_menu.show_in_asset_browser
                && self.texture_context_menu.target.is_some()
            {
                let path = (*engine
                    .user_interfaces
                    .first_mut()
                    .node(self.texture_context_menu.target)
                    .cast::<Image>()
                    .unwrap()
                    .texture)
                    .clone()
                    .and_then(|t| t.kind().into_path());

                if let Some(path) = path {
                    sender.send(Message::ShowInAssetBrowser(path));
                }
            } else if message.destination() == self.texture_context_menu.unassign
                && self.texture_context_menu.target.is_some()
            {
                if let Some(binding_name) = engine
                    .user_interfaces
                    .first_mut()
                    .node(self.texture_context_menu.target)
                    .user_data_cloned::<ImmutableString>()
                {
                    sender.do_command(SetMaterialBindingCommand::new(
                        material.clone(),
                        binding_name.clone(),
                        MaterialResourceBindingValue::Sampler {
                            value: None,
                            fallback: Default::default(),
                        },
                    ));
                }
            }
        }

        for resource_view in self.resource_views.iter() {
            match resource_view.kind {
                ResourceViewKind::Sampler => {
                    if let Some(WidgetMessage::Drop(handle)) = message.data::<WidgetMessage>() {
                        if let Some(asset_item) = engine
                            .user_interfaces
                            .first_mut()
                            .node(*handle)
                            .cast::<AssetItem>()
                        {
                            if resource_view.editor == message.destination() {
                                let texture = asset_item.resource::<Texture>();

                                engine.user_interfaces.first_mut().send_message(
                                    ImageMessage::texture(
                                        message.destination(),
                                        MessageDirection::ToWidget,
                                        texture.clone().map(Into::into),
                                    ),
                                );

                                sender.do_command(SetMaterialBindingCommand::new(
                                    material.clone(),
                                    resource_view.name.clone(),
                                    MaterialResourceBindingValue::Sampler {
                                        value: texture,
                                        fallback: Default::default(),
                                    },
                                ));
                            }
                        }
                    }
                }
                ResourceViewKind::PropertyGroup { ref property_views } => {
                    for (property_name, property_view) in property_views {
                        if *property_view == message.destination()
                            && message.direction() == MessageDirection::FromWidget
                        {
                            let property_value =
                                if let Some(NumericUpDownMessage::<f32>::Value(value)) =
                                    message.data()
                                {
                                    Some(MaterialPropertyValue::Float(*value))
                                } else if let Some(NumericUpDownMessage::<i32>::Value(value)) =
                                    message.data()
                                {
                                    Some(MaterialPropertyValue::Int(*value))
                                } else if let Some(NumericUpDownMessage::<u32>::Value(value)) =
                                    message.data()
                                {
                                    Some(MaterialPropertyValue::UInt(*value))
                                } else if let Some(Vec2EditorMessage::Value(value)) = message.data()
                                {
                                    Some(MaterialPropertyValue::Vector2(*value))
                                } else if let Some(Vec3EditorMessage::Value(value)) = message.data()
                                {
                                    Some(MaterialPropertyValue::Vector3(*value))
                                } else if let Some(Vec4EditorMessage::Value(value)) = message.data()
                                {
                                    Some(MaterialPropertyValue::Vector4(*value))
                                } else if let Some(ColorFieldMessage::Color(color)) = message.data()
                                {
                                    Some(MaterialPropertyValue::Color(*color))
                                } else if let Some(MatrixEditorMessage::<2, 2, f32>::Value(value)) =
                                    message.data()
                                {
                                    Some(MaterialPropertyValue::Matrix2(*value))
                                } else if let Some(MatrixEditorMessage::<3, 3, f32>::Value(value)) =
                                    message.data()
                                {
                                    Some(MaterialPropertyValue::Matrix3(*value))
                                } else if let Some(MatrixEditorMessage::<4, 4, f32>::Value(value)) =
                                    message.data()
                                {
                                    Some(MaterialPropertyValue::Matrix4(*value))
                                } else if let Some(CheckBoxMessage::Check(Some(value))) =
                                    message.data()
                                {
                                    Some(MaterialPropertyValue::Bool(*value))
                                } else {
                                    None
                                };

                            if let Some(property_value) = property_value {
                                sender.do_command(
                                    SetMaterialPropertyGroupPropertyValueCommand::new(
                                        material.clone(),
                                        resource_view.name.clone(),
                                        property_name.clone(),
                                        property_value,
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn update(&mut self, engine: &mut Engine) {
        self.preview.update(engine)
    }
}
