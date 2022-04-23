use crate::{
    absm::{
        command::{
            AbsmCommand, AddParameterCommand, RemoveParameterCommand,
            SetParameterIndexValueCommand, SetParameterNameCommand, SetParameterRuleValueCommand,
            SetParameterValueCommand, SetParameterWeightValueCommand,
        },
        message::MessageSender,
        AbsmDataModel,
    },
    inspector::editors::make_property_editors_container,
    Message, MessageDirection, MSG_SYNC_FLAG,
};
use fyrox::{
    animation::machine::parameter::{Parameter, ParameterContainerDefinition, ParameterDefinition},
    core::pool::Handle,
    gui::{
        inspector::{
            editors::{
                collection::VecCollectionPropertyEditorDefinition,
                enumeration::EnumPropertyEditorDefinition,
                inspectable::InspectablePropertyEditorDefinition,
                PropertyEditorDefinitionContainer,
            },
            CollectionChanged, FieldKind, InspectorBuilder, InspectorContext, InspectorMessage,
            PropertyChanged,
        },
        message::UiMessage,
        scroll_viewer::ScrollViewerBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        BuildContext, UiNode, UserInterface,
    },
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
        let mut property_editors = make_property_editors_container(sender);
        property_editors
            .insert(VecCollectionPropertyEditorDefinition::<ParameterDefinition>::new());
        property_editors.insert(InspectablePropertyEditorDefinition::<ParameterDefinition>::new());
        property_editors.insert(EnumPropertyEditorDefinition::<Parameter>::new());

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
            .build(ctx);

        Self {
            window,
            inspector,
            property_editors: Rc::new(property_editors),
        }
    }

    pub fn reset(&mut self, ui: &mut UserInterface, data_model: Option<&AbsmDataModel>) {
        let inspector_context = data_model
            .map(|data_model| {
                InspectorContext::from_object(
                    &data_model.absm_definition.parameters,
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

    pub fn sync_to_model(&mut self, ui: &mut UserInterface, data_model: &AbsmDataModel) {
        let ctx = ui
            .node(self.inspector)
            .cast::<fyrox::gui::inspector::Inspector>()
            .unwrap()
            .context()
            .clone();

        if let Err(sync_errors) = ctx.sync(&data_model.absm_definition.parameters, ui, 0) {
            for error in sync_errors {
                Log::err(format!("Failed to sync property. Reason: {:?}", error))
            }
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, sender: &MessageSender) {
        if message.destination() == self.inspector
            && message.direction() == MessageDirection::FromWidget
        {
            if let Some(InspectorMessage::PropertyChanged(args)) =
                message.data::<InspectorMessage>()
            {
                let command = match args.name.as_ref() {
                    ParameterContainerDefinition::CONTAINER => {
                        if let FieldKind::Collection(ref collection_args) = args.value {
                            match **collection_args {
                                CollectionChanged::Add => Some(AbsmCommand::new(
                                    AddParameterCommand::new((), Default::default()),
                                )),
                                CollectionChanged::Remove(i) => {
                                    Some(AbsmCommand::new(RemoveParameterCommand::new((), i)))
                                }
                                CollectionChanged::ItemChanged {
                                    index,
                                    ref property,
                                } => handle_parameter_property_change(index, property),
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some(command) = command {
                    sender.do_command_value(command);
                } else {
                    Log::err(format!("Failed to handle a property {}", args.path()))
                }
            }
        }
    }
}

fn handle_parameter_property_change(
    index: usize,
    property_changed: &PropertyChanged,
) -> Option<AbsmCommand> {
    match property_changed.value {
        FieldKind::Inspectable(ref inner) => {
            if property_changed.name == ParameterDefinition::VALUE {
                if inner.name == "0" {
                    if let FieldKind::Object(ref value) = inner.value {
                        if let Some(weight) = value.cast_clone::<f32>() {
                            Some(AbsmCommand::new(SetParameterWeightValueCommand {
                                handle: index,
                                value: weight,
                            }))
                        } else if let Some(rule) = value.cast_clone::<bool>() {
                            Some(AbsmCommand::new(SetParameterRuleValueCommand {
                                handle: index,
                                value: rule,
                            }))
                        } else if let Some(idx) = value.cast_clone::<u32>() {
                            Some(AbsmCommand::new(SetParameterIndexValueCommand {
                                handle: index,
                                value: idx,
                            }))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
        FieldKind::Object(ref value) => match property_changed.name.as_ref() {
            ParameterDefinition::NAME => Some(AbsmCommand::new(SetParameterNameCommand {
                handle: index,
                value: value.cast_clone()?,
            })),
            ParameterDefinition::VALUE => Some(AbsmCommand::new(SetParameterValueCommand {
                handle: index,
                value: value.cast_clone()?,
            })),
            _ => None,
        },
        _ => None,
    }
}
