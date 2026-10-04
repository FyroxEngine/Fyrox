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
    border::BorderBuilder,
    brush::Brush,
    core::{algebra::Vector2, color::Color, err, pool::Handle},
    formatted_text::WrapMode,
    grid::{Column, GridBuilder, Row},
    stack_panel::StackPanelBuilder,
    style::{resource::StyleResourceExt, Style, StyledProperty},
    text::{TextBuilder, TextMessage},
    vector_image::{Primitive, VectorImageBuilder},
    widget::{WidgetBuilder, WidgetMessage},
    BuildContext, Thickness, UiNode, UserInterface, VerticalAlignment,
};

fn heading_depth_to_font_size(heading_depth: u8) -> StyledProperty<f32> {
    match heading_depth {
        1 => 24.0,
        2 => 22.0,
        3 => 20.0,
        4 => 18.0,
        5 => 16.0,
        _ => 14.0,
    }
    .into()
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
                    .with_font_size(heading_depth_to_font_size(heading_depth))
                    .with_text(text)
                    .with_wrap(wrap_mode)
                    .build(ctx),
            ),
    )
    .with_corner_radius(5.0f32.into())
    .build(ctx)
    .to_base()
}

fn make_list_item(widget_builder: WidgetBuilder, ctx: &mut BuildContext) -> Handle<UiNode> {
    let bullet = VectorImageBuilder::new(
        WidgetBuilder::new()
            .with_vertical_alignment(VerticalAlignment::Top)
            .on_row(0)
            .on_column(0)
            .with_width(16.0)
            .with_height(16.0),
    )
    .with_primitives(vec![Primitive::Circle {
        center: Vector2::new(8.0, 8.0),
        radius: 4.0,
        segments: 16,
    }])
    .build(ctx);
    let mut columns = Vec::with_capacity(widget_builder.children.capacity());
    for (i, child) in widget_builder.children.iter().enumerate() {
        ctx[*child].column.set_value_and_mark_modified(i + 1);
        columns.push(Column::stretch())
    }
    GridBuilder::new(widget_builder.with_child(bullet))
        .add_column(Column::auto())
        .add_columns(columns)
        .add_row(Row::stretch())
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
        let prev_heading_depth = *heading_depth;

        match ast_node {
            Node::Paragraph(_)
            | Node::Link(_)
            | Node::Heading(_)
            | Node::Strong(_)
            | Node::Emphasis(_) => {
                let mut widget_builder = WidgetBuilder::new();

                let default_font_resource = ui.default_font.clone();
                let mut font = default_font_resource.clone();
                let default_font = default_font_resource.data_ref();

                match ast_node {
                    Node::Heading(heading) => {
                        *heading_depth = heading.depth;
                        widget_builder = widget_builder.with_margin(Thickness::top_bottom(8.0));
                    }
                    Node::Link(_) => {
                        // TODO: Add navigation for links.
                        widget_builder = widget_builder
                            .with_foreground(Brush::Solid(Color::DEEP_SKY_BLUE).into());
                    }
                    Node::Strong(_) => {
                        font = default_font
                            .bold
                            .clone()
                            .unwrap_or_else(|| default_font_resource.clone());
                    }
                    Node::Emphasis(_) => {
                        font = default_font
                            .italic
                            .clone()
                            .unwrap_or_else(|| default_font_resource.clone());
                    }
                    _ => {}
                }
                drop(default_font);

                let paragraph_text = TextBuilder::new(widget_builder)
                    .with_font_size(heading_depth_to_font_size(*heading_depth))
                    .with_font(font)
                    .with_wrap(WrapMode::Word)
                    .with_baseline_alignment(VerticalAlignment::Center)
                    .build(&mut ui.build_ctx())
                    .to_base();

                let mut full_text = String::new();

                if let Some(children) = ast_node.children() {
                    for child_ast_node in children {
                        if let Node::Text(text) = child_ast_node {
                            for mut ch in text.value.chars() {
                                if ch == '\n' {
                                    ch = ' ';
                                }
                                full_text.push(ch);
                            }
                        } else {
                            let child_widget =
                                traverse_ast_recursively(child_ast_node, heading_depth, ui);
                            let child_widget_ref = &mut ui[child_widget];
                            child_widget_ref.set_column(full_text.chars().count());
                            child_widget_ref.set_vertical_alignment(VerticalAlignment::Center);
                            ui.send(child_widget, WidgetMessage::LinkWith(paragraph_text));
                        }
                    }
                }

                *heading_depth = prev_heading_depth;

                ui.send(paragraph_text, TextMessage::Text(full_text));

                paragraph_text
            }
            ast_node => {
                let mut widget_builder = WidgetBuilder::new();

                if let Some(children) = ast_node.children() {
                    for child_ast_node in children {
                        let child_widget =
                            traverse_ast_recursively(child_ast_node, heading_depth, ui);
                        widget_builder = widget_builder.with_child(child_widget);
                    }
                }

                let ctx = &mut ui.build_ctx();

                match ast_node {
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
                    Node::ListItem(_) => {
                        // TODO: Support checked items.
                        make_list_item(widget_builder, ctx)
                    }
                    _ => StackPanelBuilder::new(widget_builder)
                        .build(ctx)
                        .to_base::<UiNode>(),
                }
            }
        }
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
