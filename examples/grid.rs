use fyrox::{
    engine::{framework::prelude::*, Engine},
    gui::{
        button::ButtonBuilder,
        grid::{Column, GridBuilder, Row},
        text::TextBuilder,
        widget::WidgetBuilder,
    },
};

struct Game {}

impl GameState for Game {
    fn init(engine: &mut Engine) -> Self
    where
        Self: Sized,
    {
        let ctx = &mut engine.user_interface.build_ctx();

        GridBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    TextBuilder::new(WidgetBuilder::new().on_row(0).on_column(0))
                        .with_text("Some")
                        .build(ctx),
                )
                .with_child(
                    TextBuilder::new(WidgetBuilder::new().on_row(0).on_column(1))
                        .with_text("Text")
                        .build(ctx),
                )
                .with_child(
                    ButtonBuilder::new(WidgetBuilder::new().on_row(0).on_column(2))
                        .with_text("TEST BUTTON")
                        .build(ctx),
                ),
        )
        .add_row(Row::stretch())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .draw_border(true)
        .build(ctx);

        Self {}
    }
}

fn main() {
    Framework::<Game>::new()
        .unwrap()
        .title("Example - Grid")
        .run();
}
