// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::{
    fyrox::{
        core::{log::Log, pool::Handle},
        fxhash::FxHashMap,
        graph::constructor::{VariantConstructor, VariantResult},
        gui::{
            constructor::WidgetConstructorContainer, menu::MenuItemMessage,
            message::MessageDirection, message::UiMessage, BuildContext, UiNode, UserInterface,
        },
    },
    menu::create_menu_item,
    message::MessageSender,
    scene::Selection,
    ui_scene::{commands::graph::AddWidgetCommand, UiScene},
};
use fyrox::gui::menu::SortingPredicate;

pub struct UiMenu {
    pub menu: Handle<UiNode>,
    constructor_views: FxHashMap<Handle<UiNode>, VariantConstructor<UiNode, UserInterface>>,
}

impl UiMenu {
    pub fn new(
        constructors: &WidgetConstructorContainer,
        name: &str,
        ctx: &mut BuildContext,
    ) -> Self {
        let mut root_items = vec![];
        let mut groups = FxHashMap::default();
        let mut constructor_views = FxHashMap::default();
        let constructors = constructors.map();
        for constructor in constructors.values() {
            for variant in constructor.variants.iter() {
                let item = create_menu_item(&variant.name, vec![], ctx);
                constructor_views.insert(item, variant.constructor.clone());
                if constructor.group.is_empty() {
                    root_items.push(item);
                } else {
                    let group = *groups.entry(constructor.group).or_insert_with(|| {
                        let group = create_menu_item(constructor.group, vec![], ctx);
                        root_items.push(group);
                        group
                    });
                    ctx.inner().send_message(MenuItemMessage::add_item(
                        group,
                        MessageDirection::ToWidget,
                        item,
                    ));
                }
            }
        }

        let menu = create_menu_item(name, root_items.clone(), ctx);

        for root_item in root_items.iter().chain(&[menu]) {
            ctx.inner().send_message(MenuItemMessage::sort(
                *root_item,
                MessageDirection::ToWidget,
                SortingPredicate::sort_by_text(),
            ))
        }

        Self {
            menu,
            constructor_views,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        sender: &MessageSender,
        message: &UiMessage,
        scene: &mut UiScene,
        selection: &Selection,
    ) {
        if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
            if let Some(constructor) = self.constructor_views.get_mut(&message.destination()) {
                if let VariantResult::Handle(ui_node_handle) = constructor(&mut scene.ui) {
                    let sub_graph = scene.ui.take_reserve_sub_graph(ui_node_handle);
                    let parent = if let Some(selection) = selection.as_ui() {
                        selection.widgets.first().cloned().unwrap_or_default()
                    } else {
                        Handle::NONE
                    };
                    sender.do_command(AddWidgetCommand::new(sub_graph, parent, true));
                } else {
                    Log::err("Unsupported");
                }
            }
        }
    }
}
