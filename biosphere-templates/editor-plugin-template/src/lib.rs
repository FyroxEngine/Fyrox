// BioSpark Quantum Genesis — EditorPlugin Template
//
// Copy this file to:
//   editor/src/plugins/YOUR_PLUGIN_NAME/mod.rs
//
// Then register it in two places:
//   1. editor/src/plugins/mod.rs  →  add:  pub mod YOUR_PLUGIN_NAME;
//   2. editor/src/lib.rs          →  add to the plugins chain:
//        .with(YourPlugin::default())
//      and import it in the use block:
//        plugins::YOUR_PLUGIN_NAME::YourPlugin,
//
// Search for RENAME_ME and replace throughout.

use crate::{
    fyrox::{
        core::pool::Handle,
        engine::ApplicationLoopController,
        gui::{
            button::{ButtonBuilder, ButtonMessage},
            menu::{MenuItem, MenuItemMessage},
            message::UiMessage,
            stack_panel::StackPanelBuilder,
            text::{Text, TextBuilder, TextMessage},
            widget::{WidgetBuilder, WidgetMessage},
            window::{Window, WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, Thickness, UiNode,
        },
    },
    menu::create_menu_item,
    plugin::EditorPlugin,
    Editor,
};
use fyrox::core::uuid::{uuid, Uuid};

// ── Plugin State ────────────────────────────────────────────────────────────

pub struct RenamePlugin {
    // Window handle — Handle::NONE when closed
    window: Handle<Window>,
    // The menu item that opens this window (added to Utils menu on_start)
    open_menu_item: Handle<MenuItem>,

    // Status/output text
    status_text: Handle<Text>,

    // Action buttons — store as Handle<UiNode>
    my_action_btn: Handle<UiNode>,

    // Internal state (not UI — derive from messages as they arrive)
    some_state: bool,
}

impl Default for RenamePlugin {
    fn default() -> Self {
        Self {
            window: Handle::NONE,
            open_menu_item: Handle::NONE,
            status_text: Handle::NONE,
            my_action_btn: Handle::NONE,
            some_state: false,
        }
    }
}

// ── UUID constants ──────────────────────────────────────────────────────────
// Generate a new UUID for each menu item. Use: https://www.uuidgenerator.net/
// or run: python3 -c "import uuid; print(uuid.uuid4())"

impl RenamePlugin {
    pub const OPEN_WINDOW: Uuid = uuid!("00000000-0000-0000-0000-000000000000"); // REPLACE THIS UUID

    // ── Window builder ──────────────────────────────────────────────────────

    fn build_window(&mut self, ctx: &mut BuildContext) -> Handle<Window> {
        self.my_action_btn = ButtonBuilder::new(
            WidgetBuilder::new()
                .with_height(26.0)
                .with_margin(Thickness::uniform(2.0)),
        )
        .with_text("Do Something")
        .build(ctx)
        .to_base::<UiNode>();

        self.status_text = TextBuilder::new(
            WidgetBuilder::new().with_margin(Thickness::uniform(4.0)),
        )
        .with_text("Ready.")
        .build(ctx);

        let content = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(self.my_action_btn)
                .with_child(self.status_text),
        )
        .build(ctx);

        WindowBuilder::new(WidgetBuilder::new().with_width(400.0).with_height(300.0))
            .with_title(WindowTitle::text("Rename Me"))
            .with_content(content)
            .open(false)
            .build(ctx)
    }

    // ── Helpers ─────────────────────────────────────────────────────────────

    fn set_status(&self, editor: &mut Editor, msg: &str) {
        let ui = editor.engine.user_interfaces.first();
        ui.send(self.status_text, TextMessage::Text(msg.to_string()));
    }

    fn handle_my_action(&mut self, editor: &mut Editor) {
        // TODO: implement action
        self.some_state = !self.some_state;
        self.set_status(editor, &format!("State toggled: {}", self.some_state));
    }
}

// ── EditorPlugin trait ──────────────────────────────────────────────────────

impl EditorPlugin for RenamePlugin {
    /// Called once when the editor starts. Build menu items here.
    /// Do NOT build windows here — defer to on_ui_message (lazy init).
    fn on_start(&mut self, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();
        let ctx = &mut ui.build_ctx();

        // Add to Utils menu (or help_menu, or a custom submenu)
        self.open_menu_item =
            create_menu_item("Rename Me", Self::OPEN_WINDOW, vec![], ctx);
        ui.send(
            editor.menu.utils_menu.menu,
            MenuItemMessage::AddItem(self.open_menu_item),
        );

        // To add to Help menu instead:
        //   ui.send(editor.menu.help_menu.menu, MenuItemMessage::AddItem(...));
    }

    /// Called for every UI message. Route by message type then destination.
    fn on_ui_message(&mut self, message: &mut UiMessage, editor: &mut Editor) {
        // ── Open window when menu item clicked ──
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.open_menu_item && self.window.is_none() {
                let ui = editor.engine.user_interfaces.first_mut();
                let ctx = &mut ui.build_ctx();
                self.window = self.build_window(ctx);
                ui.send(
                    self.window,
                    WindowMessage::Open {
                        alignment: WindowAlignment::Center,
                        modal: false,
                        focus_content: true,
                    },
                );
            }
        }

        // ── Clean up window handle when closed ──
        if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.window {
                let ui = editor.engine.user_interfaces.first_mut();
                ui.send(self.window, WidgetMessage::Remove);
                self.window = Handle::NONE;
            }
        }

        // ── Button clicks ──
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.my_action_btn {
                self.handle_my_action(editor);
            }
        }
    }

    /// Called every frame while the editor is running.
    /// Only use for continuous updates (live stats, polling).
    /// Leave empty if not needed.
    fn on_update(&mut self, _editor: &mut Editor, _loop_controller: ApplicationLoopController) {}
}

// ── UI builder helpers (module-local) ──────────────────────────────────────

fn _make_button(ctx: &mut BuildContext, label: &str) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_height(26.0)
            .with_margin(Thickness::uniform(2.0)),
    )
    .with_text(label)
    .build(ctx)
    .to_base::<UiNode>()
}

fn _make_text_items(ctx: &mut BuildContext, items: &[&str]) -> Vec<Handle<UiNode>> {
    items
        .iter()
        .map(|text| {
            use crate::fyrox::gui::text::TextBuilder;
            TextBuilder::new(
                WidgetBuilder::new()
                    .with_height(22.0)
                    .with_margin(Thickness::uniform(2.0)),
            )
            .with_text(*text)
            .build(ctx)
            .to_base::<UiNode>()
        })
        .collect()
}
