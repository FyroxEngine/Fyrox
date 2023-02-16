use fyrox::{
    core::pool::Handle,
    gui::{
        border::BorderBuilder,
        grid::{Column, GridBuilder, Row},
        message::MessageDirection,
        numeric::NumericUpDownBuilder,
        stack_panel::StackPanelBuilder,
        text_box::TextBoxBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, Orientation, UiNode, UserInterface, VerticalAlignment,
    },
};

pub struct BlendSpaceEditor {
    pub window: Handle<UiNode>,
    min_x: Handle<UiNode>,
    max_x: Handle<UiNode>,
    min_y: Handle<UiNode>,
    max_y: Handle<UiNode>,
    x_axis_name: Handle<UiNode>,
    y_axis_name: Handle<UiNode>,
}

impl BlendSpaceEditor {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let min_x;
        let max_x;
        let min_y;
        let max_y;
        let x_axis_name;
        let y_axis_name;
        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    StackPanelBuilder::new(WidgetBuilder::new().on_row(0).on_column(0))
                        .with_orientation(Orientation::Horizontal)
                        .build(ctx),
                )
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .on_row(1)
                            .on_column(0)
                            .with_child(
                                GridBuilder::new(
                                    WidgetBuilder::new()
                                        .with_child({
                                            max_y = NumericUpDownBuilder::new(
                                                WidgetBuilder::new()
                                                    .on_row(0)
                                                    .with_vertical_alignment(
                                                        VerticalAlignment::Top,
                                                    ),
                                            )
                                            .with_value(0.0f32)
                                            .build(ctx);
                                            max_y
                                        })
                                        .with_child({
                                            y_axis_name = TextBoxBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_height(22.0)
                                                    .on_row(1)
                                                    .with_vertical_alignment(
                                                        VerticalAlignment::Center,
                                                    ),
                                            )
                                            .build(ctx);
                                            y_axis_name
                                        })
                                        .with_child({
                                            min_y = NumericUpDownBuilder::new(
                                                WidgetBuilder::new()
                                                    .on_row(2)
                                                    .with_vertical_alignment(
                                                        VerticalAlignment::Bottom,
                                                    ),
                                            )
                                            .with_value(0.0f32)
                                            .build(ctx);
                                            min_y
                                        }),
                                )
                                .add_row(Row::stretch())
                                .add_row(Row::stretch())
                                .add_row(Row::stretch())
                                .add_column(Column::strict(50.0))
                                .build(ctx),
                            )
                            .with_child(
                                BorderBuilder::new(WidgetBuilder::new().on_row(0).on_column(1))
                                    .build(ctx),
                            )
                            .with_child(
                                GridBuilder::new(
                                    WidgetBuilder::new()
                                        .on_row(1)
                                        .on_column(1)
                                        .with_child({
                                            min_x = NumericUpDownBuilder::new(
                                                WidgetBuilder::new()
                                                    .on_column(0)
                                                    .with_vertical_alignment(
                                                        VerticalAlignment::Top,
                                                    ),
                                            )
                                            .with_value(0.0f32)
                                            .build(ctx);
                                            min_x
                                        })
                                        .with_child({
                                            x_axis_name = TextBoxBuilder::new(
                                                WidgetBuilder::new()
                                                    .on_column(1)
                                                    .with_vertical_alignment(
                                                        VerticalAlignment::Center,
                                                    ),
                                            )
                                            .build(ctx);
                                            x_axis_name
                                        })
                                        .with_child({
                                            max_x = NumericUpDownBuilder::new(
                                                WidgetBuilder::new()
                                                    .on_column(2)
                                                    .with_vertical_alignment(
                                                        VerticalAlignment::Bottom,
                                                    ),
                                            )
                                            .with_value(0.0f32)
                                            .build(ctx);
                                            max_x
                                        }),
                                )
                                .add_column(Column::stretch())
                                .add_column(Row::stretch())
                                .add_column(Row::stretch())
                                .add_row(Column::strict(22.0))
                                .build(ctx),
                            ),
                    )
                    .add_row(Row::stretch())
                    .add_row(Row::auto())
                    .add_column(Column::auto())
                    .add_column(Column::stretch())
                    .build(ctx),
                ),
        )
        .add_row(Row::strict(22.0))
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(400.0).with_height(300.0))
            .open(false)
            .with_content(content)
            .with_title(WindowTitle::text("Blend Space Editor"))
            .build(ctx);

        Self {
            window,
            min_x,
            max_x,
            min_y,
            max_y,
            x_axis_name,
            y_axis_name,
        }
    }

    pub fn open(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::open(
            self.window,
            MessageDirection::ToWidget,
            true,
        ));
    }
}
