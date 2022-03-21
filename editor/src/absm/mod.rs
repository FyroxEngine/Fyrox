use crate::{
    absm::{canvas::AbsmCanvasBuilder, node::AbsmStateNodeBuilder},
    BuildContext, Color, WidgetBuilder,
};
use fyrox::{
    core::{algebra::Vector2, pool::Handle},
    gui::{
        window::{WindowBuilder, WindowTitle},
        UiNode,
    },
};

mod canvas;
mod node;

const NORMAL_BACKGROUND: Color = Color::opaque(60, 60, 60);
const SELECTED_BACKGROUND: Color = Color::opaque(80, 80, 80);
const BORDER_COLOR: Color = Color::opaque(70, 70, 70);

pub struct AbsmEditor {
    #[allow(dead_code)] // TODO
    window: Handle<UiNode>,
}

impl AbsmEditor {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(700.0).with_height(400.0))
            .with_content(
                AbsmCanvasBuilder::new(
                    WidgetBuilder::new()
                        .with_child(AbsmStateNodeBuilder::new(WidgetBuilder::new()).build(ctx))
                        .with_child(
                            AbsmStateNodeBuilder::new(
                                WidgetBuilder::new()
                                    .with_desired_position(Vector2::new(300.0, 200.0)),
                            )
                            .build(ctx),
                        )
                        .with_child(
                            AbsmStateNodeBuilder::new(
                                WidgetBuilder::new()
                                    .with_desired_position(Vector2::new(300.0, 400.0)),
                            )
                            .build(ctx),
                        ),
                )
                .build(ctx),
            )
            .with_title(WindowTitle::text("ABSM Editor"))
            .build(ctx);

        Self { window }
    }
}
