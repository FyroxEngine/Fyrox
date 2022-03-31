use fyrox::{
    core::pool::Handle,
    gui::{
        inspector::InspectorBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        BuildContext, UiNode,
    },
};

pub struct Inspector {
    pub window: Handle<UiNode>,
    inspector: Handle<UiNode>,
}

impl Inspector {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let inspector = InspectorBuilder::new(WidgetBuilder::new()).build(ctx);
        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_title(WindowTitle::text("Inspector"))
            .with_content(inspector)
            .build(ctx);

        Self { window, inspector }
    }
}
