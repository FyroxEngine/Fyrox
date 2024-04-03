use crate::core::algebra::{Quaternion, Unit, UnitQuaternion, Vector3};
use crate::core::log::Log;
use crate::core::math::curve::{Curve, CurveKey, CurveKeyKind};
use crate::core::pool::Handle;
use crate::generic_animation::container::{TrackDataContainer, TrackValueKind};
use crate::generic_animation::track::Track;
use crate::generic_animation::value::{ValueBinding, ValueType};
use crate::scene::animation::Animation;
use crate::scene::node::Node;
use gltf::animation::util::ReadOutputs;
use gltf::animation::Channel;
use gltf::animation::{Interpolation, Property};
use gltf::Buffer;

type Result<T> = std::result::Result<T, ()>;

use super::iter::*;

/// Extract a list of Animations from the give glTF document, if that document contains any.
/// The resulting list of animations is not guaranteed to be the same length as the list of animations
/// in the document. If any error is encountered in translating an animation from glTF to Fyrox,
/// that animation will be excluded from the resulting list and an error message will be logged.
///
/// * `doc`: The document in which to find the animations.
///
/// * `node_handles`: A slice containing a [Handle] for every node defined in the document, in that order, so that
/// a handle can be looked up using the index of a node within the document. Animations in glTF specify their target
/// nodes by their index within the node list of the document, and these indices need to be translated into handles.
///
/// * `buffers`: A slice containing a list of byte-vectors, one for each buffer in the glTF document.
/// Animations in glTF make reference to data stored in the document's list of buffers by index.
/// This slcie allows an index into the document's list of buffers to be translated into actual bytes of data.
pub fn import_animations(
    doc: &gltf::Document,
    node_handles: &[Handle<Node>],
    buffers: &[Vec<u8>],
) -> Vec<Animation> {
    let mut result: Vec<Animation> = Vec::with_capacity(doc.animations().len());
    for animation in doc.animations() {
        if let Ok(animation) = import_animation(&animation, node_handles, buffers) {
            result.push(animation);
        } else {
            Log::err(format!(
                "Failed to import animation: {}",
                animation.name().unwrap_or("[Unnamed]")
            ));
        }
    }
    result
}

fn import_animation(
    animation: &gltf::Animation,
    node_handles: &[Handle<Node>],
    buffers: &[Vec<u8>],
) -> Result<Animation> {
    let mut result = Animation::default();
    let name = animation
        .name()
        .map(str::to_owned)
        .unwrap_or(default_animation_name(animation));
    //Log::info(format!("ANIMATION: {}", name));
    result.set_name(name);
    import_channels(animation, &mut result, node_handles, buffers)?;
    Ok(result)
}

fn default_animation_name(animation: &gltf::Animation) -> String {
    format!("Animation {}", animation.index())
}

fn import_channels(
    source_animation: &gltf::Animation,
    build_animation: &mut Animation,
    node_handles: &[Handle<Node>],
    buffers: &[Vec<u8>],
) -> Result<()> {
    let mut end_time: f32 = 0.0;
    for channel in source_animation.channels() {
        let channel_end = find_end_time_for_channel(&channel, |buf: Buffer| {
            buffers.get(buf.index()).map(Vec::as_slice)
        })?;
        if end_time < channel_end {
            end_time = channel_end;
        }
        let target_index = channel.target().node().index();
        let target: Handle<Node> = node_handles.get(target_index).ok_or(())?.clone();
        if let Property::MorphTargetWeights = channel.target().property() {
            import_weight_channel(&channel, target, build_animation, |buf: Buffer| {
                buffers.get(buf.index()).map(Vec::as_slice)
            })?;
        } else {
            import_transform_channel(&channel, target, build_animation, |buf: Buffer| {
                buffers.get(buf.index()).map(Vec::as_slice)
            })?;
        }
    }
    build_animation.set_time_slice(0.0..end_time);
    build_animation.set_enabled(true);
    Ok(())
}

