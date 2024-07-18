use crate::{
    asset::item::AssetItem,
    fyrox::{
        asset::untyped::ResourceKind,
        core::{
            algebra::{Matrix4, Vector2, Vector3, Vector4},
            parking_lot::Mutex,
            pool::Handle,
            sstorage::ImmutableString,
            BiDirHashMap,
        },
        graph::BaseSceneGraph,
        gui::{
            border::BorderBuilder,
            check_box::{CheckBoxBuilder, CheckBoxMessage},
            color::{ColorFieldBuilder, ColorFieldMessage},
            grid::{Column, GridBuilder, Row},
            image::{Image, ImageBuilder, ImageMessage},
            list_view::{ListViewBuilder, ListViewMessage},
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
        material::{shader::Shader, MaterialResource, PropertyValue},
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
    scene::commands::material::{SetMaterialPropertyValueCommand, SetMaterialShaderCommand},
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

pub struct MaterialEditor {
    pub window: Handle<UiNode>,
    properties_panel: Handle<UiNode>,
    properties: BiDirHashMap<ImmutableString, Handle<UiNode>>,
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
    .add_row(Row::strict(24.0))
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
    ListViewBuilder::new(WidgetBuilder::new().with_height(24.0))
        .with_items(value.iter().map(|v| item_builder(ctx, v.clone())).collect())
        .build(ctx)
}

fn create_array_of_array_view<'a, T, B, I>(
    ctx: &mut BuildContext,
    value: I,
    mut item_builder: B,
) -> Handle<UiNode>
where
    T: 'a + Clone,
    B: FnMut(&mut BuildContext, T) -> Handle<UiNode>,
    I: Iterator<Item = &'a [T]>,
{
    ListViewBuilder::new(WidgetBuilder::new().with_height(24.0))
        .with_items(
            value
                .map(|v| create_array_view(ctx, v, &mut item_builder))
                .collect(),
        )
        .build(ctx)
}

fn create_float_view(ctx: &mut BuildContext, value: f32) -> Handle<UiNode> {
    NumericUpDownBuilder::new(WidgetBuilder::new().with_height(24.0))
        .with_value(value)
        .build(ctx)
}

fn create_int_view(ctx: &mut BuildContext, value: i32) -> Handle<UiNode> {
    NumericUpDownBuilder::new(WidgetBuilder::new().with_height(24.0))
        .with_value(value as f32)
        .with_precision(0)
        .with_max_value(i32::MAX as f32)
        .with_min_value(-i32::MAX as f32)
        .build(ctx)
}

fn create_uint_view(ctx: &mut BuildContext, value: u32) -> Handle<UiNode> {
    NumericUpDownBuilder::new(WidgetBuilder::new().with_height(24.0))
        .with_value(value as f32)
        .with_precision(0)
        .with_max_value(u32::MAX as f32)
        .with_min_value(0.0)
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

fn sync_array<T, B>(
    ui: &mut UserInterface,
    handle: Handle<UiNode>,
    array: &[T],
    mut item_builder: B,
) where
    T: Clone,
    B: FnMut(&mut BuildContext, T) -> Handle<UiNode>,
{
    let ctx = &mut ui.build_ctx();

    let new_items = array.iter().map(|v| item_builder(ctx, v.clone())).collect();

    send_sync_message(
        ui,
        ListViewMessage::items(handle, MessageDirection::ToWidget, new_items),
    );
}

fn sync_array_of_arrays<'a, T, I, B>(
    ui: &mut UserInterface,
    handle: Handle<UiNode>,
    array: I,
    mut item_builder: B,
) where
    T: 'a + Clone,
    I: Iterator<Item = &'a [T]>,
    B: FnMut(&mut BuildContext, T) -> Handle<UiNode>,
{
    let ctx = &mut ui.build_ctx();

    let new_items = array
        .map(|v| create_array_view(ctx, v, &mut item_builder))
        .collect();

    send_sync_message(
        ui,
        ListViewMessage::items(handle, MessageDirection::ToWidget, new_items),
    );
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
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(350.0))
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
            properties: Default::default(),
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

        self.sync_to_model(engine.user_interfaces.first_mut());
    }

    pub fn sync_to_model(&mut self, ui: &mut UserInterface) {
        if let Some(material) = self.material.as_ref() {
            let mut material_state = material.state();
            let Some(material) = material_state.data() else {
                return;
            };

            // Remove properties from ui.
            for name in self
                .properties
                .forward_map()
                .keys()
                .cloned()
                .collect::<Vec<_>>()
            {
                if !material.properties().contains_key(&name) {
                    let item_to_delete = ui
                        .node(
                            self.properties
                                .remove_by_key(&name)
                                .expect("Desync has occurred!"),
                        )
                        .parent();

                    send_sync_message(
                        ui,
                        WidgetMessage::remove(item_to_delete, MessageDirection::ToWidget),
                    );
                }
            }

            let mut sorted_properties = material
                .properties()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect::<Vec<_>>();
            sorted_properties.sort_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b));

            // Add missing properties.
            for (name, property_value) in sorted_properties.iter() {
                if !self.properties.contains_key(name) {
                    let ctx = &mut ui.build_ctx();

                    let item = match property_value {
                        PropertyValue::Float(value) => create_float_view(ctx, *value),
                        PropertyValue::FloatArray(value) => {
                            create_array_view(ctx, value, create_float_view)
                        }
                        PropertyValue::Int(value) => create_int_view(ctx, *value),
                        PropertyValue::IntArray(value) => {
                            create_array_view(ctx, value, create_int_view)
                        }
                        PropertyValue::UInt(value) => create_uint_view(ctx, *value),
                        PropertyValue::UIntArray(value) => {
                            create_array_view(ctx, value, create_uint_view)
                        }
                        PropertyValue::Vector2(value) => create_vec2_view(ctx, *value),
                        PropertyValue::Vector2Array(value) => {
                            create_array_view(ctx, value, create_vec2_view)
                        }
                        PropertyValue::Vector3(value) => create_vec3_view(ctx, *value),
                        PropertyValue::Vector3Array(value) => {
                            create_array_view(ctx, value, create_vec3_view)
                        }
                        PropertyValue::Vector4(value) => create_vec4_view(ctx, *value),
                        PropertyValue::Vector4Array(value) => {
                            create_array_view(ctx, value, create_vec4_view)
                        }
                        PropertyValue::Matrix2(value) => {
                            create_array_view(ctx, value.data.as_slice(), create_float_view)
                        }
                        PropertyValue::Matrix2Array(value) => create_array_of_array_view(
                            ctx,
                            value.iter().map(|m| m.data.as_slice()),
                            create_float_view,
                        ),
                        PropertyValue::Matrix3(value) => {
                            create_array_view(ctx, value.data.as_slice(), create_float_view)
                        }
                        PropertyValue::Matrix3Array(value) => create_array_of_array_view(
                            ctx,
                            value.iter().map(|m| m.data.as_slice()),
                            create_float_view,
                        ),
                        PropertyValue::Matrix4(value) => {
                            create_array_view(ctx, value.data.as_slice(), create_float_view)
                        }
                        PropertyValue::Matrix4Array(value) => create_array_of_array_view(
                            ctx,
                            value.iter().map(|m| m.data.as_slice()),
                            create_float_view,
                        ),
                        PropertyValue::Bool(value) => CheckBoxBuilder::new(WidgetBuilder::new())
                            .checked(Some(*value))
                            .build(ctx),
                        PropertyValue::Color(value) => ColorFieldBuilder::new(WidgetBuilder::new())
                            .with_color(*value)
                            .build(ctx),
                        PropertyValue::Sampler { value, .. } => ImageBuilder::new(
                            WidgetBuilder::new()
                                .with_user_data(Arc::new(Mutex::new(name.clone())))
                                .with_allow_drop(true)
                                .with_context_menu(self.texture_context_menu.popup.clone()),
                        )
                        .with_opt_texture(value.clone().map(Into::into))
                        .build(ctx),
                    };

                    self.properties.insert(name.to_owned(), item);

                    let container = create_item_container(ctx, name, item);

                    send_sync_message(
                        ui,
                        WidgetMessage::link(
                            container,
                            MessageDirection::ToWidget,
                            self.properties_panel,
                        ),
                    );
                }
            }

            // Sync values.
            for (name, property_value) in material.properties() {
                let item = *self
                    .properties
                    .value_of(name)
                    .unwrap_or_else(|| panic!("Property not found {}", name));

                match property_value {
                    PropertyValue::Float(value) => {
                        send_sync_message(
                            ui,
                            NumericUpDownMessage::value(item, MessageDirection::ToWidget, *value),
                        );
                    }
                    PropertyValue::FloatArray(value) => {
                        sync_array(ui, item, value, create_float_view)
                    }
                    PropertyValue::Int(value) => {
                        send_sync_message(
                            ui,
                            NumericUpDownMessage::value(
                                item,
                                MessageDirection::ToWidget,
                                *value as f32,
                            ),
                        );
                    }
                    PropertyValue::IntArray(value) => sync_array(ui, item, value, create_int_view),
                    PropertyValue::UInt(value) => {
                        send_sync_message(
                            ui,
                            NumericUpDownMessage::value(
                                item,
                                MessageDirection::ToWidget,
                                *value as f32,
                            ),
                        );
                    }
                    PropertyValue::UIntArray(value) => {
                        sync_array(ui, item, value, create_uint_view)
                    }
                    PropertyValue::Vector2(value) => send_sync_message(
                        ui,
                        Vec2EditorMessage::value(item, MessageDirection::ToWidget, *value),
                    ),
                    PropertyValue::Vector2Array(value) => {
                        sync_array(ui, item, value, create_vec2_view)
                    }
                    PropertyValue::Vector3(value) => send_sync_message(
                        ui,
                        Vec3EditorMessage::value(item, MessageDirection::ToWidget, *value),
                    ),
                    PropertyValue::Vector3Array(value) => {
                        sync_array(ui, item, value, create_vec3_view)
                    }
                    PropertyValue::Vector4(value) => send_sync_message(
                        ui,
                        Vec4EditorMessage::value(item, MessageDirection::ToWidget, *value),
                    ),
                    PropertyValue::Vector4Array(value) => {
                        sync_array(ui, item, value, create_vec4_view)
                    }
                    PropertyValue::Matrix2(value) => {
                        sync_array(ui, item, value.as_slice(), create_float_view)
                    }
                    PropertyValue::Matrix2Array(value) => sync_array_of_arrays(
                        ui,
                        item,
                        value.iter().map(|m| m.as_slice()),
                        create_float_view,
                    ),
                    PropertyValue::Matrix3(value) => {
                        sync_array(ui, item, value.as_slice(), create_float_view)
                    }
                    PropertyValue::Matrix3Array(value) => sync_array_of_arrays(
                        ui,
                        item,
                        value.iter().map(|m| m.as_slice()),
                        create_float_view,
                    ),
                    PropertyValue::Matrix4(value) => {
                        sync_array(ui, item, value.as_slice(), create_float_view)
                    }
                    PropertyValue::Matrix4Array(value) => sync_array_of_arrays(
                        ui,
                        item,
                        value.iter().map(|m| m.as_slice()),
                        create_float_view,
                    ),
                    PropertyValue::Bool(value) => {
                        send_sync_message(
                            ui,
                            CheckBoxMessage::checked(
                                item,
                                MessageDirection::ToWidget,
                                Some(*value),
                            ),
                        );
                    }
                    PropertyValue::Color(value) => {
                        send_sync_message(
                            ui,
                            ColorFieldMessage::color(item, MessageDirection::ToWidget, *value),
                        );
                    }
                    PropertyValue::Sampler { value, .. } => send_sync_message(
                        ui,
                        ImageMessage::texture(
                            item,
                            MessageDirection::ToWidget,
                            value.clone().map(Into::into),
                        ),
                    ),
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
        } else {
            send_sync_message(
                ui,
                ListViewMessage::items(self.properties_panel, MessageDirection::ToWidget, vec![]),
            );
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        engine: &mut Engine,
        sender: &MessageSender,
    ) {
        self.preview.handle_message(message, engine);

        if let Some(material) = self.material.clone() {
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
                    if let Some(property_name) = engine
                        .user_interfaces
                        .first_mut()
                        .node(self.texture_context_menu.target)
                        .user_data_cloned::<ImmutableString>()
                    {
                        sender.do_command(SetMaterialPropertyValueCommand::new(
                            material.clone(),
                            property_name.clone(),
                            PropertyValue::Sampler {
                                value: None,
                                fallback: Default::default(),
                            },
                        ));
                    }
                }
            }

            if let Some(property_name) = self.properties.key_of(&message.destination()) {
                let property_value = if let Some(NumericUpDownMessage::Value(value)) =
                    message.data::<NumericUpDownMessage<f32>>()
                {
                    if message.direction() == MessageDirection::FromWidget {
                        // NumericUpDown is used for Float, Int, UInt properties, so we have to check
                        // the actual property "type" to create suitable value from f32.
                        if let Some(material) = material.state().data() {
                            match material.property_ref(property_name).unwrap() {
                                PropertyValue::Float(_) => Some(PropertyValue::Float(*value)),
                                PropertyValue::Int(_) => Some(PropertyValue::Int(*value as i32)),
                                PropertyValue::UInt(_) => Some(PropertyValue::UInt(*value as u32)),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else if let Some(Vec2EditorMessage::Value(value)) =
                    message.data::<Vec2EditorMessage<f32>>()
                {
                    if message.direction() == MessageDirection::FromWidget {
                        Some(PropertyValue::Vector2(*value))
                    } else {
                        None
                    }
                } else if let Some(Vec3EditorMessage::Value(value)) =
                    message.data::<Vec3EditorMessage<f32>>()
                {
                    if message.direction() == MessageDirection::FromWidget {
                        Some(PropertyValue::Vector3(*value))
                    } else {
                        None
                    }
                } else if let Some(Vec4EditorMessage::Value(value)) =
                    message.data::<Vec4EditorMessage<f32>>()
                {
                    if message.direction() == MessageDirection::FromWidget {
                        Some(PropertyValue::Vector4(*value))
                    } else {
                        None
                    }
                } else if let Some(ColorFieldMessage::Color(color)) =
                    message.data::<ColorFieldMessage>()
                {
                    if message.direction() == MessageDirection::FromWidget {
                        Some(PropertyValue::Color(*color))
                    } else {
                        None
                    }
                } else if let Some(WidgetMessage::Drop(handle)) = message.data::<WidgetMessage>() {
                    if let Some(asset_item) = engine
                        .user_interfaces
                        .first_mut()
                        .node(*handle)
                        .cast::<AssetItem>()
                    {
                        let texture = asset_item.resource::<Texture>();

                        engine
                            .user_interfaces
                            .first_mut()
                            .send_message(ImageMessage::texture(
                                message.destination(),
                                MessageDirection::ToWidget,
                                texture.clone().map(Into::into),
                            ));

                        Some(PropertyValue::Sampler {
                            value: texture,
                            fallback: Default::default(),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(property_value) = property_value {
                    sender.do_command(SetMaterialPropertyValueCommand::new(
                        material,
                        property_name.clone(),
                        property_value,
                    ));
                }
            }
        }
    }

    pub fn update(&mut self, engine: &mut Engine) {
        self.preview.update(engine)
    }
}
