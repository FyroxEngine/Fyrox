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
        core::{
            pool::Handle,
            uuid::{uuid, Uuid},
        },
        gui::{
            menu::{ContextMenuBuilder, MenuItem, MenuItemMessage},
            message::UiMessage,
            popup::PopupBuilder,
            stack_panel::StackPanelBuilder,
            widget::WidgetBuilder,
            BuildContext, RcUiNodeHandle,
        },
    },
    menu::create_menu_item,
};
use fyrox::graph::SceneGraph;
use fyrox::gui::popup::{Placement, PopupMessage};
use fyrox::gui::{UiNode, UserInterface};

pub struct SceneItemMenu {
    pub menu: RcUiNodeHandle,
    close: Handle<MenuItem>,
    show_in_explorer: Handle<MenuItem>,
    placement_target: Handle<UiNode>,
}

pub enum SceneItemAction {
    None,
    Close(Uuid),
    ShowInExplorer(Uuid),
}

impl SceneItemMenu {
    pub const CLOSE: Uuid = uuid!("e4c7f0eb-c9a7-4cf2-af4b-78c028be9c58");
    pub const SHOW_IN_EXPLORER: Uuid = uuid!("97a43a80-7860-4fce-b0fa-26ae6677d132");

    pub fn new(ctx: &mut BuildContext) -> Self {
        let close = create_menu_item("Close", Self::CLOSE, vec![], ctx);
        let show_in_explorer =
            create_menu_item("Show In Explorer", Self::SHOW_IN_EXPLORER, vec![], ctx);
        let content = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(close)
                .with_child(show_in_explorer),
        )
        .build(ctx);
        let menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new().with_visibility(false)).with_content(content),
        )
        .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        Self {
            menu,
            close,
            show_in_explorer,
            placement_target: Default::default(),
        }
    }

    pub fn handle_ui_message(
        &mut self,
        ui: &UserInterface,
        message: &UiMessage,
    ) -> SceneItemAction {
        if let Some(MenuItemMessage::Click) = message.data() {
            if let Some(placement_target_id) = self.placement_target_id(ui) {
                if message.destination == self.close {
                    return SceneItemAction::Close(placement_target_id);
                } else if message.destination == self.show_in_explorer {
                    return SceneItemAction::ShowInExplorer(placement_target_id);
                }
            }
        } else if let Some(PopupMessage::Placement(Placement::Cursor(target))) =
            message.data_from(self.menu.handle())
        {
            self.placement_target = *target;
        }
        SceneItemAction::None
    }

    fn placement_target_id(&self, ui: &UserInterface) -> Option<Uuid> {
        ui.try_get(self.placement_target).ok().map(|n| n.id)
    }
}
