use crate::menu::{create_menu_item, create_root_menu_item, Panels};
use fyrox::{
    asset::core::pool::Handle,
    gui::{
        menu::MenuItemMessage,
        message::{MessageDirection, UiMessage},
        window::WindowMessage,
        BuildContext, UiNode, UserInterface,
    },
};

pub struct UtilsMenu {
    pub menu: Handle<UiNode>,
    open_path_fixer: Handle<UiNode>,
    open_curve_editor: Handle<UiNode>,
}

impl UtilsMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let open_path_fixer;
        let open_curve_editor;
        let menu = create_root_menu_item(
            "Utils",
            vec![
                {
                    open_path_fixer = create_menu_item("Path Fixer", vec![], ctx);
                    open_path_fixer
                },
                {
                    open_curve_editor = create_menu_item("Curve Editor", vec![], ctx);
                    open_curve_editor
                },
            ],
            ctx,
        );

        Self {
            menu,
            open_path_fixer,
            open_curve_editor,
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, panels: &Panels, ui: &UserInterface) {
        if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
            if message.destination() == self.open_path_fixer {
                ui.send_message(WindowMessage::open_modal(
                    panels.path_fixer,
                    MessageDirection::ToWidget,
                    true,
                ));
            } else if message.destination() == self.open_curve_editor {
                panels.curve_editor.open(ui);
            }
        }
    }
}
