use crate::core::algebra::{Quaternion, Unit, UnitQuaternion, Vector3};
use crate::core::log::Log;
use crate::core::math::curve::{Curve, CurveKey, CurveKeyKind};
use crate::core::pool::Handle;
use crate::fxhash::FxHashSet;
use crate::generic_animation::container::{TrackDataContainer, TrackValueKind};
use crate::generic_animation::track::Track;
use crate::generic_animation::value::{ValueBinding, ValueType};
use crate::scene::animation::Animation;
use crate::scene::graph::Graph;
use crate::scene::mesh::Mesh;
use crate::scene::node::Node;
use fyrox_graph::BaseSceneGraph;
use gltf::animation::util::ReadOutputs;
use gltf::animation::Channel;
use gltf::animation::{Interpolation, Property};
use gltf::Buffer;

use super::iter::*;
use super::simplify::*;

type Result<T> = std::result::Result<T, ()>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ImportedBinding {
    Position,
    Rotation,
    Scale,
    Weight(usize),
}

impl ImportedBinding {
    fn epsilon(&self) -> f32 {
        match self {
            ImportedBinding::Position => 0.001,
            ImportedBinding::Rotation => std::f32::consts::PI / 180.0,
            ImportedBinding::Scale => 0.1,
            ImportedBinding::Weight(_) => 0.001,
        }
    }
    fn max_step(&self) -> f32 {
        match self {
            ImportedBinding::Position => f32::INFINITY,
            ImportedBinding::Rotation => std::f32::consts::PI / 4.0,
            ImportedBinding::Scale => f32::INFINITY,
            ImportedBinding::Weight(_) => f32::INFINITY,
        }
    }
    fn morph_index(&self) -> Result<usize> {
        match self {
            ImportedBinding::Position => Err(()),
            ImportedBinding::Rotation => Err(()),
            ImportedBinding::Scale => Err(()),
            ImportedBinding::Weight(i) => Ok(*i),
        }
    }
    fn kind(&self) -> TrackValueKind {
        match self {
            ImportedBinding::Position => TrackValueKind::Vector3,
            ImportedBinding::Rotation => TrackValueKind::UnitQuaternion,
            ImportedBinding::Scale => TrackValueKind::Vector3,
            ImportedBinding::Weight(_) => TrackValueKind::Real,
        }
    }
    fn value_binding(&self) -> ValueBinding {
        match self {
            ImportedBinding::Position => ValueBinding::Position,
            ImportedBinding::Rotation => ValueBinding::Rotation,
            ImportedBinding::Scale => ValueBinding::Scale,
            ImportedBinding::Weight(i) => ValueBinding::Property {
                name: format!("blend_shapes[{}].weight", i),
                value_type: ValueType::F32,
            },
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ImportedTarget {
    pub handle: Handle<Node>,
    pub binding: ImportedBinding,
}

impl ImportedTarget {
    fn morph_index(&self) -> Result<usize> {
        self.binding.morph_index()
    }
    fn kind(&self) -> TrackValueKind {
        self.binding.kind()
    }
    fn value_binding(&self) -> ValueBinding {
        self.binding.value_binding()
    }
    fn value_in_graph(&self, graph: &Graph) -> Option<Box<[f32]>> {
        let node: &Node = graph.try_get(self.handle)?;
        match self.binding {
            ImportedBinding::Position => {
                Some(node.local_transform.position().data.as_slice().into())
            }
            ImportedBinding::Rotation => Some(
                quaternion_to_euler(*node.local_transform.rotation().get_value_ref())
                    .data
                    .as_slice()
                    .into(),
            ),
            ImportedBinding::Scale => Some(node.local_transform.scale().data.as_slice().into()),
            ImportedBinding::Weight(index) => match node.cast::<Mesh>() {
                Some(mesh) => mesh.blend_shapes().get(index).map(|s| [s.weight].into()),
                None => None,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImportedTrack {
    pub target: ImportedTarget,
    pub curves: Box<[Vec<CurveKey>]>,
}

impl ImportedTrack {
    fn new(target: ImportedTarget) -> Self {
        Self {
            target,
            curves: match target.binding {
                ImportedBinding::Position => <[Vec<CurveKey>; 3]>::default().into(),
                ImportedBinding::Rotation => <[Vec<CurveKey>; 3]>::default().into(),
                ImportedBinding::Scale => <[Vec<CurveKey>; 3]>::default().into(),
                ImportedBinding::Weight(_) => <[Vec<CurveKey>; 1]>::default().into(),
            },
        }
    }
    fn simplify_curves(&mut self) {
        for curve in self.curves.iter_mut() {
            *curve = simplify(
                curve.as_slice(),
                self.target.binding.epsilon(),
                self.target.binding.max_step(),
            );
        }
    }
    fn fixed_value(&self) -> Option<Box<[f32]>> {
        let mut result: Box<[f32]> = if let ImportedBinding::Weight(_) = self.target.binding {
            <[f32; 1]>::default().into()
        } else {
            <[f32; 3]>::default().into()
        };
        for (i, curve) in self.curves.iter().enumerate() {
            if curve.len() != 1 {
                return None;
            }
            result[i] = curve[0].value;
        }
        Some(result)
    }
    fn is_fixed_to_graph(&self, graph: &Graph) -> bool {
        if let (Some(x), Some(y)) = (self.target.value_in_graph(graph), self.fixed_value()) {
            for (x0, y0) in x.iter().zip(y.iter()) {
                if f32::abs(x0 - y0) > self.target.binding.epsilon() {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }
    fn into_track(self) -> Track<Handle<Node>> {
        let mut data = TrackDataContainer::new(self.target.kind());
        for (i, curve) in self.curves.into_vec().into_iter().enumerate() {
            data.curves_mut()[i] = Curve::from(curve);
        }
        let mut track = Track::new(data, self.target.value_binding());
        track.set_target(self.target.handle);
        track.set_enabled(true);
        track
    }
}

struct ImportedAnimation {
    pub name: Box<str>,
    pub start: f32,
    pub end: f32,
    pub tracks: Vec<ImportedTrack>,
}

impl ImportedAnimation {
    fn targets(&self) -> impl Iterator<Item = ImportedTarget> + '_ {
        self.tracks.iter().map(|t| t.target)
    }
    fn get(&self, target: ImportedTarget) -> Option<&ImportedTrack> {
        self.tracks.iter().find(|t| t.target == target)
    }
    fn remove_target(&mut self, target: ImportedTarget) {
        self.tracks.retain(|t| t.target != target);
    }
    fn simplify_curves(&mut self) {
        for t in self.tracks.iter_mut() {
            t.simplify_curves();
        }
    }
    fn into_animation(self) -> Animation {
        let mut result = Animation::default();
        result.set_name(self.name);
        result.set_time_slice(self.start..self.end);
        for t in self.tracks {
            result.add_track(t.into_track());
        }
        result
    }
}

fn all_targets(source: &[ImportedAnimation]) -> impl Iterator<Item = ImportedTarget> {
    let mut set: FxHashSet<ImportedTarget> = FxHashSet::default();
    for anim in source {
        set.extend(anim.targets());
    }
    set.into_iter()
}

fn target_is_fixed_in_all(
    target: ImportedTarget,
    anims: &[ImportedAnimation],
    graph: &Graph,
) -> bool {
    for anim in anims {
        if let Some(track) = anim.get(target) {
            if !track.is_fixed_to_graph(graph) {
                return false;
            }
        }
    }
    true
}

fn remove_fixed_targets(anims: &mut [ImportedAnimation], graph: &Graph) {
    for target in all_targets(anims) {
        if target_is_fixed_in_all(target, anims, graph) {
            for anim in anims.iter_mut() {
                anim.remove_target(target);
            }
        }
    }
}

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
    graph: &Graph,
    buffers: &[Vec<u8>],
) -> Vec<Animation> {
    let mut imports: Vec<ImportedAnimation> = Vec::with_capacity(doc.animations().len());
    for animation in doc.animations() {
        if let Ok(mut import) = import_animation(&animation, node_handles, buffers) {
            import.simplify_curves();
            imports.push(import);
        } else {
            Log::err(format!(
                "Failed to import animation: {}",
                animation.name().unwrap_or("[Unnamed]")
            ));
        }
    }
    remove_fixed_targets(imports.as_mut_slice(), graph);
    let mut result: Vec<Animation> = Vec::with_capacity(imports.len());
    for import in imports {
        result.push(import.into_animation());
    }
    result
}

fn import_animation(
    animation: &gltf::Animation,
    node_handles: &[Handle<Node>],
    buffers: &[Vec<u8>],
) -> Result<ImportedAnimation> {
    let name = animation
        .name()
        .map(Box::<str>::from)
        .unwrap_or(default_animation_name(animation));
    let end = animation
        .channels()
        .map(|c| get_channel_end(&c))
        .max_by(f32::total_cmp)
        .unwrap_or(0.0);
    Ok(ImportedAnimation {
        name,
        start: 0.0,
        end,
        tracks: import_channels(animation, node_handles, buffers)?,
    })
}

fn get_channel_end(channel: &Channel) -> f32 {
    get_accessor_max(channel.sampler().input()).unwrap_or(0.0)
}

fn get_accessor_max(accessor: gltf::Accessor) -> Option<f32> {
    let max = accessor.max()?;
    let vec: &Vec<gltf::json::Value> = max.as_array()?;
    Some(vec.first()?.as_f64()? as f32)
}

fn default_animation_name(animation: &gltf::Animation) -> Box<str> {
    format!("Animation {}", animation.index()).into()
}

/// Build the given `build_animation` based on the given `source_animation`.
fn import_channels(
    source_animation: &gltf::Animation,
    node_handles: &[Handle<Node>],
    buffers: &[Vec<u8>],
) -> Result<Vec<ImportedTrack>> {
    let mut result: Vec<ImportedTrack> = Vec::with_capacity(source_animation.channels().count());
    for channel in source_animation.channels() {
        let target_index = channel.target().node().index();
        let handle = *node_handles.get(target_index).ok_or(())?;
        if let Property::MorphTargetWeights = channel.target().property() {
            import_weight_channel(&channel, handle, &mut result, |buf: Buffer| {
                buffers.get(buf.index()).map(Vec::as_slice)
            })?
        } else {
            import_transform_channel(&channel, handle, &mut result, |buf: Buffer| {
                buffers.get(buf.index()).map(Vec::as_slice)
            })?
        };
    }
    Ok(result)
}

fn import_transform_channel<'a, 's, F>(
    channel: &'a Channel,
    handle: Handle<Node>,
    result: &mut Vec<ImportedTrack>,
    get_buffer_data: F,
) -> Result<()>
where
    F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>,
{
    let track: ImportedTrack = import_sampler(channel, handle, get_buffer_data)?;
    result.push(track);
    Ok(())
}

fn import_weight_channel<'a, 's, F>(
    channel: &'a Channel,
    target: Handle<Node>,
    result: &mut Vec<ImportedTrack>,
    get_buffer_data: F,
) -> Result<()>
where
    F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>,
{
    let target_count = count_morph_targets(&channel.target().node());
    for i in 0..target_count {
        let target = ImportedTarget {
            handle: target,
            binding: ImportedBinding::Weight(i),
        };
        let track = match channel.sampler().interpolation() {
            Interpolation::Linear => import_simple_morph_sampler(
                channel,
                target,
                target_count,
                CurveKeyKind::Linear,
                get_buffer_data.clone(),
            )?,
            Interpolation::Step => import_simple_morph_sampler(
                channel,
                target,
                target_count,
                CurveKeyKind::Constant,
                get_buffer_data.clone(),
            )?,
            Interpolation::CubicSpline => {
                import_cubic_morph_sampler(channel, target, target_count, get_buffer_data.clone())?
            }
        };
        result.push(track);
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
        quaternion_to_euler(a.rotation_to(&v)),
        quaternion_to_euler(v.rotation_to(&b)),
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

fn import_sampler<'a, 's, F>(
    channel: &'a Channel,
    target_handle: Handle<Node>,
    get_buffer_data: F,
) -> Result<ImportedTrack>
where
    F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>,
{
    if let Property::Rotation = channel.target().property() {
        let target = ImportedTarget {
            handle: target_handle,
            binding: ImportedBinding::Rotation,
        };
        match channel.sampler().interpolation() {
            Interpolation::Linear => {
                import_simple_rotation(channel, target, CurveKeyKind::Linear, get_buffer_data)
            }
            Interpolation::Step => {
                import_simple_rotation(channel, target, CurveKeyKind::Constant, get_buffer_data)
            }
            Interpolation::CubicSpline => import_cubic_rotation(channel, target, get_buffer_data),
        }
    } else {
        let target = ImportedTarget {
            handle: target_handle,
            binding: match channel.target().property() {
                Property::Translation => ImportedBinding::Position,
                Property::Scale => ImportedBinding::Scale,
                _ => return Err(()),
            },
        };
        match channel.sampler().interpolation() {
            Interpolation::Linear => {
                import_simple_sampler(channel, target, CurveKeyKind::Linear, get_buffer_data)
            }
            Interpolation::Step => {
                import_simple_sampler(channel, target, CurveKeyKind::Constant, get_buffer_data)
            }
            Interpolation::CubicSpline => import_cubic_sampler(channel, target, get_buffer_data),
        }
    }
}

fn import_simple_sampler<'a, 's, F>(
    channel: &'a gltf::animation::Channel,
    target: ImportedTarget,
    kind: CurveKeyKind,
    get_buffer_data: F,
) -> Result<ImportedTrack>
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
    let mut track = ImportedTrack::new(target);
    for i in 0..3 {
        let curve_keys: Vec<CurveKey> = inputs
            .clone()
            .zip(out_iter.clone())
            .map(|(time, o)| CurveKey::new(time, o[i], kind.clone()))
            .collect::<Vec<_>>();
        track.curves[i] = curve_keys;
    }
    Ok(track)
}

fn import_simple_morph_sampler<'a, 's, F>(
    channel: &'a gltf::animation::Channel,
    target: ImportedTarget,
    morph_count: usize,
    kind: CurveKeyKind,
    get_buffer_data: F,
) -> Result<ImportedTrack>
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
    let out_iter = select_iterator(out_iter, target.morph_index()?, morph_count);
    let mut track = ImportedTrack::new(target);
    track.curves[0] = inputs
        .zip(out_iter)
        .map(|(time, o)| CurveKey::new(time, o * 100.0, kind.clone()))
        .collect();
    Ok(track)
}

fn import_cubic_sampler<'a, 's, F>(
    channel: &'a gltf::animation::Channel,
    target: ImportedTarget,
    get_buffer_data: F,
) -> Result<ImportedTrack>
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
    let mut track: ImportedTrack = ImportedTrack::new(target);
    for i in 0..3 {
        let curve_keys: Vec<CurveKey> = iter_cubic_data(inputs.clone(), out_iter.clone())
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
            .collect::<Vec<_>>();
        track.curves[i] = curve_keys;
    }
    Ok(track)
}

fn import_cubic_morph_sampler<'a, 's, F>(
    channel: &'a gltf::animation::Channel,
    target: ImportedTarget,
    morph_count: usize,
    get_buffer_data: F,
) -> Result<ImportedTrack>
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
    let out_iter = select_iterator(out_iter, target.morph_index()?, morph_count);
    let iter = iter_cubic_data(inputs, out_iter);
    let mut track = ImportedTrack::new(target);
    let curve_keys: Vec<CurveKey> = iter
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
        .collect();
    track.curves[0] = curve_keys;
    Ok(track)
}

fn import_simple_rotation<'a, 's, F>(
    channel: &'a gltf::animation::Channel,
    target: ImportedTarget,
    kind: CurveKeyKind,
    get_buffer_data: F,
) -> Result<ImportedTrack>
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
    let mut track = ImportedTrack::new(target);
    for i in 0..3 {
        let curve_keys: Vec<CurveKey> = inputs
            .clone()
            .zip(out_iter.clone())
            .map(|(time, o)| CurveKey::new(time, array_to_euler(o)[i], kind.clone()))
            .collect::<Vec<_>>();
        track.curves[i] = curve_keys;
    }
    Ok(track)
}

fn import_cubic_rotation<'a, 's, F>(
    channel: &'a gltf::animation::Channel,
    target: ImportedTarget,
    get_buffer_data: F,
) -> Result<ImportedTrack>
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
    let mut track = ImportedTrack::new(target);
    for i in 0..3 {
        let curve_keys: Vec<CurveKey> = iter_cubic_data(inputs.clone(), out_iter.clone())
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
            .collect::<Vec<_>>();
        track.curves[i] = curve_keys;
    }
    Ok(track)
}

impl CurvePoint for CurveKey {
    fn x(&self) -> f32 {
        self.location
    }
    fn y(&self) -> f32 {
        self.value
    }
}
