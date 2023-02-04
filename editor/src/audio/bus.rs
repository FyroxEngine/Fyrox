use fyrox::gui::stack_panel::StackPanelBuilder;
use fyrox::{
    core::pool::Handle,
    gui::{
        border::BorderBuilder,
        decorator::DecoratorBuilder,
        define_widget_deref,
        grid::{Column, GridBuilder, Row},
        list_view::ListViewBuilder,
        message::UiMessage,
        text::TextBuilder,
        widget::{Widget, WidgetBuilder},
        BuildContext, Control, HorizontalAlignment, Thickness, UiNode, UserInterface,
    },
    scene::sound::{AudioBus, AudioBusGraph},
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

#[derive(Clone)]
pub struct AudioBusView {
    widget: Widget,
    pub bus: Handle<AudioBus>,
}

define_widget_deref!(AudioBusView);

impl Control for AudioBusView {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message)
    }
}

pub struct AudioBusViewBuilder {
    widget_builder: WidgetBuilder,
    name: String,
    effect_names: Vec<String>,
    bus: Handle<AudioBus>,
}

impl AudioBusViewBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            name: AudioBusGraph::PRIMARY_BUS.to_string(),
            effect_names: Default::default(),
            bus: Default::default(),
        }
    }

    pub fn with_name<S: AsRef<str>>(mut self, name: S) -> Self {
        self.name = name.as_ref().to_owned();
        self
    }

    pub fn with_audio_bus(mut self, bus: Handle<AudioBus>) -> Self {
        self.bus = bus;
        self
    }

    pub fn with_effect_names(mut self, names: Vec<String>) -> Self {
        self.effect_names = names;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let effects;
        let name;
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    name = TextBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(0)
                            .with_horizontal_alignment(HorizontalAlignment::Center),
                    )
                    .with_text(self.name)
                    .build(ctx);
                    name
                })
                .with_child(
                    BorderBuilder::new(
                        WidgetBuilder::new()
                            .on_row(1)
                            .on_column(0)
                            .with_margin(Thickness::uniform(1.0))
                            .with_child({
                                effects = ListViewBuilder::new(
                                    WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                                )
                                .with_items(
                                    self.effect_names
                                        .into_iter()
                                        .map(|n| {
                                            TextBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_margin(Thickness::uniform(1.0)),
                                            )
                                            .with_text(n)
                                            .build(ctx)
                                        })
                                        .collect::<Vec<_>>(),
                                )
                                .build(ctx);
                                effects
                            }),
                    )
                    .build(ctx),
                ),
        )
        .add_row(Row::strict(25.0))
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let view = AudioBusView {
            widget: self
                .widget_builder
                .with_child(
                    DecoratorBuilder::new(BorderBuilder::new(
                        WidgetBuilder::new().with_child(grid),
                    ))
                    .build(ctx),
                )
                .build(),
            bus: self.bus,
        };
        ctx.add_node(UiNode::new(view))
    }
}
