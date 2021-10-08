use crate::physics::Joint;
use crate::world::physics::selection::JointSelection;
use crate::{
    physics::RigidBody,
    scene::{commands::ChangeSelectionCommand, EditorScene, Selection},
    send_sync_message,
    world::physics::{
        item::{PhysicsItem, PhysicsItemBuilder, PhysicsItemMessage},
        selection::RigidBodySelection,
    },
    GameEngine, Message,
};
use rg3d::core::pool::Pool;
use rg3d::gui::text::TextBuilder;
use rg3d::{
    core::pool::Handle,
    engine::Engine,
    gui::{
        button::ButtonBuilder,
        grid::{Column, GridBuilder, Row},
        list_view::{ListView, ListViewBuilder},
        message::{ListViewMessage, MessageDirection, UiMessage, UiMessageData},
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        UiNode, UserInterface,
    },
    scene::graph::Graph,
};
use std::{cmp::Ordering, sync::mpsc::Sender};

pub mod item;
pub mod selection;

pub struct PhysicsViewer {
    pub window: Handle<UiNode>,
    pub bodies: Handle<UiNode>,
    pub joints: Handle<UiNode>,
}

fn fetch_physics_entity<T: 'static>(handle: Handle<UiNode>, ui: &UserInterface) -> Handle<T> {
    if let Some(item) = ui.node(handle).cast::<PhysicsItem<T>>() {
        item.physics_entity
    } else {
        unreachable!()
    }
}

fn fetch_name(body: Handle<RigidBody>, editor_scene: &EditorScene, graph: &Graph) -> String {
    if let Some(associated_node) = editor_scene.physics.binder.backward_map().get(&body) {
        graph[*associated_node].name_owned()
    } else {
        "Rigid Body".to_string()
    }
}

pub fn sync<T, N>(
    list_view: Handle<UiNode>,
    pool: &Pool<T>,
    ui: &mut UserInterface,
    selection: Option<&[Handle<T>]>,
    mut make_name: N,
) where
    T: 'static,
    N: FnMut(Handle<T>) -> String,
{
    let list_view_items = ui
        .node(list_view)
        .cast::<ListView>()
        .unwrap()
        .items()
        .to_vec();

    match pool.alive_count().cmp(&list_view_items.len()) {
        Ordering::Less => {
            // A source was removed.
            for &item in list_view_items.iter() {
                let associated_source = fetch_physics_entity(item, ui);

                if pool.pair_iter().all(|(h, _)| h != associated_source) {
                    send_sync_message(
                        ui,
                        ListViewMessage::remove_item(list_view, MessageDirection::ToWidget, item),
                    );
                }
            }
        }
        Ordering::Greater => {
            // A source was added.
            for (handle, _) in pool.pair_iter() {
                if list_view_items
                    .iter()
                    .all(|i| fetch_physics_entity(*i, ui) != handle)
                {
                    let item = PhysicsItemBuilder::<T>::new(WidgetBuilder::new())
                        .with_name((make_name)(handle))
                        .with_physics_entity(handle)
                        .build(&mut ui.build_ctx());
                    send_sync_message(
                        ui,
                        ListViewMessage::add_item(list_view, MessageDirection::ToWidget, item),
                    );
                }
            }
        }
        _ => (),
    }

    // Sync selection.
    send_sync_message(
        ui,
        ListViewMessage::selection(
            list_view,
            MessageDirection::ToWidget,
            if let Some(selection) = selection {
                if let Some(first) = selection.first() {
                    ui.node(list_view)
                        .cast::<ListView>()
                        .unwrap()
                        .items()
                        .iter()
                        .position(|i| fetch_physics_entity(*i, ui) == *first)
                } else {
                    None
                }
            } else {
                None
            },
        ),
    );

    // Sync sound names. Since rigid body cannot have a name, we just take the name of an associated
    // scene node (if any), or a placeholder "Rigid Body" if there is no associated scene node.
    for item in ui.node(list_view).cast::<ListView>().unwrap().items() {
        let rigid_body = fetch_physics_entity::<T>(*item, ui);
        ui.send_message(UiMessage::user(
            *item,
            MessageDirection::ToWidget,
            Box::new(PhysicsItemMessage::Name((make_name)(rigid_body))),
        ));
    }
}

