use crate::{
    utils::pool::Handle,
    gui::{
        border::BorderBuilder,
        node::{UINode, UINodeKind},
        builder::{CommonBuilderFields, GenericNodeBuilder},
        UserInterface,
        draw::Color,
        grid::{GridBuilder, Column, Row},
        HorizontalAlignment,
        event::RoutedEventHandlerType,
        text::TextBuilder,
        Thickness,
        button::ButtonBuilder,
        scroll_content_presenter::ScrollContentPresenterBuilder,
        VerticalAlignment,
        event::RoutedEventHandler,
    },
    math::vec2::Vec2,
};

pub struct Window {
    pub(in crate::gui) owner_handle: Handle<UINode>,
}

pub struct WindowBuilder {
    common: CommonBuilderFields,
    content: Handle<UINode>,
}

impl WindowBuilder {
    pub fn new() -> Self {
        Self {
            common: CommonBuilderFields::new(),
            content: Handle::none(),
        }
    }

    impl_default_builder_methods!();

    pub fn with_content(mut self, content: Handle<UINode>) -> Self {
        self.content = content;
        self
    }

    pub fn build(self, ui: &mut UserInterface) -> Handle<UINode> {
        let window = Window {
            owner_handle: Default::default()
        };

        GenericNodeBuilder::new(UINodeKind::Window(window), self.common)
            .with_child(BorderBuilder::new()
                .with_color(Color::opaque(120, 120, 120))
                .with_child(GridBuilder::new()
                    .add_column(Column::stretch())
                    .add_row(Row::auto())
                    .add_row(Row::stretch())
                    .with_child(ScrollContentPresenterBuilder::new()
                        .with_content(self.content)
                        .on_row(1)
                        .build(ui))
                    .with_child(BorderBuilder::new()
                        .with_color(Color::opaque(120, 120, 120))
                        .on_row(0)
                        .with_horizontal_alignment(HorizontalAlignment::Stretch)
                        .with_height(30.0)
                        .with_handler(RoutedEventHandlerType::MouseDown, Box::new(|_ui, _handle, _evt| {}))
                        .with_handler(RoutedEventHandlerType::MouseUp, Box::new(|_ui, _handle, _evt| {}))
                        .with_handler(RoutedEventHandlerType::MouseMove, Box::new(|_ui, _handle, _evt| {}))
                        .with_child(GridBuilder::new()
                            .add_column(Column::stretch())
                            .add_column(Column::strict(30.0))
                            .add_column(Column::strict(30.0))
                            .add_row(Row::stretch())
                            .with_child(TextBuilder::new()
                                .with_text("Unnamed window")
                                .with_margin(Thickness::uniform(5.0))
                                .on_row(0)
                                .on_column(0)
                                .build(ui))
                            .with_child(ButtonBuilder::new()
                                .on_row(0)
                                .on_column(1)
                                .with_margin(Thickness::uniform(2.0))
                                .with_text("_")
                                .build(ui))
                            .with_child(ButtonBuilder::new()
                                .on_row(0)
                                .on_column(2)
                                .with_margin(Thickness::uniform(2.0))
                                .with_text("X")
                                .build(ui))
                            .build(ui))
                        .build(ui))
                    .build(ui))
                .build(ui))
            .build(ui)
    }
}