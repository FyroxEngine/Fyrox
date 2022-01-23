use crate::{command::Command, define_swap_command, scene::commands::SceneContext};
use fyrox::scene::sound::Status;
use fyrox::scene::{node::Node, sound::SoundBufferResource};

define_swap_command! {
    Node::as_sound_mut,
    SetSoundSourceGainCommand(f32): gain, set_gain, "Set Sound Source Gain";
    SetSoundSourceBufferCommand(Option<SoundBufferResource>): buffer, set_buffer, "Set Sound Source Buffer";
    SetSoundSourcePanningCommand(f32): panning, set_panning, "Set Sound Source Panning";
    SetSoundSourcePitchCommand(f64): pitch, set_pitch, "Set Sound Source Pitch";
    SetSoundSourceLoopingCommand(bool): is_looping, set_looping, "Set Sound Source Looping";
    SetSoundSourceStatusCommand(Status): status, set_status, "Set Sound Source Status";
    SetSoundSourcePlayOnceCommand(bool): is_play_once, set_play_once, "Set Sound Source Play Once";
    SetSpatialSoundSourceRadiusCommand(f32): radius, set_radius, "Set Spatial Sound Source Radius";
    SetRolloffFactorCommand(f32): rolloff_factor, set_rolloff_factor, "Set Spatial Sound Source Rolloff Factor";
    SetMaxDistanceCommand(f32): max_distance, set_max_distance, "Set Max Distance";
    SetSpatialBlendCommand(f32): spatial_blend, set_spatial_blend, "Set Spatial Blend";
}
