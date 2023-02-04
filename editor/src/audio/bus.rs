use crate::gui::make_dropdown_list_option;
use fyrox::{
    core::pool::Handle,
    gui::{
        border::BorderBuilder,
        decorator::DecoratorBuilder,
        define_constructor, define_widget_deref,
        dropdown_list::{DropdownListBuilder, DropdownListMessage},
        grid::{Column, GridBuilder, Row},
        list_view::ListViewBuilder,
        message::{MessageDirection, UiMessage},
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

#[derive(Debug, Clone, PartialEq)]
pub enum AudioBusViewMessage {
    ChangeParent(Handle<AudioBus>),
    PossibleParentBuses(Vec<(Handle<AudioBus>, String)>),
}

impl AudioBusViewMessage {
    define_constructor!(AudioBusViewMessage:ChangeParent => fn change_parent(Handle<AudioBus>), layout: false);
    define_constructor!(AudioBusViewMessage:PossibleParentBuses => fn possible_parent_buses(Vec<(Handle<AudioBus>, String)>), layout: false);
}

#[derive(Clone)]
pub struct AudioBusView {
    widget: Widget,
    pub bus: Handle<AudioBus>,
    parent_bus_selector: Handle<UiNode>,
    parent_bus: Handle<AudioBus>,
    possible_parent_buses: Vec<Handle<AudioBus>>,
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
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle && message.direction() == MessageDirection::ToWidget
        {
            if let Some(AudioBusViewMessage::PossibleParentBuses(buses)) = message.data() {
                self.possible_parent_buses =
                    buses.iter().map(|(handle, _)| *handle).collect::<Vec<_>>();

                let items = make_items(&buses, &mut ui.build_ctx());

                ui.send_message(DropdownListMessage::items(
                    self.parent_bus_selector,
                    MessageDirection::ToWidget,
                    items,
                ))
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
        .map(|(_, name)| make_dropdown_list_option(ctx, &name))
        .collect::<Vec<_>>()
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
        let effects;
        let name;
        let parent_bus_selector;
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
                )
                .with_child({
                    parent_bus_selector =
                        DropdownListBuilder::new(WidgetBuilder::new().on_row(2).on_column(0))
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
            parent_bus: self.parent_bus,
            possible_parent_buses: self
                .possible_parent_buses
                .into_iter()
                .map(|(handle, _)| handle)
                .collect::<Vec<_>>(),
        };
        ctx.add_node(UiNode::new(view))
    }
}
