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
    command::{Command, CommandGroup},
    fyrox::{
        core::{algebra::Vector3, pool::Handle},
        engine::{Engine, SerializationContext},
        fxhash::FxHashMap,
        gui::{
            menu::MenuItemMessage, message::MessageDirection, message::UiMessage,
            widget::WidgetMessage, BuildContext, UiNode, UserInterface,
        },
        scene::node::Node,
    },
    menu::{create_menu_item, create_root_menu_item, ui::UiMenu},
    message::MessageSender,
    scene::{
        commands::graph::{AddNodeCommand, MoveNodeCommand},
        controller::SceneController,
        GameScene, Selection,
    },
    ui_scene::UiScene,
    Mode,
};
use fyrox::core::log::Log;
use fyrox::graph::constructor::{VariantConstructor, VariantResult};
use fyrox::gui::constructor::WidgetConstructorContainer;
use fyrox::gui::menu::SortingPredicate;
use fyrox::scene::graph::Graph;

pub struct CreateEntityRootMenu {
    pub menu: Handle<UiNode>,
    pub sub_menus: CreateEntityMenu,
}

impl CreateEntityRootMenu {
    pub fn new(
        serialization_context: &SerializationContext,
        widget_constructors_container: &WidgetConstructorContainer,
        ctx: &mut BuildContext,
    ) -> Self {
        let sub_menus =
            CreateEntityMenu::new(serialization_context, widget_constructors_container, ctx);

        let menu = create_root_menu_item("Create", sub_menus.root_items.clone(), ctx);

        ctx.inner().send_message(MenuItemMessage::sort(
            menu,
            MessageDirection::ToWidget,
            SortingPredicate::sort_by_text(),
        ));

        Self { menu, sub_menus }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        sender: &MessageSender,
        controller: &mut dyn SceneController,
        selection: &Selection,
        engine: &mut Engine,
    ) {
        if let Some(node) = self
            .sub_menus
            .handle_ui_message(message, sender, controller, selection, engine)
        {
            if let Some(game_scene) = controller.downcast_ref::<GameScene>() {
                let scene = &engine.scenes[game_scene.scene];

                let position = game_scene
                    .camera_controller
                    .placement_position(&scene.graph, Default::default());

                let node_handle = scene.graph.generate_free_handles(1)[0];
                sender.do_command(CommandGroup::from(vec![
                    Command::new(AddNodeCommand::new(node, Handle::NONE, true)),
                    Command::new(MoveNodeCommand::new(
                        node_handle,
                        Vector3::default(),
                        position,
                    )),
                ]));
            }
        }
    }

    pub fn on_scene_changed(&self, controller: &dyn SceneController, ui: &UserInterface) {
        self.sub_menus.on_scene_changed(controller, ui);
    }

    pub fn on_mode_changed(&mut self, ui: &UserInterface, mode: &Mode) {
        ui.send_message(WidgetMessage::enabled(
            self.menu,
            MessageDirection::ToWidget,
            mode.is_edit(),
        ));
    }
}

pub struct CreateEntityMenu {
    ui_menu: UiMenu,
    pub root_items: Vec<Handle<UiNode>>,
    constructor_views: FxHashMap<Handle<UiNode>, VariantConstructor<Node, Graph>>,
}

impl CreateEntityMenu {
    pub fn new(
        serialization_context: &SerializationContext,
        widget_constructor_container: &WidgetConstructorContainer,
        ctx: &mut BuildContext,
    ) -> Self {
        let ui_menu = UiMenu::new(widget_constructor_container, "UI", ctx);

        let mut root_items = vec![ui_menu.menu];
        let mut groups = FxHashMap::default();
        let mut constructor_views = FxHashMap::default();
        let constructors = serialization_context.node_constructors.map();
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
                    ctx.send_message(MenuItemMessage::add_item(
                        group,
                        MessageDirection::ToWidget,
                        item,
                    ))
                }
            }
        }

        for root_item in root_items.iter() {
            ctx.inner().send_message(MenuItemMessage::sort(
                *root_item,
                MessageDirection::ToWidget,
                SortingPredicate::sort_by_text(),
            ))
        }

        Self {
            ui_menu,
            constructor_views,
            root_items,
        }
    }

    pub fn on_scene_changed(&self, controller: &dyn SceneController, ui: &UserInterface) {
        let is_ui_scene = controller.downcast_ref::<UiScene>().is_some();

        ui.send_message(WidgetMessage::enabled(
            self.ui_menu.menu,
            MessageDirection::ToWidget,
            is_ui_scene,
        ));

        for widget in self.root_items.iter() {
            if *widget == self.ui_menu.menu {
                continue;
            }

            ui.send_message(WidgetMessage::enabled(
                *widget,
                MessageDirection::ToWidget,
                !is_ui_scene,
            ));
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        sender: &MessageSender,
        controller: &mut dyn SceneController,
        selection: &Selection,
        engine: &mut Engine,
    ) -> Option<Node> {
        if let Some(ui_scene) = controller.downcast_mut::<UiScene>() {
            self.ui_menu
                .handle_ui_message(sender, message, ui_scene, selection);
        } else if let Some(game_scene) = controller.downcast_mut::<GameScene>() {
            let graph = &mut engine.scenes[game_scene.scene].graph;
            if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
                if let Some(constructor) = self.constructor_views.get(&message.destination()) {
                    if let VariantResult::Owned(node) = constructor(graph) {
                        return Some(node);
                    } else {
                        Log::err("Unsupported");
                    }
                }
            }
        }

        None
    }
}