impl PhysicsViewer {
    pub fn new(engine: &mut Engine) -> Self {
        let ctx = &mut engine.user_interface.build_ctx();

        let bodies;
        let joints;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            GridBuilder::new(
                                WidgetBuilder::new()
                                    .on_column(0)
                                    .with_child(
                                        TextBuilder::new(WidgetBuilder::new())
                                            .with_text("Rigid Bodies")
                                            .build(ctx),
                                    )
                                    .with_child({
                                        bodies =
                                            ListViewBuilder::new(WidgetBuilder::new().on_row(1))
                                                .build(ctx);
                                        bodies
                                    }),
                            )
                            .add_row(Row::strict(20.0))
                            .add_row(Row::stretch())
                            .add_column(Column::stretch())
                            .build(ctx),
                        )
                        .with_child(
                            GridBuilder::new(
                                WidgetBuilder::new()
                                    .on_column(1)
                                    .with_child(
                                        TextBuilder::new(WidgetBuilder::new())
                                            .with_text("Joints")
                                            .build(ctx),
                                    )
                                    .with_child({
                                        joints =
                                            ListViewBuilder::new(WidgetBuilder::new().on_row(1))
                                                .build(ctx);
                                        joints
                                    }),
                            )
                            .add_row(Row::strict(20.0))
                            .add_row(Row::stretch())
                            .add_column(Column::stretch())
                            .build(ctx),
                        ),
                )
                .add_column(Column::stretch())
                .add_column(Column::stretch())
                .add_row(Row::stretch())
                .build(ctx),
            )
            .with_title(WindowTitle::text("Physics"))
            .build(ctx);

        Self {
            window,
            joints,
            bodies,
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let ui = &mut engine.user_interface;

        let graph = &engine.scenes[editor_scene.scene].graph;

        sync(
            self.bodies,
            &editor_scene.physics.bodies,
            ui,
            if let Selection::RigidBody(ref s) = editor_scene.selection {
                Some(&s.bodies)
            } else {
                None
            },
            |b| fetch_name(b, editor_scene, graph),
        );

        sync(
            self.joints,
            &editor_scene.physics.joints,
            ui,
            if let Selection::Joint(ref s) = editor_scene.selection {
                Some(&s.joints)
            } else {
                None
            },
            |j| format!("Joint ({}:{})", j.index(), j.generation()),
        )
    }

    pub fn handle_ui_message(
        &mut self,
        sender: &Sender<Message>,
        editor_scene: &EditorScene,
        message: &UiMessage,
        engine: &GameEngine,
    ) {
        let ui = &engine.user_interface;

        if let UiMessageData::ListView(ListViewMessage::SelectionChanged(selection)) =
            message.data()
        {
            if message.direction() == MessageDirection::FromWidget {
                let new_selection = if message.destination() == self.bodies {
                    Some(match selection {
                        None => Default::default(),
                        Some(index) => Selection::RigidBody(RigidBodySelection {
                            bodies: vec![fetch_physics_entity::<RigidBody>(
                                ui.node(self.bodies).cast::<ListView>().unwrap().items()[*index],
                                ui,
                            )],
                        }),
                    })
                } else if message.destination() == self.joints {
                    Some(match selection {
                        None => Default::default(),
                        Some(index) => Selection::Joint(JointSelection {
                            joints: vec![fetch_physics_entity::<Joint>(
                                ui.node(self.joints).cast::<ListView>().unwrap().items()[*index],
                                ui,
                            )],
                        }),
                    })
                } else {
                    None
                };

                if let Some(new_selection) = new_selection {
                    if new_selection != editor_scene.selection {
                        sender
                            .send(Message::do_scene_command(ChangeSelectionCommand::new(
                                new_selection,
                                editor_scene.selection.clone(),
                            )))
                            .unwrap();
                    }
                }
            }
        }
    }
}
