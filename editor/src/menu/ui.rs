use crate::{
    menu::create_menu_item, message::MessageSender, ui_scene::commands::graph::AddUiNodeCommand,
    ui_scene::UiScene,
};
use fyrox::{
    core::pool::Handle,
    fxhash::FxHashMap,
    gui::{
        button::ButtonBuilder, menu::MenuItemMessage, message::UiMessage, widget::WidgetBuilder,
        BuildContext, UiNode,
    },
};

pub struct UiMenu {
    pub menu: Handle<UiNode>,
    constructors: FxHashMap<Handle<UiNode>, UiMenuEntry>,
}

#[allow(clippy::type_complexity)]
pub struct UiMenuEntry {
    pub name: String,
    pub constructor: Box<dyn FnMut(&str, &mut BuildContext) -> UiNode>,
}

impl UiMenu {
    pub fn default_entries() -> Vec<UiMenuEntry> {
        vec![UiMenuEntry {
            name: "Button".to_string(),
            constructor: Box::new(|name, ctx| {
                ButtonBuilder::new(WidgetBuilder::new().with_name(name)).build_node(ctx)
            }),
        }]
    }

    pub fn new(entries: Vec<UiMenuEntry>, ctx: &mut BuildContext) -> Self {
        let items = entries
            .iter()
            .map(|e| create_menu_item(&e.name, Default::default(), ctx))
            .collect::<Vec<_>>();

        let constructors = entries
            .into_iter()
            .zip(items.iter().cloned())
            .map(|(entry, node)| (node, entry))
            .collect::<FxHashMap<_, _>>();

        let menu = create_menu_item("UI", items, ctx);

        Self { menu, constructors }
    }

    pub fn handle_ui_message(
        &mut self,
        sender: &MessageSender,
        message: &UiMessage,
        scene: &mut UiScene,
    ) {
        if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
            if let Some(entry) = self.constructors.get_mut(&message.destination()) {
                let ui_node = (entry.constructor)(&entry.name, &mut scene.ui.build_ctx());
                sender.do_ui_scene_command(AddUiNodeCommand::new(ui_node, Handle::NONE, true));
            }
        }
    }
}
