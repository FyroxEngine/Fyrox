use crate::{
    absm::message::MessageSender,
    preview::PreviewPanel,
    utils::{create_file_selector, open_file_selector},
};
use fyrox::{
    animation::machine::MachineDefinition,
    core::{futures::executor::block_on, pool::Handle},
    engine::Engine,
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        file_browser::{FileBrowserMode, FileSelectorMessage},
        message::UiMessage,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        Thickness, UiNode,
    },
};
use std::path::Path;

pub struct Previewer {
    pub window: Handle<UiNode>,
    pub panel: PreviewPanel,
    load_preview_model: Handle<UiNode>,
    load_dialog: Handle<UiNode>,
}

impl Previewer {
    pub fn new(engine: &mut Engine) -> Self {
        let panel = PreviewPanel::new(engine, 300, 300);

        let ctx = &mut engine.user_interface.build_ctx();
        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_title(WindowTitle::text("Previewer"))
            .with_content(panel.root)
            .build(ctx);

        let load_preview_model =
            ButtonBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                .with_text("Load")
                .build(ctx);

        ctx.link(load_preview_model, panel.tools_panel);

        // TODO: Support more formats here.
        let load_dialog = create_file_selector(ctx, "fbx", FileBrowserMode::Open);

        Self {
            window,
            panel,
            load_preview_model,
            load_dialog,
        }
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        sender: &MessageSender,
        engine: &mut Engine,
    ) {
        self.panel.handle_message(message, engine);

        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.load_preview_model {
                open_file_selector(self.load_dialog, &engine.user_interface);
            }
        } else if let Some(FileSelectorMessage::Commit(path)) = message.data() {
            if message.destination() == self.load_dialog {
                sender.set_preview_model(path.clone());
            }
        }
    }

    pub fn update(&mut self, engine: &mut Engine) {
        self.panel.update(engine)
    }

    pub fn set_preview_model(
        &mut self,
        engine: &mut Engine,
        path: &Path,
        definition: &MachineDefinition,
    ) {
        // TODO: Implement async loading for this.
        if block_on(self.panel.load_model(path, engine)) {
            let scene = &mut engine.scenes[self.panel.scene()];
            block_on(definition.instantiate(
                self.panel.model(),
                scene,
                engine.resource_manager.clone(),
            ))
            .unwrap();
        }
    }
}
