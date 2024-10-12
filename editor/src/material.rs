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
    command::make_command,
    fyrox::{
        asset::untyped::ResourceKind,
        core::{algebra::Matrix4, log::Log, pool::Handle},
        graph::BaseSceneGraph,
        gui::{
            border::BorderBuilder,
            grid::{Column, GridBuilder, Row},
            inspector::{
                editors::{
                    collection::VecCollectionPropertyEditorDefinition,
                    enumeration::EnumPropertyEditorDefinition,
                    inspectable::InspectablePropertyEditorDefinition,
                    PropertyEditorDefinitionContainer,
                },
                Inspector, InspectorBuilder, InspectorContext, InspectorMessage,
            },
            message::{MessageDirection, UiMessage},
            scroll_viewer::ScrollViewerBuilder,
            widget::WidgetBuilder,
            window::{WindowBuilder, WindowTitle},
            UiNode, UserInterface,
        },
        material::{
            shader::SamplerFallback, Material, MaterialProperty, MaterialResource,
            MaterialResourceBinding, MaterialResourceBindingValue, PropertyGroup, PropertyValue,
        },
        scene::{
            base::BaseBuilder,
            mesh::{
                surface::{SurfaceBuilder, SurfaceData, SurfaceResource},
                MeshBuilder,
            },
        },
    },
    inspector::{editors::make_property_editors_container, EditorEnvironment},
    message::MessageSender,
    preview::PreviewPanel,
    Engine, Message, MSG_SYNC_FLAG,
};
use std::sync::Arc;

pub struct MaterialEditor {
    pub window: Handle<UiNode>,
    inspector: Handle<UiNode>,
    preview: PreviewPanel,
    material: Option<MaterialResource>,
    property_editors: Arc<PropertyEditorDefinitionContainer>,
    sender: MessageSender,
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

        let inspector = InspectorBuilder::new(WidgetBuilder::new()).build(ctx);
        let mut property_editors = make_property_editors_container(sender.clone());
        property_editors.insert(VecCollectionPropertyEditorDefinition::<
            MaterialResourceBinding,
        >::new());
        property_editors
            .insert(InspectablePropertyEditorDefinition::<MaterialResourceBinding>::new());
        property_editors
            .insert(EnumPropertyEditorDefinition::<MaterialResourceBindingValue>::new());
        property_editors.insert(EnumPropertyEditorDefinition::<SamplerFallback>::new());
        property_editors.insert(InspectablePropertyEditorDefinition::<PropertyGroup>::new());
        property_editors.insert(VecCollectionPropertyEditorDefinition::<MaterialProperty>::new());
        property_editors.insert(InspectablePropertyEditorDefinition::<MaterialProperty>::new());
        property_editors.insert(EnumPropertyEditorDefinition::<PropertyValue>::new());
        let property_editors = Arc::new(property_editors);

        let panel;
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(350.0))
            .open(false)
            .with_title(WindowTitle::text("Material Editor"))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            ScrollViewerBuilder::new(
                                WidgetBuilder::new().with_height(300.0).on_row(0),
                            )
                            .with_content(inspector)
                            .build(ctx),
                        )
                        .with_child({
                            panel = BorderBuilder::new(WidgetBuilder::new().on_row(1).on_column(0))
                                .build(ctx);
                            panel
                        }),
                )
                .add_row(Row::stretch())
                .add_row(Row::strict(300.0))
                .add_column(Column::stretch())
                .build(ctx),
            )
            .build(ctx);

        ctx.link(preview.root, panel);

        Self {
            window,
            inspector,
            preview,
            property_editors,
            material: None,
            sender,
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
                .set_material(material.clone());

            let mut material_state = material.state();
            if let Some(material) = material_state.data() {
                let ui = engine.user_interfaces.first_mut();
                let environment = Arc::new(EditorEnvironment {
                    resource_manager: engine.resource_manager.clone(),
                    serialization_context: engine.serialization_context.clone(),
                    available_animations: Default::default(),
                    sender: self.sender.clone(),
                });
                let ctx = InspectorContext::from_object(
                    material,
                    &mut ui.build_ctx(),
                    self.property_editors.clone(),
                    Some(environment),
                    MSG_SYNC_FLAG,
                    0,
                    true,
                    Default::default(),
                    110.0,
                );
                ui.send_message(InspectorMessage::context(
                    self.inspector,
                    MessageDirection::ToWidget,
                    ctx,
                ));
            };
        }

        self.sync_to_model(engine.user_interfaces.first_mut());
    }

    pub fn sync_to_model(&mut self, ui: &mut UserInterface) {
        if let Some(material) = self.material.as_ref() {
            let mut material_state = material.state();
            let Some(material) = material_state.data() else {
                return;
            };

            let ctx = ui
                .node(self.inspector)
                .cast::<Inspector>()
                .unwrap()
                .context()
                .clone();

            if let Err(sync_errors) = ctx.sync(material, ui, 0, true, Default::default()) {
                for error in sync_errors {
                    Log::err(format!("Failed to sync property. Reason: {:?}", error))
                }
            }
        } else {
            ui.send_message(InspectorMessage::context(
                self.inspector,
                MessageDirection::ToWidget,
                InspectorContext::default(),
            ));
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        engine: &mut Engine,
        sender: &MessageSender,
    ) {
        self.preview.handle_message(message, engine);

        if message.destination() == self.inspector
            && message.direction() == MessageDirection::FromWidget
        {
            if let Some(InspectorMessage::PropertyChanged(args)) =
                message.data::<InspectorMessage>()
            {
                if let Some(material) = self.material.clone() {
                    let command = make_command(args, move |_| {
                        let mut material_data = material.data_ref();
                        let data = &mut *material_data;
                        // FIXME: HACK!
                        unsafe {
                            std::mem::transmute::<&'_ mut Material, &'static mut Material>(data)
                        }
                    });

                    if let Some(command) = command {
                        sender.send(Message::DoCommand(command));
                    }
                }
            }
        }
    }

    pub fn update(&mut self, engine: &mut Engine) {
        self.preview.update(engine)
    }
}
