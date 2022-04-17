use crate::{
    absm::{AbsmDataModel, SelectedEntity},
    inspector::editors::make_property_editors_container,
    Message, MessageDirection, MSG_SYNC_FLAG,
};
use fyrox::{
    animation::machine::node::{
        blend::{BlendPoseDefinition, IndexedBlendInputDefinition},
        BasePoseNodeDefinition,
    },
    core::{inspect::Inspect, pool::Handle},
    gui::{
        inspector::{
            editors::{
                inspectable::InspectablePropertyEditorDefinition, PropertyEditorDefinitionContainer,
            },
            InspectorBuilder, InspectorContext, InspectorMessage,
        },
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        BuildContext, UiNode, UserInterface,
    },
    utils::log::Log,
};
use std::{rc::Rc, sync::mpsc::Sender};

pub struct Inspector {
    pub window: Handle<UiNode>,
    inspector: Handle<UiNode>,
    selection: Vec<SelectedEntity>,
    property_editors: Rc<PropertyEditorDefinitionContainer>,
}

impl Inspector {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let inspector = InspectorBuilder::new(WidgetBuilder::new()).build(ctx);
        let window = WindowBuilder::new(WidgetBuilder::new())
            .can_close(false)
            .can_minimize(false)
            .with_title(WindowTitle::text("Inspector"))
            .with_content(inspector)
            .build(ctx);

        let mut property_editors = make_property_editors_container(sender);
        property_editors
            .insert(InspectablePropertyEditorDefinition::<BasePoseNodeDefinition>::new());
        property_editors.insert(InspectablePropertyEditorDefinition::<
            IndexedBlendInputDefinition,
        >::new());
        property_editors.insert(InspectablePropertyEditorDefinition::<BlendPoseDefinition>::new());

        Self {
            window,
            inspector,
            selection: Default::default(),
            property_editors: Rc::new(property_editors),
        }
    }

    fn first_selected_entity<'a>(&self, data_model: &'a AbsmDataModel) -> Option<&'a dyn Inspect> {
        self.selection.first().map(|first| match first {
            SelectedEntity::Transition(transition) => {
                &data_model.absm_definition.transitions[*transition] as &dyn Inspect
            }
            SelectedEntity::State(state) => {
                &data_model.absm_definition.states[*state] as &dyn Inspect
            }
            SelectedEntity::PoseNode(pose) => {
                &data_model.absm_definition.nodes[*pose] as &dyn Inspect
            }
        })
    }

    pub fn sync_to_model(&mut self, ui: &mut UserInterface, data_model: &AbsmDataModel) {
        if self.selection != data_model.selection {
            self.selection = data_model.selection.clone();

            if let Some(obj_ref) = self.first_selected_entity(data_model) {
                let ctx = InspectorContext::from_object(
                    obj_ref,
                    &mut ui.build_ctx(),
                    self.property_editors.clone(),
                    None,
                    MSG_SYNC_FLAG,
                    0,
                );

                ui.send_message(InspectorMessage::context(
                    self.inspector,
                    MessageDirection::ToWidget,
                    ctx,
                ));
            }
        } else if let Some(obj_ref) = self.first_selected_entity(data_model) {
            let ctx = ui
                .node(self.inspector)
                .cast::<fyrox::gui::inspector::Inspector>()
                .unwrap()
                .context()
                .clone();

            if let Err(sync_errors) = ctx.sync(obj_ref, ui, 0) {
                for error in sync_errors {
                    Log::err(format!("Failed to sync property. Reason: {:?}", error))
                }
            }
        }
    }
}
