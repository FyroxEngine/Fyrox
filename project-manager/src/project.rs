use crate::{
    make_button,
    settings::{Project, Settings},
    utils::make_dropdown_list_option,
};
use fyrox::{
    core::pool::Handle,
    gui::{
        button::ButtonMessage,
        dropdown_list::{DropdownListBuilder, DropdownListMessage},
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        path::{PathEditorBuilder, PathEditorMessage},
        stack_panel::StackPanelBuilder,
        text::{TextBuilder, TextMessage},
        text_box::TextBoxBuilder,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
        VerticalAlignment,
    },
};
use std::path::PathBuf;

enum Style {
    TwoD,
    ThreeD,
}

impl Style {
    fn from_index(index: usize) -> Self {
        match index {
            0 => Self::TwoD,
            1 => Self::ThreeD,
            _ => unreachable!(),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Style::TwoD => "2d",
            Style::ThreeD => "3d",
        }
    }
}

enum Vcs {
    None,
    Git,
    Mercurial,
    Pijul,
    Fossil,
}

impl Vcs {
    fn from_index(index: usize) -> Self {
        match index {
            0 => Self::None,
            1 => Self::Git,
            2 => Self::Mercurial,
            3 => Self::Pijul,
            4 => Self::Fossil,
            _ => unreachable!(),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Vcs::None => "none",
            Vcs::Git => "git",
            Vcs::Mercurial => "hg",
            Vcs::Pijul => "pijul",
            Vcs::Fossil => "fossil",
        }
    }
}

pub struct ProjectWizard {
    pub window: Handle<UiNode>,
    create: Handle<UiNode>,
    cancel: Handle<UiNode>,
    path_field: Handle<UiNode>,
    name_field: Handle<UiNode>,
    style_field: Handle<UiNode>,
    vcs_field: Handle<UiNode>,
    name: String,
    style: Style,
    vcs: Vcs,
    path: PathBuf,
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
        let path_field = PathEditorBuilder::new(
            WidgetBuilder::new()
                .with_height(22.0)
                .with_margin(Thickness::uniform(1.0))
                .on_row(0)
                .on_column(1),
        )
        .with_path("./")
        .build(ctx);

        let name_field = TextBoxBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .on_row(1)
                .on_column(1),
        )
        .with_text("MyProject")
        .build(ctx);

        let style_field = DropdownListBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .with_height(22.0)
                .on_row(2)
                .on_column(1),
        )
        .with_items(vec![
            make_dropdown_list_option(ctx, "2D"),
            make_dropdown_list_option(ctx, "3D"),
        ])
        .with_selected(1)
        .build(ctx);

        let vcs_field = DropdownListBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .with_height(22.0)
                .on_row(3)
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

        let create = make_button("Create", 100.0, 22.0, 0, ctx);
        let cancel = make_button("Cancel", 100.0, 22.0, 0, ctx);
        let buttons = StackPanelBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .with_horizontal_alignment(HorizontalAlignment::Right)
                .with_child(create)
                .with_child(cancel),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(0)
                .with_child(make_text("Path", 0, ctx))
                .with_child(path_field)
                .with_child(make_text("Name", 1, ctx))
                .with_child(name_field)
                .with_child(make_text("Style", 2, ctx))
                .with_child(style_field)
                .with_child(make_text("Version Control", 3, ctx))
                .with_child(vcs_field),
        )
        .add_row(Row::auto())
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

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(160.0))
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

        Self {
            window,
            name: "MyProject".to_string(),
            style: Style::ThreeD,
            vcs: Vcs::Git,
            create,
            cancel,
            path_field,
            name_field,
            style_field,
            vcs_field,
            path: Default::default(),
        }
    }

    fn close_and_remove(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::close(
            self.window,
            MessageDirection::ToWidget,
        ));
        ui.send_message(WidgetMessage::remove(
            self.window,
            MessageDirection::ToWidget,
        ));
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        ui: &UserInterface,
        settings: &mut Settings,
    ) -> bool {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.create {
                let _ = fyrox_template_core::init_project(
                    &self.path,
                    &self.name,
                    self.style.as_str(),
                    self.vcs.as_str(),
                    true,
                );
                let manifest_path = self
                    .path
                    .join(&self.name)
                    .join("Cargo.toml")
                    .canonicalize()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                // Remove "\\?\" prefix on Windows, otherwise it will be impossible to compile anything,
                // because there are some quirks on Unicode path handling on Windows and any path starting
                // from two slashes will not work correctly as a working directory for a child process.
                let manifest_path = manifest_path.replace(r"\\?\", r"");
                settings.projects.push(Project {
                    manifest_path: manifest_path.into(),
                    name: self.name.clone(),
                    hot_reload: false,
                });
                self.close_and_remove(ui);
                return true;
            } else if message.destination() == self.cancel {
                self.close_and_remove(ui);
            }
        } else if let Some(TextMessage::Text(text)) = message.data() {
            if message.direction() == MessageDirection::FromWidget
                && message.destination() == self.name_field
            {
                self.name.clone_from(text);
            }
        } else if let Some(DropdownListMessage::SelectionChanged(Some(index))) = message.data() {
            if message.direction() == MessageDirection::FromWidget {
                if message.destination() == self.style_field {
                    self.style = Style::from_index(*index);
                } else if message.destination() == self.vcs_field {
                    self.vcs = Vcs::from_index(*index);
                }
            }
        } else if let Some(PathEditorMessage::Path(path)) = message.data() {
            if message.destination() == self.path_field
                && message.direction() == MessageDirection::FromWidget
            {
                self.path.clone_from(path);
            }
        }
        false
    }
}
