use crate::formatted_text::{FormattedText, FormattedTextBuilder};
use crate::{
    brush::Brush,
    core::{
        algebra::{Matrix3, Point2, Vector2, Vector3},
        color::Color,
        curve::{Curve, CurveKey, CurveKeyKind},
        math::{cubicf, lerpf, wrap_angle, Rect},
        pool::Handle,
        uuid::Uuid,
    },
    curve::key::CurveKeyView,
    draw::{CommandTexture, Draw, DrawingContext},
    menu::{MenuItemBuilder, MenuItemContent},
    message::{
        ButtonState, CurveEditorMessage, KeyCode, MenuItemMessage, MessageData, MessageDirection,
        MouseButton, UiMessage, UiMessageData, WidgetMessage,
    },
    popup::PopupBuilder,
    stack_panel::StackPanelBuilder,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UINode, UserInterface,
};
use std::cell::RefCell;
use std::{
    cell::Cell,
    cmp::Ordering,
    collections::HashSet,
    ops::{Deref, DerefMut},
};

pub mod key;

#[derive(Clone)]
pub struct CurveEditor<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    keys: Vec<CurveKeyView>,
    draw_keys: RefCell<Vec<CurveKeyView>>,
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
    context_menu: ContextMenu<M, C>,
    text: RefCell<FormattedText>,
}

crate::define_widget_deref!(CurveEditor<M, C>);

#[derive(Clone)]
struct ContextMenu<M: MessageData, C: Control<M, C>> {
    widget: Handle<UINode<M, C>>,
    add_key: Handle<UINode<M, C>>,
    remove: Handle<UINode<M, C>>,
    key: Handle<UINode<M, C>>,
    make_constant: Handle<UINode<M, C>>,
    make_linear: Handle<UINode<M, C>>,
    make_cubic: Handle<UINode<M, C>>,
}

#[derive(Clone)]
struct DragEntry {
    key: usize,
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
}

#[derive(Clone)]
enum Selection {
    Keys { keys: HashSet<usize> },
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
    fn single_key(key: usize) -> Self {
        let mut keys = HashSet::new();
        keys.insert(key);
        Self::Keys { keys }
    }
}

