use crate::{
    core::{algebra::Vector2, log::Log, pool::Handle},
    draw::DrawingContext,
    menu::{MenuItemBuilder, MenuItemContent, MenuItemMessage},
    message::{MessageDirection, OsEvent, UiMessage},
    messagebox::{MessageBoxBuilder, MessageBoxButtons, MessageBoxMessage, MessageBoxResult},
    popup::{Placement, Popup, PopupBuilder, PopupMessage},
    stack_panel::StackPanelBuilder,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    window::{WindowBuilder, WindowTitle},
    BuildContext, Control, NodeHandleMapping, Thickness, UiNode, UserInterface,
};
use std::{
    any::{Any, TypeId},
    cell::Cell,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

#[derive(Clone)]
pub struct ItemContextMenu {
    pub popup: Popup,
    pub delete: Handle<UiNode>,
    pub make_folder: Handle<UiNode>,
    pub delete_message_box: Cell<Handle<UiNode>>,
}

impl Deref for ItemContextMenu {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.popup.widget
    }
}

impl DerefMut for ItemContextMenu {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.popup.widget
    }
}

impl Control for ItemContextMenu {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.popup.query_component(type_id).or_else(|| {
            if type_id == TypeId::of::<Self>() {
                Some(self)
            } else {
                None
            }
        })
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        self.popup.resolve(node_map)
    }

    fn on_remove(&self, sender: &Sender<UiMessage>) {
        self.popup.on_remove(sender)
    }

    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        self.popup.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        self.popup.arrange_override(ui, final_size)
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        self.popup.draw(drawing_context)
    }

    fn update(&mut self, dt: f32, sender: &Sender<UiMessage>) {
        self.popup.update(dt, sender)
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.popup.handle_routed_message(ui, message);

        if let Some(PopupMessage::Placement(Placement::Cursor(_))) = message.data() {
            if message.destination() == self.handle {
                if let Some(item_path) = self.item_path(ui) {
                    ui.send_message(WidgetMessage::enabled(
                        self.make_folder,
                        MessageDirection::ToWidget,
                        item_path.is_dir(),
                    ));
                }
            }
        } else if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.delete {
                if let Some(item_path) = self.item_path(ui) {
                    self.delete_message_box.set(
                        MessageBoxBuilder::new(
                            WindowBuilder::new(
                                WidgetBuilder::new().with_width(250.0).with_height(100.0),
                            )
                            .with_title(WindowTitle::text("Confirm Action"))
                            .open(false),
                        )
                        .with_text(format!("Delete {} file?", item_path.display()).as_str())
                        .with_buttons(MessageBoxButtons::YesNo)
                        .build(&mut ui.build_ctx()),
                    );

                    ui.send_message(MessageBoxMessage::open(
                        self.delete_message_box.get(),
                        MessageDirection::ToWidget,
                        None,
                        None,
                    ));
                }
            } else if message.destination() == self.make_folder {
                // TODO
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        self.popup.preview_message(ui, message);

        if let Some(MessageBoxMessage::Close(result)) = message.data() {
            if message.destination() == self.delete_message_box.get() {
                if let MessageBoxResult::Yes = *result {
                    if let Some(item_path) = self.item_path(ui) {
                        if item_path.is_dir() {
                            Log::verify(std::fs::remove_dir_all(item_path));
                        } else {
                            Log::verify(std::fs::remove_file(item_path));
                        }
                    }
                }

                ui.send_message(WidgetMessage::remove(
                    self.delete_message_box.get(),
                    MessageDirection::ToWidget,
                ));

                self.delete_message_box.set(Handle::NONE);
            }
        }
    }

    fn handle_os_event(
        &mut self,
        self_handle: Handle<UiNode>,
        ui: &mut UserInterface,
        event: &OsEvent,
    ) {
        self.popup.handle_os_event(self_handle, ui, event)
    }
}

impl ItemContextMenu {
    pub fn build(ctx: &mut BuildContext) -> Handle<UiNode> {
        let delete;
        let make_folder;
        let popup = PopupBuilder::new(
            WidgetBuilder::new()
                .with_preview_messages(true)
                .with_visibility(false),
        )
        .with_content(
            StackPanelBuilder::new(
                WidgetBuilder::new()
                    .with_width(120.0)
                    .with_child({
                        delete = MenuItemBuilder::new(
                            WidgetBuilder::new().with_margin(Thickness::uniform(2.0)),
                        )
                        .with_content(MenuItemContent::text("Delete"))
                        .build(ctx);
                        delete
                    })
                    .with_child({
                        make_folder = MenuItemBuilder::new(
                            WidgetBuilder::new().with_margin(Thickness::uniform(2.0)),
                        )
                        .with_content(MenuItemContent::text("Make Folder"))
                        .build(ctx);
                        make_folder
                    }),
            )
            .build(ctx),
        )
        .build_popup(ctx);

        let menu = Self {
            popup,
            delete,
            make_folder,
            delete_message_box: Default::default(),
        };

        ctx.add_node(UiNode::new(menu))
    }

    fn item_path<'a>(&self, ui: &'a UserInterface) -> Option<&'a Path> {
        ui.try_get_node(self.popup.placement.target())
            .and_then(|n| n.user_data_ref::<PathBuf>())
            .map(|p| p.as_path())
    }
}
