use crate::{make_button, utils::make_dropdown_list_option};
use fyrox::{
    core::pool::Handle,
    gui::{
        dropdown_list::DropdownListBuilder,
        grid::{Column, GridBuilder, Row},
        message::MessageDirection,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        text_box::TextBoxBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, VerticalAlignment,
    },
};

pub struct ProjectWizard {
    pub window: Handle<UiNode>,
}

fn make_text(text: &str, row: usize, ctx: &mut BuildContext) -> Handle<UiNode> {
    TextBuilder::new(
        WidgetBuilder::new()
            .with_vertical_alignment(VerticalAlignment::Center)
            .with_margin(Thickness {
                left: 5.0,
                top: 1.0,
                right: 1.0,
                bottom: 1.0,
            })
            .on_row(row),
    )
    .with_text(text)
    .build(ctx)
}

impl ProjectWizard {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let name = TextBoxBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .on_row(0)
                .on_column(1),
        )
        .with_text("MyProject")
        .build(ctx);

        let style = DropdownListBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .with_height(22.0)
                .on_row(1)
                .on_column(1),
        )
        .with_items(vec![
            make_dropdown_list_option(ctx, "2D"),
            make_dropdown_list_option(ctx, "3D"),
        ])
        .with_selected(1)
        .build(ctx);

        let vcs = DropdownListBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .with_height(22.0)
                .on_row(2)
                .on_column(1),
        )
        .with_items(vec![
            make_dropdown_list_option(ctx, "None"),
            make_dropdown_list_option(ctx, "Git"),
            make_dropdown_list_option(ctx, "Mercurial"),
            make_dropdown_list_option(ctx, "Pijul"),
            make_dropdown_list_option(ctx, "Fossil"),
        ])
        .with_selected(1)
        .build(ctx);

        let ok = make_button("Create", 100.0, 22.0, 0, ctx);
        let cancel = make_button("Cancel", 100.0, 22.0, 0, ctx);
        let buttons = StackPanelBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .with_horizontal_alignment(HorizontalAlignment::Right)
                .with_child(ok)
                .with_child(cancel),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(0)
                .with_child(make_text("Name", 0, ctx))
                .with_child(name)
                .with_child(make_text("Style", 1, ctx))
                .with_child(style)
                .with_child(make_text("Version Control", 2, ctx))
                .with_child(vcs),
        )
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_column(Column::strict(120.0))
        .add_column(Column::stretch())
        .build(ctx);

        let outer_grid =
            GridBuilder::new(WidgetBuilder::new().with_child(grid).with_child(buttons))
                .add_row(Row::stretch())
                .add_row(Row::auto())
                .add_column(Column::auto())
                .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(130.0))
            .with_content(outer_grid)
            .open(false)
            .with_title(WindowTitle::text("Project Wizard"))
            .build(ctx);

        ctx.sender()
            .send(WindowMessage::open_modal(
                window,
                MessageDirection::ToWidget,
                true,
                true,
            ))
            .unwrap();

        Self { window }
    }
}
