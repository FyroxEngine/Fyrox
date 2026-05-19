// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! UI node is a type-agnostic wrapper for any widget type. See [`UiNode`] docs for more info.

use crate::{
    core::{
        pool::Handle, reflect::prelude::*, uuid_provider, variable, visitor::prelude::*,
        NameProvider,
    },
    widget::Widget,
    Control, ControlAsAny, UserInterface,
};

use fyrox_graph::SceneGraphNode;
use fyrox_resource::{untyped::UntypedResource, Resource};
use std::any::type_name;
use std::{
    any::TypeId,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
};
use uuid::Uuid;

pub mod constructor;
pub mod container;

/// UI node is a type-agnostic wrapper for any widget type. Internally, it is just a trait object
/// that provides a common widget interface. Its main use is to reduce code bloat (no need to type
/// `Box<dyn Control>` everywhere, just `UiNode`) and to provide some useful methods such as type
/// casting, field fetching, etc. You could also be interested in [`Control`] docs, since it
/// contains all the interesting stuff and detailed description for each method.
pub struct UiNode(pub Box<dyn Control>);

impl<T: Control> From<T> for UiNode {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}

uuid_provider!(UiNode = "d9b45ecc-91b0-40ea-a92a-4a7dee4667c9");

impl Clone for UiNode {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone_boxed())
    }
}

impl SceneGraphNode for UiNode {
    type Base = Widget;
    type SceneGraph = UserInterface;
    type ResourceData = UserInterface;

    fn inner_ref(&self) -> &dyn Reflect {
        self.0.deref()
    }

    fn inner_mut(&mut self) -> &mut dyn Reflect {
        self.0.deref_mut()
    }

    fn base(&self) -> &Self::Base {
        self.0.deref()
    }

    fn set_base(&mut self, base: Self::Base) {
        ***self = base;
    }

    fn is_resource_instance_root(&self) -> bool {
        self.is_resource_instance_root
    }

    fn original_handle_in_resource(&self) -> Handle<Self> {
        self.original_handle_in_resource
    }

    fn set_original_handle_in_resource(&mut self, handle: Handle<Self>) {
        self.original_handle_in_resource = handle;
    }

    fn resource(&self) -> Option<Resource<Self::ResourceData>> {
        self.resource.clone()
    }

    fn self_handle(&self) -> Handle<Self> {
        self.handle
    }

    fn parent(&self) -> Handle<Self> {
        self.parent
    }

    fn children(&self) -> &[Handle<Self>] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Handle<Self>] {
        &mut self.children
    }

    fn instance_id(&self) -> Uuid {
        self.id
    }
}

impl NameProvider for UiNode {
    fn name(&self) -> &str {
        &self.0.name
    }
}

impl Debug for UiNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

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
    /// #     core::{visitor::prelude::*, reflect::prelude::*, type_traits::prelude::*,},
    /// #     message::UiMessage,
    /// #     widget::{Widget, WidgetBuilder},
    /// #     BuildContext, Control, UiNode, UserInterface,
    /// # };
    /// # use std::{
    /// #     any::{Any, TypeId},
    /// #     ops::{Deref, DerefMut},
    /// # };
    /// # use fyrox_core::uuid_provider;
    /// #
    /// #[derive(Clone, Visit, Reflect, Debug)]
    /// #[reflect(derived_type = "UiNode")]
    /// struct MyWidget {
    ///     widget: Widget,
    /// }
    /// #
    /// # define_widget_deref!(MyWidget);
    /// #
    /// # uuid_provider!(MyWidget = "a93ec1b5-e7c8-4919-ac19-687d8c99f6bd");
    /// #
    /// # impl Control for MyWidget {
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
    ///     pub fn build(self, ctx: &mut BuildContext) -> Handle<MyWidget> {
    ///         let my_widget = MyWidget {
    ///             widget: self.widget_builder.build(ctx),
    ///         };
    ///
    ///         ctx.add(my_widget)
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
        ControlAsAny::as_any(&*self.0).downcast_ref::<T>()
    }

    /// Tries to perform **direct** downcasting to a particular widget type. It is just a simple wrapper
    /// for `Any::downcast_mut`.
    pub fn cast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Control,
    {
        ControlAsAny::as_any_mut(&mut *self.0).downcast_mut::<T>()
    }

    /// Tries to downcast self to the specified type, or if it is not possible, tries to find a
    /// field of the specified type.
    pub fn self_or_field_ref<T>(&self) -> Option<&T>
    where
        T: Reflect,
    {
        (self.0.deref() as &dyn Reflect).self_or_field_ref()
    }

    /// Tries to downcast self to the specified type, or if it is not possible, tries to find a
    /// field of the specified type. Returns `true` if any of the aforementioned actions succeeded,
    /// `false` - otherwise.
    pub fn is_or_has_field<T>(&self) -> bool
    where
        T: Reflect,
    {
        self.self_or_field_ref::<T>().is_some()
    }

    pub(crate) fn set_inheritance_data(
        &mut self,
        original_handle: Handle<UiNode>,
        model: Resource<UserInterface>,
    ) {
        // Notify instantiated node about the resource it was created from.
        self.resource = Some(model.clone());

        // Reset resource instance root flag, this is needed because a node after instantiation cannot
        // be a root anymore.
        self.is_resource_instance_root = false;

        // Reset inheritable properties, so property inheritance system will take properties
        // from parent objects on resolve stage.
        variable::mark_inheritable_properties_non_modified(
            self,
            &[TypeId::of::<UntypedResource>()],
        );

        // Fill original handles to instances.
        self.original_handle_in_resource = original_handle;
    }
}

impl Visit for UiNode {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.0.visit(name, visitor)
    }
}

impl Reflect for UiNode {
    fn type_info() -> TypeInfo {
        TypeInfo {
            source_path: file!(),
            type_name: type_name::<Self>(),
            assembly_name: env!("CARGO_PKG_NAME"),
            doc_comment: "",
            derived_types: &[],
        }
    }

    fn type_info_ref(&self) -> TypeInfo {
        Self::type_info()
    }

    fn try_clone_box(&self) -> Option<Box<dyn Reflect>> {
        Some(Box::new(self.clone()))
    }

    fn fields_ref(&self, func: &mut dyn FnMut(&[FieldRef])) {
        self.0.deref().fields_ref(func)
    }

    fn fields_mut(&mut self, func: &mut dyn FnMut(&mut [FieldMut])) {
        self.0.deref_mut().fields_mut(func)
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
        self.0.deref_mut().set(value)
    }

    fn field_direct_ref(&self, index: usize) -> Option<FieldRef> {
        self.0.deref().field_direct_ref(index)
    }

    fn field_direct_mut(&mut self, index: usize) -> Option<FieldMut> {
        self.0.deref_mut().field_direct_mut(index)
    }

    fn set_field(
        &mut self,
        field: &str,
        value: Box<dyn Reflect>,
        func: &mut dyn FnMut(Result<Box<dyn Reflect>, SetFieldError>),
    ) {
        self.0.deref_mut().set_field(field, value, func)
    }

    fn find_field(&self, name: &str, func: &mut dyn FnMut(Option<&dyn Reflect>)) {
        self.0.deref().find_field(name, func)
    }

    fn find_field_mut(&mut self, name: &str, func: &mut dyn FnMut(Option<&mut dyn Reflect>)) {
        self.0.deref_mut().find_field_mut(name, func)
    }
}
