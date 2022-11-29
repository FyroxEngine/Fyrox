use crate::{
    absm::command::parameter::make_set_parameters_property_command,
    inspector::editors::make_property_editors_container, Message, MessageDirection, MSG_SYNC_FLAG,
};
use fyrox::{
    animation::machine::parameter::{Parameter, ParameterDefinition},
    core::pool::Handle,
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
    scene::{animation::absm::AnimationBlendingStateMachine, node::Node},
    utils::log::Log,
};
use std::{rc::Rc, sync::mpsc::Sender};

pub struct ParameterPanel {
    pub window: Handle<UiNode>,
    inspector: Handle<UiNode>,
    property_editors: Rc<PropertyEditorDefinitionContainer>,
}

impl ParameterPanel {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let property_editors = make_property_editors_container(sender);
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
            property_editors: Rc::new(property_editors),
        }
    }

    pub fn on_selection_changed(
        &self,
        ui: &mut UserInterface,
        absm_node: Option<&AnimationBlendingStateMachine>,
    ) {
        let inspector_context = absm_node
            .map(|absm_node| {
                InspectorContext::from_object(
                    absm_node.machine().parameters(),
                    &mut ui.build_ctx(),
                    self.property_editors.clone(),
                    None,
                    MSG_SYNC_FLAG,
                    0,
                )
            })
            .unwrap_or_default();

        ui.send_message(InspectorMessage::context(
            self.inspector,
            MessageDirection::ToWidget,
            inspector_context,
        ));
    }

    pub fn reset(&mut self, ui: &mut UserInterface) {
        ui.send_message(InspectorMessage::context(
            self.inspector,
            MessageDirection::ToWidget,
            Default::default(),
        ));
    }

    pub fn sync_to_model(
        &mut self,
        ui: &mut UserInterface,
        absm_node: &AnimationBlendingStateMachine,
    ) {
        let ctx = ui
            .node(self.inspector)
            .cast::<fyrox::gui::inspector::Inspector>()
            .unwrap()
            .context()
            .clone();

        if let Err(sync_errors) = ctx.sync(absm_node.machine().parameters(), ui, 0) {
            for error in sync_errors {
                Log::err(format!("Failed to sync property. Reason: {:?}", error))
            }
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        sender: &Sender<Message>,
        absm_node_handle: Handle<Node>,
        absm_node: &mut AnimationBlendingStateMachine,
        is_in_preview_mode: bool,
    ) {
        if message.destination() == self.inspector
            && message.direction() == MessageDirection::FromWidget
        {
            if let Some(InspectorMessage::PropertyChanged(args)) =
                message.data::<InspectorMessage>()
            {
                if is_in_preview_mode {
                    Log::verify(PropertyAction::from_field_kind(&args.value).apply(
                        &args.path(),
                        absm_node.machine_mut().get_mut_silent().parameters_mut(),
                    ))
                } else {
                    sender
                        .send(Message::DoSceneCommand(
                            make_set_parameters_property_command((), args, absm_node_handle)
                                .unwrap(),
                        ))
                        .unwrap();
                }
            }
        }
    }
}
