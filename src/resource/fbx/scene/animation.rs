use crate::{
    core::pool::Handle,
    resource::fbx::{
        document::{FbxNode, FbxNodeContainer},
        scene::{FbxComponent, FBX_TIME_UNIT},
    },
};
use fxhash::FxHashMap;

pub struct FbxTimeValuePair {
    pub time: f32,
    pub value: f32,
}

pub struct FbxAnimationCurve {
    pub keys: Vec<FbxTimeValuePair>,
}

impl FbxAnimationCurve {
    pub(in crate::resource::fbx) fn read(
        curve_handle: Handle<FbxNode>,
        nodes: &FbxNodeContainer,
    ) -> Result<Self, String> {
        let key_time_handle = nodes.find(curve_handle, "KeyTime")?;
        let key_time_array = nodes.get_by_name(key_time_handle, "a")?;

        let key_value_handle = nodes.find(curve_handle, "KeyValueFloat")?;
        let key_value_array = nodes.get_by_name(key_value_handle, "a")?;

        if key_time_array.attrib_count() != key_value_array.attrib_count() {
            return Err(String::from(
                "FBX: Animation curve contains wrong key data!",
            ));
        }

        let mut curve = FbxAnimationCurve { keys: Vec::new() };

        for i in 0..key_value_array.attrib_count() {
            curve.keys.push(FbxTimeValuePair {
                time: ((key_time_array.get_attrib(i)?.as_i64()? as f64) * FBX_TIME_UNIT) as f32,
                value: key_value_array.get_attrib(i)?.as_f32()?,
            });
        }

        Ok(curve)
    }
}

#[derive(PartialEq, Eq)]
pub enum FbxAnimationCurveNodeType {
    Unknown,
    Translation,
    Rotation,
    Scale,
}

pub struct FbxAnimationCurveNode {
    pub actual_type: FbxAnimationCurveNodeType,

    /// Parameter name to curve mapping, usually it has `d|X`, `d|Y`, `d|Z` as key.
    pub curves: FxHashMap<String, Handle<FbxComponent>>,
}

impl FbxAnimationCurveNode {
    pub fn read(node_handle: Handle<FbxNode>, nodes: &FbxNodeContainer) -> Result<Self, String> {
        let node = nodes.get(node_handle);
        Ok(FbxAnimationCurveNode {
            actual_type: match node.get_attrib(1)?.as_string().as_str() {
                "T" | "AnimCurveNode::T" => FbxAnimationCurveNodeType::Translation,
                "R" | "AnimCurveNode::R" => FbxAnimationCurveNodeType::Rotation,
                "S" | "AnimCurveNode::S" => FbxAnimationCurveNodeType::Scale,
                _ => FbxAnimationCurveNodeType::Unknown,
            },
            curves: Default::default(),
        })
    }
}
