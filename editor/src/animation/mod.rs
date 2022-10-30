#![allow(dead_code)] // TODO

use fyrox::{
    core::pool::Handle,
    gui::{
        button::ButtonBuilder,
        curve::CurveEditorBuilder,
        grid::{Column, GridBuilder, Row},
        list_view::ListViewBuilder,
        menu::{MenuBuilder, MenuItemBuilder, MenuItemContent},
        message::MessageDirection,
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, Orientation, Thickness, UiNode, UserInterface,
    },
    resource::animation::AnimationResource,
};

struct Menu {
    menu: Handle<UiNode>,
    new: Handle<UiNode>,
    exit: Handle<UiNode>,
    undo: Handle<UiNode>,
    redo: Handle<UiNode>,
}

impl Menu {
    fn new(ctx: &mut BuildContext) -> Self {
        let new;
        let exit;
        let undo;
        let redo;
        let menu = MenuBuilder::new(WidgetBuilder::new().on_row(0).on_column(0))
            .with_items(vec![
                MenuItemBuilder::new(WidgetBuilder::new())
                    .with_content(MenuItemContent::text_no_arrow("File"))
                    .with_items(vec![
                        {
                            new = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text_no_arrow("New"))
                                .build(ctx);
                            new
                        },
                        {
                            exit = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text_no_arrow("Exit"))
                                .build(ctx);
                            exit
                        },
                    ])
                    .build(ctx),
                MenuItemBuilder::new(WidgetBuilder::new())
                    .with_content(MenuItemContent::text_no_arrow("Edit"))
                    .with_items(vec![
                        {
                            undo = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text_no_arrow("Undo"))
                                .build(ctx);
                            undo
                        },
                        {
                            redo = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text_no_arrow("Redo"))
                                .build(ctx);
                            redo
                        },
                    ])
                    .build(ctx),
            ])
            .build(ctx);

        Self {
            menu,
            new,
            exit,
            undo,
            redo,
        }
    }
}

struct TrackList {
    panel: Handle<UiNode>,
    list: Handle<UiNode>,
    add_track: Handle<UiNode>,
}

impl TrackList {
    fn new(ctx: &mut BuildContext) -> Self {
        let list;
        let add_track;

        let panel = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    list = ListViewBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(0)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .build(ctx);
                    list
                })
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .on_row(1)
                            .on_column(0)
                            .with_margin(Thickness::uniform(1.0))
                            .with_child({
                                add_track = ButtonBuilder::new(WidgetBuilder::new())
                                    .with_text("Add Track..")
                                    .build(ctx);
                                add_track
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                ),
        )
        .add_row(Row::stretch())
        .add_row(Row::strict(22.0))
        .add_column(Column::stretch())
        .build(ctx);

        Self {
            panel,
            list,
            add_track,
        }
    }
}

pub struct AnimationEditor {
    pub window: Handle<UiNode>,
    track_list: TrackList,
    curve_editor: Handle<UiNode>,
    resource: Option<AnimationResource>,
    menu: Menu,
}

impl AnimationEditor {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let curve_editor;

        let menu = Menu::new(ctx);
        let track_list = TrackList::new(ctx);

        let payload = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .on_column(0)
                .with_child(track_list.panel)
                .with_child({
                    curve_editor = CurveEditorBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(1)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .build(ctx);
                    curve_editor
                }),
        )
        .add_row(Row::stretch())
        .add_column(Column::strict(250.0))
        .add_column(Column::stretch())
        .build(ctx);

        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(menu.menu)
                .with_child(payload),
        )
        .add_row(Row::strict(22.0))
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(600.0).with_height(500.0))
            .with_content(content)
            .open(false)
            .with_title(WindowTitle::text("Animation Editor"))
            .build(ctx);

        Self {
            window,
            track_list,
            curve_editor,
            resource: None,
            menu,
        }
    }

    pub fn open(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::open(
            self.window,
            MessageDirection::ToWidget,
            true,
        ));
    }
}
