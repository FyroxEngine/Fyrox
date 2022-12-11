use crate::{
    absm::{
        command::{AddLayerCommand, SetLayerMaskCommand, SetLayerNameCommand},
        fetch_selection,
        selection::AbsmSelection,
    },
    gui::make_dropdown_list_option,
    load_image,
    scene::{
        commands::ChangeSelectionCommand,
        selector::{HierarchyNode, NodeSelectorMessage, NodeSelectorWindowBuilder},
        EditorScene, Selection,
    },
    send_sync_message, Message,
};
use fyrox::{
    animation::machine::{LayerMask, MachineLayer},
    core::pool::Handle,
    fxhash::FxHashSet,
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        check_box::{CheckBoxBuilder, CheckBoxMessage},
        dropdown_list::{DropdownListBuilder, DropdownListMessage},
        image::ImageBuilder,
        message::{MessageDirection, UiMessage},
        stack_panel::StackPanelBuilder,
        text::{TextBuilder, TextMessage},
        text_box::{TextBox, TextBoxBuilder},
        utils::make_simple_tooltip,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, Orientation, Thickness, UiNode, UserInterface, VerticalAlignment,
        BRUSH_BRIGHT,
    },
    scene::{
        animation::{absm::AnimationBlendingStateMachine, AnimationPlayer},
        graph::Graph,
    },
};
use std::sync::mpsc::Sender;

pub struct Toolbar {
    pub panel: Handle<UiNode>,
    pub preview: Handle<UiNode>,
    pub layers: Handle<UiNode>,
    pub layer_name: Handle<UiNode>,
    pub add_layer: Handle<UiNode>,
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
                            .with_tooltip(make_simple_tooltip(ctx, "Change selected layer name.")),
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
                        .with_opt_texture(load_image(include_bytes!(
                            "../../resources/embed/filter.png"
                        )))
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
            edit_mask,
            node_selector: Handle::NONE,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        sender: &Sender<Message>,
        graph: &Graph,
        ui: &mut UserInterface,
    ) -> ToolbarAction {
        let selection = fetch_selection(&editor_scene.selection);

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
                if let Selection::Absm(ref selection) = editor_scene.selection {
                    let mut new_selection = selection.clone();
                    new_selection.layer = *index;
                    sender
                        .send(Message::do_scene_command(ChangeSelectionCommand::new(
                            Selection::Absm(new_selection),
                            editor_scene.selection.clone(),
                        )))
                        .unwrap();
                }
            }
        } else if let Some(TextMessage::Text(text)) = message.data() {
            if message.destination() == self.layer_name
                && message.direction() == MessageDirection::FromWidget
            {
                sender
                    .send(Message::do_scene_command(SetLayerNameCommand {
                        absm_node_handle: selection.absm_node_handle,
                        layer_index: selection.layer,
                        name: text.clone(),
                    }))
                    .unwrap();
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

                sender
                    .send(Message::do_scene_command(AddLayerCommand {
                        absm_node_handle: selection.absm_node_handle,
                        layer: Some(layer),
                    }))
                    .unwrap();
            } else if message.destination() == self.edit_mask {
                let mut root = HierarchyNode {
                    name: "root".to_string(),
                    handle: Default::default(),
                    children: vec![],
                };

                // Collect all scene nodes from every animation in the associated animation player.
                let mut unique_nodes = FxHashSet::default();
                if let Some(absm_node) = graph
                    .try_get(selection.absm_node_handle)
                    .and_then(|n| n.query_component_ref::<AnimationBlendingStateMachine>())
                {
                    if let Some(animation_player) = graph
                        .try_get(absm_node.animation_player())
                        .and_then(|n| n.query_component_ref::<AnimationPlayer>())
                    {
                        for animation in animation_player.animations().iter() {
                            for track in animation.tracks() {
                                unique_nodes.insert(track.target());
                            }
                        }
                    }

                    // TODO: Ideally we should preserve tree structure here.
                    for node in unique_nodes.into_iter() {
                        if let Some(target) = graph.try_get(node) {
                            root.children.push(HierarchyNode {
                                name: target.name_owned(),
                                handle: node,
                                children: Default::default(),
                            });
                        }
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
                    ));

                    if let Some(layer) = absm_node.machine().layers().get(selection.layer) {
                        let selection = layer.mask().inner().iter().cloned().collect::<Vec<_>>();

                        ui.send_message(NodeSelectorMessage::selection(
                            self.node_selector,
                            MessageDirection::ToWidget,
                            selection,
                        ));
                    }
                }
            }
        } else if let Some(NodeSelectorMessage::Selection(mask_selection)) = message.data() {
            if message.destination() == self.node_selector
                && message.direction() == MessageDirection::FromWidget
            {
                let new_mask =
                    LayerMask::from(FxHashSet::from_iter(mask_selection.iter().cloned()));
                sender
                    .send(Message::do_scene_command(SetLayerMaskCommand {
                        absm_node_handle: selection.absm_node_handle,
                        layer_index: selection.layer,
                        mask: new_mask,
                    }))
                    .unwrap();

                ui.send_message(WidgetMessage::remove(
                    self.node_selector,
                    MessageDirection::ToWidget,
                ));

                self.node_selector = Handle::NONE;
            }
        }

        ToolbarAction::None
    }

    pub fn sync_to_model(
        &mut self,
        absm_node: &AnimationBlendingStateMachine,
        ui: &mut UserInterface,
        selection: &AbsmSelection,
    ) {
        let layers = absm_node
            .machine()
            .layers()
            .iter()
            .map(|l| make_dropdown_list_option(&mut ui.build_ctx(), l.name()))
            .collect();

        ui.send_message(DropdownListMessage::items(
            self.layers,
            MessageDirection::ToWidget,
            layers,
        ));

        ui.send_message(DropdownListMessage::selection(
            self.layers,
            MessageDirection::ToWidget,
            Some(selection.layer),
        ));

        if let Some(layer) = absm_node.machine().layers().get(selection.layer) {
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
