#![warn(missing_docs)]

//! Script is used to add custom logic to scene nodes. See [ScriptTrait] for more info.

use crate::{
    core::{
        pool::Handle,
        reflect::{FieldInfo, Reflect, ReflectArray, ReflectList},
        uuid::Uuid,
        visitor::{Visit, VisitResult, Visitor},
    },
    engine::resource_manager::ResourceManager,
    event::Event,
    plugin::Plugin,
    scene::{node::Node, Scene},
    utils::{component::ComponentProvider, log::Log},
};
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

pub mod constructor;

/// A script event's payload.
pub trait ScriptEventPayload: Any {
    /// Returns `self` as `&dyn Any`
    fn as_any_ref(&self) -> &dyn Any;

    /// Returns `self` as `&dyn Any`
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl dyn ScriptEventPayload {
    /// Tries to cast the payload to a particular type.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.as_any_ref().downcast_ref::<T>()
    }

    /// Tries to cast the payload to a particular type.
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut::<T>()
    }
}

impl<T> ScriptEventPayload for T
where
    T: 'static,
{
    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Defines how a script event will be delivered for each node in a hierarchy.
pub enum RoutingStrategy {
    /// An event will be passed to the specified root node and then to every node up in the hierarchy.
    Up,
    /// An event will be passed to every node down the tree in the hierarchy.
    Down,
}

/// An event for a node with a script.
pub enum ScriptEvent {
    /// An event for a specific scene node.
    Targeted {
        /// A handle of a target scene node.
        target: Handle<Node>,

        /// Actual event payload.
        payload: Box<dyn ScriptEventPayload>,
    },

    /// An event for a hierarchy of nodes.
    Hierarchical {
        /// Starting node in a scene graph. Event will be delivered to each node in hierarchy in the order
        /// defined by `routing`.
        root: Handle<Node>,

        /// [Routing strategy](RoutingStrategy) for the event.
        routing: RoutingStrategy,

        /// Actual event payload.
        payload: Box<dyn ScriptEventPayload>,
    },

    /// An event that will be delivered for **every** scene node.
    Global {
        /// Actual event payload.
        payload: Box<dyn ScriptEventPayload>,
    },
}

/// A script event sender.
#[derive(Clone)]
pub struct ScriptEventSender {
    pub(crate) sender: Sender<ScriptEvent>,
}

impl ScriptEventSender {
    /// Send a generic script event.
    pub fn send(&self, event: ScriptEvent) {
        if self.sender.send(event).is_err() {
            Log::err("Failed to send script message, it means the scene is already deleted!");
        }
    }

    /// Sends a targeted script event with the given payload.
    pub fn send_to_target<T>(&self, target: Handle<Node>, payload: T)
    where
        T: 'static,
    {
        self.send(ScriptEvent::Targeted {
            target,
            payload: Box::new(payload),
        })
    }

    /// Sends a global script event with the given payload.
    pub fn send_global<T>(&self, payload: T)
    where
        T: 'static,
    {
        self.send(ScriptEvent::Global {
            payload: Box::new(payload),
        })
    }

    /// Sends a hierarchical script event with the given payload.
    pub fn send_hierarchical<T>(&self, root: Handle<Node>, routing: RoutingStrategy, payload: T)
    where
        T: 'static,
    {
        self.send(ScriptEvent::Hierarchical {
            root,
            routing,
            payload: Box::new(payload),
        })
    }
}

/// Base script trait is used to automatically implement some trait to reduce amount of boilerplate code.
pub trait BaseScript: Visit + Reflect + Send + Debug + 'static {
    /// Creates exact copy of the script.
    fn clone_box(&self) -> Box<dyn ScriptTrait>;
}

impl<T> BaseScript for T
where
    T: Clone + ScriptTrait + Any,
{
    fn clone_box(&self) -> Box<dyn ScriptTrait> {
        Box::new(self.clone())
    }
}

