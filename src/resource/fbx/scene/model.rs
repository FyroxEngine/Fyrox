use crate::core::algebra::{Matrix4, Vector3};
use crate::{
    core::pool::Handle,
    resource::fbx::{
        document::{FbxNode, FbxNodeContainer},
        scene::FbxComponent,
    },
};

pub struct FbxModel {
    pub name: String,
    pub pre_rotation: Vector3<f32>,
    pub post_rotation: Vector3<f32>,
    pub rotation_offset: Vector3<f32>,
    pub rotation_pivot: Vector3<f32>,
    pub scaling_offset: Vector3<f32>,
    pub scaling_pivot: Vector3<f32>,
    pub rotation: Vector3<f32>,
    pub scale: Vector3<f32>,
    pub translation: Vector3<f32>,
    pub geometric_translation: Vector3<f32>,
    pub geometric_rotation: Vector3<f32>,
    pub geometric_scale: Vector3<f32>,
    pub inv_bind_transform: Matrix4<f32>,
    pub geoms: Vec<Handle<FbxComponent>>,
    /// List of handles of materials
    pub materials: Vec<Handle<FbxComponent>>,
    /// List of handles of animation curve nodes
    pub animation_curve_nodes: Vec<Handle<FbxComponent>>,
    /// List of handles of children models
    pub children: Vec<Handle<FbxComponent>>,
    /// Handle to light component
    pub light: Handle<FbxComponent>,
}

impl FbxModel {
    pub fn read(
        model_node_handle: Handle<FbxNode>,
        nodes: &FbxNodeContainer,
    ) -> Result<FbxModel, String> {
        let mut name = String::from("Unnamed");

        let model_node = nodes.get(model_node_handle);
        if let Ok(name_attrib) = model_node.get_attrib(1) {
            name = name_attrib.as_string();
        }

        // Remove prefix
        if name.starts_with("Model::") {
            name = name.chars().skip(7).collect();
        }

        let mut model = FbxModel {
            name,
            pre_rotation: Vector3::default(),
            post_rotation: Vector3::default(),
            rotation_offset: Vector3::default(),
            rotation_pivot: Vector3::default(),
            scaling_offset: Vector3::default(),
            scaling_pivot: Vector3::default(),
            rotation: Vector3::default(),
            scale: Vector3::new(1.0, 1.0, 1.0),
            translation: Vector3::default(),
            geometric_translation: Vector3::default(),
            geometric_rotation: Vector3::default(),
            geometric_scale: Vector3::new(1.0, 1.0, 1.0),
            inv_bind_transform: Matrix4::identity(),
            geoms: Vec::new(),
            materials: Vec::new(),
            animation_curve_nodes: Vec::new(),
            children: Vec::new(),
            light: Handle::NONE,
        };

        let properties70_node_handle = nodes.find(model_node_handle, "Properties70")?;
        let properties70_node = nodes.get(properties70_node_handle);
        for property_handle in properties70_node.children() {
            let property_node = nodes.get(*property_handle);
            let name_attrib = property_node.get_attrib(0)?;
            match name_attrib.as_string().as_str() {
                "Lcl Translation" => model.translation = property_node.get_vec3_at(4)?,
                "Lcl Rotation" => model.rotation = property_node.get_vec3_at(4)?,
                "Lcl Scaling" => model.scale = property_node.get_vec3_at(4)?,
                "PreRotation" => model.pre_rotation = property_node.get_vec3_at(4)?,
                "PostRotation" => model.post_rotation = property_node.get_vec3_at(4)?,
                "RotationOffset" => model.rotation_offset = property_node.get_vec3_at(4)?,
                "RotationPivot" => model.rotation_pivot = property_node.get_vec3_at(4)?,
                "ScalingOffset" => model.scaling_offset = property_node.get_vec3_at(4)?,
                "ScalingPivot" => model.scaling_pivot = property_node.get_vec3_at(4)?,
                "GeometricTranslation" => {
                    model.geometric_translation = property_node.get_vec3_at(4)?
                }
                "GeometricScaling" => model.geometric_scale = property_node.get_vec3_at(4)?,
                "GeometricRotation" => model.geometric_rotation = property_node.get_vec3_at(4)?,
                _ => (), // Unused properties
            }
        }
        Ok(model)
    }
}
