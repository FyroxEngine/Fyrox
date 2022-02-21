use crate::core::{
    inspect::{Inspect, PropertyInfo},
    uuid::Uuid,
    visitor::{Visit, VisitResult, Visitor},
};
use fxhash::FxHashMap;
use std::fmt::Debug;

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

pub trait ScriptTrait: BaseScript {
    fn on_init(&mut self);

    fn type_uuid(&self) -> Uuid;
}

pub struct ScriptDefinition {
    pub name: String,
    pub type_uuid: Uuid,
    pub constructor: Box<dyn FnMut() -> Box<dyn ScriptTrait>>,
}

#[derive(Debug)]
pub struct Script(pub Box<dyn ScriptTrait>);

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
    map: FxHashMap<Uuid, ScriptDefinition>,
}

impl ScriptDefinitionStorage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, script_definition: ScriptDefinition) {
        self.map
            .insert(script_definition.type_uuid, script_definition);
    }
}