/// A set of data, that provides contextual information for script methods.
pub struct ScriptContext<'a, 'b, 'c> {
    /// Amount of time that passed from last call. It has valid values only when called from `on_update`.
    pub dt: f32,

    /// Amount of time (in seconds) that passed from creation of the engine. Keep in mind, that
    /// this value is **not** guaranteed to match real time. A user can change delta time with
    /// which the engine "ticks" and this delta time affects elapsed time.
    pub elapsed_time: f32,

    /// A reference to the plugin which the script instance belongs to. You can use it to access plugin data
    /// inside script methods. For example you can store some "global" data in the plugin - for example a
    /// controls configuration, some entity managers and so on.
    pub plugins: &'a mut [Box<dyn Plugin>],

    /// Handle of a node to which the script instance belongs to. To access the node itself use `scene` field:
    ///
    /// ```rust
    /// # use fyrox::script::ScriptContext;
    /// # fn foo(context: ScriptContext) {
    /// let node_mut = &mut context.scene.graph[context.handle];
    /// # }
    /// ```
    pub handle: Handle<Node>,

    /// A reference to a scene the script instance belongs to. You have full mutable access to scene content
    /// in most of the script methods.
    pub scene: &'b mut Scene,

    /// A reference to resource manager, use it to load resources.
    pub resource_manager: &'a ResourceManager,

    /// An event sender. Every event sent via this sender will be then passed to every [`ScriptTrait::on_event`]
    /// method of every script.
    pub event_sender: &'c ScriptEventSender,
}

/// A set of data that will be passed to a script instance just before its destruction.
pub struct ScriptDeinitContext<'a, 'b, 'c> {
    /// Amount of time (in seconds) that passed from creation of the engine. Keep in mind, that
    /// this value is **not** guaranteed to match real time. A user can change delta time with
    /// which the engine "ticks" and this delta time affects elapsed time.
    pub elapsed_time: f32,

    /// A reference to the plugin which the script instance belongs to. You can use it to access plugin data
    /// inside script methods. For example you can store some "global" data in the plugin - for example a
    /// controls configuration, some entity managers and so on.
    pub plugins: &'a mut [Box<dyn Plugin>],

    /// A reference to resource manager, use it to load resources.
    pub resource_manager: &'a ResourceManager,

    /// A reference to a scene the script instance was belonging to. You have full mutable access to scene content
    /// in most of the script methods.
    pub scene: &'b mut Scene,

    /// Handle to a parent scene node. Use it with caution because parent node could be deleted already and
    /// any unchecked borrowing using the handle will cause panic!
    pub node_handle: Handle<Node>,

    /// An event sender. Every event sent via this sender will be then passed to every [`ScriptTrait::on_event`]
    /// method of every script.
    pub event_sender: &'c ScriptEventSender,
}

