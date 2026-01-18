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
        core::pool::Handle,
        fxhash::FxHashSet,
        generic_animation::machine::{mask::LayerMask, Machine, MachineLayer},
        graph::{PrefabData, SceneGraph, SceneGraphNode},
        gui::{
            button::{ButtonBuilder, ButtonMessage},
            check_box::{CheckBoxBuilder, CheckBoxMessage},
            dropdown_list::{DropdownListBuilder, DropdownListMessage},
            image::ImageBuilder,
            message::UiMessage,
            stack_panel::StackPanelBuilder,
            text::{TextBuilder, TextMessage},
            text_box::{TextBox, TextBoxBuilder},
            utils::{make_cross, make_simple_tooltip},
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, Orientation, Thickness, UiNode, UserInterface, VerticalAlignment,
        },
    },
    load_image,
    message::MessageSender,
    plugins::absm::{
        animation_container_ref,
        command::{AddLayerCommand, RemoveLayerCommand, SetLayerMaskCommand, SetLayerNameCommand},
        fetch_selection, machine_container_ref,
        selection::AbsmSelection,
    },
    scene::selector::{AllowedType, SelectedHandle},
    scene::{
        commands::ChangeSelectionCommand,
        selector::{HierarchyNode, NodeSelectorMessage, NodeSelectorWindowBuilder},
        Selection,
    },
};
use fyrox::gui::button::Button;
use fyrox::gui::stack_panel::StackPanel;
use fyrox::gui::style::resource::StyleResourceExt;
use fyrox::gui::style::Style;
use fyrox::gui::utils::make_dropdown_list_option;
use fyrox::gui::window::WindowAlignment;
use std::any::TypeId;

pub struct Toolbar {
    pub panel: Handle<StackPanel>,
    pub preview: Handle<UiNode>,
    pub layers: Handle<UiNode>,
    pub layer_name: Handle<TextBox>,
    pub add_layer: Handle<Button>,
    pub remove_layer: Handle<Button>,
    pub edit_mask: Handle<Button>,
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
                                .with_background(ctx.style.property(Style::BRUSH_BRIGHT)),
                        )
                        .with_opt_texture(load_image!("../../../resources/filter.png"))
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

        if let Some(CheckBoxMessage::Check(Some(value))) = message.data_from(self.preview) {
            return if *value {
                ToolbarAction::EnterPreviewMode
            } else {
                ToolbarAction::LeavePreviewMode
            };
        } else if let Some(DropdownListMessage::Selection(Some(index))) =
            message.data_from(self.layers)
        {
            let mut new_selection = selection;
            new_selection.layer = Some(*index);
            new_selection.entities.clear();
            sender.do_command(ChangeSelectionCommand::new(Selection::new(new_selection)));
        } else if let Some(TextMessage::Text(text)) = message.data_from(self.layer_name) {
            if let Some(layer_index) = selection.layer {
                sender.do_command(SetLayerNameCommand {
                    absm_node_handle: selection.absm_node_handle,
                    layer_index,
                    name: text.clone(),
                });
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.add_layer {
                let mut layer = MachineLayer::new();

                layer.set_name(ui[self.layer_name].text());

                sender.do_command(AddLayerCommand {
                    absm_node_handle: selection.absm_node_handle,
                    layer: Some(layer),
                });
            } else if message.destination() == self.edit_mask {
                let mut root = HierarchyNode {
                    name: "root".to_string(),
                    inner_type_name: std::any::type_name::<N>().to_string(),
                    handle: Default::default(),
                    inner_type_id: TypeId::of::<N>(),
                    derived_type_ids: N::derived_types().to_vec(),
                    children: vec![],
                };

                // Collect all scene nodes from every animation in the associated animation player.
                let mut unique_nodes = FxHashSet::default();
                if let Some(machine) = machine_container_ref(graph, selection.absm_node_handle) {
                    if let Some((_, animations)) =
                        animation_container_ref(graph, selection.absm_node_handle)
                    {
                        for animation in animations.iter() {
                            for track_binding in animation.track_bindings().values() {
                                unique_nodes.insert(track_binding.target());
                            }
                        }
                    }

                    let local_roots = unique_nodes
                        .iter()
                        .cloned()
                        .filter(|n| {
                            graph
                                .try_get_node(*n)
                                .ok()
                                .is_some_and(|n| !unique_nodes.contains(&n.parent()))
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
                    .with_allowed_types(
                        [AllowedType {
                            id: TypeId::of::<N>(),
                            name: std::any::type_name::<N>().to_string(),
                        }]
                        .into_iter()
                        .collect(),
                    )
                    .with_hierarchy(root)
                    .build(&mut ui.build_ctx());

                    ui.send(
                        self.node_selector,
                        WindowMessage::Open {
                            alignment: WindowAlignment::Center,
                            modal: true,
                            focus_content: true,
                        },
                    );

                    if let Some(layer_index) = selection.layer {
                        if let Some(layer) = machine.layers().get(layer_index) {
                            let selection = layer
                                .mask()
                                .inner()
                                .iter()
                                .cloned()
                                .map(SelectedHandle::from)
                                .collect::<Vec<_>>();

                            ui.send(
                                self.node_selector,
                                NodeSelectorMessage::Selection(selection),
                            );
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
        } else if let Some(NodeSelectorMessage::Selection(mask_selection)) =
            message.data_from(self.node_selector)
        {
            if let Some(layer_index) = selection.layer {
                let new_mask = LayerMask::from(
                    mask_selection
                        .iter()
                        .map(|h| Handle::<N>::from(h.handle))
                        .collect::<Vec<_>>(),
                );
                sender.do_command(SetLayerMaskCommand {
                    absm_node_handle: selection.absm_node_handle,
                    layer_index,
                    mask: new_mask,
                });

                ui.send(self.node_selector, WidgetMessage::Remove);

                self.node_selector = Handle::NONE;
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

        ui.send_sync(self.layers, DropdownListMessage::Items(layers));
        ui.send_sync(self.layers, DropdownListMessage::Selection(selection.layer));

        if let Some(layer_index) = selection.layer {
            if let Some(layer) = machine.layers().get(layer_index) {
                ui.send_sync(self.layer_name, TextMessage::Text(layer.name().to_string()));
            }
        }
    }
}
