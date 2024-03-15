use crate::command::{Command, CommandGroup};
use crate::fyrox::graph::{BaseSceneGraph, SceneGraphNode};
use crate::fyrox::{
    asset::{manager::ResourceManager, untyped::UntypedResource},
    core::{
        algebra::Vector3,
        futures::executor::block_on,
        make_relative_path,
        pool::{ErasedHandle, Handle},
    },
    graph::SceneGraph,
    resource::model::{Model, ModelResourceExtension},
    scene::{node::Node, Scene},
};
use crate::scene::commands::graph::SetGraphNodeChildPosition;
use crate::{
    load_image,
    message::MessageSender,
    scene::{
        commands::{
            graph::{AddModelCommand, LinkNodesCommand},
            ChangeSelectionCommand,
        },
        GameScene, Selection,
    },
    world::{
        graph::{item::DropAnchor, selection::GraphSelection},
        WorldViewerDataProvider,
    },
};
use std::{borrow::Cow, path::Path, path::PathBuf};

pub mod item;
pub mod menu;
pub mod selection;

pub struct EditorSceneWrapper<'a> {
    pub selection: &'a Selection,
    pub game_scene: &'a GameScene,
    pub scene: &'a mut Scene,
    pub path: Option<&'a Path>,
    pub sender: &'a MessageSender,
    pub resource_manager: &'a ResourceManager,
    pub instantiation_scale: Vector3<f32>,
}

impl<'a> WorldViewerDataProvider for EditorSceneWrapper<'a> {
    fn root_node(&self) -> ErasedHandle {
        self.game_scene.scene_content_root.into()
    }

    fn path(&self) -> Option<&Path> {
        self.path
    }

