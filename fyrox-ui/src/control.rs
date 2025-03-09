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

use crate::{
    core::{algebra::Vector2, pool::Handle, reflect::Reflect, uuid::Uuid, visitor::Visit},
    core::{ComponentProvider, TypeUuidProvider},
    draw::DrawingContext,
    message::{OsEvent, UiMessage},
    widget::Widget,
    UiNode, UserInterface,
};
use fyrox_core::define_as_any_trait;

use std::{
    any::Any,
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

define_as_any_trait!(ControlAsAny => BaseControl);

/// Base trait for all UI widgets. It has auto-impl and you don't need to implement it manually. Your widget
/// must implement [`Clone`] and [`Control`] traits for impl to be generated for you, also your widget must
/// not contain any references (due to `'static` lifetime requirement).
pub trait BaseControl: Send + ControlAsAny {
    /// Returns the exact copy of the widget in "type-erased" form.
    fn clone_boxed(&self) -> Box<dyn Control>;

    /// Returns type name of the widget.
    fn type_name(&self) -> &'static str;

    fn id(&self) -> Uuid;

    /// Returns total amount of memory used by this widget (in bytes), in other words it returns
    /// `size_of::<WidgetType>()`.
    fn self_size(&self) -> usize;
}

impl<T> BaseControl for T
where
    T: Any + Clone + 'static + Control + TypeUuidProvider,
{
    fn clone_boxed(&self) -> Box<dyn Control> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn self_size(&self) -> usize {
        size_of::<T>()
    }
}

/// Trait for all UI controls in library.
pub trait Control:
    BaseControl + Deref<Target = Widget> + DerefMut + Reflect + Visit + ComponentProvider
{
    /// This method will be called before the widget is destroyed (dropped). At the moment, when this
    /// method is called, the widget is still in the widget graph and can be accessed via handles. It
    /// is guaranteed to be called once, and only if the widget is deleted via [`crate::widget::WidgetMessage::remove`].
    fn on_remove(&self, #[allow(unused_variables)] sender: &Sender<UiMessage>) {}

    /// This method is used to override measurement step of the layout system. It should return desired size of
    /// the widget (how many space it wants to occupy).
    ///
    /// ## Example
    ///
    /// ```rust
    /// # use fyrox_ui::{
    /// #     core::algebra::Vector2, define_widget_deref, message::UiMessage, Control, UserInterface,
    /// #     core::{visitor::prelude::*, reflect::prelude::*, type_traits::prelude::*,},
    /// #     widget::Widget, UiNode
    /// # };
    /// # use std::{
    /// #     any::{Any, TypeId},
    /// #     ops::{Deref, DerefMut},
    /// # };
    /// # use fyrox_core::uuid_provider;
    /// # use fyrox_graph::BaseSceneGraph;
    /// #
    /// #[derive(Clone, Visit, Reflect, Debug, ComponentProvider)]
    /// #[reflect(derived_type = "UiNode")]
    /// struct MyWidget {
    ///     widget: Widget,
    /// }
    /// #
    /// # define_widget_deref!(MyWidget);
    /// # uuid_provider!(MyWidget = "a93ec1b5-e7c8-4919-ac19-687d8c99f6bd");
    /// impl Control for MyWidget {
    ///     fn measure_override(
    ///         &self,
    ///         ui: &UserInterface,
    ///         available_size: Vector2<f32>,
    ///     ) -> Vector2<f32> {
    ///         let mut size: Vector2<f32> = Vector2::default();
    ///
    ///         // Measure children nodes and find the largest size of them.
    ///         for &child in self.children.iter() {
    ///             // Recursively measure children nodes. Measured size will be put in `desired_size`
    ///             // of the widget.
    ///             ui.measure_node(child, available_size);
    ///
    ///             // Find max size across all the children widgets.
    ///             size = size.sup(&ui.node(child).desired_size());
    ///         }
    ///
    ///         size
    ///     }
    ///     #
    ///     # fn handle_routed_message(&mut self, _ui: &mut UserInterface, _message: &mut UiMessage) {
    ///     #     todo!()
    ///     # }
    /// }
    /// ```
    ///
    /// The goal of this method is to supply the UI system with the size requirements of all descendants
    /// of the widget. In this example we measure all descendants recursively and finding the max desired
    /// size of across all the children widgets. This effectively does the following: size of this widget
    /// will be the max size of children widgets. Some widgets (like [`crate::canvas::Canvas`]), can provide infinite
    /// constraints to children nodes, to fetch unconstrained desired size.
    ///
    /// It is recommended to check implementation of this method of built-in widgets (such as [`crate::canvas::Canvas`],
    /// [`crate::stack_panel::StackPanel`], [`crate::wrap_panel::WrapPanel`], [`crate::grid::Grid`]). It should help you to
    /// understand measurement step better.
    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        self.deref().measure_override(ui, available_size)
    }

    /// This method is used to override arrangement step of the layout system. Arrangement step is used to
    /// commit the final location and size of the widget in local coordinates. It is done after the measurement
    /// step; when all desired sizes of every widget is known. This fact allows you to calculate final location
    /// and size of every child widget, based in their desired size. Usually this method is used in some panel
    /// widgets, that takes their children and arranges them in some specific way. For example, it may stack
    /// widgets on top of each other, or put them in a line with wrapping, etc.
    ///
    /// ## Example
    ///
    /// ```rust
    /// # use fyrox_ui::{
    /// #     core::{algebra::Vector2, math::Rect},
    /// #     core::{visitor::prelude::*, reflect::prelude::*, type_traits::prelude::*,},
    /// #     define_widget_deref,
    /// #     message::UiMessage,
    /// #     Control, UserInterface, widget::Widget, UiNode
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
    /// # uuid_provider!(MyWidget = "a93ec1b5-e7c8-4919-ac19-687d8c99f6bd");
    /// impl Control for MyWidget {
    ///     fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
    ///         let final_rect = Rect::new(0.0, 0.0, final_size.x, final_size.y);
    ///
    ///         // Commit final locations and size for each child node.
    ///         for &child in self.children.iter() {
    ///             ui.arrange_node(child, &final_rect);
    ///         }
    ///
    ///         final_size
    ///     }
    ///     #
    ///     # fn handle_routed_message(&mut self, _ui: &mut UserInterface, _message: &mut UiMessage) {
    ///     #     todo!()
    ///     # }
    /// }
    /// ```
    ///
    /// This example arranges all the children widgets using the given `final_size`, that comes from the
    /// parent widget, so all children will have exactly the same size as the parent and be located at (0;0)
    /// point in local coordinates.
    ///
    /// It is recommended to check implementation of this method of built-in widgets (such as [`crate::canvas::Canvas`],
    /// [`crate::stack_panel::StackPanel`], [`crate::wrap_panel::WrapPanel`], [`crate::grid::Grid`]). It should help you to
    /// understand arrangement step better.
    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        self.deref().arrange_override(ui, final_size)
    }

    /// This method is used to emit drawing commands that will be used later to draw your widget on screen.
    /// Keep in mind that any emitted geometry (quads, lines, text, etc), will be used to perform hit test.
    /// In other words, all the emitted geometry will make your widget "clickable". Widgets with no geometry
    /// emitted by this method are mouse input transparent.
    ///
    /// ## Example
    ///
    /// ```rust
    /// # use fyrox_ui::{
    /// #     define_widget_deref,
    /// #     draw::{CommandTexture, Draw, DrawingContext},
    /// #     core::{visitor::prelude::*, reflect::prelude::*, type_traits::prelude::*,},
    /// #     message::UiMessage,
    /// #     Control, UserInterface, widget::Widget, UiNode
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
    /// # uuid_provider!(MyWidget = "a93ec1b5-e7c8-4919-ac19-687d8c99f6bd");
    /// impl Control for MyWidget {
    /// fn draw(&self, drawing_context: &mut DrawingContext) {
    ///     let bounds = self.widget.bounding_rect();
    ///
    ///     // Push a rect.
    ///     drawing_context.push_rect_filled(&bounds, None);
    ///
    ///     // Commit the geometry, it is mandatory step, otherwise your widget's geometry
    ///     // will be "attached" to some other widget that will call `commit`.
    ///     drawing_context.commit(
    ///         self.clip_bounds(),
    ///         self.widget.background(),
    ///         CommandTexture::None,
    ///         None,
    ///     );
    /// }
    ///     #
    ///     # fn handle_routed_message(&mut self, _ui: &mut UserInterface, _message: &mut UiMessage) {
    ///     #     todo!()
    ///     # }
    /// }
    /// ```
    ///
    /// This example shows how to draw a simple quad using the background brush of the widget. See docs
    /// for [`DrawingContext`] for more info.
    fn draw(&self, #[allow(unused_variables)] drawing_context: &mut DrawingContext) {}

    fn on_visual_transform_changed(&self) {}

    /// The same as [`Self::draw`], but it runs after all descendant widgets are rendered.
    fn post_draw(&self, #[allow(unused_variables)] drawing_context: &mut DrawingContext) {}

    /// This method is called every frame and can be used to update internal variables of the widget, that
    /// can be used to animated your widget. Its main difference from other methods, is that it does **not**
    /// provide access to any other widget in the UI. Instead, you can only send messages to widgets to
    /// force them to change their state.
    ///
    /// ## Important notes
    ///
    /// Due to performance reasons, you **must** set `.with_need_update(true)` in widget builder to
    /// force library to call `update` method!
    fn update(
        &mut self,
        #[allow(unused_variables)] dt: f32,
        #[allow(unused_variables)] ui: &mut UserInterface,
    ) {
    }

    /// Performs event-specific actions. Must call widget.handle_message()!
    ///
    /// # Notes
    ///
    /// Do *not* try to borrow node by `self_handle` in UI - at this moment node has been moved
    /// out of pool and attempt of borrowing will cause panic! `self_handle` should be used only
    /// to check if event came from/for this node or to capture input on node.
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage);

    /// Used to react to a message (by producing another message) that was posted outside of current
    /// hierarchy. In other words this method is used when you need to "peek" a message before it'll
    /// be passed into bubbling router. Most common use case is to catch messages from popups: popup
    /// in 99.9% cases is a child of root canvas and it **won't** receive a message from a its *logical*
    /// parent during bubbling message routing. For example `preview_message` used in a dropdown list:
    /// dropdown list has two separate parts - a field with selected value and a popup for all possible
    /// options. Visual parent of the popup in this case is the root canvas, but logical parent is the
    /// dropdown list. Because of this fact, the field won't receive any messages from popup, to solve
    /// this we use `preview_message`. This method is much more restrictive - it does not allow you to
    /// modify a node and ui, you can either *request* changes by sending a message or use internal
    /// mutability (`Cell`, `RefCell`, etc).
    ///
    /// ## Important notes
    ///
    /// Due to performance reasons, you **must** set `.with_preview_messages(true)` in widget builder to
    /// force library to call `preview_message`!
    ///
    /// The order of execution of this method is undefined! There is no guarantee that it will be called
    /// hierarchically as widgets connected.
    fn preview_message(
        &self,
        #[allow(unused_variables)] ui: &UserInterface,
        #[allow(unused_variables)] message: &mut UiMessage,
    ) {
        // This method is optional.
    }

    /// Provides a way to respond to OS specific events. Can be useful to detect if a key or mouse
    /// button was pressed. This method significantly differs from `handle_message` because os events
    /// are not dispatched - they'll be passed to this method in any case.
    ///
    /// ## Important notes
    ///
    /// Due to performance reasons, you **must** set `.with_handle_os_messages(true)` in widget builder to
    /// force library to call `handle_os_event`!
    fn handle_os_event(
        &mut self,
        #[allow(unused_variables)] self_handle: Handle<UiNode>,
        #[allow(unused_variables)] ui: &mut UserInterface,
        #[allow(unused_variables)] event: &OsEvent,
    ) {
    }
}
