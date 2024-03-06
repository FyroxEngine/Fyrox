use crate::{
    brush::Brush,
    core::{
        algebra::{Matrix3, Point2, SimdPartialOrd, Vector2, Vector3},
        color::Color,
        math::curve::{Curve, CurveKeyKind},
        math::{cubicf, lerpf, wrap_angle, Rect},
        pool::Handle,
        type_traits::prelude::*,
    },
    core::{reflect::prelude::*, visitor::prelude::*},
    curve::key::{CurveKeyView, KeyContainer},
    define_constructor,
    draw::{CommandTexture, Draw, DrawingContext},
    formatted_text::{FormattedText, FormattedTextBuilder},
    grid::{Column, GridBuilder, Row},
    menu::{MenuItemBuilder, MenuItemContent, MenuItemMessage},
    message::{ButtonState, KeyCode, MessageDirection, MouseButton, UiMessage},
    numeric::{NumericUpDownBuilder, NumericUpDownMessage},
    popup::PopupBuilder,
    stack_panel::StackPanelBuilder,
    text::TextBuilder,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, RcUiNodeHandle, Thickness, UiNode, UserInterface, VerticalAlignment,
};
use fxhash::FxHashSet;
use fyrox_core::uuid_provider;
use fyrox_graph::BaseSceneGraph;
use std::sync::mpsc::Sender;
use std::{
    cell::{Cell, RefCell},
    ops::{Deref, DerefMut},
};

pub mod key;

#[derive(Debug, Clone, PartialEq)]
pub enum CurveEditorMessage {
    Sync(Curve),
    ViewPosition(Vector2<f32>),
    Zoom(Vector2<f32>),
    ZoomToFit {
        /// Should the zoom to fit be performed on some of the next update cycle (up to 10 frames delay), or immediately when
        /// processing the message.
        after_layout: bool,
    },
    HighlightZones(Vec<HighlightZone>),

    // Internal messages. Use only when you know what you're doing.
    // These are internal because you must use Sync message to request changes
    // in the curve editor.
    ChangeSelectedKeysKind(CurveKeyKind),
    ChangeSelectedKeysValue(f32),
    ChangeSelectedKeysLocation(f32),
    RemoveSelection,
    // Position in screen coordinates.
    AddKey(Vector2<f32>),
}

impl CurveEditorMessage {
    define_constructor!(CurveEditorMessage:Sync => fn sync(Curve), layout: false);
    define_constructor!(CurveEditorMessage:ViewPosition => fn view_position(Vector2<f32>), layout: false);
    define_constructor!(CurveEditorMessage:Zoom => fn zoom(Vector2<f32>), layout: false);
    define_constructor!(CurveEditorMessage:ZoomToFit => fn zoom_to_fit(after_layout: bool), layout: true);
    define_constructor!(CurveEditorMessage:HighlightZones => fn hightlight_zones(Vec<HighlightZone>), layout: false);
    // Internal. Use only when you know what you're doing.
    define_constructor!(CurveEditorMessage:RemoveSelection => fn remove_selection(), layout: false);
    define_constructor!(CurveEditorMessage:ChangeSelectedKeysKind => fn change_selected_keys_kind(CurveKeyKind), layout: false);
    define_constructor!(CurveEditorMessage:ChangeSelectedKeysValue => fn change_selected_keys_value(f32), layout: false);
    define_constructor!(CurveEditorMessage:ChangeSelectedKeysLocation => fn change_selected_keys_location(f32), layout: false);
    define_constructor!(CurveEditorMessage:AddKey => fn add_key(Vector2<f32>), layout: false);
}

/// Highlight zone in values space.
#[derive(Clone, Debug, PartialEq, Visit, Reflect, Default)]
pub struct HighlightZone {
    pub rect: Rect<f32>,
    pub brush: Brush,
}

#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct CurveEditor {
    widget: Widget,
    key_container: KeyContainer,
    zoom: Vector2<f32>,
    view_position: Vector2<f32>,
    // Transforms a point from local to view coordinates.
    #[visit(skip)]
    #[reflect(hidden)]
    view_matrix: Cell<Matrix3<f32>>,
    // Transforms a point from local to screen coordinates.
    // View and screen coordinates are different:
    //  - view is a local viewer of curve editor
    //  - screen is global space
    #[visit(skip)]
    #[reflect(hidden)]
    screen_matrix: Cell<Matrix3<f32>>,
    // Transform a point from screen space (i.e. mouse position) to the
    // local space (the space where all keys are)
    #[visit(skip)]
    #[reflect(hidden)]
    inv_screen_matrix: Cell<Matrix3<f32>>,
    key_brush: Brush,
    selected_key_brush: Brush,
    key_size: f32,
    grid_brush: Brush,
    #[visit(skip)]
    #[reflect(hidden)]
    operation_context: Option<OperationContext>,
    #[visit(skip)]
    #[reflect(hidden)]
    selection: Option<Selection>,
    handle_radius: f32,
    context_menu: ContextMenu,
    #[visit(skip)]
    #[reflect(hidden)]
    text: RefCell<FormattedText>,
    view_bounds: Option<Rect<f32>>,
    show_x_values: bool,
    show_y_values: bool,
    grid_size: Vector2<f32>,
    min_zoom: Vector2<f32>,
    max_zoom: Vector2<f32>,
    highlight_zones: Vec<HighlightZone>,
    #[visit(skip)]
    #[reflect(hidden)]
    zoom_to_fit_timer: Option<usize>,
}

crate::define_widget_deref!(CurveEditor);

#[derive(Default, Clone, Visit, Reflect, Debug)]
struct ContextMenu {
    widget: RcUiNodeHandle,
    add_key: Handle<UiNode>,
    remove: Handle<UiNode>,
    key: Handle<UiNode>,
    make_constant: Handle<UiNode>,
    make_linear: Handle<UiNode>,
    make_cubic: Handle<UiNode>,
    zoom_to_fit: Handle<UiNode>,
    key_properties: Handle<UiNode>,
    key_value: Handle<UiNode>,
    key_location: Handle<UiNode>,
}

