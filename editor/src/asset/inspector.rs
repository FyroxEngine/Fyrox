use crate::{inspector::editors::make_property_editors_container, Message, MSG_SYNC_FLAG};
use rg3d::{
    core::{inspect::Inspect, pool::Handle},
    gui::{
        button::ButtonBuilder,
        grid::{Column, GridBuilder, Row},
        inspector::{InspectorBuilder, InspectorContext, InspectorMessage},
        message::MessageDirection,
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
    },
};
use std::sync::mpsc::Sender;

pub struct AssetInspector {
    pub container: Handle<UiNode>,
    inspector: Handle<UiNode>,
    apply: Handle<UiNode>,
    revert: Handle<UiNode>,
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
                                    WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text("Apply")
                                .build(ctx);
                                apply
                            })
                            .with_child({
                                revert = ButtonBuilder::new(
                                    WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
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
        }
    }

    pub fn inspect_resource_import_options(
        &self,
        import_options: &dyn Inspect,
        ui: &mut UserInterface,
        sender: Sender<Message>,
    ) {
        let context = InspectorContext::from_object(
            import_options,
            &mut ui.build_ctx(),
            make_property_editors_container(sender),
            None,
            MSG_SYNC_FLAG,
            0,
        );
        ui.send_message(InspectorMessage::context(
            self.inspector,
            MessageDirection::ToWidget,
            context,
        ));
    }
}
