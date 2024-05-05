use crate::fyrox::{
    core::{
        pool::Handle, reflect::prelude::*, type_traits::prelude::*, uuid_provider,
        visitor::prelude::*,
    },
    gui::{
        border::BorderBuilder,
        decorator::DecoratorBuilder,
        define_constructor, define_widget_deref,
        dropdown_list::{DropdownListBuilder, DropdownListMessage},
        grid::{Column, GridBuilder, Row},
        list_view::{ListViewBuilder, ListViewMessage},
        message::{MessageDirection, UiMessage},
        text::{TextBuilder, TextMessage},
        utils::make_simple_tooltip,
        widget::{Widget, WidgetBuilder},
        BuildContext, Control, HorizontalAlignment, Thickness, UiNode, UserInterface,
        VerticalAlignment, BRUSH_LIGHTER,
    },
    scene::sound::{AudioBus, AudioBusGraph},
};
use crate::gui::make_dropdown_list_option;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, PartialEq)]
pub enum AudioBusViewMessage {
    ChangeParent(Handle<AudioBus>),
    PossibleParentBuses(Vec<(Handle<AudioBus>, String)>),
    EffectNames(Vec<String>),
    Name(String),
}

impl AudioBusViewMessage {
    define_constructor!(AudioBusViewMessage:ChangeParent => fn change_parent(Handle<AudioBus>), layout: false);
    define_constructor!(AudioBusViewMessage:PossibleParentBuses => fn possible_parent_buses(Vec<(Handle<AudioBus>, String)>), layout: false);
    define_constructor!(AudioBusViewMessage:EffectNames => fn effect_names(Vec<String>), layout: false);
    define_constructor!(AudioBusViewMessage:Name => fn name(String), layout: false);
}

#[derive(Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct AudioBusView {
    widget: Widget,
    pub bus: Handle<AudioBus>,
    parent_bus_selector: Handle<UiNode>,
    possible_parent_buses: Vec<Handle<AudioBus>>,
    effect_names_list: Handle<UiNode>,
    name: Handle<UiNode>,
}

define_widget_deref!(AudioBusView);

uuid_provider!(AudioBusView = "5439e3a9-096a-4155-922c-ed57a76a46f3");

impl Control for AudioBusView {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle && message.direction() == MessageDirection::ToWidget
        {
            if let Some(msg) = message.data::<AudioBusViewMessage>() {
                match msg {
                    AudioBusViewMessage::ChangeParent(_) => {
                        // Do nothing.
                    }
                    AudioBusViewMessage::PossibleParentBuses(buses) => {
                        self.possible_parent_buses =
                            buses.iter().map(|(handle, _)| *handle).collect::<Vec<_>>();

                        let items = make_items(buses, &mut ui.build_ctx());

                        ui.send_message(DropdownListMessage::items(
                            self.parent_bus_selector,
                            MessageDirection::ToWidget,
                            items,
                        ))
                    }
                    AudioBusViewMessage::EffectNames(names) => {
                        let items = make_effect_names(names, &mut ui.build_ctx());
                        ui.send_message(ListViewMessage::items(
                            self.effect_names_list,
                            MessageDirection::ToWidget,
                            items,
                        ));
                    }
                    AudioBusViewMessage::Name(new_name) => {
                        ui.send_message(TextMessage::text(
                            self.name,
                            MessageDirection::ToWidget,
                            new_name.clone(),
                        ));
                    }
                }
            }
        }

        if message.destination == self.parent_bus_selector
            && message.direction() == MessageDirection::FromWidget
        {
            if let Some(DropdownListMessage::SelectionChanged(Some(selection))) = message.data() {
                ui.send_message(AudioBusViewMessage::change_parent(
                    self.handle,
                    MessageDirection::FromWidget,
                    self.possible_parent_buses[*selection],
                ));
            }
        }
    }
}

fn make_items(buses: &[(Handle<AudioBus>, String)], ctx: &mut BuildContext) -> Vec<Handle<UiNode>> {
    buses
        .iter()
        .map(|(_, name)| make_dropdown_list_option(ctx, name))
        .collect::<Vec<_>>()
}