#[derive(Clone, Debug)]
struct DragEntry {
    key: Uuid,
    initial_position: Vector2<f32>,
}

#[derive(Clone, Debug)]
enum OperationContext {
    DragKeys {
        // In local coordinates.
        initial_mouse_pos: Vector2<f32>,
        entries: Vec<DragEntry>,
    },
    MoveView {
        initial_mouse_pos: Vector2<f32>,
        initial_view_pos: Vector2<f32>,
    },
    DragTangent {
        key: usize,
        left: bool,
    },
    BoxSelection {
        // In local coordinates.
        initial_mouse_pos: Vector2<f32>,
        min: Cell<Vector2<f32>>,
        max: Cell<Vector2<f32>>,
    },
}

#[derive(Clone, Debug)]
enum Selection {
    Keys { keys: FxHashSet<Uuid> },
    // It is ok to use index directly in case of tangents since
    // we won't change position of keys so index will be valid.
    LeftTangent { key: usize },
    RightTangent { key: usize },
}

#[derive(Copy, Clone)]
enum PickResult {
    Key(usize),
    LeftTangent(usize),
    RightTangent(usize),
}

impl Selection {
    fn single_key(key: Uuid) -> Self {
        let mut keys = FxHashSet::default();
        keys.insert(key);
        Self::Keys { keys }
    }
}

uuid_provider!(CurveEditor = "5c7b087e-871e-498d-b064-187b604a37d8");

