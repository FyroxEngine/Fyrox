use crate::{
    fyrox::{
        core::pool::Handle,
        engine::ApplicationLoopController,
        gui::{menu::MenuItem, menu::MenuItemMessage, message::UiMessage},
    },
    menu::create_menu_item,
    plugin::EditorPlugin,
    utils::doc::DocWindow,
    Editor,
};
use fyrox::core::uuid::{uuid, Uuid};

include!(concat!(env!("OUT_DIR"), "/biosphere_docs.rs"));

fn find_doc(key: &str) -> String {
    BIOSPHERE_DOCS
        .iter()
        .find(|(name, _)| *name == key)
        .map(|(_, content)| content.to_string())
        .unwrap_or_else(|| format!("[Doc '{}' not found in biosphere-templates/]", key))
}

pub struct BiosphereHelpPlugin {
    open_blueprint: Handle<MenuItem>,
    open_heraldry_ref: Handle<MenuItem>,
    open_wire_ref: Handle<MenuItem>,
    open_plugin_guide: Handle<MenuItem>,
    doc_window: Option<DocWindow>,
}

impl Default for BiosphereHelpPlugin {
    fn default() -> Self {
        Self {
            open_blueprint: Handle::NONE,
            open_heraldry_ref: Handle::NONE,
            open_wire_ref: Handle::NONE,
            open_plugin_guide: Handle::NONE,
            doc_window: None,
        }
    }
}

impl BiosphereHelpPlugin {
    pub const OPEN_BLUEPRINT: Uuid = uuid!("612bd42e-9347-42ab-af56-cbbdb6c06a17");
    pub const OPEN_HERALDRY: Uuid = uuid!("e3bb6939-4a14-493b-ad32-f79329d28644");
    pub const OPEN_WIRE_REF: Uuid = uuid!("29730a6e-23c7-48d2-8137-82356d71f9dc");
    pub const OPEN_PLUGIN_GUIDE: Uuid = uuid!("1434ceae-65fd-4bda-a898-3a1e01ef0dec");
}

impl EditorPlugin for BiosphereHelpPlugin {
    fn on_start(&mut self, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();
        let ctx = &mut ui.build_ctx();

        self.doc_window = Some(DocWindow::new(ctx));

        self.open_blueprint =
            create_menu_item("Project Blueprint", Self::OPEN_BLUEPRINT, vec![], ctx);
        self.open_heraldry_ref =
            create_menu_item("Heraldry Reference", Self::OPEN_HERALDRY, vec![], ctx);
        self.open_wire_ref =
            create_menu_item("Wire Types Reference", Self::OPEN_WIRE_REF, vec![], ctx);
        self.open_plugin_guide =
            create_menu_item("Plugin Dev Guide", Self::OPEN_PLUGIN_GUIDE, vec![], ctx);

        let biosphere_submenu = create_menu_item(
            "BioSpark Quantum Genesis",
            uuid!("7f27b28d-36e7-4c34-8008-c674e2875e5f"),
            vec![
                self.open_blueprint,
                self.open_heraldry_ref,
                self.open_wire_ref,
                self.open_plugin_guide,
            ],
            ctx,
        );

        ui.send(
            editor.menu.help_menu.menu,
            MenuItemMessage::AddItem(biosphere_submenu),
        );
    }

    fn on_ui_message(&mut self, message: &mut UiMessage, editor: &mut Editor) {
        if let Some(MenuItemMessage::Click) = message.data() {
            let dest = message.destination();
            let doc = if dest == self.open_blueprint {
                Some(find_doc("BLUEPRINT"))
            } else if dest == self.open_heraldry_ref {
                Some(find_doc("HERALDRY_REFERENCE"))
            } else if dest == self.open_wire_ref {
                Some(find_doc("WIRE_REFERENCE"))
            } else if dest == self.open_plugin_guide {
                Some(find_doc("PLUGIN_GUIDE"))
            } else {
                None
            };

            if let Some(text) = doc {
                let ui = editor.engine.user_interfaces.first();
                if let Some(window) = &self.doc_window {
                    window.open(text, ui);
                }
            }
        }
    }

    fn on_update(&mut self, _editor: &mut Editor, _loop_controller: ApplicationLoopController) {}
}
