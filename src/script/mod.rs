use crate::{
    core::{
        inspect::{Inspect, PropertyInfo},
        uuid::Uuid,
        visitor::{Visit, VisitResult, Visitor},
    },
    plugin::Plugin,
};
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

pub trait BaseScript: Visit + Inspect + Send + Debug + 'static {
    fn clone_box(&self) -> Box<dyn ScriptTrait>;
}

impl<T> BaseScript for T
where
    T: Clone + ScriptTrait,
{
    fn clone_box(&self) -> Box<dyn ScriptTrait> {
        Box::new(self.clone())
    }
}

pub struct ScriptContext<'a> {
    pub plugin: &'a mut dyn Plugin,
}

pub trait ScriptTrait: BaseScript {
    fn on_init(&mut self, context: &mut ScriptContext);

    fn on_update(&mut self, context: &mut ScriptContext);

    fn type_uuid(&self) -> Uuid;

    fn plugin_uuid(&self) -> Uuid;
}

pub struct ScriptDefinition {
    pub name: String,
    pub type_uuid: Uuid,
    pub constructor: Box<dyn Fn() -> Box<dyn ScriptTrait> + Send>,
}

#[derive(Debug)]
pub struct Script(pub Box<dyn ScriptTrait>);

impl Deref for Script {
    type Target = dyn ScriptTrait;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl DerefMut for Script {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

impl Inspect for Script {
    fn properties(&self) -> Vec<PropertyInfo<'_>> {
        self.0.properties()
    }
}

impl Visit for Script {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.0.visit(name, visitor)
    }
}

impl Clone for Script {
    fn clone(&self) -> Self {
        Self(self.0.clone_box())
    }
}

impl Script {
    pub fn new<T: ScriptTrait>(script_object: T) -> Self {
        Self(Box::new(script_object))
    }
}

#[derive(Default)]
pub struct ScriptDefinitionStorage {
    vec: Vec<ScriptDefinition>,
}

impl ScriptDefinitionStorage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, script_definition: ScriptDefinition) {
        self.vec.push(script_definition);
    }

    pub fn iter(&self) -> impl Iterator<Item = &ScriptDefinition> {
        self.vec.iter()
    }

    pub fn len(&self) -> usize {
        self.vec.len()
    }
}
