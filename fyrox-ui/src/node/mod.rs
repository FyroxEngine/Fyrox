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
    constructor::WidgetConstructorContainer,
    core::{
        pool::Handle, reflect::prelude::*, uuid_provider, variable, visitor::prelude::*,
        ComponentProvider, NameProvider,
    },
    widget::Widget,
    Control, ControlAsAny, UserInterface,
};

use fyrox_core::pool::{BorrowNodeVariant, MismatchedTypeError, NodeOrNodeVariant};
use fyrox_graph::SceneGraphNode;
use fyrox_resource::{untyped::UntypedResource, Resource};
use std::{
    any::{Any, TypeId},
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
};
use uuid::Uuid;

pub mod constructor;
pub mod container;

/// UI node is a type-agnostic wrapper for any widget type. Internally, it is just a trait object
/// that provides common widget interface. Its main use is to reduce code bloat (no need to type
/// `Box<dyn Control>` everywhere, just `UiNode`) and to provide some useful methods such as type
/// casting, component querying, etc. You could also be interested in [`Control`] docs, since it
/// contains all the interesting stuff and detailed description for each method.
pub struct UiNode(pub Box<dyn Control>);

/// UiNode is a Box that doesn't implement Default, so we need custom impl of VisitAsOption.
impl VisitAsOption for UiNode {
    fn visit_as_option(
        option_self: &mut Option<Self>,
        name: &str,
        visitor: &mut fyrox_core::visitor::Visitor,
    ) -> fyrox_core::visitor::VisitResult {
        let mut region = visitor.enter_region(name)?;
        // the following code references the implementation of Visit for Option<T>
        let mut is_some = u8::from(option_self.is_some());
        is_some.visit("IsSome", &mut region)?;
        if is_some != 0 {
            if region.is_reading() {
                // calling read_widget avoids requiring Node to implement Default and assigning to it indirectly.
                // See impl<T> Visit for Option<T> for the difference.
                *option_self = Some(read_widget("Data", &mut region)?);
            } else {
                let unwrapped_self = option_self
                    .as_mut()
                    .expect("is_some is accidentally modified by is_some.visit() in write mode");
                write_widget("Data", unwrapped_self, &mut region)?;
            }
        } else if region.is_reading() {
            *option_self = None;
        }
        Ok(())
    }
}

impl NodeOrNodeVariant<UiNode> for UiNode {
    fn convert_to_dest_type(node: &UiNode) -> Result<&Self, fyrox_core::pool::MismatchedTypeError> {
        Ok(node)
    }
    fn convert_to_dest_type_mut(
        node: &mut UiNode,
    ) -> Result<&mut Self, fyrox_core::pool::MismatchedTypeError> {
        Ok(node)
    }
}

impl<T: Control> From<T> for UiNode {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}

uuid_provider!(UiNode = "d9b45ecc-91b0-40ea-a92a-4a7dee4667c9");

impl ComponentProvider for UiNode {
    #[inline]
    fn query_component_ref(
        &self,
        type_id: TypeId,
    ) -> Result<&dyn Any, (TypeId, TypeId, Vec<TypeId>)> {
        self.0.query_component_ref(type_id)
    }

    #[inline]
    fn query_component_mut(
        &mut self,
        type_id: TypeId,
    ) -> Result<&mut dyn Any, (TypeId, TypeId, Vec<TypeId>)> {
        self.0.query_component_mut(type_id)
    }
}

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
    /// #[derive(Clone, Visit, Reflect, Debug, ComponentProvider)]
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
    ///     pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
    ///         let my_widget = MyWidget {
    ///             widget: self.widget_builder.build(ctx),
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

    /// Tries to fetch a component of the given type `T`. At very basis it mimics [`Self::cast`] behaviour, but
    /// also allows you to fetch components of other types as well. For example, your widget may be built on
    /// top of existing one (via composition) and you have it as a field inside your widget. In this case, you
    /// can fetch it by using this method with the appropriate type. See docs for [`fyrox_core::type_traits::ComponentProvider::query_component_ref`]
    /// for more info.
    pub fn query_component<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        self.0
            .query_component_ref(TypeId::of::<T>())
            .ok()
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

    pub(crate) fn set_inheritance_data(
        &mut self,
        original_handle: Handle<UiNode>,
        model: Resource<UserInterface>,
    ) {
        // Notify instantiated node about resource it was created from.
        self.resource = Some(model.clone());

        // Reset resource instance root flag, this is needed because a node after instantiation cannot
        // be a root anymore.
        self.is_resource_instance_root = false;

        // Reset inheritable properties, so property inheritance system will take properties
        // from parent objects on resolve stage.
        self.as_reflect_mut(&mut |reflect| {
            variable::mark_inheritable_properties_non_modified(
                reflect,
                &[TypeId::of::<UntypedResource>()],
            )
        });

        // Fill original handles to instances.
        self.original_handle_in_resource = original_handle;
    }
}

