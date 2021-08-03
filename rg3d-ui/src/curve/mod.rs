#![allow(dead_code)] // TODO

use crate::message::ButtonState;
use crate::{
    brush::Brush,
    core::{
        algebra::{Matrix3, Point2, Vector2},
        color::Color,
        curve::{Curve, CurveKeyKind},
        math::{cubicf, lerpf, Rect},
        pool::Handle,
    },
    curve::key::CurveKeyView,
    draw::{CommandTexture, Draw, DrawingContext},
    message::{
        CurveEditorMessage, MessageData, MessageDirection, MouseButton, UiMessage, UiMessageData,
        WidgetMessage,
    },
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UINode, UserInterface,
};
use std::collections::HashSet;
use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
};

pub mod key;

#[derive(Clone)]
pub struct CurveEditor<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    keys: Vec<CurveKeyView>,
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
}

crate::define_widget_deref!(CurveEditor<M, C>);

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
        // In local coordinates.
        initial_mouse_pos: Vector2<f32>,
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

impl Selection {
    fn single_key(key: usize) -> Self {
        let mut keys = HashSet::new();
        keys.insert(key);
        Self::Keys { keys }
    }
}

impl<M: MessageData, C: Control<M, C>> Control<M, C> for CurveEditor<M, C> {
    fn draw(&self, ctx: &mut DrawingContext) {
        self.update_matrices();

        let screen_bounds = self.screen_bounds();
        // Draw background.
        ctx.push_rect_filled(&screen_bounds, None);
        ctx.commit(screen_bounds, self.background(), CommandTexture::None, None);

        self.draw_grid(ctx);

        // Draw curve.
        if let Some(first) = self.keys.first() {
            let screen_pos = self.point_to_screen_space(first.position);
            ctx.push_line(Vector2::new(0.0, screen_pos.y), screen_pos, 1.0);
        }
        if let Some(last) = self.keys.last() {
            let screen_pos = self.point_to_screen_space(last.position);
            ctx.push_line(
                screen_pos,
                Vector2::new(screen_bounds.x() + screen_bounds.w(), screen_pos.y),
                1.0,
            );
        }

        for pair in self.keys.windows(2) {
            let left = &pair[0];
            let right = &pair[1];

            let left_pos = self.point_to_screen_space(left.position);
            let right_pos = self.point_to_screen_space(right.position);

            match left.kind {
                CurveKeyKind::Constant => {
                    ctx.push_line(left_pos, Vector2::new(right_pos.x, left_pos.y), 1.0);
                    ctx.push_line(Vector2::new(right_pos.x, left_pos.y), right_pos, 1.0);
                }
                CurveKeyKind::Linear => ctx.push_line(left_pos, right_pos, 1.0),
                CurveKeyKind::Cubic {
                    left_tangent,
                    right_tangent,
                } => {
                    let steps = ((right_pos.x - left_pos.x).abs() / 5.0) as usize;
                    let mut prev = left_pos;
                    for i in 0..steps {
                        let t = i as f32 / (steps - 1) as f32;
                        let middle_x = lerpf(left_pos.x, right_pos.x, t);
                        let middle_y =
                            cubicf(left_pos.y, right_pos.y, t, left_tangent, right_tangent);
                        let pt = Vector2::new(middle_x, middle_y);
                        ctx.push_line(prev, pt, 1.0);
                        prev = pt;
                    }
                }
            }
        }
        ctx.commit(screen_bounds, self.foreground(), CommandTexture::None, None);

        // Draw keys.
        for (i, key) in self.keys.iter().enumerate() {
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
                if let Selection::Keys { keys } = selection {
                    selected = keys.contains(&i);
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

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle {
            match message.data() {
                UiMessageData::Widget(msg) => match msg {
                    WidgetMessage::MouseMove { pos, state } => {
                        let local_mouse_pos = self.point_to_local_space(*pos);
                        if let Some(operation_context) = self.operation_context.as_ref() {
                            match operation_context {
                                OperationContext::DragKeys {
                                    entries,
                                    initial_mouse_pos,
                                } => {
                                    let mut local_delta = local_mouse_pos - initial_mouse_pos;
                                    local_delta.y *= -1.0;
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
                                OperationContext::DragTangent { .. } => {
                                    // TODO
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
                                    Selection::LeftTangent { .. } => {
                                        // TODO
                                    }
                                    Selection::RightTangent { .. } => {
                                        // TODO
                                    }
                                }
                            }
                        }
                    }
                    WidgetMessage::MouseUp { .. } => {
                        if let Some(context) = self.operation_context.take() {
                            ui.release_mouse_capture();

                            match context {
                                OperationContext::DragKeys { .. } => {
                                    // Commit
                                }
                                OperationContext::DragTangent { .. } => { // TODO
                                }
                                _ => (),
                            }
                        }
                    }
                    WidgetMessage::MouseDown { pos, button } => match button {
                        MouseButton::Left => {
                            let picked = self.pick(*pos);

                            if let Some(picked) = picked {
                                if let Some(selection) = self.selection.as_mut() {
                                    match selection {
                                        Selection::Keys { keys } => {
                                            if ui.keyboard_modifiers().control {
                                                keys.insert(picked);
                                            } else {
                                                if !keys.contains(&picked) {
                                                    self.selection =
                                                        Some(Selection::single_key(picked));
                                                }
                                            }
                                        }
                                        Selection::LeftTangent { .. }
                                        | Selection::RightTangent { .. } => {
                                            self.selection = Some(Selection::single_key(picked))
                                        }
                                    }
                                } else {
                                    self.selection = Some(Selection::single_key(picked));
                                }
                            } else {
                                self.selection = None;
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
                            let self_keys = self.keys.clone();
                            if curve.keys().len() < self_keys.len() {
                                // A key was deleted.
                                for (i, key) in self_keys.iter().enumerate() {
                                    if curve.keys().iter().all(|k| k.id != key.id) {
                                        self.keys.remove(i);
                                    }
                                }
                            } else if curve.keys().len() > self.keys.len() {
                                // A key was added.
                                for key in curve.keys() {
                                    if !self.keys.iter().all(|k| k.id != key.id) {
                                        let key_view = CurveKeyView::from(key);
                                        self.keys.push(key_view);
                                    }
                                }
                            }

                            // Sync values.
                        }
                        CurveEditorMessage::ViewPosition(view_position) => {
                            self.view_position = *view_position;
                        }
                        CurveEditorMessage::Zoom(zoom) => {
                            self.zoom = *zoom;
                        }
                    }
                }
                _ => {}
            }
        }
    }
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

    /// Transforms a point to local space.
    pub fn point_to_local_space(&self, point: Vector2<f32>) -> Vector2<f32> {
        self.inv_screen_matrix
            .get()
            .transform_point(&Point2::from(point))
            .coords
    }

    /// `pos` must be in screen space.
    fn pick(&self, pos: Vector2<f32>) -> Option<usize> {
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
                return Some(i);
            }
        }
        None
    }

    fn clear_selection(&mut self) {
        self.selection = None;
    }

    // TODO: Fix.
    fn draw_grid(&self, ctx: &mut DrawingContext) {
        let screen_bounds = self.screen_bounds();

        // Draw grid.
        let local_left_bottom = Point2::new(0.0, 0.0).coords;
        let local_right_top = Point2::from(self.actual_size()).coords;
        let mut y = local_left_bottom.y;
        while y < local_right_top.y - local_left_bottom.y {
            ctx.push_line(
                self.point_to_screen_space(Vector2::new(local_left_bottom.x, y)),
                self.point_to_screen_space(Vector2::new(local_right_top.x, y)),
                1.0,
            );
            y += 5.0;
        }

        let mut x = local_left_bottom.x;
        while x < local_right_top.x - local_left_bottom.x {
            ctx.push_line(
                self.point_to_screen_space(Vector2::new(x, local_left_bottom.y)),
                self.point_to_screen_space(Vector2::new(x, local_right_top.y)),
                1.0,
            );
            x += 5.0;
        }
        ctx.commit(
            screen_bounds,
            self.grid_brush.clone(),
            CommandTexture::None,
            None,
        );
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

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let keys = self
            .curve
            .keys()
            .iter()
            .map(|k| CurveKeyView::from(k))
            .collect::<Vec<_>>();

        let editor = CurveEditor {
            widget: self.widget_builder.build(),
            keys,
            zoom: 1.0,
            view_position: Default::default(),
            view_matrix: Default::default(),
            screen_matrix: Default::default(),
            inv_screen_matrix: Default::default(),
            key_brush: Brush::Solid(Color::opaque(140, 140, 140)),
            selected_key_brush: Brush::Solid(Color::opaque(220, 220, 220)),
            key_size: 7.0,
            operation_context: None,
            grid_brush: Brush::Solid(Color::from_rgba(130, 130, 130, 50)),
            selection: None,
        };

        ctx.add_node(UINode::CurveEditor(editor))
    }
}
