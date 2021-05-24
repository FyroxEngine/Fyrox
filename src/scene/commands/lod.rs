use crate::{command::Command, define_node_command, get_set_swap, scene::commands::SceneContext};
use rg3d::{
    core::pool::Handle,
    scene::{base::LevelOfDetail, base::LodGroup, graph::Graph, node::Node},
};

#[derive(Debug)]
pub struct AddLodGroupLevelCommand {
    handle: Handle<Node>,
    level: LevelOfDetail,
}

impl AddLodGroupLevelCommand {
    pub fn new(handle: Handle<Node>, level: LevelOfDetail) -> Self {
        Self { handle, level }
    }
}

impl<'a> Command<'a> for AddLodGroupLevelCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Add Lod Group Level".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        context.scene.graph[self.handle]
            .lod_group_mut()
            .unwrap()
            .levels
            .push(self.level.clone());
    }

    fn revert(&mut self, context: &mut Self::Context) {
        context.scene.graph[self.handle]
            .lod_group_mut()
            .unwrap()
            .levels
            .pop();
    }
}

#[derive(Debug)]
pub struct RemoveLodGroupLevelCommand {
    handle: Handle<Node>,
    level: Option<LevelOfDetail>,
    index: usize,
}

impl RemoveLodGroupLevelCommand {
    pub fn new(handle: Handle<Node>, index: usize) -> Self {
        Self {
            handle,
            level: None,
            index,
        }
    }
}

impl<'a> Command<'a> for RemoveLodGroupLevelCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Remove Lod Group Level".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        self.level = Some(
            context.scene.graph[self.handle]
                .lod_group_mut()
                .unwrap()
                .levels
                .remove(self.index),
        );
    }

    fn revert(&mut self, context: &mut Self::Context) {
        let group = context.scene.graph[self.handle].lod_group_mut().unwrap();
        let level = self.level.take().unwrap();
        if group.levels.is_empty() {
            group.levels.push(level);
        } else {
            group.levels.insert(self.index, level)
        }
    }
}

#[derive(Debug)]
pub struct AddLodObjectCommand {
    handle: Handle<Node>,
    lod_index: usize,
    object: Handle<Node>,
    object_index: usize,
}

impl AddLodObjectCommand {
    pub fn new(handle: Handle<Node>, lod_index: usize, object: Handle<Node>) -> Self {
        Self {
            handle,
            lod_index,
            object,
            object_index: 0,
        }
    }
}

impl<'a> Command<'a> for AddLodObjectCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Add Lod Object".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        let objects = &mut context.scene.graph[self.handle]
            .lod_group_mut()
            .unwrap()
            .levels[self.lod_index]
            .objects;
        self.object_index = objects.len();
        objects.push(self.object);
    }

    fn revert(&mut self, context: &mut Self::Context) {
        context.scene.graph[self.handle]
            .lod_group_mut()
            .unwrap()
            .levels[self.lod_index]
            .objects
            .remove(self.object_index);
    }
}

#[derive(Debug)]
pub struct RemoveLodObjectCommand {
    handle: Handle<Node>,
    lod_index: usize,
    object: Handle<Node>,
    object_index: usize,
}

impl RemoveLodObjectCommand {
    pub fn new(handle: Handle<Node>, lod_index: usize, object_index: usize) -> Self {
        Self {
            handle,
            lod_index,
            object: Default::default(),
            object_index,
        }
    }
}

impl<'a> Command<'a> for RemoveLodObjectCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Remove Lod Object".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        self.object = context.scene.graph[self.handle]
            .lod_group_mut()
            .unwrap()
            .levels[self.lod_index]
            .objects
            .remove(self.object_index);
    }

    fn revert(&mut self, context: &mut Self::Context) {
        let objects = &mut context.scene.graph[self.handle]
            .lod_group_mut()
            .unwrap()
            .levels[self.lod_index]
            .objects;
        if objects.is_empty() {
            objects.push(self.object);
        } else {
            objects.insert(self.object_index, self.object);
        }
    }
}

#[derive(Debug)]
pub struct ChangeLodRangeBeginCommand {
    handle: Handle<Node>,
    lod_index: usize,
    new_value: f32,
}

impl ChangeLodRangeBeginCommand {
    pub fn new(handle: Handle<Node>, lod_index: usize, new_value: f32) -> Self {
        Self {
            handle,
            lod_index,
            new_value,
        }
    }

    fn swap(&mut self, context: &mut SceneContext) {
        let level = &mut context.scene.graph[self.handle]
            .lod_group_mut()
            .unwrap()
            .levels[self.lod_index];
        let old = level.begin();
        level.set_begin(self.new_value);
        self.new_value = old;
    }
}

impl<'a> Command<'a> for ChangeLodRangeBeginCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Change Lod Range Begin".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        self.swap(context);
    }

    fn revert(&mut self, context: &mut Self::Context) {
        self.swap(context);
    }
}

#[derive(Debug)]
pub struct ChangeLodRangeEndCommand {
    handle: Handle<Node>,
    lod_index: usize,
    new_value: f32,
}

impl ChangeLodRangeEndCommand {
    pub fn new(handle: Handle<Node>, lod_index: usize, new_value: f32) -> Self {
        Self {
            handle,
            lod_index,
            new_value,
        }
    }

    fn swap(&mut self, context: &mut SceneContext) {
        let level = &mut context.scene.graph[self.handle]
            .lod_group_mut()
            .unwrap()
            .levels[self.lod_index];
        let old = level.end();
        level.set_end(self.new_value);
        self.new_value = old;
    }
}

impl<'a> Command<'a> for ChangeLodRangeEndCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Change Lod Range End".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        self.swap(context);
    }

    fn revert(&mut self, context: &mut Self::Context) {
        self.swap(context);
    }
}

define_node_command!(SetLodGroupCommand("Set Lod Group", Option<LodGroup>) where fn swap(self, node) {
    get_set_swap!(self, node, take_lod_group, set_lod_group);
});