fn import_transform_channel<'a, 's, F>(
    channel: &'a Channel,
    target: Handle<Node>,
    build_animation: &mut Animation,
    get_buffer_data: F,
) -> Result<()>
where
    F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>,
{
    let prop = match channel.target().property() {
        Property::Translation => ValueBinding::Position,
        Property::Rotation => ValueBinding::Rotation,
        Property::Scale => ValueBinding::Scale,
        Property::MorphTargetWeights => return Err(()),
    };
    let sampler = import_sampler(&channel, get_buffer_data)?;
    let mut track = Track::new(sampler, prop);
    track.set_target(target);

    //print_track(&track);

    build_animation.add_track(track);
    Ok(())
}

#[allow(dead_code)]
fn print_track(track: &Track<Handle<Node>>) {
    let data = track.data_container();
    println!("Track: {:?}", data.value_kind());
    for (i, curve) in data.curves_ref().iter().enumerate() {
        println!("Curve: {}", i);
        for key in curve.keys() {
            println!("{:4.2}: {}: {:?}", key.location(), key.value, key.kind);
        }
    }
}

fn import_weight_channel<'a, 's, F>(
    channel: &'a Channel,
    target: Handle<Node>,
    build_animation: &mut Animation,
    get_buffer_data: F,
) -> Result<()>
where
    F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>,
{
    let target_count = count_morph_targets(&channel.target().node());
    for i in 0..target_count {
        let prop = ValueBinding::Property {
            name: format!("blend_shapes[{}].weight", i),
            value_type: ValueType::F32,
        };
        let sampler = match channel.sampler().interpolation() {
            Interpolation::Linear => import_simple_morph_sampler(
                channel,
                i,
                target_count,
                CurveKeyKind::Linear,
                get_buffer_data.clone(),
            )?,
            Interpolation::Step => import_simple_morph_sampler(
                channel,
                i,
                target_count,
                CurveKeyKind::Constant,
                get_buffer_data.clone(),
            )?,
            Interpolation::CubicSpline => {
                import_cubic_morph_sampler(channel, i, target_count, get_buffer_data.clone())?
            }
        };
        let mut track = Track::new(sampler, prop);
        track.set_target(target);
        build_animation.add_track(track);
    }
    Ok(())
}

fn count_morph_targets(target: &gltf::Node) -> usize {
    if let Some(mesh) = target.mesh() {
        if let Some(prim) = mesh.primitives().next() {
            prim.morph_targets().count()
        } else {
            0
        }
    } else {
        0
    }
}

fn array_to_euler(quaternion: [f32; 4]) -> Vector3<f32> {
    quaternion_to_euler(Unit::new_normalize(Quaternion::from(quaternion)))
}

//  X - pitch, Y - yaw, Z - roll
// Or maybe X - roll, Y - yaw, Z - pitch
// Or maybe X - roll, Y - pitch, Z - yaw
fn quaternion_to_euler(q: UnitQuaternion<f32>) -> Vector3<f32> {
    let (roll, pitch, yaw) = q.euler_angles();
    //let (x, y, z) = (pitch, yaw, roll);
    let (x, y, z) = (roll, pitch, yaw);
    Vector3::new(x, y, z)
}

fn quaternion_to_euler_tangents(
    in_tangent: [f32; 4],
    value: [f32; 4],
    out_tangent: [f32; 4],
) -> (Vector3<f32>, Vector3<f32>) {
    let a: UnitQuaternion<f32> = Unit::new_normalize(quaternion_subtract(value, in_tangent).into());
    let v: UnitQuaternion<f32> = Unit::new_normalize(value.into());
    let b: UnitQuaternion<f32> = Unit::new_normalize(quaternion_add(value, out_tangent).into());
    (
        quaternion_to_euler(a.rotation_to(&v).into()),
        quaternion_to_euler(v.rotation_to(&b).into()),
    )
}

fn quaternion_add(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let mut r = [0f32; 4];
    for i in 0..4 {
        r[i] = a[i] + b[i];
    }
    r
}
fn quaternion_subtract(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let mut r = [0f32; 4];
    for i in 0..4 {
        r[i] = a[i] - b[i];
    }
    r
}