impl BorrowNodeVariant for UiNode {
    fn borrow_variant<T: fyrox_core::pool::NodeVariant<Self>>(
        &self,
    ) -> Result<&T, MismatchedTypeError> {
        ControlAsAny::as_any(self.0.deref())
            .downcast_ref()
            .ok_or_else(|| MismatchedTypeError::new(TypeId::of::<T>(), self.0.deref().type_id()))
    }
    fn borrow_variant_mut<T: fyrox_core::pool::NodeVariant<Self>>(
        &mut self,
    ) -> Result<&mut T, MismatchedTypeError> {
        let actual_type_id = self.0.deref().type_id();
        ControlAsAny::as_any_mut(self.0.deref_mut())
            .downcast_mut()
            .ok_or_else(|| MismatchedTypeError::new(TypeId::of::<T>(), actual_type_id))
    }
}

fn read_widget(name: &str, visitor: &mut Visitor) -> Result<UiNode, VisitError> {
    let mut region = visitor.enter_region(name)?;

    let mut id = Uuid::default();
    id.visit("TypeUuid", &mut region)?;

    let serialization_context = region
        .blackboard
        .get::<WidgetConstructorContainer>()
        .expect("Visitor environment must contain serialization context!");

    let mut widget = serialization_context
        .try_create(&id)
        .ok_or_else(|| panic!("Unknown widget type uuid {id}!"))
        .unwrap();

    widget.visit("WidgetData", &mut region)?;

    Ok(widget)
}

fn write_widget(name: &str, widget: &mut UiNode, visitor: &mut Visitor) -> VisitResult {
    let mut region = visitor.enter_region(name)?;

    let mut id = widget.id();
    id.visit("TypeUuid", &mut region)?;

    widget.visit("WidgetData", &mut region)?;

    Ok(())
}

impl Reflect for UiNode {
    fn source_path() -> &'static str {
        file!()
    }

    fn derived_types() -> &'static [TypeId] {
        &[]
    }

    fn query_derived_types(&self) -> &'static [TypeId] {
        Self::derived_types()
    }

    fn type_name(&self) -> &'static str {
        Reflect::type_name(self.0.deref())
    }

    fn doc(&self) -> &'static str {
        self.0.deref().doc()
    }

    fn assembly_name(&self) -> &'static str {
        self.0.deref().assembly_name()
    }

    fn type_assembly_name() -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    fn fields_ref(&self, func: &mut dyn FnMut(&[FieldRef])) {
        self.0.deref().fields_ref(func)
    }

    fn fields_mut(&mut self, func: &mut dyn FnMut(&mut [FieldMut])) {
        self.0.deref_mut().fields_mut(func)
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        Reflect::into_any(self.0)
    }

    fn as_any(&self, func: &mut dyn FnMut(&dyn Any)) {
        Reflect::as_any(self.0.deref(), func)
    }

    fn as_any_mut(&mut self, func: &mut dyn FnMut(&mut dyn Any)) {
        Reflect::as_any_mut(self.0.deref_mut(), func)
    }

    fn as_reflect(&self, func: &mut dyn FnMut(&dyn Reflect)) {
        self.0.deref().as_reflect(func)
    }

    fn as_reflect_mut(&mut self, func: &mut dyn FnMut(&mut dyn Reflect)) {
        self.0.deref_mut().as_reflect_mut(func)
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
        self.0.deref_mut().set(value)
    }

    fn set_field(
        &mut self,
        field: &str,
        value: Box<dyn Reflect>,
        func: &mut dyn FnMut(Result<Box<dyn Reflect>, SetFieldError>),
    ) {
        self.0.deref_mut().set_field(field, value, func)
    }

    fn field(&self, name: &str, func: &mut dyn FnMut(Option<&dyn Reflect>)) {
        self.0.deref().field(name, func)
    }

    fn field_mut(&mut self, name: &str, func: &mut dyn FnMut(Option<&mut dyn Reflect>)) {
        self.0.deref_mut().field_mut(name, func)
    }

    fn try_clone_box(&self) -> Option<Box<dyn Reflect>> {
        Some(Box::new(self.clone()))
    }
}
