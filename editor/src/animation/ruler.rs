use crate::fyrox::graph::BaseSceneGraph;
use crate::fyrox::{
    core::{
        algebra::{Point2, Vector2},
        math::Rect,
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid_provider,
        visitor::prelude::*,
    },
    gui::{
        define_constructor, define_widget_deref,
        draw::{CommandTexture, Draw, DrawingContext},
        formatted_text::{FormattedText, FormattedTextBuilder},
        menu::MenuItemMessage,
        message::{MessageDirection, MouseButton, UiMessage},
        popup::PopupBuilder,
        popup::{Placement, PopupMessage},
        stack_panel::StackPanelBuilder,
        widget::{Widget, WidgetBuilder, WidgetMessage},
        BuildContext, Control, RcUiNodeHandle, UiNode, UserInterface, BRUSH_BRIGHT, BRUSH_DARKER,
        BRUSH_LIGHTER, BRUSH_LIGHTEST,
    },
};
use crate::menu::create_menu_item;
use fyrox::gui::curve::{CurveTransformCell, STANDARD_GRID_SIZE};
use fyrox::gui::menu::ContextMenuBuilder;
use std::{
    cell::{Cell, RefCell},
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, PartialEq)]
pub enum RulerMessage {
    Zoom(f32),
    ViewPosition(f32),
    Value(f32),
    AddSignal(f32),
    RemoveSignal(Uuid),
    SyncSignals(Vec<SignalView>),
    MoveSignal { id: Uuid, new_position: f32 },
    SelectSignal(Uuid),
}

impl RulerMessage {
    define_constructor!(RulerMessage:Zoom => fn zoom(f32), layout: false);
    define_constructor!(RulerMessage:ViewPosition => fn view_position(f32), layout: false);
    define_constructor!(RulerMessage:Value => fn value(f32), layout: false);
    define_constructor!(RulerMessage:AddSignal => fn add_signal(f32), layout: false);
    define_constructor!(RulerMessage:RemoveSignal => fn remove_signal(Uuid), layout: false);
    define_constructor!(RulerMessage:SyncSignals => fn sync_signals(Vec<SignalView>), layout: false);
    define_constructor!(RulerMessage:MoveSignal => fn move_signal(id: Uuid, new_position: f32), layout: false);
    define_constructor!(RulerMessage:SelectSignal => fn select_signal(Uuid), layout: false);
}

#[derive(Clone)]
struct ContextMenu {
    menu: RcUiNodeHandle,
    add_signal: Handle<UiNode>,
    remove_signal: Handle<UiNode>,
    selected_position: Cell<f32>,
}

