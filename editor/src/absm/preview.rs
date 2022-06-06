use crate::{
    absm::message::MessageSender,
    preview::PreviewPanel,
    utils::{create_file_selector, open_file_selector},
};
use fyrox::{
    animation::machine::Machine,
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
    resource::absm::AbsmResource,
    scene::Scene,
};
use std::path::Path;

pub struct Previewer {
    pub window: Handle<UiNode>,
    panel: PreviewPanel,
    load_preview_model: Handle<UiNode>,
    load_dialog: Handle<UiNode>,
    current_absm: Handle<Machine>,
    current_resource: Option<AbsmResource>,
}

impl Previewer {
    pub fn new(engine: &mut Engine) -> Self {
        let panel = PreviewPanel::new(engine, 300, 300);

        let ctx = &mut engine.user_interface.build_ctx();
        let window = WindowBuilder::new(WidgetBuilder::new())
            .can_close(false)
            .can_minimize(false)
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
            current_absm: Default::default(),
            current_resource: None,
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

    pub fn clear(&mut self, engine: &mut Engine) {
        self.remove_current_absm(engine);

        self.panel.clear(engine);
    }

    fn remove_current_absm(&mut self, engine: &mut Engine) {
        let scene = &mut engine.scenes[self.panel.scene()];

        if scene
            .animation_machines
            .try_get(self.current_absm)
            .is_some()
        {
            scene
                .animation_machines
                .remove_with_animations(self.current_absm, &mut scene.animations);
        }
    }

    pub fn set_absm(&mut self, engine: &mut Engine, resource: &AbsmResource) {
        if self.panel.model().is_none() {
            return;
        }

        if self
            .current_resource
            .as_ref()
            .map_or(false, |current_resource| current_resource == resource)
        {
            // Just sync instance to resource.
            block_on(engine.scenes[self.panel.scene()].resolve(engine.resource_manager.clone()));
        } else {
            self.current_resource = Some(resource.clone());

            // Remove previous machine first (if any).
            self.remove_current_absm(engine);

            // Instantiate new immediately.
            self.current_absm = block_on(resource.instantiate(
                self.panel.model(),
                &mut engine.scenes[self.panel.scene()],
                engine.resource_manager.clone(),
            ))
            .unwrap();
        }
    }

    pub fn set_preview_model(&mut self, engine: &mut Engine, path: &Path, resource: &AbsmResource) {
        // TODO: Implement async loading for this.
        if block_on(self.panel.load_model(path, false, engine)) {
            self.set_absm(engine, resource)
        }
    }

    pub fn current_absm(&self) -> Handle<Machine> {
        self.current_absm
    }

    pub fn scene(&self) -> Handle<Scene> {
        self.panel.scene()
    }
}