    fn children_of(&self, node: ErasedHandle) -> Vec<ErasedHandle> {
        self.scene
            .graph
            .try_get(node.into())
            .map(|n| {
                n.children()
                    .iter()
                    .map(|h| ErasedHandle::from(*h))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    fn child_count_of(&self, node: ErasedHandle) -> usize {
        self.scene
            .graph
            .try_get(node.into())
            .map_or(0, |node| node.children().len())
    }

    fn nth_child(&self, node: ErasedHandle, i: usize) -> ErasedHandle {
        self.scene
            .graph
            .node(node.into())
            .children()
            .get(i)
            .cloned()
            .unwrap_or_default()
            .into()
    }

    fn is_node_has_child(&self, node: ErasedHandle, child: ErasedHandle) -> bool {
        self.scene
            .graph
            .try_get(node.into())
            .map_or(false, |node| node.children().contains(&child.into()))
    }

    fn parent_of(&self, node: ErasedHandle) -> ErasedHandle {
        self.scene
            .graph
            .try_get(node.into())
            .map(|node| node.parent().into())
            .unwrap_or_default()
    }

    fn name_of(&self, node: ErasedHandle) -> Option<Cow<str>> {
        self.scene
            .graph
            .try_get(node.into())
            .map(|n| Cow::Borrowed(n.name()))
    }

    fn is_valid_handle(&self, node: ErasedHandle) -> bool {
        self.scene.graph.is_valid_handle(node.into())
    }

    fn icon_of(&self, node: ErasedHandle) -> Option<UntypedResource> {
        let node = self.scene.graph.try_get(node.into()).unwrap();
        if node.is_point_light() || node.is_directional_light() || node.is_spot_light() {
            load_image(include_bytes!("../../../resources/light.png"))
        } else if node.is_joint() || node.is_joint2d() {
            load_image(include_bytes!("../../../resources/joint.png"))
        } else if node.is_rigid_body() || node.is_rigid_body2d() {
            load_image(include_bytes!("../../../resources/rigid_body.png"))
        } else if node.is_collider() || node.is_collider2d() {
            load_image(include_bytes!("../../../resources/collider.png"))
        } else if node.is_sound() {
            load_image(include_bytes!("../../../resources/sound_source.png"))
        } else {
            load_image(include_bytes!("../../../resources/cube.png"))
        }
    }

    fn is_instance(&self, node: ErasedHandle) -> bool {
        self.scene
            .graph
            .try_get(node.into())
            .map_or(false, |n| n.resource().is_some())
    }

    fn selection(&self) -> Vec<ErasedHandle> {
        if let Some(graph_selection) = self.selection.as_graph() {
            graph_selection
                .nodes
                .iter()
                .map(|h| ErasedHandle::from(*h))
                .collect::<Vec<_>>()
        } else {
            Default::default()
        }
    }

    fn on_change_hierarchy_request(
        &self,
        child: ErasedHandle,
        parent: ErasedHandle,
        anchor: DropAnchor,
    ) {
        let child: Handle<Node> = child.into();
        let parent: Handle<Node> = parent.into();

        if let Some(selection) = self.selection.as_graph() {
            if selection.nodes.contains(&child) {
                let mut commands = CommandGroup::default();

                'selection_loop: for &node_handle in selection.nodes.iter() {
                    // Make sure we won't create any loops - child must not have parent in its
                    // descendants.
                    let mut p = parent;
                    while p.is_some() {
                        if p == node_handle {
                            continue 'selection_loop;
                        }
                        p = self.scene.graph[p].parent();
                    }

                    match anchor {
                        DropAnchor::Side { index_offset, .. } => {
                            if let Some((parents_parent, position)) =
                                self.scene.graph.relative_position(parent, index_offset)
                            {
                                if let Some(node) = self.scene.graph.try_get(node_handle) {
                                    if node.parent() != parents_parent {
                                        commands.push(LinkNodesCommand::new(
                                            node_handle,
                                            parents_parent,
                                        ));
                                    }
                                }

                                commands.push(SetGraphNodeChildPosition {
                                    node: parents_parent,
                                    child: node_handle,
                                    position,
                                });
                            }
                        }
                        DropAnchor::OnTop => {
                            commands.push(LinkNodesCommand::new(node_handle, parent));
                        }
                    }
                }

                if !commands.is_empty() {
                    self.sender.do_command(commands);
                }
            }
        }
    }

    fn on_asset_dropped(&mut self, path: PathBuf, node: ErasedHandle) {
        if let Ok(relative_path) = make_relative_path(path) {
            if let Some(model) = self
                .resource_manager
                .try_request::<Model>(relative_path)
                .and_then(|m| block_on(m).ok())
            {
                // Instantiate the model.
                let instance = model.instantiate(self.scene);

                self.scene.graph[instance]
                    .local_transform_mut()
                    .set_scale(self.instantiation_scale);

                let sub_graph = self.scene.graph.take_reserve_sub_graph(instance);

                let group = vec![
                    Command::new(AddModelCommand::new(sub_graph)),
                    Command::new(LinkNodesCommand::new(instance, node.into())),
                    Command::new(ChangeSelectionCommand::new(Selection::new(
                        GraphSelection::single_or_empty(instance),
                    ))),
                ];

                self.sender.do_command(CommandGroup::from(group));
            }
        }
    }

    fn validate(&self) -> Vec<(ErasedHandle, Result<(), String>)> {
        self.scene
            .graph
            .pair_iter()
            .map(|(handle, node)| (handle.into(), node.validate(self.scene)))
            .collect()
    }

    fn on_selection_changed(&self, selection: &[ErasedHandle]) {
        let mut new_selection = Selection::default();
        for &selected_item in selection {
            match new_selection.as_graph_mut() {
                Some(selection) => selection.insert_or_exclude(selected_item.into()),
                None => {
                    new_selection =
                        Selection::new(GraphSelection::single_or_empty(selected_item.into()));
                }
            }
        }

        if &new_selection != self.selection {
            self.sender
                .do_command(ChangeSelectionCommand::new(new_selection));
        }
    }
}