/// Script is a set predefined methods that are called on various stages by the engine. It is used to add
/// custom behaviour to game entities.
pub trait ScriptTrait: BaseScript + ComponentProvider {
    /// The method is called when the script wasn't initialized yet. It is guaranteed to be called once,
    /// and before any other methods of the script.
    ///
    /// # Important
    ///
    /// The method **will not** be called in case if you serialized initialized script instance and then
    /// loaded the instance. Internal flag will tell the engine that the script is initialized and this
    /// method **will not** be called. This is intentional design decision to be able to create save files
    /// in games. If you need a method that will be called in any case, use [`ScriptTrait::on_start`].
    fn on_init(&mut self, #[allow(unused_variables)] ctx: &mut ScriptContext) {}

    /// The method is called after [`ScriptTrait::on_init`], but in separate pass, which means that all
    /// script instances are already initialized. However, if implementor of this method creates a new
    /// node with a script, there will be a second pass of initialization. The method is guaranteed to
    /// be called once.
    fn on_start(&mut self, #[allow(unused_variables)] ctx: &mut ScriptContext) {}

    /// The method is called when the script is about to be destroyed. It is guaranteed to be called last.
    fn on_deinit(&mut self, #[allow(unused_variables)] ctx: &mut ScriptDeinitContext) {}

    /// Called when there is an event from the OS. The method allows you to "listen" for events
    /// coming from the main window of your game (or the editor if the game running inside the
    /// editor.
    fn on_os_event(
        &mut self,
        #[allow(unused_variables)] event: &Event<()>,
        #[allow(unused_variables)] ctx: &mut ScriptContext,
    ) {
    }

    /// Performs a single update tick of the script. The method may be called multiple times per
    /// frame, but it is guaranteed that the rate of call is stable and usually it will be called
    /// 60 times per second (this may change in future releases).
    fn on_update(&mut self, #[allow(unused_variables)] ctx: &mut ScriptContext) {}

    /// Allows you to restore resources after deserialization.
    ///
    /// # Motivation
    ///
    /// Some scripts may store resources "handles" (for example a texture or a 3d model), when the
    /// handle is saved, only path to resource is saved. When you loading a save, you must ask resource
    /// manager to restore handles.
    fn restore_resources(&mut self, #[allow(unused_variables)] resource_manager: ResourceManager) {}

    /// Allows you to react to certain script events. It could be used for communication between scripts; to
    /// bypass borrowing issues.
    fn on_event(
        &mut self,
        #[allow(unused_variables)] event: &mut dyn ScriptEventPayload,
        #[allow(unused_variables)] ctx: &mut ScriptContext,
    ) {
    }

    /// Script instance type UUID. The value will be used for serialization, to write type
    /// identifier to a data source so the engine can restore the script from data source.
    ///
    /// # Important notes
    ///
    /// Do **not** use [`Uuid::new_v4`] or any other [`Uuid`] methods that generates ids, ids
    /// generated using these methods are **random** and are not suitable for serialization!
    ///
    /// # Example
    ///
    /// All you need to do in the method is to return `Self::type_uuid`.
    ///
    /// ```rust
    /// use std::str::FromStr;
    /// use fyrox::{
    ///     scene::node::TypeUuidProvider,
    ///     core::visitor::prelude::*,
    ///     core::reflect::prelude::*,
    ///     core::uuid::Uuid,
    ///     script::ScriptTrait,
    ///     core::uuid::uuid, impl_component_provider
    /// };
    ///
    /// #[derive(Reflect, Visit, Debug, Clone)]
    /// struct MyScript { }
    ///
    /// // Implement TypeUuidProvider trait that will return type uuid of the type.
    /// // Every script must implement the trait so the script can be registered in
    /// // serialization context of the engine.
    /// impl TypeUuidProvider for MyScript {
    ///     fn type_uuid() -> Uuid {
    ///         // Use https://www.uuidgenerator.net/ to generate new UUID.
    ///         uuid!("4cfbe65e-a2c1-474f-b123-57516d80b1f8")
    ///     }
    /// }
    ///
    /// impl_component_provider!(MyScript);
    ///
    /// impl ScriptTrait for MyScript {
    ///     fn id(&self) -> Uuid {
    ///         Self::type_uuid()
    ///     }
    /// }
    /// ```
    fn id(&self) -> Uuid;
}

/// A wrapper for actual script instance internals, it used by the engine.
#[derive(Debug)]
pub struct Script {
    instance: Box<dyn ScriptTrait>,
    pub(crate) initialized: bool,
    pub(crate) started: bool,
}

impl Reflect for Script {
    fn type_name(&self) -> &'static str {
        self.instance.type_name()
    }

    fn fields_info(&self) -> Vec<FieldInfo> {
        self.instance.fields_info()
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self.instance.into_any()
    }

    fn as_any(&self) -> &dyn Any {
        self.instance.deref().as_any()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self.instance.deref_mut().as_any_mut()
    }

    fn as_reflect(&self) -> &dyn Reflect {
        self.instance.deref().as_reflect()
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self.instance.deref_mut().as_reflect_mut()
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
        self.instance.deref_mut().set(value)
    }

    fn fields(&self) -> Vec<&dyn Reflect> {
        self.instance.deref().fields()
    }

    fn fields_mut(&mut self) -> Vec<&mut dyn Reflect> {
        self.instance.deref_mut().fields_mut()
    }

    fn field(&self, name: &str) -> Option<&dyn Reflect> {
        self.instance.deref().field(name)
    }

    fn field_mut(&mut self, name: &str) -> Option<&mut dyn Reflect> {
        self.instance.deref_mut().field_mut(name)
    }

    fn as_array(&self) -> Option<&dyn ReflectArray> {
        self.instance.deref().as_array()
    }

    fn as_array_mut(&mut self) -> Option<&mut dyn ReflectArray> {
        self.instance.deref_mut().as_array_mut()
    }

    fn as_list(&self) -> Option<&dyn ReflectList> {
        self.instance.deref().as_list()
    }

    fn as_list_mut(&mut self) -> Option<&mut dyn ReflectList> {
        self.instance.deref_mut().as_list_mut()
    }
}

impl Deref for Script {
    type Target = dyn ScriptTrait;

    fn deref(&self) -> &Self::Target {
        &*self.instance
    }
}

impl DerefMut for Script {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.instance
    }
}

impl Visit for Script {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region_guard = visitor.enter_region(name)?;

        // Check for new format first, this branch will fail only on attempt to deserialize
        // scripts in old format.
        if self.instance.visit("Data", &mut region_guard).is_ok() {
            // Visit flags.
            self.initialized.visit("Initialized", &mut region_guard)?;
        } else {
            Log::warn(format!(
                "Unable to load script instance of id {} in new format! Trying to load in old format...",
                self.id()
            ));

            // Leave region and try to load in old format.
            drop(region_guard);

            self.instance.visit(name, visitor)?;

            Log::warn(format!(
                "Script instance of id {} loaded successfully using compatibility loader! Resave the script!",
                self.id()
            ));
        }

        Ok(())
    }
}

impl Clone for Script {
    fn clone(&self) -> Self {
        Self {
            instance: self.instance.clone_box(),
            initialized: false,
            started: false,
        }
    }
}

impl Script {
    /// Creates new script wrapper using given script instance.
    #[inline]
    pub fn new<T: ScriptTrait>(script_object: T) -> Self {
        Self {
            instance: Box::new(script_object),
            initialized: false,
            started: false,
        }
    }

    /// Performs downcasting to a particular type.
    #[inline]
    pub fn cast<T: ScriptTrait>(&self) -> Option<&T> {
        fyrox_core::reflect::Reflect::as_any(&self.instance).downcast_ref::<T>()
    }

    /// Performs downcasting to a particular type.
    #[inline]
    pub fn cast_mut<T: ScriptTrait>(&mut self) -> Option<&mut T> {
        fyrox_core::reflect::Reflect::as_any_mut(&mut self.instance).downcast_mut::<T>()
    }

    /// Tries to borrow a component of given type.
    #[inline]
    pub fn query_component_ref<T: Any>(&self) -> Option<&T> {
        self.instance
            .query_component_ref(TypeId::of::<T>())
            .and_then(|c| c.downcast_ref())
    }

    /// Tries to borrow a component of given type.
    #[inline]
    pub fn query_component_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.instance
            .query_component_mut(TypeId::of::<T>())
            .and_then(|c| c.downcast_mut())
    }
}

#[cfg(test)]
mod test {
    use crate::{
        core::{
            reflect::prelude::*, uuid::Uuid, variable::try_inherit_properties,
            variable::InheritableVariable, visitor::prelude::*,
        },
        impl_component_provider,
        scene::base::Base,
        script::{Script, ScriptTrait},
    };