fn find_end_time_for_channel<'a, 's, F>(channel: &'a Channel, get_buffer_data: F) -> Result<f32>
where
    F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>,
{
    let inputs = channel.reader(get_buffer_data).read_inputs().ok_or(())?;
    Ok(inputs.last().unwrap_or(0.0f32))
}

fn import_sampler<'a, 's, F>(channel: &'a Channel, get_buffer_data: F) -> Result<TrackDataContainer>
where
    F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>,
{
    if let Property::Rotation = channel.target().property() {
        match channel.sampler().interpolation() {
            Interpolation::Linear => {
                import_simple_rotation(channel, CurveKeyKind::Linear, get_buffer_data)
            }
            Interpolation::Step => {
                import_simple_rotation(channel, CurveKeyKind::Constant, get_buffer_data)
            }
            Interpolation::CubicSpline => import_cubic_rotation(channel, get_buffer_data),
        }
    } else {
        match channel.sampler().interpolation() {
            Interpolation::Linear => {
                import_simple_sampler(channel, CurveKeyKind::Linear, get_buffer_data)
            }
            Interpolation::Step => {
                import_simple_sampler(channel, CurveKeyKind::Constant, get_buffer_data)
            }
            Interpolation::CubicSpline => import_cubic_sampler(channel, get_buffer_data),
        }
    }
}

fn import_simple_sampler<'a, 's, F>(
    channel: &'a gltf::animation::Channel,
    kind: CurveKeyKind,
    get_buffer_data: F,
) -> Result<TrackDataContainer>
where
    F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>,
{
    let inputs = channel
        .reader(get_buffer_data.clone())
        .read_inputs()
        .ok_or(())?;
    let outputs = channel.reader(get_buffer_data).read_outputs().ok_or(())?;
    let out_iter = match outputs {
        ReadOutputs::Translations(iter) => iter,
        ReadOutputs::Scales(iter) => iter,
        ReadOutputs::Rotations(_) => return Err(()),
        ReadOutputs::MorphTargetWeights(_) => return Err(()),
    };
    let mut frames_container = TrackDataContainer::new(TrackValueKind::Vector3);
    for i in 0..3 {
        let curve_keys: Curve = inputs
            .clone()
            .zip(out_iter.clone())
            .map(|(time, o)| CurveKey::new(time, o[i], kind.clone()))
            .collect::<Vec<_>>()
            .into();
        frames_container.curves_mut()[i] = curve_keys;
    }
    Ok(frames_container)
}

fn import_simple_morph_sampler<'a, 's, F>(
    channel: &'a gltf::animation::Channel,
    morph_index: usize,
    morph_count: usize,
    kind: CurveKeyKind,
    get_buffer_data: F,
) -> Result<TrackDataContainer>
where
    F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>,
{
    let inputs = channel
        .reader(get_buffer_data.clone())
        .read_inputs()
        .ok_or(())?;
    let outputs = channel.reader(get_buffer_data).read_outputs().ok_or(())?;
    let out_iter = match outputs {
        ReadOutputs::MorphTargetWeights(weights) => weights.into_f32(),
        _ => return Err(()),
    };
    let out_iter = select_iterator(out_iter, morph_index, morph_count);
    let mut frames_container = TrackDataContainer::new(TrackValueKind::Real);
    let curve_keys: Curve = inputs
        .zip(out_iter)
        .map(|(time, o)| CurveKey::new(time, o * 100.0, kind.clone()))
        .collect::<Vec<_>>()
        .into();
    frames_container.curves_mut()[0] = curve_keys;
    Ok(frames_container)
}

fn import_cubic_sampler<'a, 's, F>(
    channel: &'a gltf::animation::Channel,
    get_buffer_data: F,
) -> Result<TrackDataContainer>
where
    F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>,
{
    let inputs = channel
        .reader(get_buffer_data.clone())
        .read_inputs()
        .ok_or(())?;
    let outputs = channel.reader(get_buffer_data).read_outputs().ok_or(())?;
    let out_iter = match outputs {
        ReadOutputs::Translations(iter) => iter,
        ReadOutputs::Scales(iter) => iter,
        ReadOutputs::Rotations(_) => return Err(()),
        ReadOutputs::MorphTargetWeights(_) => return Err(()),
    };
    let mut frames_container = TrackDataContainer::new(TrackValueKind::Vector3);
    for i in 0..3 {
        let curve_keys: Curve = iter_cubic_data(inputs.clone(), out_iter.clone())
            .map(|(time, o)| {
                CurveKey::new(
                    time,
                    o[1][i],
                    CurveKeyKind::Cubic {
                        left_tangent: o[0][i],
                        right_tangent: o[2][i],
                    },
                )
            })
            .collect::<Vec<_>>()
            .into();
        frames_container.curves_mut()[i] = curve_keys;
    }
    Ok(frames_container)
}

fn import_cubic_morph_sampler<'a, 's, F>(
    channel: &'a gltf::animation::Channel,
    morph_index: usize,
    morph_count: usize,
    get_buffer_data: F,
) -> Result<TrackDataContainer>
where
    F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>,
{
    let inputs = channel
        .reader(get_buffer_data.clone())
        .read_inputs()
        .ok_or(())?;
    let outputs = channel.reader(get_buffer_data).read_outputs().ok_or(())?;
    let out_iter = match outputs {
        ReadOutputs::MorphTargetWeights(weights) => weights.into_f32(),
        _ => return Err(()),
    };
    let out_iter = select_iterator(out_iter, morph_index, morph_count);
    let iter = iter_cubic_data(inputs, out_iter);
    let mut frames_container = TrackDataContainer::new(TrackValueKind::Real);
    let curve_keys: Curve = iter
        .map(|(time, o)| {
            CurveKey::new(
                time,
                o[1] * 100.0,
                CurveKeyKind::Cubic {
                    left_tangent: o[0] * 100.0,
                    right_tangent: o[2] * 100.0,
                },
            )
        })
        .collect::<Vec<_>>()
        .into();
    frames_container.curves_mut()[0] = curve_keys;
    Ok(frames_container)
}

fn import_simple_rotation<'a, 's, F>(
    channel: &'a gltf::animation::Channel,
    kind: CurveKeyKind,
    get_buffer_data: F,
) -> Result<TrackDataContainer>
where
    F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>,
{
    let inputs = channel
        .reader(get_buffer_data.clone())
        .read_inputs()
        .ok_or(())?;
    let outputs = channel.reader(get_buffer_data).read_outputs().ok_or(())?;
    let out_iter = match outputs {
        ReadOutputs::Translations(_) => return Err(()),
        ReadOutputs::Scales(_) => return Err(()),
        ReadOutputs::Rotations(iter) => iter.into_f32(),
        ReadOutputs::MorphTargetWeights(_) => return Err(()),
    };
    let mut frames_container = TrackDataContainer::new(TrackValueKind::UnitQuaternion);
    for i in 0..3 {
        let curve_keys: Curve = inputs
            .clone()
            .zip(out_iter.clone())
            .map(|(time, o)| CurveKey::new(time, array_to_euler(o)[i], kind.clone()))
            .collect::<Vec<_>>()
            .into();
        frames_container.curves_mut()[i] = curve_keys;
    }
    Ok(frames_container)
}

fn import_cubic_rotation<'a, 's, F>(
    channel: &'a gltf::animation::Channel,
    get_buffer_data: F,
) -> Result<TrackDataContainer>
where
    F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>,
{
    let inputs = channel
        .reader(get_buffer_data.clone())
        .read_inputs()
        .ok_or(())?;
    let outputs = channel.reader(get_buffer_data).read_outputs().ok_or(())?;
    let out_iter = match outputs {
        ReadOutputs::Translations(_) => return Err(()),
        ReadOutputs::Scales(_) => return Err(()),
        ReadOutputs::Rotations(iter) => iter.into_f32(),
        ReadOutputs::MorphTargetWeights(_) => return Err(()),
    };
    let mut frames_container = TrackDataContainer::new(TrackValueKind::UnitQuaternion);
    for i in 0..3 {
        let curve_keys: Curve = iter_cubic_data(inputs.clone(), out_iter.clone())
            .map(|(time, o)| {
                let (in_tang, out_tang) = quaternion_to_euler_tangents(o[0], o[1], o[2]);
                let value = array_to_euler(o[1]);
                CurveKey::new(
                    time,
                    value[i],
                    CurveKeyKind::Cubic {
                        left_tangent: in_tang[i],
                        right_tangent: out_tang[i],
                    },
                )
            })
            .collect::<Vec<_>>()
            .into();
        frames_container.curves_mut()[i] = curve_keys;
    }
    Ok(frames_container)
}
