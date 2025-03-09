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

//! Screen is a widget that always has the size of the screen of the UI in which it is used. See
//! docs for [`Screen`] for more info and usage examples.

#![warn(missing_docs)]

use crate::{
    core::{
        algebra::Vector2, math::Rect, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        uuid_provider, visitor::prelude::*,
    },
    message::UiMessage,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UiNode, UserInterface,
};

use fyrox_graph::constructor::{ConstructorProvider, GraphNodeConstructor};
use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
};

/// Screen is a widget that always has the size of the screen of the UI in which it is used. It is
/// main use case is to provide automatic layout functionality, that will always provide screen size
/// to its children widgets. This is needed, because the root node of any UI is [`crate::canvas::Canvas`]
/// which provides infinite bounds as a layout constraint, thus making it impossible for automatic
/// fitting to the current screen size. For example, Screen widget could be used as a root node for
/// [`crate::grid::Grid`] widget - in this case the grid instance will always have the size of the
/// screen and will automatically shrink or expand when the screen size changes. It is ideal choice if
/// you want to have some widgets always centered on screen (for example - crosshair, main menu of
/// your game, etc.).
///
/// ## Example
///
/// The following examples creates a simple main menu of a game with just two buttons. The buttons
/// will always be centered in the current screen bounds.
///
/// ```rust
/// use fyrox_ui::{
///     core::pool::Handle,
///     button::ButtonBuilder,
///     grid::{Column, GridBuilder, Row},
///     screen::ScreenBuilder,
///     stack_panel::StackPanelBuilder,
///     widget::WidgetBuilder,
///     BuildContext, UiNode,
/// };
///
/// fn create_always_centered_game_menu(ctx: &mut BuildContext) -> Handle<UiNode> {
///     // Screen widget will provide current screen size to its Grid widget as a layout constraint,
///     // thus making it fit to the current screen bounds.
///     ScreenBuilder::new(
///         WidgetBuilder::new().with_child(
///             GridBuilder::new(
///                 WidgetBuilder::new()
///                     .with_width(300.0)
///                     .with_height(400.0)
///                     .with_child(
///                         // Buttons will be stacked one on top of another.
///                         StackPanelBuilder::new(
///                             WidgetBuilder::new()
///                                 .on_row(1)
///                                 .on_column(1)
///                                 .with_child(
///                                     ButtonBuilder::new(WidgetBuilder::new())
///                                         .with_text("New Game")
///                                         .build(ctx),
///                                 )
///                                 .with_child(
///                                     ButtonBuilder::new(WidgetBuilder::new())
///                                         .with_text("Exit")
///                                         .build(ctx),
///                                 ),
///                         )
///                         .build(ctx),
///                     ),
///             )
///             // Split the grid into 3 rows and 3 columns. The center cell contain the stack panel
///             // instance, that basically stacks main menu buttons one on top of another. The center
///             // cell will also be always centered in screen bounds.
///             .add_row(Row::stretch())
///             .add_row(Row::auto())
///             .add_row(Row::stretch())
///             .add_column(Column::stretch())
///             .add_column(Column::auto())
///             .add_column(Column::stretch())
///             .build(ctx),
///         ),
///     )
///     .build(ctx)
/// }
/// ```
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct Screen {
    /// Base widget of the screen.
    pub widget: Widget,
    /// Last screen size.
    #[visit(skip)]
    #[reflect(hidden)]
    pub last_screen_size: Cell<Vector2<f32>>,
}

impl ConstructorProvider<UiNode, UserInterface> for Screen {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>()
            .with_variant("Screen", |ui| {
                ScreenBuilder::new(WidgetBuilder::new().with_name("Screen"))
                    .build(&mut ui.build_ctx())
                    .into()
            })
            .with_group("Layout")
    }
}

crate::define_widget_deref!(Screen);

uuid_provider!(Screen = "3bc7649f-a1ba-49be-bc4e-e0624654e40c");

impl Control for Screen {
    fn measure_override(&self, ui: &UserInterface, _available_size: Vector2<f32>) -> Vector2<f32> {
        for &child in self.children.iter() {
            ui.measure_node(child, ui.screen_size());
        }

        ui.screen_size()
    }

    fn arrange_override(&self, ui: &UserInterface, _final_size: Vector2<f32>) -> Vector2<f32> {
        let final_rect = Rect::new(0.0, 0.0, ui.screen_size().x, ui.screen_size().y);

        for &child in self.children.iter() {
            ui.arrange_node(child, &final_rect);
        }

        ui.screen_size()
    }

    fn update(&mut self, _dt: f32, ui: &mut UserInterface) {
        if self.last_screen_size.get() != ui.screen_size {
            self.invalidate_layout();
            self.last_screen_size.set(ui.screen_size);
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);
    }
}

/// Screen builder creates instances of [`Screen`] widgets and adds them to the user interface.
pub struct ScreenBuilder {
    widget_builder: WidgetBuilder,
}

impl ScreenBuilder {
    /// Creates a new instance of the screen builder.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self { widget_builder }
    }

    /// Finishes building a [`Screen`] widget instance and adds it to the user interface, returning a
    /// handle to the instance.
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let screen = Screen {
            widget: self.widget_builder.with_need_update(true).build(ctx),
            last_screen_size: Cell::new(Default::default()),
        };
        ctx.add_node(UiNode::new(screen))
    }
}

#[cfg(test)]
mod test {
    use crate::screen::ScreenBuilder;
    use crate::{test::test_widget_deletion, widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| ScreenBuilder::new(WidgetBuilder::new()).build(ctx));
    }
}
