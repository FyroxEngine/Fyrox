use crate::{
    asset::{define_new_resource, Resource, ResourceData, ResourceState},
    core::{
        io::{self, FileLoadError},
        visitor::prelude::*,
    },
    lazy_static::lazy_static,
};
use ron::Error;
use serde::Deserialize;
use std::{
    borrow::Cow,
    io::Cursor,
    path::{Path, PathBuf},
};

pub const STANDARD_SHADER: &str = include_str!("standard.ron");

#[derive(Default, Debug)]
pub struct ShaderState {
    path: PathBuf,
    pub definition: ShaderDefinition,
}

impl Visit for ShaderState {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.path.visit("Path", visitor)?;

        if visitor.is_reading() && self.path == PathBuf::default() {
            self.definition = ShaderDefinition::from_str(STANDARD_SHADER).unwrap();
        }

        visitor.leave_region()
    }
}

#[derive(Deserialize, Debug, PartialEq, Clone, Copy, Visit)]
pub enum SamplerFallback {
    White,
    Normal,
    Black,
}

impl Default for SamplerFallback {
    fn default() -> Self {
        Self::White
    }
}

#[derive(Deserialize, Debug, PartialEq)]
pub enum PropertyKind {
    Float(f32),
    Int(i32),
    UInt(u32),
    Bool(bool),
    Vector2 {
        x: f32,
        y: f32,
    },
    Vector3 {
        x: f32,
        y: f32,
        z: f32,
    },
    Vector4 {
        x: f32,
        y: f32,
        z: f32,
        w: f32,
    },
    Color {
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    },
    Sampler {
        default: Option<PathBuf>,
        fallback: SamplerFallback,
    },
}

impl Default for PropertyKind {
    fn default() -> Self {
        Self::Float(0.0)
    }
}

#[derive(Default, Deserialize, Debug, PartialEq)]
pub struct PropertyDefinition {
    pub name: String,
    pub kind: PropertyKind,
}

#[derive(Default, Deserialize, Debug, PartialEq)]
pub struct RenderPassDefinition {
    pub name: String,
    pub vertex_shader: String,
    pub fragment_shader: String,
}

#[derive(Default, Deserialize, Debug, PartialEq)]
pub struct ShaderDefinition {
    pub passes: Vec<RenderPassDefinition>,
    pub properties: Vec<PropertyDefinition>,
}

impl ShaderDefinition {
    fn from_buf(buf: Vec<u8>) -> Result<Self, ShaderError> {
        Ok(ron::de::from_reader(Cursor::new(buf))?)
    }

    fn from_str(str: &str) -> Result<Self, ShaderError> {
        Ok(ron::de::from_str(str)?)
    }
}

impl ShaderState {
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ShaderError> {
        let content = io::load_file(path.as_ref()).await?;
        Ok(Self {
            path: path.as_ref().to_owned(),
            definition: ShaderDefinition::from_buf(content)?,
        })
    }

    pub fn from_str(str: &str) -> Result<Self, ShaderError> {
        Ok(Self {
            path: Default::default(),
            definition: ShaderDefinition::from_str(str)?,
        })
    }
}

impl ResourceData for ShaderState {
    fn path(&self) -> Cow<Path> {
        Cow::from(&self.path)
    }

    fn set_path(&mut self, path: PathBuf) {
        self.path = path;
    }
}

#[derive(Debug)]
pub enum ShaderError {
    Io(FileLoadError),
    ParseError(ron::Error),
}

impl From<ron::Error> for ShaderError {
    fn from(e: Error) -> Self {
        Self::ParseError(e)
    }
}

impl From<FileLoadError> for ShaderError {
    fn from(e: FileLoadError) -> Self {
        Self::Io(e)
    }
}

define_new_resource!(
    #[doc = "See module docs."],
    Shader<ShaderState, ShaderError>
);

impl Shader {
    pub fn from_str(str: &str) -> Result<Self, ShaderError> {
        Ok(Self(Resource::new(ResourceState::Ok(
            ShaderState::from_str(str)?,
        ))))
    }

    fn standard() -> Self {
        Self(Resource::new(ResourceState::Ok(
            ShaderState::from_str(STANDARD_SHADER).unwrap(),
        )))
    }
}

lazy_static! {
    pub static ref STANDARD: Shader = Shader::standard();
}

#[cfg(test)]
mod test {
    use crate::material::shader::{
        PropertyDefinition, PropertyKind, RenderPassDefinition, SamplerFallback, Shader,
        ShaderDefinition,
    };

    #[test]
    fn test_shader_load() {
        let code = r##"
            (
                properties: [
                    (
                        name: "diffuseTexture",
                        kind: Sampler(value: None, fallback: White),
                    ),
                ],

                passes: [
                    (
                        name: "GBuffer",
                        vertex_shader: "<CODE>",
                        fragment_shader: "<CODE>",
                    ),
                ],
            )
            "##;

        let shader = Shader::from_str(code).unwrap();
        let data = shader.data_ref();

        let reference_definition = ShaderDefinition {
            properties: vec![PropertyDefinition {
                name: "diffuseTexture".to_string(),
                kind: PropertyKind::Sampler {
                    default: None,
                    fallback: SamplerFallback::White,
                },
            }],
            passes: vec![RenderPassDefinition {
                name: "GBuffer".to_string(),
                vertex_shader: "<CODE>".to_string(),
                fragment_shader: "<CODE>".to_string(),
            }],
        };

        assert_eq!(data.definition, reference_definition);
    }
}
