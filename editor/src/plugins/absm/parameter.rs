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

use crate::fyrox::{
    core::{log::Log, pool::Handle},
    generic_animation::machine::parameter::{Parameter, ParameterContainer, ParameterDefinition},
    graph::{BaseSceneGraph, PrefabData, SceneGraph, SceneGraphNode},
    gui::{
        inspector::{
            editors::{
                collection::VecCollectionPropertyEditorDefinition,
                enumeration::EnumPropertyEditorDefinition,
                inspectable::InspectablePropertyEditorDefinition,
                PropertyEditorDefinitionContainer,
            },
            InspectorBuilder, InspectorContext, InspectorMessage, PropertyAction,
        },
        message::UiMessage,
        scroll_viewer::ScrollViewerBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        BuildContext, UiNode, UserInterface,
    },
};
use crate::plugins::absm::command::fetch_machine;
use crate::plugins::inspector::editors::make_property_editors_container;
use crate::{
    command::make_command, message::MessageSender, Message, MessageDirection, MSG_SYNC_FLAG,
};
use fyrox::asset::manager::ResourceManager;
use std::sync::Arc;

pub struct ParameterPanel {
    pub window: Handle<UiNode>,
    inspector: Handle<UiNode>,
    property_editors: Arc<PropertyEditorDefinitionContainer>,
}

impl ParameterPanel {
    pub fn new(
        ctx: &mut BuildContext,
        sender: MessageSender,
        resource_manager: ResourceManager,
    ) -> Self {
        let property_editors = make_property_editors_container(sender, resource_manager);
        property_editors
            .insert(VecCollectionPropertyEditorDefinition::<ParameterDefinition>::new());
        property_editors.insert(EnumPropertyEditorDefinition::<Parameter>::new());
        property_editors.insert(InspectablePropertyEditorDefinition::<ParameterDefinition>::new());

        let inspector;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_title(WindowTitle::text("Parameters"))
            .with_content(
                ScrollViewerBuilder::new(WidgetBuilder::new())
                    .with_content({
                        inspector = InspectorBuilder::new(WidgetBuilder::new()).build(ctx);
                        inspector
                    })
                    .build(ctx),
            )
            .can_close(false)
            .can_minimize(false)
            .build(ctx);

        Self {
            window,
            inspector,
            property_editors: Arc::new(property_editors),
        }
    }

    pub fn on_selection_changed(
        &self,
        ui: &mut UserInterface,
        parameters: Option<&ParameterContainer>,
    ) {
        let inspector_context = parameters
            .map(|parameters| {
                InspectorContext::from_object(
                    parameters,
                    &mut ui.build_ctx(),
                    self.property_editors.clone(),
                    None,
                    MSG_SYNC_FLAG,
                    0,
                    true,
                    Default::default(),
                    150.0,
                )
            })
            .unwrap_or_default();

        ui.send_message(InspectorMessage::context(
            self.inspector,
            MessageDirection::ToWidget,
            inspector_context,
        ));
    }

    pub fn reset(&self, ui: &UserInterface) {
        ui.send_message(InspectorMessage::context(
            self.inspector,
            MessageDirection::ToWidget,
            Default::default(),
        ));
    }

    pub fn sync_to_model(&mut self, ui: &mut UserInterface, parameters: &ParameterContainer) {
        let ctx = ui
            .node(self.inspector)
            .cast::<fyrox::gui::inspector::Inspector>()
            .unwrap()
            .context()
            .clone();

        if let Err(sync_errors) = ctx.sync(parameters, ui, 0, true, Default::default()) {
            for error in sync_errors {
                Log::err(format!("Failed to sync property. Reason: {error:?}"))
            }
        }
    }

    pub fn handle_ui_message<P, G, N>(
        &mut self,
        message: &UiMessage,
        sender: &MessageSender,
        absm_node_handle: Handle<N>,
        parameters: &mut ParameterContainer,
        is_in_preview_mode: bool,
    ) where
        P: PrefabData<Graph = G>,
        G: SceneGraph<Node = N, Prefab = P>,
        N: SceneGraphNode<SceneGraph = G, ResourceData = P>,
    {
        if message.destination() == self.inspector
            && message.direction() == MessageDirection::FromWidget
        {
            if let Some(InspectorMessage::PropertyChanged(args)) =
                message.data::<InspectorMessage>()
            {
                if is_in_preview_mode {
                    PropertyAction::from_field_kind(&args.value).apply(
                        &args.path(),
                        parameters,
                        &mut |result| {
                            Log::verify(result);
                        },
                    );
                } else {
                    sender.send(Message::DoCommand(
                        make_command(args, move |ctx| {
                            fetch_machine(ctx, absm_node_handle).parameters_mut()
                        })
                        .unwrap(),
                    ));
                }
            }
        }
    }
}
