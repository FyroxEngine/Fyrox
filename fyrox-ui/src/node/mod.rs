//! UI node is a type-agnostic wrapper for any widget type. See [`UiNode`] docs for more info.

use crate::{BaseControl, Control};
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

pub mod constructor;

/// UI node is a type-agnostic wrapper for any widget type. Internally, it is just a trait object
/// that provides common widget interface. Its main use is to reduce code bloat (no need to type
/// `Box<dyn Control>` everywhere, just `UiNode`) and to provide some useful methods such as type
/// casting, component querying, etc. You could also be interested in [`Control`] docs, since it
/// contains all the interesting stuff and detailed description for each method.
pub struct UiNode(pub Box<dyn Control>);

impl Deref for UiNode {
    type Target = dyn Control;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl DerefMut for UiNode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

impl UiNode {
    /// Creates a new UI node from any object that implements [`Control`] trait. Its main use
    /// is to finish widget creation like so:
    ///
    /// ```rust
    /// # use fyrox_ui::{
    /// #     core::pool::Handle,
    /// #     define_widget_deref,
    /// #     core::{visitor::prelude::*, reflect::prelude::*},
    /// #     message::UiMessage,
    /// #     widget::{Widget, WidgetBuilder},
    /// #     BuildContext, Control, UiNode, UserInterface,
    /// # };
    /// # use std::{
    /// #     any::{Any, TypeId},
    /// #     ops::{Deref, DerefMut},
    /// # };
    /// #
    /// #[derive(Clone, Visit, Reflect, Debug)]
    /// struct MyWidget {
    ///     widget: Widget,
    /// }
    /// #
    /// # define_widget_deref!(MyWidget);
    /// #
    /// # impl Control for MyWidget {
    /// #     fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
    /// #         todo!()
    /// #     }
    /// #
    /// #     fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
    /// #         todo!()
    /// #     }
    /// # }
    ///
    /// struct MyWidgetBuilder {
    ///     widget_builder: WidgetBuilder,
    /// }
    ///
    /// impl MyWidgetBuilder {
    ///     pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
    ///         let my_widget = MyWidget {
    ///             widget: self.widget_builder.build(),
    ///         };
    ///
    ///         // Wrap your widget in the type-agnostic wrapper so it can be placed in the UI.
    ///         let node = UiNode::new(my_widget);
    ///
    ///         ctx.add_node(node)
    ///     }
    /// }
    /// ```
    pub fn new<T>(widget: T) -> Self
    where
        T: Control,
    {
        Self(Box::new(widget))
    }

    /// Tries to perform **direct** downcasting to a particular widget type. It is just a simple wrapper
    /// for `Any::downcast_ref`.
    pub fn cast<T>(&self) -> Option<&T>
    where
        T: Control,
    {
        BaseControl::as_any(&*self.0).downcast_ref::<T>()
    }

    /// Tries to perform **direct** downcasting to a particular widget type. It is just a simple wrapper
    /// for `Any::downcast_mut`.
    pub fn cast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Control,
    {
        BaseControl::as_any_mut(&mut *self.0).downcast_mut::<T>()
    }

    /// Tries to fetch a component of the given type `T`. At very basis it mimics [`Self::cast`] behaviour, but
    /// also allows you to fetch components of other types as well. For example, your widget may be built on
    /// top of existing one (via composition) and you have it as a field inside your widget. In this case, you
    /// can fetch it by using this method with the appropriate type. See docs for [`Control::query_component`]
    /// for more info.
    pub fn query_component<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        self.0
            .query_component(TypeId::of::<T>())
            .and_then(|c| c.downcast_ref::<T>())
    }

    /// This method checks if the widget has a component of the given type `T`. Internally, it queries the component
    /// of the given type and checks if it exists.
    pub fn has_component<T>(&self) -> bool
    where
        T: 'static,
    {
        self.query_component::<T>().is_some()
    }
}
