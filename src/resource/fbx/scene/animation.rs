use crate::core::algebra::{UnitQuaternion, Vector3};
use crate::utils::log::MessageKind;
use crate::{
    core::pool::Handle,
    resource::fbx::{
        document::{FbxNode, FbxNodeContainer},
        quat_from_euler,
        scene::{FbxComponent, FbxScene, FBX_TIME_UNIT},
    },
    utils::log::Log,
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

    fn eval(&self, time: f32) -> f32 {
        if self.keys.is_empty() {
            Log::writeln(
                MessageKind::Warning,
                "FBX: Trying to evaluate curve with no keys!".to_owned(),
            );

            return 0.0;
        }

        if time <= self.keys[0].time {
            return self.keys[0].value;
        }

        if time >= self.keys[self.keys.len() - 1].time {
            return self.keys[self.keys.len() - 1].value;
        }

        // Do linear search for span
        for i in 0..(self.keys.len() - 1) {
            let cur = &self.keys[i];
            if cur.time >= time {
                let next = &self.keys[i + 1];

                // calculate interpolation coefficient
                let time_span = next.time - cur.time;
                let k = (time - cur.time) / time_span;

                // TODO: for now assume that we have only linear transitions
                let val_span = next.value - cur.value;
                return cur.value + k * val_span;
            }
        }

        // Edge-case when we are at the end of curve.
        self.keys.last().unwrap().value
    }
}

#[derive(PartialEq)]
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

    pub fn eval_vec3(&self, scene: &FbxScene, default: Vector3<f32>, time: f32) -> Vector3<f32> {
        if self.curves.is_empty() {
            default
        } else {
            let fetch_curve = |name: &str, default: f32| {
                if let Some(FbxComponent::AnimationCurve(curve)) =
                    self.curves.get(name).map(|c| scene.get(*c))
                {
                    curve.eval(time)
                } else {
                    default
                }
            };

            Vector3::new(
                fetch_curve("d|X", default[0]),
                fetch_curve("d|Y", default[1]),
                fetch_curve("d|Z", default[2]),
            )
        }
    }

    pub fn eval_quat(
        &self,
        scene: &FbxScene,
        default_euler: Vector3<f32>,
        time: f32,
    ) -> UnitQuaternion<f32> {
        quat_from_euler(self.eval_vec3(scene, default_euler, time))
    }
}