fn make_effect_names(names: &[String], ctx: &mut BuildContext) -> Vec<Handle<UiNode>> {
    if names.is_empty() {
        vec![
            TextBuilder::new(WidgetBuilder::new().with_foreground(BRUSH_LIGHTER))
                .with_text("No Effects")
                .with_horizontal_text_alignment(HorizontalAlignment::Center)
                .build(ctx),
        ]
    } else {
        names
            .iter()
            .map(|n| {
                TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                    .with_text(n)
                    .with_horizontal_text_alignment(HorizontalAlignment::Center)
                    .build(ctx)
            })
            .collect::<Vec<_>>()
    }
}

pub struct AudioBusViewBuilder {
    widget_builder: WidgetBuilder,
    name: String,
    effect_names: Vec<String>,
    bus: Handle<AudioBus>,
    parent_bus: Handle<AudioBus>,
    possible_parent_buses: Vec<(Handle<AudioBus>, String)>,
}

impl AudioBusViewBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            name: AudioBusGraph::PRIMARY_BUS.to_string(),
            effect_names: Default::default(),
            bus: Default::default(),
            parent_bus: Default::default(),
            possible_parent_buses: Default::default(),
        }
    }

    pub fn with_name<S: AsRef<str>>(mut self, name: S) -> Self {
        name.as_ref().clone_into(&mut self.name);
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

    pub fn with_parent_bus(mut self, parent_bus: Handle<AudioBus>) -> Self {
        self.parent_bus = parent_bus;
        self
    }

    pub fn with_possible_parent_buses(
        mut self,
        possible_parent_buses: Vec<(Handle<AudioBus>, String)>,
    ) -> Self {
        self.possible_parent_buses = possible_parent_buses;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let effect_names_list;
        let name;
        let parent_bus_selector;
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    BorderBuilder::new(WidgetBuilder::new().on_row(0).on_column(0).with_child({
                        name = TextBuilder::new(
                            WidgetBuilder::new()
                                .with_horizontal_alignment(HorizontalAlignment::Center)
                                .with_vertical_alignment(VerticalAlignment::Center),
                        )
                        .with_text(self.name)
                        .build(ctx);
                        name
                    }))
                    .build(ctx),
                )
                .with_child(
                    BorderBuilder::new(
                        WidgetBuilder::new()
                            .on_row(1)
                            .on_column(0)
                            .with_margin(Thickness::uniform(1.0))
                            .with_tooltip(make_simple_tooltip(
                                ctx,
                                "A list of effects applied to the audio bus.",
                            ))
                            .with_child({
                                effect_names_list = ListViewBuilder::new(
                                    WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                                )
                                .with_items(make_effect_names(&self.effect_names, ctx))
                                .build(ctx);
                                effect_names_list
                            }),
                    )
                    .build(ctx),
                )
                .with_child({
                    parent_bus_selector = DropdownListBuilder::new(
                        WidgetBuilder::new()
                            .with_visibility(self.parent_bus.is_some())
                            .on_row(2)
                            .on_column(0)
                            .with_margin(Thickness::uniform(1.0))
                            .with_tooltip(make_simple_tooltip(
                                ctx,
                                "A parent audio bus to which this audio bus will send its data.",
                            )),
                    )
                    .with_opt_selected(
                        self.possible_parent_buses
                            .iter()
                            .position(|(h, _)| *h == self.parent_bus),
                    )
                    .with_items(make_items(&self.possible_parent_buses, ctx))
                    .build(ctx);
                    parent_bus_selector
                }),
        )
        .add_row(Row::strict(25.0))
        .add_row(Row::stretch())
        .add_row(Row::strict(25.0))
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
            parent_bus_selector,
            possible_parent_buses: self
                .possible_parent_buses
                .into_iter()
                .map(|(handle, _)| handle)
                .collect::<Vec<_>>(),
            effect_names_list,
            name,
        };
        ctx.add_node(UiNode::new(view))
    }
}
