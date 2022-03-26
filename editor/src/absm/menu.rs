use fyrox::{
    core::pool::Handle,
    gui::{
        menu::{MenuBuilder, MenuItemBuilder, MenuItemContent},
        widget::WidgetBuilder,
        BuildContext, UiNode,
    },
};

pub struct Menu {
    pub menu: Handle<UiNode>,
    pub edit_menu: EditMenu,
}

impl Menu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let edit_menu = EditMenu::new(ctx);

        let menu = MenuBuilder::new(WidgetBuilder::new())
            .with_items(vec![edit_menu.menu])
            .build(ctx);

        Self { menu, edit_menu }
    }
}

pub struct EditMenu {
    pub menu: Handle<UiNode>,
    undo: Handle<UiNode>,
    redo: Handle<UiNode>,
}

impl EditMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let undo;
        let redo;
        let menu = MenuItemBuilder::new(WidgetBuilder::new())
            .with_content(MenuItemContent::text_no_arrow("Edit"))
            .with_items(vec![
                {
                    undo = MenuItemBuilder::new(WidgetBuilder::new())
                        .with_content(MenuItemContent::text("Undo"))
                        .build(ctx);
                    undo
                },
                {
                    redo = MenuItemBuilder::new(WidgetBuilder::new())
                        .with_content(MenuItemContent::text("Redo"))
                        .build(ctx);
                    redo
                },
            ])
            .build(ctx);

        Self { menu, undo, redo }
    }
}