impl<M: MessageData, C: Control<M, C>> Control<M, C> for CurveEditor<M, C> {
    fn draw(&self, ctx: &mut DrawingContext) {
        // We use separate array for drawing which syncs with main array of keys.
        // This is needed to be able to sort keys by their location while
        // preserve original keys location in memory.
        let mut draw_keys = self.draw_keys.borrow_mut();
        draw_keys.clear();
        draw_keys.extend_from_slice(&self.keys);
        sort_keys(&mut draw_keys);
        drop(draw_keys);

        self.update_matrices();
        self.draw_background(ctx);
        self.draw_grid(ctx);
        self.draw_curve(ctx);
        self.draw_keys(ctx);
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle {
            match message.data() {
                UiMessageData::Widget(msg) => match msg {
                    WidgetMessage::KeyUp(key) => {
                        if let KeyCode::Delete = key {
                            self.remove_selection(ui);
                        }
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
                                        let key = &mut self.keys[entry.key];
                                        key.position = entry.initial_position + local_delta;
                                    }
                                }
                                OperationContext::MoveView {
                                    initial_mouse_pos,
                                    initial_view_pos,
                                } => {
                                    let delta = (pos - initial_mouse_pos).scale(1.0 / self.zoom);
                                    self.view_position = initial_view_pos + delta;
                                }
                                OperationContext::DragTangent { key, left } => {
                                    let key_pos = self.keys[*key].position;
                                    let screen_key_pos = self.point_to_screen_space(key_pos);
                                    let key = &mut self.keys[*key];
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
                        } else if let Some(selection) = self.selection.as_ref() {
                            if state.left == ButtonState::Pressed {
                                match selection {
                                    Selection::Keys { keys } => {
                                        self.operation_context = Some(OperationContext::DragKeys {
                                            entries: keys
                                                .iter()
                                                .map(|k| DragEntry {
                                                    key: *k,
                                                    initial_position: self.keys[*k].position,
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
                            }
                        }
                    }
                    WidgetMessage::MouseUp { .. } => {
                        if let Some(context) = self.operation_context.take() {
                            ui.release_mouse_capture();

                            // Send modified curve back to user.
                            if let OperationContext::DragKeys { .. }
                            | OperationContext::DragTangent { .. } = context
                            {
                                // Ensure that the order of keys is correct.
                                self.sort_keys();

                                self.send_curve(ui);
                            }
                        }
                    }
                    WidgetMessage::MouseDown { pos, button } => match button {
                        MouseButton::Left => {
                            let pick_result = self.pick(*pos);

                            if let Some(picked) = pick_result {
                                match picked {
                                    PickResult::Key(picked_key) => {
                                        if let Some(selection) = self.selection.as_mut() {
                                            match selection {
                                                Selection::Keys { keys } => {
                                                    if ui.keyboard_modifiers().control {
                                                        keys.insert(picked_key);
                                                    }
                                                    if !keys.contains(&picked_key) {
                                                        self.set_selection(
                                                            Some(Selection::single_key(picked_key)),
                                                            ui,
                                                        );
                                                    }
                                                }
                                                Selection::LeftTangent { .. }
                                                | Selection::RightTangent { .. } => self
                                                    .set_selection(
                                                        Some(Selection::single_key(picked_key)),
                                                        ui,
                                                    ),
                                            }
                                        } else {
                                            self.set_selection(
                                                Some(Selection::single_key(picked_key)),
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
                },
                UiMessageData::CurveEditor(msg)
                    if message.destination() == self.handle
                        && message.direction() == MessageDirection::ToWidget =>
                {
                    match msg {
                        CurveEditorMessage::Sync(curve) => {
                            self.keys = curve
                                .keys()
                                .iter()
                                .map(CurveKeyView::from)
                                .collect::<Vec<_>>();
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
                            self.keys.push(CurveKeyView {
                                position: local_pos,
                                kind: CurveKeyKind::Linear,
                                id: Uuid::new_v4(),
                            });
                            self.set_selection(None, ui);
                            self.sort_keys();
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        if let UiMessageData::MenuItem(MenuItemMessage::Click) = message.data() {
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

fn sort_keys(keys: &mut Vec<CurveKeyView>) {
    keys.sort_by(|a, b| {
        if a.position.x < b.position.x {
            Ordering::Less
        } else if a.position.x > b.position.x {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    })
}

fn round_to_step(x: f32, step: f32) -> f32 {
    x - x % step
}

impl<M: MessageData, C: Control<M, C>> CurveEditor<M, C> {
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
        sort_keys(&mut self.keys);
    }

    fn set_selection(&mut self, selection: Option<Selection>, ui: &UserInterface<M, C>) {
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

    fn remove_selection(&mut self, ui: &mut UserInterface<M, C>) {
        if let Some(selection) = self.selection.as_ref() {
            if let Selection::Keys { keys } = selection {
                let mut new_keys = Vec::new();
                for (i, key) in self.keys.iter().enumerate() {
                    if !keys.contains(&i) {
                        new_keys.push(key.clone());
                    }
                }
                self.keys = new_keys;

                self.set_selection(None, ui);

                // Send modified curve back to user.
                self.send_curve(ui);
            }
        }
    }

    fn change_selected_keys_kind(&mut self, kind: CurveKeyKind, ui: &mut UserInterface<M, C>) {
        if let Some(selection) = self.selection.as_ref() {
            if let Selection::Keys { keys } = selection {
                for key in keys {
                    self.keys[*key].kind = kind.clone();
                }

                self.send_curve(ui);
            }
        }
    }

    /// `pos` must be in screen space.
    fn pick(&self, pos: Vector2<f32>) -> Option<PickResult> {
        // Linear search is fine here, having a curve with thousands of
        // points is insane anyway.
        for (i, key) in self.keys.iter().enumerate() {
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

    fn send_curve(&self, ui: &UserInterface<M, C>) {
        let curve = Curve::from(
            self.keys
                .iter()
                .map(|k| CurveKey::new(k.position.x, k.position.y, k.kind.clone()))
                .collect::<Vec<_>>(),
        );

        ui.send_message(CurveEditorMessage::sync(
            self.handle,
            MessageDirection::FromWidget,
            curve,
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

        let step_size = 40.0 / self.zoom.clamp(0.001, 1000.0);

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
                self.point_to_screen_space(Vector2::new(local_left_bottom.x, y)),
                self.point_to_screen_space(Vector2::new(local_right_top.x, y)),
                1.0,
            );
        }

        for nx in 0..=nw {
            let k = nx as f32 / (nw) as f32;
            let x = local_left_bottom.x + k * w;
            ctx.push_line(
                self.point_to_screen_space(Vector2::new(x, local_left_bottom.y)),
                self.point_to_screen_space(Vector2::new(x, local_right_top.y)),
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
        let draw_keys = self.draw_keys.borrow();

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
        let keys_to_draw = self.draw_keys.borrow();

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
                        selected = keys.contains(&i);
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
                        _ => ((false, false)),
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
}

pub struct CurveEditorBuilder<M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    curve: Curve,
    view_position: Vector2<f32>,
    zoom: f32,
}

impl<M: MessageData, C: Control<M, C>> CurveEditorBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
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

    pub fn build(mut self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let keys = self
            .curve
            .keys()
            .iter()
            .map(CurveKeyView::from)
            .collect::<Vec<_>>();

        let add_key;
        let remove;
        let make_constant;
        let make_linear;
        let make_cubic;
        let key;
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
            keys,
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
            draw_keys: Default::default(),
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
            },
        };

        ctx.add_node(UINode::CurveEditor(editor))
    }
}
