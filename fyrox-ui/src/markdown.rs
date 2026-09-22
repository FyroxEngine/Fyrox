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

use crate::wrap_panel::WrapPanelBuilder;
use crate::{
    border::BorderBuilder,
    core::{err, pool::Handle},
    formatted_text::WrapMode,
    stack_panel::StackPanelBuilder,
    style::{resource::StyleResourceExt, Style},
    text::TextBuilder,
    widget::WidgetBuilder,
    BuildContext, Orientation, Thickness, UiNode, UserInterface,
};

fn heading_depth_to_font_size(heading_depth: u8) -> f32 {
    match heading_depth {
        1 => 24.0,
        2 => 22.0,
        3 => 20.0,
        4 => 18.0,
        5 => 16.0,
        _ => 14.0,
    }
}

fn make_text_with_border(
    widget_builder: WidgetBuilder,
    heading_depth: u8,
    text: &str,
    wrap_mode: WrapMode,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    BorderBuilder::new(
        widget_builder
            .with_background(ctx.style.property(Style::BRUSH_DARK))
            .with_child(
                TextBuilder::new(WidgetBuilder::new())
                    .with_font_size(heading_depth_to_font_size(heading_depth).into())
                    .with_text(text)
                    .with_wrap(wrap_mode)
                    .build(ctx),
            ),
    )
    .with_corner_radius(10.0f32.into())
    .build(ctx)
    .to_base()
}

pub fn markdown_to_visual_tree(ui: &mut UserInterface, text: impl AsRef<str>) -> Handle<UiNode> {
    use markdown::{mdast::Node, *};

    fn traverse_ast_recursively(
        ast_node: &Node,
        heading_depth: &mut u8,
        ui: &mut UserInterface,
    ) -> Handle<UiNode> {
        let mut widget_builder = WidgetBuilder::new();

        let prev_heading_depth = *heading_depth;

        match ast_node {
            Node::Paragraph(_) => {
                widget_builder = widget_builder.with_margin(Thickness::top(16.0));
            }
            Node::Heading(heading) => {
                *heading_depth = heading.depth;
            }
            _ => (),
        }

        if let Some(children) = ast_node.children() {
            for child_ast_node in children {
                let child_widget = traverse_ast_recursively(child_ast_node, heading_depth, ui);
                widget_builder = widget_builder.with_child(child_widget);
            }
        }

        *heading_depth = prev_heading_depth;

        let ctx = &mut ui.build_ctx();

        let widget = match ast_node {
            Node::Text(text) => TextBuilder::new(widget_builder)
                .with_text(&text.value)
                .with_font_size(heading_depth_to_font_size(*heading_depth).into())
                .with_wrap(WrapMode::Word)
                .build(ctx)
                .to_base(),
            Node::InlineCode(inline_code) => make_text_with_border(
                widget_builder,
                *heading_depth,
                &inline_code.value,
                WrapMode::NoWrap,
                ctx,
            ),
            Node::Code(code) => make_text_with_border(
                widget_builder,
                *heading_depth,
                &code.value,
                WrapMode::Word,
                ctx,
            ),
            Node::Paragraph(_) => WrapPanelBuilder::new(widget_builder)
                .with_orientation(Orientation::Horizontal)
                .build(ctx)
                .to_base(),
            _ => StackPanelBuilder::new(widget_builder)
                .build(ctx)
                .to_base::<UiNode>(),
        };

        widget
    }

    let text = text.as_ref();

    match to_mdast(text, &Default::default()) {
        Ok(root) => {
            let mut heading_depth = 0;
            return traverse_ast_recursively(&root, &mut heading_depth, ui);
        }
        Err(error) => {
            err!("Unable to apply markdown text. Reason: {error}");
        }
    }

    Handle::NONE
}
