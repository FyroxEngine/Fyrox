use crate::GridBuilder;
use rg3d::{
    core::pool::Handle,
    gui::{
        button::ButtonBuilder,
        grid::{Column, Row},
        inspector::InspectorBuilder,
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode,
    },
};

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
                .with_child({
                    inspector = InspectorBuilder::new(WidgetBuilder::new().on_row(0).on_column(0))
                        .build(ctx);
                    inspector
                })
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
}
