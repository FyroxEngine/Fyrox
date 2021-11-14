use crate::{
    brush::Brush,
    core::{
        algebra::{Matrix3, Point2, Vector2, Vector3},
        color::Color,
        curve::{Curve, CurveKeyKind},
        math::{cubicf, inf_sup_cubicf, lerpf, wrap_angle, Rect},
        pool::Handle,
        uuid::Uuid,
    },
    curve::key::{CurveKeyView, KeyContainer},
    define_constructor,
    draw::{CommandTexture, Draw, DrawingContext},
    formatted_text::{FormattedText, FormattedTextBuilder},
    menu::{MenuItemBuilder, MenuItemContent, MenuItemMessage},
    message::{ButtonState, KeyCode, MessageDirection, MouseButton, UiMessage},
    popup::PopupBuilder,
    stack_panel::StackPanelBuilder,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, UiNode, UserInterface,
};
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    ops::{Deref, DerefMut},
};

pub mod key;

#[derive(Debug, Clone, PartialEq)]
pub enum CurveEditorMessage {
    Sync(Curve),
    ViewPosition(Vector2<f32>),
    Zoom(f32),
    ZoomToFit,

    // Internal messages. Use only when you know what you're doing.
    // These are internal because you must use Sync message to request changes
    // in the curve editor.
    ChangeSelectedKeysKind(CurveKeyKind),
    RemoveSelection,
    // Position in screen coordinates.
    AddKey(Vector2<f32>),
}

impl CurveEditorMessage {
    define_constructor!(CurveEditorMessage:Sync => fn sync(Curve), layout: false);
    define_constructor!(CurveEditorMessage:ViewPosition => fn view_position(Vector2<f32>), layout: false);
    define_constructor!(CurveEditorMessage:Zoom => fn zoom(f32), layout: false);
    define_constructor!(CurveEditorMessage:ZoomToFit => fn zoom_to_fit(), layout: false);
    // Internal. Use only when you know what you're doing.
    define_constructor!(CurveEditorMessage:RemoveSelection => fn remove_selection(), layout: false);
    define_constructor!(CurveEditorMessage:ChangeSelectedKeysKind => fn change_selected_keys_kind(CurveKeyKind), layout: false);
    define_constructor!(CurveEditorMessage:AddKey => fn add_key(Vector2<f32>), layout: false);
}

#[derive(Clone)]
pub struct CurveEditor {
    widget: Widget,
    key_container: KeyContainer,
    zoom: f32,
    view_position: Vector2<f32>,
    // Transforms a point from local to view coordinates.
    view_matrix: Cell<Matrix3<f32>>,
    // Transforms a point from local to screen coordinates.
    // View and screen coordinates are different:
    //  - view is a local viewer of curve editor
    //  - screen is global space
    screen_matrix: Cell<Matrix3<f32>>,
    // Transform a point from screen space (i.e. mouse position) to the
    // local space (the space where all keys are)
    inv_screen_matrix: Cell<Matrix3<f32>>,
    key_brush: Brush,
    selected_key_brush: Brush,
    key_size: f32,
    grid_brush: Brush,
    operation_context: Option<OperationContext>,
    selection: Option<Selection>,
    handle_radius: f32,
    context_menu: ContextMenu,
    text: RefCell<FormattedText>,
}

crate::define_widget_deref!(CurveEditor);

#[derive(Clone)]
struct ContextMenu {
    widget: Handle<UiNode>,
    add_key: Handle<UiNode>,
    remove: Handle<UiNode>,
    key: Handle<UiNode>,
    make_constant: Handle<UiNode>,
    make_linear: Handle<UiNode>,
    make_cubic: Handle<UiNode>,
    zoom_to_fit: Handle<UiNode>,
}

#[derive(Clone)]
struct DragEntry {
    key: Uuid,
    initial_position: Vector2<f32>,
}

#[derive(Clone)]
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

#[derive(Clone)]
enum Selection {
    Keys { keys: HashSet<Uuid> },
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
        let mut keys = HashSet::new();
        keys.insert(key);
        Self::Keys { keys }
    }
}