impl Control for CurveEditor {
    fn draw(&self, ctx: &mut DrawingContext) {
        ctx.transform_stack.push(Matrix3::identity());
        self.update_matrices();
        self.draw_background(ctx);
        self.draw_highlight_zones(ctx);
        self.draw_grid(ctx);
        self.draw_curve(ctx);
        self.draw_keys(ctx);
        self.draw_operation(ctx);
        ctx.transform_stack.pop();
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle {
            if let Some(msg) = message.data::<WidgetMessage>() {
                match msg {
                    WidgetMessage::KeyUp(KeyCode::Delete) => {
                        self.remove_selection(ui);
                    }
                    WidgetMessage::MouseMove { pos, state } => {
                        let local_mouse_pos = self.point_to_local_space(*pos);
                        if let Some(operation_context) = self.operation_context.as_ref() {
                            match operation_context {
                                OperationContext::DragKeys {
                                    entries,
                                    initial_mouse_pos,
                                } => {
                                    let local_delta = local_mouse_pos - initial_mouse_pos;
                                    for entry in entries {
                                        if let Some(key) = self.key_container.key_mut(entry.key) {
                                            key.position = entry.initial_position + local_delta;
                                        }
                                    }
                                    self.sort_keys();
                                }
                                OperationContext::MoveView {
                                    initial_mouse_pos,
                                    initial_view_pos,
                                } => {
                                    let d = pos - initial_mouse_pos;
                                    let delta = Vector2::new(d.x / self.zoom.x, d.y / self.zoom.y);
                                    ui.send_message(CurveEditorMessage::view_position(
                                        self.handle,
                                        MessageDirection::ToWidget,
                                        initial_view_pos + delta,
                                    ));
                                }
                                OperationContext::DragTangent { key, left } => {
                                    if let Some(key) = self.key_container.key_index_mut(*key) {
                                        let key_pos = key.position;

                                        let screen_key_pos = self
                                            .screen_matrix
                                            .get()
                                            .transform_point(&Point2::from(key_pos))
                                            .coords;

                                        if let CurveKeyKind::Cubic {
                                            left_tangent,
                                            right_tangent,
                                        } = &mut key.kind
                                        {
                                            let mut local_delta = pos - screen_key_pos;
                                            if *left {
                                                local_delta.x = local_delta.x.min(f32::EPSILON);
                                            } else {
                                                local_delta.x = local_delta.x.max(f32::EPSILON);
                                            }
                                            let tangent =
                                                (local_delta.y / local_delta.x).clamp(-10e6, 10e6);

                                            if *left {
                                                *left_tangent = tangent;
                                            } else {
                                                *right_tangent = tangent;
                                            }
                                        } else {
                                            unreachable!(
                                                "attempt to edit tangents of non-cubic curve key!"
                                            )
                                        }
                                    }
                                }
                                OperationContext::BoxSelection {
                                    initial_mouse_pos,
                                    min,
                                    max,
                                    ..
                                } => {
                                    min.set(local_mouse_pos.inf(initial_mouse_pos));
                                    max.set(local_mouse_pos.sup(initial_mouse_pos));
                                }
                            }
                        } else if state.left == ButtonState::Pressed {
                            if let Some(selection) = self.selection.as_ref() {
                                match selection {
                                    Selection::Keys { keys } => {
                                        self.operation_context = Some(OperationContext::DragKeys {
                                            entries: keys
                                                .iter()
                                                .map(|k| DragEntry {
                                                    key: *k,
                                                    initial_position: self
                                                        .key_container
                                                        .key_ref(*k)
                                                        .map(|k| k.position)
                                                        .unwrap_or_default(),
                                                })
                                                .collect::<Vec<_>>(),
                                            initial_mouse_pos: local_mouse_pos,
                                        });
                                    }
                                    Selection::LeftTangent { key } => {
                                        self.operation_context =
                                            Some(OperationContext::DragTangent {
                                                key: *key,
                                                left: true,
                                            })
                                    }
                                    Selection::RightTangent { key } => {
                                        self.operation_context =
                                            Some(OperationContext::DragTangent {
                                                key: *key,
                                                left: false,
                                            })
                                    }
                                }
                            } else {
                                self.operation_context = Some(OperationContext::BoxSelection {
                                    initial_mouse_pos: local_mouse_pos,
                                    min: Default::default(),
                                    max: Default::default(),
                                })
                            }

                            if self.operation_context.is_some() {
                                ui.capture_mouse(self.handle);
                            }
                        }
                    }

                    WidgetMessage::MouseUp { .. } => {
                        if let Some(context) = self.operation_context.take() {
                            ui.release_mouse_capture();

                            // Send modified curve back to user.
                            match context {
                                OperationContext::DragKeys { .. }
                                | OperationContext::DragTangent { .. } => {
                                    // Ensure that the order of keys is correct.
                                    self.sort_keys();

                                    self.send_curve(ui);
                                }
                                OperationContext::BoxSelection { min, max, .. } => {
                                    let min = min.get();
                                    let max = max.get();

                                    let rect =
                                        Rect::new(min.x, min.y, max.x - min.x, max.y - min.y);

                                    let mut selection = FxHashSet::default();
                                    for key in self.key_container.keys() {
                                        if rect.contains(key.position) {
                                            selection.insert(key.id);
                                        }
                                    }

                                    if !selection.is_empty() {
                                        self.set_selection(
                                            Some(Selection::Keys { keys: selection }),
                                            ui,
                                        );
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    WidgetMessage::MouseDown { pos, button } => match button {
                        MouseButton::Left => {
                            let pick_result = self.pick(*pos);

                            if let Some(picked) = pick_result {
                                match picked {
                                    PickResult::Key(picked_key) => {
                                        if let Some(picked_key_id) = self
                                            .key_container
                                            .key_index_ref(picked_key)
                                            .map(|key| key.id)
                                        {
                                            if let Some(selection) = self.selection.as_mut() {
                                                match selection {
                                                    Selection::Keys { keys } => {
                                                        if ui.keyboard_modifiers().control {
                                                            keys.insert(picked_key_id);
                                                        }
                                                        if !keys.contains(&picked_key_id) {
                                                            self.set_selection(
                                                                Some(Selection::single_key(
                                                                    picked_key_id,
                                                                )),
                                                                ui,
                                                            );
                                                        }
                                                    }
                                                    Selection::LeftTangent { .. }
                                                    | Selection::RightTangent { .. } => self
                                                        .set_selection(
                                                            Some(Selection::single_key(
                                                                picked_key_id,
                                                            )),
                                                            ui,
                                                        ),
                                                }
                                            } else {
                                                self.set_selection(
                                                    Some(Selection::single_key(picked_key_id)),
                                                    ui,
                                                );
                                            }
                                        }
                                    }
                                    PickResult::LeftTangent(picked_key) => {
                                        self.set_selection(
                                            Some(Selection::LeftTangent { key: picked_key }),
                                            ui,
                                        );
                                    }
                                    PickResult::RightTangent(picked_key) => {
                                        self.set_selection(
                                            Some(Selection::RightTangent { key: picked_key }),
                                            ui,
                                        );
                                    }
                                }
                            } else {
                                self.set_selection(None, ui);
                            }
                        }
                        MouseButton::Middle => {
                            ui.capture_mouse(self.handle);
                            self.operation_context = Some(OperationContext::MoveView {
                                initial_mouse_pos: *pos,
                                initial_view_pos: self.view_position,
                            });
                        }
                        _ => (),
                    },
                    WidgetMessage::MouseWheel { amount, .. } => {
                        let k = if *amount < 0.0 { 0.9 } else { 1.1 };

                        let new_zoom = if ui.keyboard_modifiers().shift {
                            Vector2::new(self.zoom.x * k, self.zoom.y)
                        } else if ui.keyboard_modifiers.control {
                            Vector2::new(self.zoom.x, self.zoom.y * k)
                        } else {
                            self.zoom * k
                        };

                        ui.send_message(CurveEditorMessage::zoom(
                            self.handle,
                            MessageDirection::ToWidget,
                            new_zoom,
                        ));

                        message.set_handled(true);
                    }
                    _ => {}
                }
            } else if let Some(msg) = message.data::<CurveEditorMessage>() {
                if message.destination() == self.handle
                    && message.direction() == MessageDirection::ToWidget
                {
                    match msg {
                        CurveEditorMessage::Sync(curve) => {
                            self.key_container = KeyContainer::from(curve);
                        }
                        CurveEditorMessage::ViewPosition(view_position) => {
                            self.set_view_position(*view_position);
                            ui.send_message(message.reverse());
                        }
                        CurveEditorMessage::Zoom(zoom) => {
                            self.zoom = zoom.simd_clamp(self.min_zoom, self.max_zoom);
                            ui.send_message(message.reverse());
                        }
                        CurveEditorMessage::RemoveSelection => {
                            self.remove_selection(ui);
                        }
                        CurveEditorMessage::ChangeSelectedKeysKind(kind) => {
                            self.change_selected_keys_kind(kind.clone(), ui);
                        }
                        CurveEditorMessage::AddKey(screen_pos) => {
                            let local_pos = self.point_to_local_space(*screen_pos);
                            self.key_container.add(CurveKeyView {
                                position: local_pos,
                                kind: CurveKeyKind::Linear,
                                id: Uuid::new_v4(),
                            });
                            self.set_selection(None, ui);
                            self.sort_keys();
                            self.send_curve(ui);
                        }
                        CurveEditorMessage::ZoomToFit { after_layout } => {
                            if *after_layout {
                                // TODO: Layout system could take up to 10 frames in worst cases. This is super hackish solution
                                // but when it works, who cares.
                                self.zoom_to_fit_timer = Some(10);
                            } else {
                                self.zoom_to_fit(&ui.sender);
                            }
                        }
                        CurveEditorMessage::ChangeSelectedKeysValue(value) => {
                            self.change_selected_keys_value(*value, ui);
                        }
                        CurveEditorMessage::ChangeSelectedKeysLocation(location) => {
                            self.change_selected_keys_location(*location, ui);
                        }
                        CurveEditorMessage::HighlightZones(zones) => {
                            self.highlight_zones = zones.clone();
                        }
                    }
                }
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
            if message.destination() == self.context_menu.remove {
                ui.send_message(CurveEditorMessage::remove_selection(
                    self.handle,
                    MessageDirection::ToWidget,
                ));
            } else if message.destination() == self.context_menu.make_constant {
                ui.send_message(CurveEditorMessage::change_selected_keys_kind(
                    self.handle,
                    MessageDirection::ToWidget,
                    CurveKeyKind::Constant,
                ));
            } else if message.destination() == self.context_menu.make_linear {
                ui.send_message(CurveEditorMessage::change_selected_keys_kind(
                    self.handle,
                    MessageDirection::ToWidget,
                    CurveKeyKind::Linear,
                ));
            } else if message.destination() == self.context_menu.make_cubic {
                ui.send_message(CurveEditorMessage::change_selected_keys_kind(
                    self.handle,
                    MessageDirection::ToWidget,
                    CurveKeyKind::Cubic {
                        left_tangent: 0.0,
                        right_tangent: 0.0,
                    },
                ));
            } else if message.destination() == self.context_menu.add_key {
                let screen_pos = ui.node(self.context_menu.widget.handle()).screen_position();
                ui.send_message(CurveEditorMessage::add_key(
                    self.handle,
                    MessageDirection::ToWidget,
                    screen_pos,
                ));
            } else if message.destination() == self.context_menu.zoom_to_fit {
                ui.send_message(CurveEditorMessage::zoom_to_fit(
                    self.handle,
                    MessageDirection::ToWidget,
                    false,
                ));
            }
        } else if let Some(NumericUpDownMessage::<f32>::Value(value)) = message.data() {
            if message.direction() == MessageDirection::FromWidget && !message.handled() {
                if message.destination() == self.context_menu.key_value {
                    ui.send_message(CurveEditorMessage::change_selected_keys_value(
                        self.handle,
                        MessageDirection::ToWidget,
                        *value,
                    ));
                } else if message.destination() == self.context_menu.key_location {
                    ui.send_message(CurveEditorMessage::change_selected_keys_location(
                        self.handle,
                        MessageDirection::ToWidget,
                        *value,
                    ));
                }
            }
        }
    }

    fn update(&mut self, _dt: f32, ui: &mut UserInterface) {
        if let Some(timer) = self.zoom_to_fit_timer.as_mut() {
            *timer = timer.saturating_sub(1);
            if *timer == 0 {
                self.zoom_to_fit(&ui.sender);
                self.zoom_to_fit_timer = None;
            }
        }
    }
}

fn draw_cubic(
    left_pos: Vector2<f32>,
    left_tangent: f32,
    right_pos: Vector2<f32>,
    right_tangent: f32,
    steps: usize,
    ctx: &mut DrawingContext,
) {
    let mut prev = left_pos;
    for i in 0..steps {
        let t = i as f32 / (steps - 1) as f32;
        let middle_x = lerpf(left_pos.x, right_pos.x, t);
        let middle_y = cubicf(left_pos.y, right_pos.y, t, left_tangent, right_tangent);
        let pt = Vector2::new(middle_x, middle_y);
        ctx.push_line(prev, pt, 1.0);
        prev = pt;
    }
}

fn round_to_step(x: f32, step: f32) -> f32 {
    x - x % step
}

impl CurveEditor {
    #[allow(clippy::let_and_return)] // Improves readability
    fn set_view_position(&mut self, position: Vector2<f32>) {
        self.view_position = self.view_bounds.map_or(position, |bounds| {
            let local_space_position = -position;
            let clamped_local_space_position = Vector2::new(
                local_space_position
                    .x
                    .clamp(bounds.position.x, bounds.position.x + 2.0 * bounds.size.x),
                local_space_position
                    .y
                    .clamp(bounds.position.y, bounds.position.y + 2.0 * bounds.size.y),
            );
            let clamped_view_space = -clamped_local_space_position;
            clamped_view_space
        });
    }

    fn zoom_to_fit(&mut self, sender: &Sender<UiMessage>) {
        let bounds = if self.key_container.keys().is_empty() {
            Rect::new(-1.0, -1.0, 2.0, 2.0)
        } else {
            self.key_container.curve().bounds()
        };
        let center = bounds.center();

        sender
            .send(CurveEditorMessage::zoom(
                self.handle,
                MessageDirection::ToWidget,
                Vector2::new(
                    self.actual_local_size().x / bounds.w().max(5.0 * f32::EPSILON),
                    self.actual_local_size().y / bounds.h().max(5.0 * f32::EPSILON),
                ),
            ))
            .unwrap();

        sender
            .send(CurveEditorMessage::view_position(
                self.handle,
                MessageDirection::ToWidget,
                Vector2::new(
                    self.actual_local_size().x * 0.5 - center.x,
                    -self.actual_local_size().y * 0.5 + center.y,
                ),
            ))
            .unwrap();
    }

    fn update_matrices(&self) {
        let vp = Vector2::new(self.view_position.x, -self.view_position.y);
        self.view_matrix.set(
            Matrix3::new_nonuniform_scaling_wrt_point(
                &self.zoom,
                &Point2::from(self.actual_local_size().scale(0.5)),
            ) * Matrix3::new_translation(&vp),
        );

        let screen_bounds = self.screen_bounds();
        self.screen_matrix.set(
            Matrix3::new_translation(&screen_bounds.position)
                // Flip Y because in math origin is in lower left corner.
                * Matrix3::new_translation(&Vector2::new(0.0, screen_bounds.h()))
                * Matrix3::new_nonuniform_scaling(&Vector2::new(1.0, -1.0))
                * self.view_matrix.get(),
        );

        self.inv_screen_matrix
            .set(self.view_matrix.get().try_inverse().unwrap_or_default());
    }

    /// Transforms a point to view space.
    pub fn point_to_view_space(&self, point: Vector2<f32>) -> Vector2<f32> {
        self.view_matrix
            .get()
            .transform_point(&Point2::from(point))
            .coords
    }

    /// Transforms a point to screen space.
    pub fn point_to_screen_space(&self, point: Vector2<f32>) -> Vector2<f32> {
        self.screen_matrix
            .get()
            .transform_point(&Point2::from(point))
            .coords
    }

    /// Transforms a vector to screen space.
    pub fn vector_to_screen_space(&self, vector: Vector2<f32>) -> Vector2<f32> {
        (self.screen_matrix.get() * Vector3::new(vector.x, vector.y, 0.0)).xy()
    }

    /// Transforms a point to local space.
    pub fn point_to_local_space(&self, point: Vector2<f32>) -> Vector2<f32> {
        let mut p = point - self.screen_position();
        p.y = self.actual_local_size().y - p.y;
        self.view_matrix
            .get()
            .try_inverse()
            .unwrap_or_default()
            .transform_point(&Point2::from(p))
            .coords
    }

    fn sort_keys(&mut self) {
        self.key_container.sort_keys();
    }

    fn set_selection(&mut self, selection: Option<Selection>, ui: &UserInterface) {
        self.selection = selection;

        ui.send_message(WidgetMessage::enabled(
            self.context_menu.remove,
            MessageDirection::ToWidget,
            self.selection.is_some(),
        ));

        ui.send_message(WidgetMessage::enabled(
            self.context_menu.key,
            MessageDirection::ToWidget,
            self.selection.is_some(),
        ));

        ui.send_message(WidgetMessage::enabled(
            self.context_menu.key_properties,
            MessageDirection::ToWidget,
            self.selection.is_some(),
        ));

        if let Some(Selection::Keys { keys }) = self.selection.as_ref() {
            if let Some(first) = keys.iter().next() {
                if let Some(key) = self.key_container.key_ref(*first) {
                    ui.send_message(
                        NumericUpDownMessage::value(
                            self.context_menu.key_location,
                            MessageDirection::ToWidget,
                            key.position.x,
                        )
                        .with_handled(true),
                    );

                    ui.send_message(
                        NumericUpDownMessage::value(
                            self.context_menu.key_value,
                            MessageDirection::ToWidget,
                            key.position.y,
                        )
                        .with_handled(true),
                    );
                }
            }
        }
    }

    fn remove_selection(&mut self, ui: &mut UserInterface) {
        if let Some(Selection::Keys { keys }) = self.selection.as_ref() {
            for &id in keys {
                self.key_container.remove(id);
            }

            self.set_selection(None, ui);

            // Send modified curve back to user.
            self.send_curve(ui);
        }
    }

    fn change_selected_keys_kind(&mut self, kind: CurveKeyKind, ui: &mut UserInterface) {
        if let Some(Selection::Keys { keys }) = self.selection.as_ref() {
            for key in keys {
                if let Some(key) = self.key_container.key_mut(*key) {
                    key.kind = kind.clone();
                }
            }

            self.send_curve(ui);
        }
    }

    fn change_selected_keys_value(&mut self, value: f32, ui: &mut UserInterface) {
        if let Some(Selection::Keys { keys }) = self.selection.as_ref() {
            let mut modified = false;
            for key in keys {
                if let Some(key) = self.key_container.key_mut(*key) {
                    let key_value = &mut key.position.y;
                    if (*key_value).ne(&value) {
                        *key_value = value;
                        modified = true;
                    }
                }
            }

            if modified {
                self.send_curve(ui);
            }
        }
    }

    fn change_selected_keys_location(&mut self, location: f32, ui: &mut UserInterface) {
        if let Some(Selection::Keys { keys }) = self.selection.as_ref() {
            let mut modified = false;
            for key in keys {
                if let Some(key) = self.key_container.key_mut(*key) {
                    let key_location = &mut key.position.x;
                    if (*key_location).ne(&location) {
                        *key_location = location;
                        modified = true;
                    }
                }
            }

            if modified {
                self.send_curve(ui);
            }
        }
    }

    /// `pos` must be in screen space.
    fn pick(&self, pos: Vector2<f32>) -> Option<PickResult> {
        // Linear search is fine here, having a curve with thousands of
        // points is insane anyway.
        for (i, key) in self.key_container.keys().iter().enumerate() {
            let screen_pos = self.point_to_screen_space(key.position);
            let bounds = Rect::new(
                screen_pos.x - self.key_size * 0.5,
                screen_pos.y - self.key_size * 0.5,
                self.key_size,
                self.key_size,
            );
            if bounds.contains(pos) {
                return Some(PickResult::Key(i));
            }

            // Check tangents.
            if let CurveKeyKind::Cubic {
                left_tangent,
                right_tangent,
            } = key.kind
            {
                let left_handle_pos = self.tangent_screen_position(
                    wrap_angle(left_tangent.atan()) + std::f32::consts::PI,
                    key.position,
                );

                if (left_handle_pos - pos).norm() <= self.key_size * 0.5 {
                    return Some(PickResult::LeftTangent(i));
                }

                let right_handle_pos =
                    self.tangent_screen_position(wrap_angle(right_tangent.atan()), key.position);

                if (right_handle_pos - pos).norm() <= self.key_size * 0.5 {
                    return Some(PickResult::RightTangent(i));
                }
            }
        }
        None
    }

    fn tangent_screen_position(&self, angle: f32, key_position: Vector2<f32>) -> Vector2<f32> {
        self.point_to_screen_space(key_position)
            + Vector2::new(angle.cos(), angle.sin()).scale(self.handle_radius)
    }

    fn send_curve(&self, ui: &UserInterface) {
        ui.send_message(CurveEditorMessage::sync(
            self.handle,
            MessageDirection::FromWidget,
            self.key_container.curve(),
        ));
    }

    fn draw_background(&self, ctx: &mut DrawingContext) {
        let screen_bounds = self.screen_bounds();
        // Draw background.
        ctx.push_rect_filled(&screen_bounds, None);
        ctx.commit(
            self.clip_bounds(),
            self.background(),
            CommandTexture::None,
            None,
        );
    }

    fn draw_highlight_zones(&self, ctx: &mut DrawingContext) {
        for zone in self.highlight_zones.iter() {
            let left_top_corner = self.point_to_screen_space(zone.rect.left_top_corner());
            let bottom_right_corner = self.point_to_screen_space(zone.rect.right_bottom_corner());
            ctx.push_rect_filled(
                &Rect::new(
                    left_top_corner.x,
                    left_top_corner.y,
                    bottom_right_corner.x - left_top_corner.x,
                    bottom_right_corner.y - left_top_corner.y,
                ),
                None,
            );
            ctx.commit(
                self.clip_bounds(),
                zone.brush.clone(),
                CommandTexture::None,
                None,
            );
        }
    }

    fn draw_grid(&self, ctx: &mut DrawingContext) {
        let screen_bounds = self.screen_bounds();

        let step_size_x = self.grid_size.x / self.zoom.x;
        let step_size_y = self.grid_size.y / self.zoom.y;

        let mut local_left_bottom = self.point_to_local_space(screen_bounds.left_top_corner());
        let local_left_bottom_n = local_left_bottom;
        local_left_bottom.x = round_to_step(local_left_bottom.x, step_size_x);
        local_left_bottom.y = round_to_step(local_left_bottom.y, step_size_y);

        let mut local_right_top = self.point_to_local_space(screen_bounds.right_bottom_corner());
        local_right_top.x = round_to_step(local_right_top.x, step_size_x);
        local_right_top.y = round_to_step(local_right_top.y, step_size_y);

        let w = (local_right_top.x - local_left_bottom.x).abs();
        let h = (local_right_top.y - local_left_bottom.y).abs();

        let nw = ((w / step_size_x).ceil()) as usize;
        let nh = ((h / step_size_y).ceil()) as usize;

        for ny in 0..=nh {
            let k = ny as f32 / (nh) as f32;
            let y = local_left_bottom.y - k * h;
            ctx.push_line(
                self.point_to_screen_space(Vector2::new(local_left_bottom.x - step_size_x, y)),
                self.point_to_screen_space(Vector2::new(local_right_top.x + step_size_x, y)),
                1.0,
            );
        }

        for nx in 0..=nw {
            let k = nx as f32 / (nw) as f32;
            let x = local_left_bottom.x + k * w;
            ctx.push_line(
                self.point_to_screen_space(Vector2::new(x, local_left_bottom.y + step_size_y)),
                self.point_to_screen_space(Vector2::new(x, local_right_top.y - step_size_y)),
                1.0,
            );
        }

        // Draw main axes.
        let vb = self.point_to_screen_space(Vector2::new(0.0, -10e6));
        let ve = self.point_to_screen_space(Vector2::new(0.0, 10e6));
        ctx.push_line(vb, ve, 2.0);

        let hb = self.point_to_screen_space(Vector2::new(-10e6, 0.0));
        let he = self.point_to_screen_space(Vector2::new(10e6, 0.0));
        ctx.push_line(hb, he, 2.0);

        ctx.commit(
            self.clip_bounds(),
            self.grid_brush.clone(),
            CommandTexture::None,
            None,
        );

        // Draw values.
        let mut text = self.text.borrow_mut();

        if self.show_y_values {
            for ny in 0..=nh {
                let k = ny as f32 / (nh) as f32;
                let y = local_left_bottom.y - k * h;
                text.set_text(format!("{:.1}", y)).build();
                ctx.draw_text(
                    self.clip_bounds(),
                    self.point_to_screen_space(Vector2::new(local_left_bottom_n.x, y)),
                    &text,
                );
            }
        }

        if self.show_x_values {
            for nx in 0..=nw {
                let k = nx as f32 / (nw) as f32;
                let x = local_left_bottom.x + k * w;
                text.set_text(format!("{:.1}", x)).build();
                ctx.draw_text(
                    self.clip_bounds(),
                    self.point_to_screen_space(Vector2::new(x, local_left_bottom_n.y)),
                    &text,
                );
            }
        }
    }

    fn draw_curve(&self, ctx: &mut DrawingContext) {
        let screen_bounds = self.screen_bounds();
        let draw_keys = self.key_container.keys();

        if let Some(first) = draw_keys.first() {
            let screen_pos = self.point_to_screen_space(first.position);
            ctx.push_line(Vector2::new(0.0, screen_pos.y), screen_pos, 1.0);
        }
        if let Some(last) = draw_keys.last() {
            let screen_pos = self.point_to_screen_space(last.position);
            ctx.push_line(
                screen_pos,
                Vector2::new(screen_bounds.x() + screen_bounds.w(), screen_pos.y),
                1.0,
            );
        }

        for pair in draw_keys.windows(2) {
            let left = &pair[0];
            let right = &pair[1];

            let left_pos = self.point_to_screen_space(left.position);
            let right_pos = self.point_to_screen_space(right.position);

            let steps = ((right_pos.x - left_pos.x).abs() / 2.0) as usize;

            match (&left.kind, &right.kind) {
                // Constant-to-any is depicted as two straight lines.
                (CurveKeyKind::Constant, CurveKeyKind::Constant)
                | (CurveKeyKind::Constant, CurveKeyKind::Linear)
                | (CurveKeyKind::Constant, CurveKeyKind::Cubic { .. }) => {
                    ctx.push_line(left_pos, Vector2::new(right_pos.x, left_pos.y), 1.0);
                    ctx.push_line(Vector2::new(right_pos.x, left_pos.y), right_pos, 1.0);
                }

                // Linear-to-any is depicted as a straight line.
                (CurveKeyKind::Linear, CurveKeyKind::Constant)
                | (CurveKeyKind::Linear, CurveKeyKind::Linear)
                | (CurveKeyKind::Linear, CurveKeyKind::Cubic { .. }) => {
                    ctx.push_line(left_pos, right_pos, 1.0)
                }

                // Cubic-to-constant and cubic-to-linear is depicted as Hermite spline with right tangent == 0.0.
                (
                    CurveKeyKind::Cubic {
                        right_tangent: left_tangent,
                        ..
                    },
                    CurveKeyKind::Constant,
                )
                | (
                    CurveKeyKind::Cubic {
                        right_tangent: left_tangent,
                        ..
                    },
                    CurveKeyKind::Linear,
                ) => draw_cubic(left_pos, *left_tangent, right_pos, 0.0, steps, ctx),

                // Cubic-to-cubic is depicted as Hermite spline.
                (
                    CurveKeyKind::Cubic {
                        right_tangent: left_tangent,
                        ..
                    },
                    CurveKeyKind::Cubic {
                        left_tangent: right_tangent,
                        ..
                    },
                ) => draw_cubic(
                    left_pos,
                    *left_tangent,
                    right_pos,
                    *right_tangent,
                    steps,
                    ctx,
                ),
            }
        }
        ctx.commit(
            self.clip_bounds(),
            self.foreground(),
            CommandTexture::None,
            None,
        );
    }

    fn draw_keys(&self, ctx: &mut DrawingContext) {
        let keys_to_draw = self.key_container.keys();

        for (i, key) in keys_to_draw.iter().enumerate() {
            let origin = self.point_to_screen_space(key.position);
            let size = Vector2::new(self.key_size, self.key_size);
            let half_size = size.scale(0.5);

            ctx.push_rect_filled(
                &Rect::new(
                    origin.x - half_size.x,
                    origin.y - half_size.x,
                    size.x,
                    size.y,
                ),
                None,
            );

            let mut selected = false;
            if let Some(selection) = self.selection.as_ref() {
                match selection {
                    Selection::Keys { keys } => {
                        selected = keys.contains(&key.id);
                    }
                    Selection::LeftTangent { key } | Selection::RightTangent { key } => {
                        selected = i == *key;
                    }
                }
            }

            // Show tangents for Cubic keys.
            if selected {
                let (show_left, show_right) = match keys_to_draw.get(i.wrapping_sub(1)) {
                    Some(left) => match (&left.kind, &key.kind) {
                        (CurveKeyKind::Cubic { .. }, CurveKeyKind::Cubic { .. }) => (true, true),
                        (CurveKeyKind::Linear, CurveKeyKind::Cubic { .. })
                        | (CurveKeyKind::Constant, CurveKeyKind::Cubic { .. }) => (false, true),
                        _ => (false, false),
                    },
                    None => match key.kind {
                        CurveKeyKind::Cubic { .. } => (false, true),
                        _ => (false, false),
                    },
                };

                if let CurveKeyKind::Cubic {
                    left_tangent,
                    right_tangent,
                } = key.kind
                {
                    if show_left {
                        let left_handle_pos = self.tangent_screen_position(
                            wrap_angle(left_tangent.atan()) + std::f32::consts::PI,
                            key.position,
                        );
                        ctx.push_line(origin, left_handle_pos, 1.0);
                        ctx.push_circle_filled(
                            left_handle_pos,
                            self.key_size * 0.5,
                            6,
                            Default::default(),
                        );
                    }

                    if show_right {
                        let right_handle_pos = self.tangent_screen_position(
                            wrap_angle(right_tangent.atan()),
                            key.position,
                        );
                        ctx.push_line(origin, right_handle_pos, 1.0);
                        ctx.push_circle_filled(
                            right_handle_pos,
                            self.key_size * 0.5,
                            6,
                            Default::default(),
                        );
                    }
                }
            }

            ctx.commit(
                self.clip_bounds(),
                if selected {
                    self.selected_key_brush.clone()
                } else {
                    self.key_brush.clone()
                },
                CommandTexture::None,
                None,
            );
        }
    }

    fn draw_operation(&self, ctx: &mut DrawingContext) {
        if let Some(OperationContext::BoxSelection { min, max, .. }) =
            self.operation_context.as_ref()
        {
            let min = self.point_to_screen_space(min.get());
            let max = self.point_to_screen_space(max.get());
            let rect = Rect::new(min.x, min.y, max.x - min.x, max.y - min.y);

            ctx.push_rect(&rect, 1.0);
            ctx.commit(
                self.clip_bounds(),
                Brush::Solid(Color::WHITE),
                CommandTexture::None,
                None,
            );
        }
    }
}

pub struct CurveEditorBuilder {
    widget_builder: WidgetBuilder,
    curve: Curve,
    view_position: Vector2<f32>,
    zoom: f32,
    view_bounds: Option<Rect<f32>>,
    show_x_values: bool,
    show_y_values: bool,
    grid_size: Vector2<f32>,
    min_zoom: Vector2<f32>,
    max_zoom: Vector2<f32>,
    highlight_zones: Vec<HighlightZone>,
}

impl CurveEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            curve: Default::default(),
            view_position: Default::default(),
            zoom: 1.0,
            view_bounds: None,
            show_x_values: true,
            show_y_values: true,
            grid_size: Vector2::new(50.0, 50.0),
            min_zoom: Vector2::new(0.001, 0.001),
            max_zoom: Vector2::new(1000.0, 1000.0),
            highlight_zones: Default::default(),
        }
    }

    pub fn with_curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }

    pub fn with_zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom;
        self
    }

    pub fn with_view_position(mut self, view_position: Vector2<f32>) -> Self {
        self.view_position = view_position;
        self
    }

    pub fn with_show_x_values(mut self, show_x_values: bool) -> Self {
        self.show_x_values = show_x_values;
        self
    }

    pub fn with_show_y_values(mut self, show_y_values: bool) -> Self {
        self.show_y_values = show_y_values;
        self
    }

    /// View bounds in value-space.
    pub fn with_view_bounds(mut self, bounds: Rect<f32>) -> Self {
        self.view_bounds = Some(bounds);
        self
    }

    pub fn with_grid_size(mut self, size: Vector2<f32>) -> Self {
        self.grid_size = size;
        self
    }

    pub fn with_min_zoom(mut self, min_zoom: Vector2<f32>) -> Self {
        self.min_zoom = min_zoom;
        self
    }

    pub fn with_max_zoom(mut self, max_zoom: Vector2<f32>) -> Self {
        self.max_zoom = max_zoom;
        self
    }

    pub fn with_highlight_zone(mut self, zones: Vec<HighlightZone>) -> Self {
        self.highlight_zones = zones;
        self
    }

    pub fn build(mut self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let keys = KeyContainer::from(&self.curve);

        let add_key;
        let remove;
        let make_constant;
        let make_linear;
        let make_cubic;
        let key;
        let zoom_to_fit;
        let key_properties;
        let key_value;
        let key_location;
        let context_menu = PopupBuilder::new(WidgetBuilder::new())
            .with_content(
                StackPanelBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            key_properties = GridBuilder::new(
                                WidgetBuilder::new()
                                    .with_enabled(false)
                                    .with_child(
                                        TextBuilder::new(
                                            WidgetBuilder::new()
                                                .with_vertical_alignment(VerticalAlignment::Center)
                                                .with_margin(Thickness::uniform(1.0))
                                                .on_row(0)
                                                .on_column(0),
                                        )
                                        .with_text("Location")
                                        .build(ctx),
                                    )
                                    .with_child({
                                        key_location = NumericUpDownBuilder::<f32>::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0))
                                                .on_row(0)
                                                .on_column(1),
                                        )
                                        .build(ctx);
                                        key_location
                                    })
                                    .with_child(
                                        TextBuilder::new(
                                            WidgetBuilder::new()
                                                .with_vertical_alignment(VerticalAlignment::Center)
                                                .with_margin(Thickness::uniform(1.0))
                                                .on_row(1)
                                                .on_column(0),
                                        )
                                        .with_text("Value")
                                        .build(ctx),
                                    )
                                    .with_child({
                                        key_value = NumericUpDownBuilder::<f32>::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0))
                                                .on_row(1)
                                                .on_column(1),
                                        )
                                        .build(ctx);
                                        key_value
                                    }),
                            )
                            .add_column(Column::auto())
                            .add_column(Column::stretch())
                            .add_row(Row::strict(22.0))
                            .add_row(Row::strict(22.0))
                            .build(ctx);
                            key_properties
                        })
                        .with_child({
                            add_key = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text("Add Key"))
                                .build(ctx);
                            add_key
                        })
                        .with_child({
                            remove = MenuItemBuilder::new(WidgetBuilder::new().with_enabled(false))
                                .with_content(MenuItemContent::text("Remove"))
                                .build(ctx);
                            remove
                        })
                        .with_child({
                            key = MenuItemBuilder::new(WidgetBuilder::new().with_enabled(false))
                                .with_content(MenuItemContent::text("Key..."))
                                .with_items(vec![
                                    {
                                        make_constant = MenuItemBuilder::new(WidgetBuilder::new())
                                            .with_content(MenuItemContent::text("Constant"))
                                            .build(ctx);
                                        make_constant
                                    },
                                    {
                                        make_linear = MenuItemBuilder::new(WidgetBuilder::new())
                                            .with_content(MenuItemContent::text("Linear"))
                                            .build(ctx);
                                        make_linear
                                    },
                                    {
                                        make_cubic = MenuItemBuilder::new(WidgetBuilder::new())
                                            .with_content(MenuItemContent::text("Cubic"))
                                            .build(ctx);
                                        make_cubic
                                    },
                                ])
                                .build(ctx);
                            key
                        })
                        .with_child({
                            zoom_to_fit = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text("Zoom To Fit"))
                                .build(ctx);
                            zoom_to_fit
                        }),
                )
                .build(ctx),
            )
            .build(ctx);
        let context_menu = RcUiNodeHandle::new(context_menu, ctx.sender());

        if self.widget_builder.foreground.is_none() {
            self.widget_builder.foreground = Some(Brush::Solid(Color::opaque(130, 130, 130)))
        }

        let editor = CurveEditor {
            widget: self
                .widget_builder
                .with_context_menu(context_menu.clone())
                .with_preview_messages(true)
                .with_need_update(true)
                .build(),
            key_container: keys,
            zoom: Vector2::new(1.0, 1.0),
            view_position: Default::default(),
            view_matrix: Default::default(),
            screen_matrix: Default::default(),
            inv_screen_matrix: Default::default(),
            key_brush: Brush::Solid(Color::opaque(140, 140, 140)),
            selected_key_brush: Brush::Solid(Color::opaque(220, 220, 220)),
            key_size: 8.0,
            handle_radius: 36.0,
            operation_context: None,
            grid_brush: Brush::Solid(Color::from_rgba(110, 110, 110, 50)),
            selection: None,
            text: RefCell::new(
                FormattedTextBuilder::new(ctx.default_font())
                    .with_brush(Brush::Solid(Color::opaque(100, 100, 100)))
                    .build(),
            ),
            context_menu: ContextMenu {
                widget: context_menu,
                add_key,
                remove,
                make_constant,
                make_linear,
                make_cubic,
                key,
                zoom_to_fit,
                key_properties,
                key_value,
                key_location,
            },
            view_bounds: self.view_bounds,
            show_x_values: self.show_x_values,
            show_y_values: self.show_y_values,
            grid_size: self.grid_size,
            min_zoom: self.min_zoom,
            max_zoom: self.max_zoom,
            highlight_zones: self.highlight_zones,
            zoom_to_fit_timer: None,
        };

        ctx.add_node(UiNode::new(editor))
    }
}