    #[derive(Reflect, Visit, Debug, Clone, Default)]
    struct MyScript {
        field: InheritableVariable<f32>,
    }

    impl_component_provider!(MyScript);

    impl ScriptTrait for MyScript {
        fn id(&self) -> Uuid {
            todo!()
        }
    }

    #[test]
    fn test_script_property_inheritance_on_nodes() {
        let mut child = Base::default();

        child.script = Some(Script::new(MyScript {
            field: InheritableVariable::new(1.23),
        }));

        let mut parent = Base::default();

        parent.script = Some(Script::new(MyScript {
            field: InheritableVariable::new(3.21),
        }));

        try_inherit_properties(child.as_reflect_mut(), parent.as_reflect()).unwrap();

        assert_eq!(
            *child.script().unwrap().cast::<MyScript>().unwrap().field,
            3.21
        );
    }

    #[test]
    fn test_script_property_inheritance() {
        let mut child = Script::new(MyScript {
            field: InheritableVariable::new(1.23),
        });

        let parent = Script::new(MyScript {
            field: InheritableVariable::new(3.21),
        });

        try_inherit_properties(child.as_reflect_mut(), parent.as_reflect()).unwrap();

        assert_eq!(*child.cast::<MyScript>().unwrap().field, 3.21);
    }

    #[test]
    fn test_script_property_inheritance_option() {
        let mut child = Some(Script::new(MyScript {
            field: InheritableVariable::new(1.23),
        }));

        let parent = Some(Script::new(MyScript {
            field: InheritableVariable::new(3.21),
        }));

        try_inherit_properties(child.as_reflect_mut(), parent.as_reflect()).unwrap();

        assert_eq!(
            *child.as_ref().unwrap().cast::<MyScript>().unwrap().field,
            3.21
        );
    }
}
