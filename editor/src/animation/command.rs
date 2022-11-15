use crate::{command::Command, scene::commands::SceneContext};
use fyrox::animation::Animation;
use fyrox::{
    animation::NodeTrack,
    core::{curve::Curve, pool::Handle},
    scene::{animation::AnimationPlayer, node::Node},
    utils::log::Log,
};
use std::fmt::Debug;

fn fetch_animation_player<'a>(
    handle: Handle<Node>,
    context: &'a mut SceneContext,
) -> &'a mut AnimationPlayer {
    context.scene.graph[handle]
        .query_component_mut::<AnimationPlayer>()
        .unwrap()
}

#[derive(Debug)]
pub struct AddTrackCommand {
    animation_player: Handle<Node>,
    animation: Handle<Animation>,
    track: Option<NodeTrack>,
}

impl AddTrackCommand {
    pub fn new(
        animation_player: Handle<Node>,
        animation: Handle<Animation>,
        track: NodeTrack,
    ) -> Self {
        Self {
            animation_player,
            animation,
            track: Some(track),
        }
    }
}

impl Command for AddTrackCommand {
    fn name(&mut self, _: &SceneContext) -> String {
        "Add Track".to_string()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        fetch_animation_player(self.animation_player, context).animations_mut()[self.animation]
            .add_track(self.track.take().unwrap());
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.track = fetch_animation_player(self.animation_player, context).animations_mut()
            [self.animation]
            .pop_track();
    }
}

#[derive(Debug)]
pub struct ReplaceTrackCurveCommand {
    pub animation_player: Handle<Node>,
    pub animation: Handle<Animation>,
    pub curve: Curve,
}

impl ReplaceTrackCurveCommand {
    fn swap(&mut self, context: &mut SceneContext) {
        for track in fetch_animation_player(self.animation_player, context).animations_mut()
            [self.animation]
            .tracks_mut()
        {
            for curve in track.frames_container_mut().curves_mut() {
                if curve.id() == self.curve.id() {
                    std::mem::swap(&mut self.curve, curve);
                    return;
                }
            }
        }

        Log::err(format!("There's no such curve with id {}", self.curve.id()))
    }
}

impl Command for ReplaceTrackCurveCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Replace Track Curve".to_string()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.swap(context)
    }
}
