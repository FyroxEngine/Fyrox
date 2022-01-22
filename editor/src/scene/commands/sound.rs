use crate::{command::Command, define_node_command, get_set_swap, scene::commands::SceneContext};
use fyrox::scene::sound::Status;
use fyrox::{
    core::pool::Handle,
    scene::{graph::Graph, node::Node, sound::SoundBufferResource},
};

define_node_command!(SetSoundSourceGainCommand("Set Sound Source Gain", f32) where fn swap(self, source) {
    get_set_swap!(self, source.as_sound_mut(), gain, set_gain);
});

define_node_command!(SetSoundSourceBufferCommand("Set Sound Source Buffer", Option<SoundBufferResource>) where fn swap(self, source) {
    get_set_swap!(self, source.as_sound_mut(), buffer, set_buffer);
});

define_node_command!(SetSoundSourcePanningCommand("Set Sound Source Panning", f32) where fn swap(self, source) {
    get_set_swap!(self, source.as_sound_mut(), panning, set_panning);
});

define_node_command!(SetSoundSourcePitchCommand("Set Sound Source Pitch", f64) where fn swap(self, source) {
    get_set_swap!(self, source.as_sound_mut(), pitch, set_pitch);
});

define_node_command!(SetSoundSourceLoopingCommand("Set Sound Source Looping", bool) where fn swap(self, source) {
    get_set_swap!(self, source.as_sound_mut(), is_looping, set_looping);
});

define_node_command!(SetSoundSourceStatusCommand("Set Sound Source Status", Status) where fn swap(self, source) {
    get_set_swap!(self, source.as_sound_mut(), status, set_status);
});

define_node_command!(SetSoundSourcePlayOnceCommand("Set Sound Source Play Once", bool) where fn swap(self, source) {
    get_set_swap!(self, source.as_sound_mut(), is_play_once, set_play_once);
});

define_node_command!(SetSpatialSoundSourceRadiusCommand("Set Spatial Sound Source Radius", f32) where fn swap(self, source) {
    get_set_swap!(self, source.as_sound_mut(), radius, set_radius);
});

define_node_command!(SetRolloffFactorCommand("Set Spatial Sound Source Rolloff Factor", f32) where fn swap(self, source) {
    get_set_swap!(self, source.as_sound_mut(), rolloff_factor, set_rolloff_factor);
});

define_node_command!(SetMaxDistanceCommand("Set Max Distance", f32) where fn swap(self, source) {
    get_set_swap!(self, source.as_sound_mut(), max_distance, set_max_distance);
});
