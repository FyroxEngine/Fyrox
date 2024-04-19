use crate::fyrox::{
    core::{pool::ErasedHandle, pool::Handle},
    fxhash::FxHashSet,
    generic_animation::machine::{mask::LayerMask, Machine, MachineLayer},
    graph::{BaseSceneGraph, PrefabData, SceneGraph, SceneGraphNode},
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        check_box::{CheckBoxBuilder, CheckBoxMessage},
        dropdown_list::{DropdownListBuilder, DropdownListMessage},
        image::ImageBuilder,
        message::{MessageDirection, UiMessage},
        stack_panel::StackPanelBuilder,
        text::{TextBuilder, TextMessage},
        text_box::{TextBox, TextBoxBuilder},
        utils::{make_cross, make_simple_tooltip},
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, Orientation, Thickness, UiNode, UserInterface, VerticalAlignment,
        BRUSH_BRIGHT,
    },
};
use crate::{
    absm::{
        animation_container_ref,
        command::{AddLayerCommand, RemoveLayerCommand, SetLayerMaskCommand, SetLayerNameCommand},
        fetch_selection, machine_container_ref,
        selection::AbsmSelection,
    },
    command::{Command, CommandGroup},
    gui::make_dropdown_list_option,
    load_image,
    message::MessageSender,
    scene::{
        commands::ChangeSelectionCommand,
        selector::{HierarchyNode, NodeSelectorMessage, NodeSelectorWindowBuilder},
        Selection,
    },
    send_sync_message,
};

pub struct Toolbar {
    pub panel: Handle<UiNode>,
    pub preview: Handle<UiNode>,
    pub layers: Handle<UiNode>,
    pub layer_name: Handle<UiNode>,
    pub add_layer: Handle<UiNode>,
    pub remove_layer: Handle<UiNode>,
    pub edit_mask: Handle<UiNode>,
    pub node_selector: Handle<UiNode>,
}

pub enum ToolbarAction {
    None,
    EnterPreviewMode,
    LeavePreviewMode,
}

