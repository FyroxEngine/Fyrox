use crate::fyrox::gui::selector::SelectorBuilder;
use crate::fyrox::{
    core::pool::Handle,
    fxhash::FxHashMap,
    gui::{
        absm::{AbsmEventProviderBuilder, AnimationBlendingStateMachineBuilder},
        animation::AnimationPlayerBuilder,
        border::BorderBuilder,
        button::ButtonBuilder,
        canvas::CanvasBuilder,
        check_box::CheckBoxBuilder,
        decorator::DecoratorBuilder,
        dropdown_list::DropdownListBuilder,
        expander::ExpanderBuilder,
        grid::GridBuilder,
        image::ImageBuilder,
        list_view::ListViewBuilder,
        menu::MenuItemMessage,
        menu::{MenuBuilder, MenuItemBuilder},
        message::UiMessage,
        messagebox::MessageBoxBuilder,
        nine_patch::NinePatchBuilder,
        numeric::NumericUpDownBuilder,
        path::PathEditorBuilder,
        popup::PopupBuilder,
        progress_bar::ProgressBarBuilder,
        screen::ScreenBuilder,
        scroll_bar::ScrollBarBuilder,
        scroll_viewer::ScrollViewerBuilder,
        searchbar::SearchBarBuilder,
        stack_panel::StackPanelBuilder,
        tab_control::TabControlBuilder,
        text::TextBuilder,
        text_box::TextBoxBuilder,
        tree::{TreeBuilder, TreeRootBuilder},
        uuid::UuidEditorBuilder,
        vector_image::VectorImageBuilder,
        widget::WidgetBuilder,
        window::WindowBuilder,
        wrap_panel::WrapPanelBuilder,
        BuildContext, UiNode,
    },
};
use crate::{
    menu::create_menu_item,
    message::MessageSender,
    scene::Selection,
    ui_scene::{commands::graph::AddWidgetCommand, UiScene},
};

pub struct UiMenu {
    pub menu: Handle<UiNode>,
    constructors: FxHashMap<Handle<UiNode>, UiMenuEntry>,
}

#[allow(clippy::type_complexity)]
pub struct UiMenuEntry {
    pub name: String,
    pub constructor: Box<dyn FnMut(&str, &mut BuildContext) -> Handle<UiNode>>,
}

impl UiMenuEntry {
    pub fn new<P, F>(name: P, constructor: F) -> Self
    where
        P: AsRef<str>,
        F: FnMut(&str, &mut BuildContext) -> Handle<UiNode> + 'static,
    {
        Self {
            name: name.as_ref().to_owned(),
            constructor: Box::new(constructor),
        }
    }
}

impl UiMenu {
    pub fn default_entries() -> Vec<UiMenuEntry> {
        vec![
            UiMenuEntry::new("Screen", |name, ctx| {
                ScreenBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("Button", |name, ctx| {
                ButtonBuilder::new(
                    WidgetBuilder::new()
                        .with_width(100.0)
                        .with_height(20.0)
                        .with_name(name),
                )
                .build(ctx)
            }),
            UiMenuEntry::new("Border", |name, ctx| {
                BorderBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("Image", |name, ctx| {
                ImageBuilder::new(
                    WidgetBuilder::new()
                        .with_height(32.0)
                        .with_width(32.0)
                        .with_name(name),
                )
                .build(ctx)
            }),
            UiMenuEntry::new("Canvas", |name, ctx| {
                CanvasBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("Grid", |name, ctx| {
                GridBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("CheckBox", |name, ctx| {
                CheckBoxBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("Decorator", |name, ctx| {
                DecoratorBuilder::new(BorderBuilder::new(WidgetBuilder::new().with_name(name)))
                    .build(ctx)
            }),
            UiMenuEntry::new("DropdownList", |name, ctx| {
                DropdownListBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("Expander", |name, ctx| {
                ExpanderBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("ListView", |name, ctx| {
                ListViewBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("Menu", |name, ctx| {
                MenuBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("MenuItem", |name, ctx| {
                MenuItemBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("MessageBox", |name, ctx| {
                MessageBoxBuilder::new(WindowBuilder::new(WidgetBuilder::new().with_name(name)))
                    .build(ctx)
            }),
            UiMenuEntry::new("NinePatch", |name, ctx| {
                NinePatchBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("NumericUpDownF32", |name, ctx| {
                NumericUpDownBuilder::<f32>::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("NumericUpDownI32", |name, ctx| {
                NumericUpDownBuilder::<i32>::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("PathEditor", |name, ctx| {
                PathEditorBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("Popup", |name, ctx| {
                PopupBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("ProgressBar", |name, ctx| {
                ProgressBarBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("ScrollBar", |name, ctx| {
                ScrollBarBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("ScrollViewer", |name, ctx| {
                ScrollViewerBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("SearchBar", |name, ctx| {
                SearchBarBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("StackPanel", |name, ctx| {
                StackPanelBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("TabControl", |name, ctx| {
                TabControlBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("Text", |name, ctx| {
                TextBuilder::new(WidgetBuilder::new().with_name(name))
                    .with_text("Text")
                    .build(ctx)
            }),
            UiMenuEntry::new("TextBox", |name, ctx| {
                TextBoxBuilder::new(WidgetBuilder::new().with_name(name))
                    .with_text("Text")
                    .build(ctx)
            }),
            UiMenuEntry::new("Tree", |name, ctx| {
                TreeBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("TreeRoot", |name, ctx| {
                TreeRootBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("UuidEditor", |name, ctx| {
                UuidEditorBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("VectorImage", |name, ctx| {
                VectorImageBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("Window", |name, ctx| {
                WindowBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("WrapPanel", |name, ctx| {
                WrapPanelBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("AnimationPlayer", |name, ctx| {
                AnimationPlayerBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("AnimationBlendingStateMachine", |name, ctx| {
                AnimationBlendingStateMachineBuilder::new(WidgetBuilder::new().with_name(name))
                    .build(ctx)
            }),
            UiMenuEntry::new("AbsmEventProvider", |name, ctx| {
                AbsmEventProviderBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
            UiMenuEntry::new("Selector", |name, ctx| {
                SelectorBuilder::new(WidgetBuilder::new().with_name(name)).build(ctx)
            }),
        ]
    }

    pub fn new(entries: Vec<UiMenuEntry>, name: &str, ctx: &mut BuildContext) -> Self {
        let items = entries
            .iter()
            .map(|e| create_menu_item(&e.name, Default::default(), ctx))
            .collect::<Vec<_>>();

        let constructors = entries
            .into_iter()
            .zip(items.iter().cloned())
            .map(|(entry, node)| (node, entry))
            .collect::<FxHashMap<_, _>>();

        let menu = create_menu_item(name, items, ctx);

        Self { menu, constructors }
    }

    pub fn handle_ui_message(
        &mut self,
        sender: &MessageSender,
        message: &UiMessage,
        scene: &mut UiScene,
        selection: &Selection,
    ) {
        if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
            if let Some(entry) = self.constructors.get_mut(&message.destination()) {
                let ui_node_handle = (entry.constructor)(&entry.name, &mut scene.ui.build_ctx());
                let sub_graph = scene.ui.take_reserve_sub_graph(ui_node_handle);
                let parent = if let Some(selection) = selection.as_ui() {
                    selection.widgets.first().cloned().unwrap_or_default()
                } else {
                    Handle::NONE
                };
                sender.do_command(AddWidgetCommand::new(sub_graph, parent, true));
            }
        }
    }
}
