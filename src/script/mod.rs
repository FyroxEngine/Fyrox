use crate::scene::node::Node;
use crate::{
    core::{
        inspect::{Inspect, PropertyInfo},
        uuid::Uuid,
        visitor::{Visit, VisitResult, Visitor},
    },
    plugin::Plugin,
};
use fyrox_ui::inspector::PropertyChanged;
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

pub mod constructor;

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

pub struct ScriptContext<'a, 'b> {
    pub plugin: &'a mut dyn Plugin,
    pub node: &'b mut Node,
}

pub trait ScriptTrait: BaseScript {
    /// Mutates the state of the script according to the [`PropertyChanged`] info. It is invoked
    /// from the editor when user changes property of the script from the inspector.
    fn on_property_changed(&mut self, args: &PropertyChanged);

    fn on_init(&mut self, context: &mut ScriptContext);

    fn on_update(&mut self, context: &mut ScriptContext);

    fn id(&self) -> Uuid;

    fn plugin_uuid(&self) -> Uuid;
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