impl Toolbar {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let preview;
        let layers;
        let layer_name;
        let add_layer;
        let remove_layer;
        let edit_mask;
        let panel = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    preview = CheckBoxBuilder::new(
                        WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                    )
                    .with_content(
                        TextBuilder::new(
                            WidgetBuilder::new().with_vertical_alignment(VerticalAlignment::Center),
                        )
                        .with_text("Preview")
                        .build(ctx),
                    )
                    .build(ctx);
                    preview
                })
                .with_child({
                    layer_name = TextBoxBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_tooltip(make_simple_tooltip(ctx, "Change selected layer name."))
                            .with_width(100.0),
                    )
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .build(ctx);
                    layer_name
                })
                .with_child({
                    add_layer = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_width(20.0)
                            .with_tooltip(make_simple_tooltip(
                                ctx,
                                "Add a new layer with the name specified in the right text box",
                            )),
                    )
                    .with_text("+")
                    .build(ctx);
                    add_layer
                })
                .with_child({
                    remove_layer = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_width(20.0)
                            .with_tooltip(make_simple_tooltip(ctx, "Removes the current layer.")),
                    )
                    .with_content(make_cross(ctx, 12.0, 2.0))
                    .build(ctx);
                    remove_layer
                })
                .with_child({
                    layers = DropdownListBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_width(100.0),
                    )
                    .build(ctx);
                    layers
                })
                .with_child({
                    edit_mask = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_tooltip(make_simple_tooltip(ctx, "Edit layer mask...")),
                    )
                    .with_content(
                        ImageBuilder::new(
                            WidgetBuilder::new()
                                .with_width(18.0)
                                .with_height(18.0)
                                .with_margin(Thickness::uniform(1.0))
                                .with_background(BRUSH_BRIGHT),
                        )
                        .with_opt_texture(load_image(include_bytes!("../../resources/filter.png")))
                        .build(ctx),
                    )
                    .build(ctx);
                    edit_mask
                }),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        Self {
            panel,
            preview,
            layers,
            layer_name,
            add_layer,
            remove_layer,
            edit_mask,
            node_selector: Handle::NONE,
        }
    }

    pub fn handle_ui_message<P, G, N>(
        &mut self,
        message: &UiMessage,
        editor_selection: &Selection,
        sender: &MessageSender,
        graph: &G,
        ui: &mut UserInterface,
    ) -> ToolbarAction
    where
        P: PrefabData<Graph = G>,
        G: SceneGraph<Node = N, Prefab = P>,
        N: SceneGraphNode<SceneGraph = G, ResourceData = P>,
    {
        let selection = fetch_selection(editor_selection);

        if let Some(CheckBoxMessage::Check(Some(value))) = message.data() {
            if message.destination() == self.preview
                && message.direction() == MessageDirection::FromWidget
            {
                return if *value {
                    ToolbarAction::EnterPreviewMode
                } else {
                    ToolbarAction::LeavePreviewMode
                };
            }
        } else if let Some(DropdownListMessage::SelectionChanged(Some(index))) = message.data() {
            if message.destination() == self.layers
                && message.direction() == MessageDirection::FromWidget
            {
                let mut new_selection = selection;
                new_selection.layer = Some(*index);
                new_selection.entities.clear();
                sender.do_command(ChangeSelectionCommand::new(Selection::new(new_selection)));
            }
        } else if let Some(TextMessage::Text(text)) = message.data() {
            if message.destination() == self.layer_name
                && message.direction() == MessageDirection::FromWidget
            {
                if let Some(layer_index) = selection.layer {
                    sender.do_command(SetLayerNameCommand {
                        absm_node_handle: selection.absm_node_handle,
                        layer_index,
                        name: text.clone(),
                    });
                }
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.add_layer {
                let mut layer = MachineLayer::new();

                layer.set_name(
                    ui.node(self.layer_name)
                        .query_component::<TextBox>()
                        .unwrap()
                        .text(),
                );

                sender.do_command(AddLayerCommand {
                    absm_node_handle: selection.absm_node_handle,
                    layer: Some(layer),
                });
            } else if message.destination() == self.edit_mask {
                let mut root = HierarchyNode {
                    name: "root".to_string(),
                    handle: Default::default(),
                    children: vec![],
                };

                // Collect all scene nodes from every animation in the associated animation player.
                let mut unique_nodes = FxHashSet::default();
                if let Some(machine) = machine_container_ref(graph, selection.absm_node_handle) {
                    if let Some((_, animations)) =
                        animation_container_ref(graph, selection.absm_node_handle)
                    {
                        for animation in animations.iter() {
                            for track in animation.tracks() {
                                unique_nodes.insert(track.target());
                            }
                        }
                    }

                    let local_roots = unique_nodes
                        .iter()
                        .cloned()
                        .filter(|n| {
                            graph
                                .try_get(*n)
                                .map_or(false, |n| !unique_nodes.contains(&n.parent()))
                        })
                        .collect::<Vec<_>>();

                    for local_root in local_roots {
                        root.children.push(HierarchyNode::from_scene_node(
                            local_root,
                            Handle::NONE,
                            graph,
                        ));
                    }

                    self.node_selector = NodeSelectorWindowBuilder::new(
                        WindowBuilder::new(
                            WidgetBuilder::new().with_width(300.0).with_height(400.0),
                        )
                        .open(false)
                        .with_title(WindowTitle::text("Select nodes that will NOT be animated")),
                    )
                    .with_hierarchy(root)
                    .build(&mut ui.build_ctx());

                    ui.send_message(WindowMessage::open_modal(
                        self.node_selector,
                        MessageDirection::ToWidget,
                        true,
                        true,
                    ));

                    if let Some(layer_index) = selection.layer {
                        if let Some(layer) = machine.layers().get(layer_index) {
                            let selection = layer
                                .mask()
                                .inner()
                                .iter()
                                .cloned()
                                .map(ErasedHandle::from)
                                .collect::<Vec<_>>();

                            ui.send_message(NodeSelectorMessage::selection(
                                self.node_selector,
                                MessageDirection::ToWidget,
                                selection,
                            ));
                        }
                    }
                }
            } else if message.destination() == self.remove_layer {
                if let Some(machine) = machine_container_ref(graph, selection.absm_node_handle) {
                    if let Some(layer_index) = selection.layer {
                        let mut commands = Vec::new();

                        commands.push(Command::new(ChangeSelectionCommand::new(Selection::new(
                            AbsmSelection {
                                absm_node_handle: selection.absm_node_handle,
                                layer: if machine.layers().len() > 1 {
                                    Some(0)
                                } else {
                                    None
                                },
                                entities: vec![],
                            },
                        ))));

                        commands.push(Command::new(RemoveLayerCommand::new(
                            selection.absm_node_handle,
                            layer_index,
                        )));

                        sender.do_command(CommandGroup::from(commands));
                    }
                }
            }
        } else if let Some(NodeSelectorMessage::Selection(mask_selection)) = message.data() {
            if message.destination() == self.node_selector
                && message.direction() == MessageDirection::FromWidget
            {
                if let Some(layer_index) = selection.layer {
                    let new_mask = LayerMask::from(
                        mask_selection
                            .iter()
                            .map(|h| Handle::<N>::from(*h))
                            .collect::<Vec<_>>(),
                    );
                    sender.do_command(SetLayerMaskCommand {
                        absm_node_handle: selection.absm_node_handle,
                        layer_index,
                        mask: new_mask,
                    });

                    ui.send_message(WidgetMessage::remove(
                        self.node_selector,
                        MessageDirection::ToWidget,
                    ));

                    self.node_selector = Handle::NONE;
                }
            }
        }

        ToolbarAction::None
    }

    pub fn sync_to_model<P, G, N>(
        &mut self,
        machine: &Machine<Handle<N>>,
        ui: &mut UserInterface,
        selection: &AbsmSelection<N>,
    ) where
        P: PrefabData<Graph = G>,
        G: SceneGraph<Node = N, Prefab = P>,
        N: SceneGraphNode<SceneGraph = G, ResourceData = P>,
    {
        let layers = machine
            .layers()
            .iter()
            .map(|l| make_dropdown_list_option(&mut ui.build_ctx(), l.name()))
            .collect();

        send_sync_message(
            ui,
            DropdownListMessage::items(self.layers, MessageDirection::ToWidget, layers),
        );

        send_sync_message(
            ui,
            DropdownListMessage::selection(
                self.layers,
                MessageDirection::ToWidget,
                selection.layer,
            ),
        );

        if let Some(layer_index) = selection.layer {
            if let Some(layer) = machine.layers().get(layer_index) {
                send_sync_message(
                    ui,
                    TextMessage::text(
                        self.layer_name,
                        MessageDirection::ToWidget,
                        layer.name().to_string(),
                    ),
                );
            }
        }
    }
}