impl ContextMenu {
    fn new(ctx: &mut BuildContext) -> Self {
        let add_signal;
        let remove_signal;
        let menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new().with_visibility(false)).with_content(
                StackPanelBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            add_signal = create_menu_item("Add Signal", vec![], ctx);
                            add_signal
                        })
                        .with_child({
                            remove_signal = create_menu_item("Remove Signal", vec![], ctx);
                            remove_signal
                        }),
                )
                .build(ctx),
            ),
        )
        .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        Self {
            menu,
            add_signal,
            remove_signal,
            selected_position: Cell::new(0.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SignalView {
    pub id: Uuid,
    pub time: f32,
    pub selected: bool,
}

impl SignalView {
    fn screen_bounds(&self, ruler: &Ruler) -> Rect<f32> {
        let view_x = ruler.local_to_view(self.time);
        let view_y = ruler.bounding_rect().size.y - SignalView::SIZE;

        let min = ruler
            .visual_transform()
            .transform_point(&Point2::new(view_x - SignalView::SIZE * 0.5, view_y))
            .coords;
        let max = ruler
            .visual_transform()
            .transform_point(&Point2::new(
                view_x + SignalView::SIZE * 0.5,
                ruler.bounding_rect().size.y,
            ))
            .coords;

        Rect::new(min.x, min.y, max.x - min.x, max.y - min.y)
    }
}

impl SignalView {
    const SIZE: f32 = 10.0;
}

#[derive(Clone)]
enum DragEntity {
    TimePosition,
    Signal(Uuid),
}

#[derive(Clone)]
struct DragContext {
    entity: DragEntity,
}

#[derive(Clone, Visit, Reflect, ComponentProvider)]
pub struct Ruler {
    widget: Widget,
    #[visit(skip)]
    #[reflect(hidden)]
    transform: CurveTransformCell,
    #[visit(skip)]
    #[reflect(hidden)]
    text: RefCell<FormattedText>,
    value: f32,
    #[visit(skip)]
    #[reflect(hidden)]
    drag_context: Option<DragContext>,
    #[visit(skip)]
    #[reflect(hidden)]
    signals: RefCell<Vec<SignalView>>,
    #[visit(skip)]
    #[reflect(hidden)]
    context_menu: ContextMenu,
}

impl Debug for Ruler {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ruler")
    }
}

define_widget_deref!(Ruler);

impl Ruler {
    fn local_to_view(&self, x: f32) -> f32 {
        self.transform
            .curve_to_local()
            .transform_point(&Point2::new(x, 0.0))
            .x
    }

    #[allow(unused)]
    fn view_to_local(&self, x: f32) -> f32 {
        self.transform
            .local_to_curve()
            .transform_point(&Point2::new(x, 0.0))
            .x
    }

    fn screen_to_value_space(&self, x: f32) -> f32 {
        self.transform
            .screen_to_curve()
            .transform_point(&Point2::new(x, 0.0))
            .x
    }
}

uuid_provider!(Ruler = "98655c9b-428f-4977-a478-ad3674cc66d4");

impl Control for Ruler {
    fn draw(&self, ctx: &mut DrawingContext) {
        self.transform.set_bounds(self.screen_bounds());
        self.transform.update_transform();
        let local_bounds = self.bounding_rect();

        // Add clickable rectangle first.
        ctx.push_rect_filled(&local_bounds, None);
        ctx.commit(
            self.clip_bounds(),
            self.background(),
            CommandTexture::None,
            None,
        );

        // Then draw the rest.
        for x in self.transform.x_step_iter(STANDARD_GRID_SIZE) {
            ctx.push_line(
                Vector2::new(x, local_bounds.size.y * 0.5),
                Vector2::new(x, local_bounds.size.y),
                1.0,
            );
        }
        ctx.commit(
            self.clip_bounds(),
            self.foreground(),
            CommandTexture::None,
            None,
        );

        // Draw values.
        let mut text = self.text.borrow_mut();

        for x in self.transform.x_step_iter(STANDARD_GRID_SIZE) {
            text.set_text(format!("{:.1}s", x)).build();
            let vx = self.local_to_view(x);
            ctx.draw_text(self.clip_bounds(), Vector2::new(vx + 1.0, 0.0), &text);
        }

        // Draw signals.
        for signal in self.signals.borrow().iter() {
            let size = SignalView::SIZE;
            let x = self.local_to_view(signal.time);

            ctx.push_triangle_filled([
                Vector2::new(x - size * 0.5, local_bounds.h() - size),
                Vector2::new(x + size * 0.5, local_bounds.h() - size),
                Vector2::new(x, local_bounds.h()),
            ]);
            let brush = if signal.selected {
                BRUSH_BRIGHT
            } else {
                BRUSH_LIGHTEST
            };
            ctx.commit(self.clip_bounds(), brush, CommandTexture::None, None);
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<RulerMessage>() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
            {
                match msg {
                    RulerMessage::Zoom(zoom) => {
                        self.transform.set_scale(Vector2::new(*zoom, 1.0));
                    }
                    RulerMessage::ViewPosition(position) => {
                        self.transform.set_position(Vector2::new(*position, 0.0));
                    }
                    RulerMessage::Value(value) => {
                        if value.ne(&self.value) {
                            self.value = *value;
                            ui.send_message(message.reverse());
                        }
                    }
                    RulerMessage::AddSignal(_)
                    | RulerMessage::RemoveSignal(_)
                    | RulerMessage::MoveSignal { .. }
                    | RulerMessage::SelectSignal(_) => {
                        // Do nothing. These messages are only for output.
                    }
                    RulerMessage::SyncSignals(signals) => {
                        self.signals.borrow_mut().clone_from(signals);
                    }
                }
            }
        } else if let Some(msg) = message.data::<WidgetMessage>() {
            if message.direction() == MessageDirection::FromWidget {
                match msg {
                    WidgetMessage::MouseDown { pos, button } => {
                        if *button == MouseButton::Left {
                            ui.capture_mouse(self.handle);

                            for signal in self.signals.borrow_mut().iter_mut() {
                                signal.selected = false;

                                let bounds = signal.screen_bounds(self);

                                if self.drag_context.is_none() && bounds.contains(*pos) {
                                    signal.selected = true;
                                    self.drag_context = Some(DragContext {
                                        entity: DragEntity::Signal(signal.id),
                                    });

                                    ui.send_message(RulerMessage::select_signal(
                                        self.handle,
                                        MessageDirection::FromWidget,
                                        signal.id,
                                    ));
                                }
                            }

                            if self.drag_context.is_none() {
                                ui.send_message(RulerMessage::value(
                                    self.handle,
                                    MessageDirection::ToWidget,
                                    self.screen_to_value_space(pos.x),
                                ));

                                self.drag_context = Some(DragContext {
                                    entity: DragEntity::TimePosition,
                                });
                            }
                        }
                    }
                    WidgetMessage::MouseUp { button, pos } => {
                        if *button == MouseButton::Left {
                            ui.release_mouse_capture();

                            if let Some(drag_context) = self.drag_context.take() {
                                if let DragEntity::Signal(id) = drag_context.entity {
                                    if let Some(signal) =
                                        self.signals.borrow_mut().iter_mut().find(|s| s.id == id)
                                    {
                                        signal.selected = false;

                                        ui.send_message(RulerMessage::move_signal(
                                            self.handle,
                                            MessageDirection::FromWidget,
                                            id,
                                            self.screen_to_value_space(pos.x),
                                        ))
                                    }
                                }
                            }
                        }
                    }
                    WidgetMessage::MouseMove { pos, .. } => {
                        if let Some(drag_context) = self.drag_context.as_ref() {
                            match drag_context.entity {
                                DragEntity::TimePosition => {
                                    ui.send_message(RulerMessage::value(
                                        self.handle,
                                        MessageDirection::ToWidget,
                                        self.screen_to_value_space(pos.x),
                                    ));
                                }

                                DragEntity::Signal(id) => {
                                    if let Some(signal) =
                                        self.signals.borrow_mut().iter_mut().find(|s| s.id == id)
                                    {
                                        signal.time = self.screen_to_value_space(pos.x);
                                    }
                                }
                            }
                        }
                    }
                    _ => (),
                };
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.context_menu.add_signal {
                ui.send_message(RulerMessage::add_signal(
                    self.handle,
                    MessageDirection::FromWidget,
                    self.context_menu.selected_position.get(),
                ));
            } else if message.destination() == self.context_menu.remove_signal {
                for signal in self.signals.borrow().iter() {
                    if signal
                        .screen_bounds(self)
                        .contains(ui.node(self.context_menu.menu.handle()).screen_position())
                    {
                        ui.send_message(RulerMessage::remove_signal(
                            self.handle,
                            MessageDirection::FromWidget,
                            signal.id,
                        ));
                        break; // No multi-selection
                    }
                }
            }
        } else if let Some(PopupMessage::Placement(Placement::Cursor(_))) = message.data() {
            self.context_menu
                .selected_position
                .set(self.screen_to_value_space(ui.cursor_position().x));

            let can_remove = self
                .signals
                .borrow()
                .iter()
                .any(|signal| signal.screen_bounds(self).contains(ui.cursor_position()));

            ui.send_message(WidgetMessage::enabled(
                self.context_menu.remove_signal,
                MessageDirection::ToWidget,
                can_remove,
            ));
        }
    }
}

pub struct RulerBuilder {
    widget_builder: WidgetBuilder,
    value: f32,
}

impl RulerBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            value: 0.0,
        }
    }

    pub fn with_value(mut self, value: f32) -> Self {
        self.value = value;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let context_menu = ContextMenu::new(ctx);

        let ruler = Ruler {
            widget: self
                .widget_builder
                .with_preview_messages(true)
                .with_context_menu(context_menu.menu.clone())
                .with_background(BRUSH_DARKER)
                .with_foreground(BRUSH_LIGHTER)
                .build(),
            transform: Default::default(),
            text: RefCell::new(FormattedTextBuilder::new(ctx.default_font()).build()),
            value: self.value,
            drag_context: None,
            signals: Default::default(),
            context_menu,
        };

        ctx.add_node(UiNode::new(ruler))
    }
}
