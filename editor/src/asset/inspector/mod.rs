use crate::{
    asset::inspector::handlers::ImportOptionsHandler,
    inspector::editors::make_property_editors_container, Message, MSG_SYNC_FLAG,
};
use fyrox::{
    core::pool::Handle,
    engine::Engine,
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        grid::{Column, GridBuilder, Row},
        inspector::{Inspector, InspectorBuilder, InspectorContext, InspectorMessage},
        message::{MessageDirection, UiMessage},
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
    },
};
use std::rc::Rc;
use std::sync::mpsc::Sender;

pub mod handlers;

pub struct AssetInspector {
    pub container: Handle<UiNode>,
    inspector: Handle<UiNode>,
    apply: Handle<UiNode>,
    revert: Handle<UiNode>,
    handler: Option<Box<dyn ImportOptionsHandler>>,
}

impl AssetInspector {
    pub fn new(ctx: &mut BuildContext, row: usize, column: usize) -> Self {
        let inspector;
        let apply;
        let revert;
        let container = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(row)
                .on_column(column)
                .with_child(
                    ScrollViewerBuilder::new(WidgetBuilder::new().on_row(0).on_column(0))
                        .with_content({
                            inspector = InspectorBuilder::new(WidgetBuilder::new()).build(ctx);
                            inspector
                        })
                        .build(ctx),
                )
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .on_row(1)
                            .on_column(0)
                            .with_horizontal_alignment(HorizontalAlignment::Right)
                            .with_child({
                                apply = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(100.0)
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text("Apply")
                                .build(ctx);
                                apply
                            })
                            .with_child({
                                revert = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(100.0)
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text("Revert")
                                .build(ctx);
                                revert
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                ),
        )
        .add_row(Row::stretch())
        .add_row(Row::strict(25.0))
        .add_column(Column::stretch())
        .build(ctx);

        Self {
            container,
            inspector,
            apply,
            revert,
            handler: None,
        }
    }

    pub fn inspect_resource_import_options<H>(
        &mut self,
        handler: H,
        ui: &mut UserInterface,
        sender: Sender<Message>,
    ) where
        H: ImportOptionsHandler + 'static,
    {
        let context = InspectorContext::from_object(
            handler.value(),
            &mut ui.build_ctx(),
            Rc::new(make_property_editors_container(sender)),
            None,
            MSG_SYNC_FLAG,
            0,
            true,
        );
        ui.send_message(InspectorMessage::context(
            self.inspector,
            MessageDirection::ToWidget,
            context,
        ));

        self.handler = Some(Box::new(handler));
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, engine: &mut Engine) {
        if let Some(handler) = self.handler.as_mut() {
            if let Some(ButtonMessage::Click) = message.data() {
                if message.destination() == self.revert {
                    handler.revert();
                    let context = engine
                        .user_interface
                        .node(self.inspector)
                        .cast::<Inspector>()
                        .expect("Must be inspector")
                        .context()
                        .clone();
                    context
                        .sync(handler.value(), &mut engine.user_interface, 0, true)
                        .unwrap();
                } else if message.destination() == self.apply {
                    handler.apply(engine.resource_manager.clone());
                }
            } else if let Some(InspectorMessage::PropertyChanged(property_changed)) = message.data()
            {
                if message.destination == self.inspector {
                    handler.handle_property_changed(property_changed)
                }
            }
        }
    }
}
