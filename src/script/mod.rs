use crate::{
    core::{
        inspect::{Inspect, PropertyInfo},
        uuid::Uuid,
        visitor::{Visit, VisitResult, Visitor},
    },
    gui::inspector::PropertyChanged,
    plugin::Plugin,
    scene::node::Node,
};
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
    pub dt: f32,
    pub plugin: &'a mut dyn Plugin,
    pub node: &'b mut Node,
}

pub trait ScriptTrait: BaseScript {
    /// Mutates the state of the script according to the [`PropertyChanged`] info. It is invoked
    /// from the editor when user changes property of the script from the inspector.
    ///
    /// # Editor mode
    ///
    /// Works only in editor mode.
    fn on_property_changed(&mut self, args: &PropertyChanged);

    fn on_init(&mut self, context: &mut ScriptContext);

    /// Performs a single update tick of the script.
    ///
    /// # Editor mode
    ///
    /// Does not work in editor mode.
    fn on_update(&mut self, context: &mut ScriptContext);

    /// Script instance type UUID.
    fn id(&self) -> Uuid;

    /// Parent plugin UUID.
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
