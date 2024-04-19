use crate::fyrox::{
    asset::core::pool::Handle,
    gui::{
        menu::MenuItemMessage,
        message::{MessageDirection, UiMessage},
        window::WindowMessage,
        BuildContext, UiNode, UserInterface,
    },
};
use crate::menu::{create_menu_item, create_root_menu_item, Panels};
use crate::stats::StatisticsWindow;

pub struct UtilsMenu {
    pub menu: Handle<UiNode>,
    open_path_fixer: Handle<UiNode>,
    open_curve_editor: Handle<UiNode>,
    absm_editor: Handle<UiNode>,
    animation_editor: Handle<UiNode>,
    ragdoll_wizard: Handle<UiNode>,
    rendering_statistics: Handle<UiNode>,
}

impl UtilsMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let open_path_fixer;
        let open_curve_editor;
        let absm_editor;
        let animation_editor;
        let ragdoll_wizard;
        let rendering_statistics;
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
                {
                    absm_editor = create_menu_item("ABSM Editor", vec![], ctx);
                    absm_editor
                },
                {
                    animation_editor = create_menu_item("Animation Editor", vec![], ctx);
                    animation_editor
                },
                {
                    ragdoll_wizard = create_menu_item("Ragdoll Wizard", vec![], ctx);
                    ragdoll_wizard
                },
                {
                    rendering_statistics = create_menu_item("Rendering Statistics", vec![], ctx);
                    rendering_statistics
                },
            ],
            ctx,
        );

        Self {
            menu,
            open_path_fixer,
            open_curve_editor,
            absm_editor,
            animation_editor,
            ragdoll_wizard,
            rendering_statistics,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        panels: &mut Panels,
        ui: &mut UserInterface,
    ) {
        if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
            if message.destination() == self.open_path_fixer {
                ui.send_message(WindowMessage::open_modal(
                    panels.path_fixer,
                    MessageDirection::ToWidget,
                    true,
                    true,
                ));
            } else if message.destination() == self.open_curve_editor {
                panels.curve_editor.open(ui);
            } else if message.destination() == self.absm_editor {
                panels.absm_editor.open(ui);
            } else if message.destination() == self.animation_editor {
                panels.animation_editor.open(ui);
            } else if message.destination() == self.ragdoll_wizard {
                panels.ragdoll_wizard.open(ui);
            } else if message.destination() == self.rendering_statistics {
                *panels.statistics_window = Some(StatisticsWindow::new(
                    &mut ui.build_ctx(),
                    panels.scene_frame,
                ))
            }
        }
    }
}