impl Control for CurveEditor {
    fn draw(&self, ctx: &mut DrawingContext) {
        self.update_matrices();
        self.draw_background(ctx);
        self.draw_grid(ctx);
        self.draw_curve(ctx);
        self.draw_keys(ctx);
        self.draw_operation(ctx);
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
                                        let key = self.key_container.key_mut(entry.key).unwrap();
                                        key.position = entry.initial_position + local_delta;
                                    }
                                    self.sort_keys();
                                }
                                OperationContext::MoveView {
                                    initial_mouse_pos,
                                    initial_view_pos,
                                } => {
                                    let delta = (pos - initial_mouse_pos).scale(1.0 / self.zoom);
                                    self.view_position = initial_view_pos + delta;
                                }
                                OperationContext::DragTangent { key, left } => {
                                    let key_pos =
                                        self.key_container.key_index_ref(*key).unwrap().position;
                                    let screen_key_pos = self.point_to_screen_space(key_pos);
                                    let key = self.key_container.key_index_mut(*key).unwrap();
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
                                                        .unwrap()
                                                        .position,
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

                                if self.operation_context.is_some() {
                                    ui.capture_mouse(self.handle);
                                }
                            } else {
                                self.operation_context = Some(OperationContext::BoxSelection {
                                    initial_mouse_pos: local_mouse_pos,
                                    min: Default::default(),
                                    max: Default::default(),
                                })
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

                                    let mut selection = HashSet::default();
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
                                        let picked_key_id = self
                                            .key_container
                                            .key_index_ref(picked_key)
                                            .unwrap()
                                            .id;
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
                                                        Some(Selection::single_key(picked_key_id)),
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
                        ui.send_message(CurveEditorMessage::zoom(
                            self.handle,
                            MessageDirection::ToWidget,
                            self.zoom * k,
                        ));
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
                            self.view_position = *view_position;
                        }
                        CurveEditorMessage::Zoom(zoom) => {
                            self.zoom = *zoom;
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
                        }
                        CurveEditorMessage::ZoomToFit => {
                            let mut max_y = -f32::MAX;
                            let mut min_y = f32::MAX;
                            let mut max_x = -f32::MAX;
                            let mut min_x = f32::MAX;

                            let mut push = |x: f32, y: f32| {
                                if x > max_x {
                                    max_x = x;
                                }
                                if x < min_x {
                                    min_x = x;
                                }
                                if y > max_y {
                                    max_y = y;
                                }
                                if y < min_y {
                                    min_y = y;
                                }
                            };

                            for keys in self.key_container.keys().windows(2) {
                                let left = &keys[0];
                                let right = &keys[1];
                                match (&left.kind, &right.kind) {
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
                                    ) => {
                                        let (y0, y1) = inf_sup_cubicf(
                                            left.position.y,
                                            right.position.y,
                                            *left_tangent,
                                            0.0,
                                        );
                                        push(left.position.x, y0);
                                        push(right.position.x, y1);
                                    }

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
                                    ) => {
                                        let (y0, y1) = inf_sup_cubicf(
                                            left.position.y,
                                            right.position.y,
                                            *left_tangent,
                                            *right_tangent,
                                        );
                                        push(left.position.x, y0);
                                        push(right.position.x, y1);
                                    }
                                    _ => {
                                        push(left.position.x, left.position.y);
                                        push(right.position.x, right.position.y);
                                    }
                                }
                            }

                            let min = Vector2::new(min_x, min_y);
                            let max = Vector2::new(max_x, max_y);
                            let center = (min + max).scale(0.5);

                            let mut offset = self.actual_size().scale(0.5 * self.zoom);
                            offset.y *= -1.0;
                            self.view_position = center + offset;
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
                let screen_pos = ui.node(self.context_menu.widget).screen_position();
                ui.send_message(CurveEditorMessage::add_key(
                    self.handle,
                    MessageDirection::ToWidget,
                    screen_pos,
                ));
            } else if message.destination() == self.context_menu.zoom_to_fit {
                ui.send_message(CurveEditorMessage::zoom_to_fit(
                    self.handle,
                    MessageDirection::ToWidget,
                ));
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
    fn update_matrices(&self) {
        let vp = Vector2::new(self.view_position.x, -self.view_position.y);
        self.view_matrix.set(
            Matrix3::new_nonuniform_scaling_wrt_point(
                &Vector2::new(self.zoom, self.zoom),
                &Point2::from(self.actual_size().scale(0.5)),
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
        p.y = self.actual_size().y - p.y;
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
                self.key_container.key_mut(*key).unwrap().kind = kind.clone();
            }

            self.send_curve(ui);
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
        ctx.commit(screen_bounds, self.background(), CommandTexture::None, None);
    }

    fn draw_grid(&self, ctx: &mut DrawingContext) {
        let screen_bounds = self.screen_bounds();

        let step_size = 50.0 / self.zoom.clamp(0.001, 1000.0);

        let mut local_left_bottom = self.point_to_local_space(screen_bounds.left_top_corner());
        let local_left_bottom_n = local_left_bottom;
        local_left_bottom.x = round_to_step(local_left_bottom.x, step_size);
        local_left_bottom.y = round_to_step(local_left_bottom.y, step_size);

        let mut local_right_top = self.point_to_local_space(screen_bounds.right_bottom_corner());
        local_right_top.x = round_to_step(local_right_top.x, step_size);
        local_right_top.y = round_to_step(local_right_top.y, step_size);

        let w = (local_right_top.x - local_left_bottom.x).abs();
        let h = (local_right_top.y - local_left_bottom.y).abs();

        let nw = ((w / step_size).ceil()) as usize;
        let nh = ((h / step_size).ceil()) as usize;

        for ny in 0..=nh {
            let k = ny as f32 / (nh) as f32;
            let y = local_left_bottom.y - k * h;
            ctx.push_line(
                self.point_to_screen_space(Vector2::new(local_left_bottom.x - step_size, y)),
                self.point_to_screen_space(Vector2::new(local_right_top.x + step_size, y)),
                1.0,
            );
        }

        for nx in 0..=nw {
            let k = nx as f32 / (nw) as f32;
            let x = local_left_bottom.x + k * w;
            ctx.push_line(
                self.point_to_screen_space(Vector2::new(x, local_left_bottom.y + step_size)),
                self.point_to_screen_space(Vector2::new(x, local_right_top.y - step_size)),
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
            screen_bounds,
            self.grid_brush.clone(),
            CommandTexture::None,
            None,
        );

        // Draw values.
        let mut text = self.text.borrow_mut();
        for ny in 0..=nh {
            let k = ny as f32 / (nh) as f32;
            let y = local_left_bottom.y - k * h;
            text.set_text(format!("{:.1}", y)).build();
            ctx.draw_text(
                screen_bounds,
                self.point_to_screen_space(Vector2::new(local_left_bottom_n.x, y)),
                &text,
            );
        }

        for nx in 0..=nw {
            let k = nx as f32 / (nw) as f32;
            let x = local_left_bottom.x + k * w;
            text.set_text(format!("{:.1}", x)).build();
            ctx.draw_text(
                screen_bounds,
                self.point_to_screen_space(Vector2::new(x, 0.0)),
                &text,
            );
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
        ctx.commit(screen_bounds, self.foreground(), CommandTexture::None, None);
    }

    fn draw_keys(&self, ctx: &mut DrawingContext) {
        let screen_bounds = self.screen_bounds();
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
                        ctx.push_circle(
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
                        ctx.push_circle(
                            right_handle_pos,
                            self.key_size * 0.5,
                            6,
                            Default::default(),
                        );
                    }
                }
            }

            ctx.commit(
                screen_bounds,
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
                self.screen_bounds(),
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
}

impl CurveEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            curve: Default::default(),
            view_position: Default::default(),
            zoom: 1.0,
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

    pub fn build(mut self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let keys = KeyContainer::from(&self.curve);

        let add_key;
        let remove;
        let make_constant;
        let make_linear;
        let make_cubic;
        let key;
        let zoom_to_fit;
        let context_menu = PopupBuilder::new(WidgetBuilder::new())
            .with_content(
                StackPanelBuilder::new(
                    WidgetBuilder::new()
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

        if self.widget_builder.foreground.is_none() {
            self.widget_builder.foreground = Some(Brush::Solid(Color::opaque(130, 130, 130)))
        }

        let editor = CurveEditor {
            widget: self
                .widget_builder
                .with_context_menu(context_menu)
                .with_preview_messages(true)
                .build(),
            key_container: keys,
            zoom: 1.0,
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
                FormattedTextBuilder::new()
                    .with_brush(Brush::Solid(Color::opaque(100, 100, 100)))
                    .with_font(crate::DEFAULT_FONT.clone())
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
            },
        };

        ctx.add_node(UiNode::new(editor))
    }
}
