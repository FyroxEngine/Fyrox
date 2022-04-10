use crate::{
    absm::{AbsmDataModel, SelectedEntity},
    inspector::editors::make_property_editors_container,
    Message, MessageDirection, MSG_SYNC_FLAG,
};
use fyrox::{
    core::{inspect::Inspect, pool::Handle},
    gui::{
        inspector::{InspectorBuilder, InspectorContext, InspectorMessage},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        BuildContext, UiNode, UserInterface,
    },
};
use std::{rc::Rc, sync::mpsc::Sender};

pub struct Inspector {
    pub window: Handle<UiNode>,
    inspector: Handle<UiNode>,
    selection: Vec<SelectedEntity>,
}

impl Inspector {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let inspector = InspectorBuilder::new(WidgetBuilder::new()).build(ctx);
        let window = WindowBuilder::new(WidgetBuilder::new())
            .can_close(false)
            .can_minimize(false)
            .with_title(WindowTitle::text("Inspector"))
            .with_content(inspector)
            .build(ctx);

        Self {
            window,
            inspector,
            selection: Default::default(),
        }
    }

    pub fn sync_to_model(
        &mut self,
        ui: &mut UserInterface,
        data_model: &AbsmDataModel,
        sender: Sender<Message>,
    ) {
        if self.selection != data_model.selection {
            self.selection = data_model.selection.clone();

            if let Some(first) = self.selection.first() {
                let obj_ref = match first {
                    SelectedEntity::Transition(transition) => {
                        &data_model.absm_definition.transitions[*transition] as &dyn Inspect
                    }
                    SelectedEntity::State(state) => {
                        &data_model.absm_definition.states[*state] as &dyn Inspect
                    }
                    SelectedEntity::PoseNode(pose) => {
                        &data_model.absm_definition.nodes[*pose] as &dyn Inspect
                    }
                };

                let ctx = InspectorContext::from_object(
                    obj_ref,
                    &mut ui.build_ctx(),
                    Rc::new(make_property_editors_container(sender)),
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
        }
    }
}
