use crate::{
    core::{
        math::{
            vec3::Vec3,
            mat4::Mat4,
        },
        pool::Handle,
    },
    resource::fbx::{
        scene::FbxComponent,
        document::{
            FbxNode,
            FbxNodeContainer
        },
    },
};

pub struct FbxModel {
    pub name: String,
    pub pre_rotation: Vec3,
    pub post_rotation: Vec3,
    pub rotation_offset: Vec3,
    pub rotation_pivot: Vec3,
    pub scaling_offset: Vec3,
    pub scaling_pivot: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
    pub translation: Vec3,
    pub geometric_translation: Vec3,
    pub geometric_rotation: Vec3,
    pub geometric_scale: Vec3,
    pub inv_bind_transform: Mat4,
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
    pub fn read(model_node_handle: Handle<FbxNode>, nodes: &FbxNodeContainer) -> Result<FbxModel, String> {
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
            pre_rotation: Vec3::ZERO,
            post_rotation: Vec3::ZERO,
            rotation_offset: Vec3::ZERO,
            rotation_pivot: Vec3::ZERO,
            scaling_offset: Vec3::ZERO,
            scaling_pivot: Vec3::ZERO,
            rotation: Vec3::ZERO,
            scale: Vec3::UNIT,
            translation: Vec3::ZERO,
            geometric_translation: Vec3::ZERO,
            geometric_rotation: Vec3::ZERO,
            geometric_scale: Vec3::UNIT,
            inv_bind_transform: Mat4::IDENTITY,
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
                "GeometricTranslation" => model.geometric_translation = property_node.get_vec3_at(4)?,
                "GeometricScaling" => model.geometric_scale = property_node.get_vec3_at(4)?,
                "GeometricRotation" => model.geometric_rotation = property_node.get_vec3_at(4)?,
                _ => () // Unused properties
            }
        }
        Ok(model)
    }
}