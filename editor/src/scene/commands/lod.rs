use crate::{command::Command, define_node_command, get_set_swap, scene::commands::SceneContext};
use fyrox::{
    core::pool::Handle,
    scene::{
        base::{LevelOfDetail, LodControlledObject, LodGroup},
        graph::Graph,
        node::Node,
    },
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

impl Command for AddLodGroupLevelCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Add Lod Group Level".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        context.scene.graph[self.handle]
            .lod_group_mut()
            .unwrap()
            .levels
            .push(self.level.clone());
    }

    fn revert(&mut self, context: &mut SceneContext) {
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

impl Command for RemoveLodGroupLevelCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Remove Lod Group Level".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        self.level = Some(
            context.scene.graph[self.handle]
                .lod_group_mut()
                .unwrap()
                .levels
                .remove(self.index),
        );
    }

    fn revert(&mut self, context: &mut SceneContext) {
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

impl Command for ChangeLodRangeBeginCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Change Lod Range Begin".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        self.swap(context);
    }

    fn revert(&mut self, context: &mut SceneContext) {
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

impl Command for ChangeLodRangeEndCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Change Lod Range End".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        self.swap(context);
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.swap(context);
    }
}

#[derive(Debug)]
pub struct AddLodObjectCommand {
    handle: Handle<Node>,
    lod_index: usize,
    object: LodControlledObject,
    object_index: usize,
}

impl AddLodObjectCommand {
    pub fn new(handle: Handle<Node>, lod_index: usize, object: LodControlledObject) -> Self {
        Self {
            handle,
            lod_index,
            object,
            object_index: 0,
        }
    }
}

impl Command for AddLodObjectCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Add Lod Object".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        let objects = &mut context.scene.graph[self.handle]
            .lod_group_mut()
            .unwrap()
            .levels[self.lod_index]
            .objects;
        self.object_index = objects.len();
        objects.push(self.object);
    }

    fn revert(&mut self, context: &mut SceneContext) {
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
    object: LodControlledObject,
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

impl Command for RemoveLodObjectCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Remove Lod Object".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        self.object = context.scene.graph[self.handle]
            .lod_group_mut()
            .unwrap()
            .levels[self.lod_index]
            .objects
            .remove(self.object_index);
    }

    fn revert(&mut self, context: &mut SceneContext) {
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
pub struct SetLodGroupLodObjectValue {
    pub handle: Handle<Node>,
    pub lod_index: usize,
    pub object_index: usize,
    pub value: Handle<Node>,
}

impl SetLodGroupLodObjectValue {
    fn swap(&mut self, context: &mut SceneContext) {
        std::mem::swap(
            &mut context.scene.graph[self.handle]
                .lod_group_mut()
                .unwrap()
                .levels[self.lod_index]
                .objects[self.object_index]
                .0,
            &mut self.value,
        );
    }
}

impl Command for SetLodGroupLodObjectValue {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Set Lod Object".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.swap(context)
    }
}

define_node_command!(SetLodGroupCommand("Set Lod Group", Option<LodGroup>) where fn swap(self, node) {
    get_set_swap!(self, node, take_lod_group, set_lod_group);
});
