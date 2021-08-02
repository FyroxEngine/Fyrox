use crate::core::curve::CurveKeyKind;
use crate::{
    brush::Brush,
    core::{
        algebra::{Matrix3, Point2, Vector2, Vector3},
        color::Color,
        curve::Curve,
        math::Rect,
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
use rg3d_core::math::{cubicf, lerpf};
use std::ops::{Deref, DerefMut};

pub mod key;

#[derive(Clone)]
pub struct CurveEditor<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    keys: Vec<CurveKeyView>,
    zoom: f32,
    view_position: Vector2<f32>,
    view_matrix: Matrix3<f32>,
    key_brush: Brush,
    key_size: f32,
    grid_brush: Brush,
    operation_context: Option<OperationContext>,
}

crate::define_widget_deref!(CurveEditor<M, C>);

fn to_screen_space(screen_bounds: Rect<f32>, pt: Vector2<f32>) -> Vector2<f32> {
    Vector2::new(
        screen_bounds.position.x + pt.x,
        // Flip Y because in math origin is in lower left corner.
        screen_bounds.position.y + screen_bounds.size.y - pt.y,
    )
}

#[derive(Clone)]
struct DragEntry {
    key: usize,
    mouse_relative_offset: Vector2<f32>,
}

#[derive(Clone)]
enum OperationContext {
    DragKeys {
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

impl<M: MessageData, C: Control<M, C>> Control<M, C> for CurveEditor<M, C> {
    fn draw(&self, ctx: &mut DrawingContext) {
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
        for key in self.keys.iter() {
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
            ctx.commit(
                screen_bounds,
                self.key_brush.clone(),
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
                        if let Some(operation_context) = self.operation_context.as_ref() {
                            match operation_context {
                                OperationContext::DragKeys { .. } => {}
                                OperationContext::MoveView {
                                    initial_mouse_pos,
                                    initial_view_pos,
                                } => {
                                    let delta = (pos - initial_mouse_pos).scale(1.0 / self.zoom);
                                    self.view_position = initial_view_pos + delta;
                                    self.update_view_matrix();
                                }
                                OperationContext::DragTangent { .. } => {}
                            }
                        }
                    }
                    WidgetMessage::MouseUp { .. } => {
                        if let Some(context) = self.operation_context.take() {
                            ui.release_mouse_capture();
                        }
                    }
                    WidgetMessage::MouseDown { pos, button } => {
                        if *button == MouseButton::Middle {
                            ui.capture_mouse(self.handle);
                            self.operation_context = Some(OperationContext::MoveView {
                                initial_mouse_pos: *pos,
                                initial_view_pos: self.view_position,
                            });
                        }
                    }
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
                            self.update_view_matrix();
                        }
                        CurveEditorMessage::Zoom(zoom) => {
                            self.zoom = *zoom;
                            self.update_view_matrix();
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

impl<M: MessageData, C: Control<M, C>> CurveEditor<M, C> {
    fn update_view_matrix(&mut self) {
        let vp = Vector2::new(self.view_position.x, -self.view_position.y);
        self.view_matrix = Matrix3::new_nonuniform_scaling_wrt_point(
            &Vector2::new(self.zoom, self.zoom),
            &Point2::from(self.actual_size().scale(0.5)),
        ) * Matrix3::new_translation(&vp);
    }

    /// Transform point into local view space.
    fn point_to_view_space(&self, point: Vector2<f32>) -> Vector2<f32> {
        self.view_matrix
            .transform_point(&Point2::from(point))
            .coords
    }

    fn vec_to_view_space(&self, point: Vector2<f32>) -> Vector2<f32> {
        (self.view_matrix * Vector3::new(point.x, point.y, 0.0)).xy()
    }

    /// Transforms point on screen.
    fn point_to_screen_space(&self, point: Vector2<f32>) -> Vector2<f32> {
        to_screen_space(self.screen_bounds(), self.point_to_view_space(point))
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

        let mut editor = CurveEditor {
            widget: self.widget_builder.build(),
            keys,
            zoom: 1.0,
            view_position: Default::default(),
            view_matrix: Default::default(),
            key_brush: Brush::Solid(Color::opaque(220, 220, 220)),
            key_size: 5.0,
            operation_context: None,
            grid_brush: Brush::Solid(Color::from_rgba(130, 130, 130, 50)),
        };

        editor.update_view_matrix();

        ctx.add_node(UINode::CurveEditor(editor))
    }
}
