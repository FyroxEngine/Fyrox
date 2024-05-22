use crate::message::CursorIcon;
use crate::{
    brush::Brush,
    core::{
        algebra::{Matrix3, Point2, SimdPartialOrd, Vector2, Vector3},
        color::Color,
        math::{
            cubicf,
            curve::{Curve, CurveKeyKind},
            lerpf, wrap_angle, Rect,
        },
        parking_lot::{MappedMutexGuard, Mutex, MutexGuard},
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid_provider,
        visitor::prelude::*,
    },
    curve::key::{CurveKeyView, CurveKeyViewContainer},
    define_constructor,
    draw::{CommandTexture, Draw, DrawingContext},
    formatted_text::{FormattedText, FormattedTextBuilder},
    grid::{Column, GridBuilder, Row},
    menu::{ContextMenuBuilder, MenuItemBuilder, MenuItemContent, MenuItemMessage},
    message::{ButtonState, KeyCode, MessageDirection, MouseButton, UiMessage},
    numeric::{NumericUpDownBuilder, NumericUpDownMessage},
    popup::PopupBuilder,
    stack_panel::StackPanelBuilder,
    text::TextBuilder,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, RcUiNodeHandle, Thickness, UiNode, UserInterface, VerticalAlignment,
    BRUSH_BRIGHT, BRUSH_LIGHT,
};
use fxhash::FxHashSet;
use fyrox_graph::BaseSceneGraph;
use std::{
    cell::{Cell, RefCell},
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

pub mod key;

#[derive(Debug, Clone, PartialEq)]
pub enum CurveEditorMessage {
    SyncBackground(Vec<Curve>),
    Sync(Vec<Curve>),
    Colorize(Vec<(Uuid, Brush)>),
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
    define_constructor!(CurveEditorMessage:SyncBackground => fn sync_background(Vec<Curve>), layout: false);
    define_constructor!(CurveEditorMessage:Sync => fn sync(Vec<Curve>), layout: false);
    define_constructor!(CurveEditorMessage:Colorize => fn colorize(Vec<(Uuid, Brush)>), layout: false);
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

#[derive(Debug, Default)]
pub struct CurveTransformCell(Mutex<CurveTransform>);

impl Clone for CurveTransformCell {
    fn clone(&self) -> Self {
        Self(Mutex::new(self.0.lock().clone()))
    }
}

impl CurveTransformCell {
    /// Position of the center of the curve editor in the curve coordinate space.
    pub fn position(&self) -> Vector2<f32> {
        self.0.lock().position
    }
    /// Scape of the curve editor: multiply curve space units by this to get screen space units.
    pub fn scale(&self) -> Vector2<f32> {
        self.0.lock().scale
    }
    /// Location of the curve editor on the screen, in screen space units.
    pub fn bounds(&self) -> Rect<f32> {
        self.0.lock().bounds
    }
    /// Modify the current position. Call this when the center of the curve view should change.
    pub fn set_position(&self, position: Vector2<f32>) {
        self.0.lock().position = position
    }
    /// Modify the current zoom of the curve view.
    pub fn set_scale(&self, scale: Vector2<f32>) {
        self.0.lock().scale = scale;
    }
    /// Update the bounds of the curve view. Call this to ensure the CurveTransform accurately
    /// reflects the actual size of the widget being drawn.
    pub fn set_bounds(&self, bounds: Rect<f32>) {
        self.0.lock().bounds = bounds;
    }
    /// Just like [CurveTransformCell::y_step_iter] but for x-coordinates.
    /// Iterate through a list of x-coordinates across the width of the bounds.
    /// The x-coordinates are in curve-space, but their distance apart should be
    /// at least `grid_size` in screen-space.
    ///
    /// This iterator indates where grid lines should be drawn to make a curve easier
    /// to read for the user.
    pub fn x_step_iter(&self, grid_size: f32) -> StepIterator {
        self.0.lock().x_step_iter(grid_size)
    }
    /// Just like [CurveTransformCell::x_step_iter] but for y-coordinates.
    /// Iterate through a list of y-coordinates across the width of the bounds.
    /// The y-coordinates are in curve-space, but their distance apart should be
    /// at least `grid_size` in screen-space.
    ///
    /// This iterator indates where grid lines should be drawn to make a curve easier
    /// to read for the user.
    pub fn y_step_iter(&self, grid_size: f32) -> StepIterator {
        self.0.lock().y_step_iter(grid_size)
    }
    /// Construct the transformation matrices to reflect the current position, scale, and bounds.
    pub fn update_transform(&self) {
        self.0.lock().update_transform();
    }
    /// Transform a point on the curve into a point in the local coordinate space of the widget.
    pub fn curve_to_local(&self) -> MappedMutexGuard<Matrix3<f32>> {
        MutexGuard::map(self.0.lock(), |t| &mut t.curve_to_local)
    }
    /// Transform a point on the curve into a point on the screen.
    pub fn curve_to_screen(&self) -> MappedMutexGuard<Matrix3<f32>> {
        MutexGuard::map(self.0.lock(), |t| &mut t.curve_to_screen)
    }
    /// Transform a point in the local coordinate space of the widget into a point in the coordinate space of the curve.
    /// After the transformation, the x-coordinate could be a key location and the y-coordinate could be a key value.
    /// Y-coordinates are flipped so that positive-y becomes the up direction.
    pub fn local_to_curve(&self) -> MappedMutexGuard<Matrix3<f32>> {
        MutexGuard::map(self.0.lock(), |t| &mut t.local_to_curve)
    }
    /// Transform a point on the screen into a point in the coordinate space of the curve.
    /// After the transformation, the x-coordinate could be a key location and the y-coordinate could be a key value.
    /// Y-coordinates are flipped so that positive-y becomes the up direction.
    pub fn screen_to_curve(&self) -> MappedMutexGuard<Matrix3<f32>> {
        MutexGuard::map(self.0.lock(), |t| &mut t.screen_to_curve)
    }
}

/// Default grid size when not otherwise specified
pub const STANDARD_GRID_SIZE: f32 = 50.0;
/// Step sizes are more readable when they are easily recognizable.
/// This is a list of step sizes to round up to when choosing a step size.
/// The values in this list are meaningless; they are just intended to be convenient and round and easy to read.
const STANDARD_STEP_SIZES: [f32; 10] = [0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0];

/// Round the given step size up to the next standard step size, if possible.
fn standardize_step(step: f32) -> f32 {
    STANDARD_STEP_SIZES
        .iter()
        .copied()
        .find(|x| step <= *x)
        .unwrap_or(step)
}

/// This object represents the transformation curve coordinates into
/// the local coordinates of the curve editor widget and into the coordinates
/// of the screen. Using this object allows other widgets to align themselves
/// perfectly with the coordinates of a curve editor.
///
/// Since widgets are not mutable during layout and rendering, a CurveTransform
/// is intended to be used within a [CurveTransformCell] which provides interior mutability.
#[derive(Clone, Debug)]
pub struct CurveTransform {
    /// Position of the center of the curve editor in the curve coordinate space.
    pub position: Vector2<f32>,
    /// Scape of the curve editor: multiply curve space units by this to get screen space units.
    pub scale: Vector2<f32>,
    /// Location of the curve editor on the screen, in screen space units.
    pub bounds: Rect<f32>,
    /// Transform a point on a curve into a point in the local space
    /// of the curve editor. Before applying this transformation, (0,0) is
    /// the origin of the curve, and positive-y is up.
    /// After the transform, (0,0) is the top-left corner of the editor,
    /// and positive-y is down.
    pub curve_to_local: Matrix3<f32>,
    /// The inverse of `curve_to_local`, transforming a point in the local space
    /// of the editor widget into a point in the space of the curve.
    pub local_to_curve: Matrix3<f32>,
    /// Transform a point on the screen into a point in the space of the curve.
    /// For example, a mouse click position would be transformed into the corresponding
    /// (x,y) coordinates of a cure key that would result from the click.
    pub screen_to_curve: Matrix3<f32>,
    /// Transform a point on the curve into a point on the screen.
    pub curve_to_screen: Matrix3<f32>,
}

impl Default for CurveTransform {
    fn default() -> Self {
        Self {
            position: Vector2::default(),
            scale: Vector2::new(1.0, 1.0),
            bounds: Rect::default(),
            curve_to_local: Matrix3::identity(),
            local_to_curve: Matrix3::identity(),
            screen_to_curve: Matrix3::identity(),
            curve_to_screen: Matrix3::identity(),
        }
    }
}

impl CurveTransform {
    /// Construct the transformations matrices for the current position, scale, and bounds.
    pub fn update_transform(&mut self) {
        let bounds = self.bounds;
        let local_center = bounds.size.scale(0.5);
        let mut curve_to_local = Matrix3::<f32>::identity();
        // Translate the view position to be at (0,0) in the curve space.
        curve_to_local.append_translation_mut(&-self.position);
        // Scale from curve units into local space units.
        curve_to_local.append_nonuniform_scaling_mut(&self.scale);
        // Flip y-positive from pointing up to pointing down
        curve_to_local.append_nonuniform_scaling_mut(&Vector2::new(1.0, -1.0));
        // Translate (0,0) to the center of the widget in local space.
        curve_to_local.append_translation_mut(&local_center);
        // Find the inverse transform matrix automatically.
        let local_to_curve = curve_to_local.try_inverse().unwrap_or_default();
        let mut curve_to_screen = curve_to_local;
        // Translate the local (0,0) to the top-left corner of the widget in screen coordinates.
        curve_to_screen.append_translation_mut(&bounds.position);
        // Find the screen-to-curve matrix automatically from the curve-to-screen matrix.
        let screen_to_curve = curve_to_screen.try_inverse().unwrap_or_default();
        *self = CurveTransform {
            curve_to_local,
            local_to_curve,
            screen_to_curve,
            curve_to_screen,
            ..*self
        };
    }
    /// Just like [CurveTransform::y_step_iter] but for x-coordinates.
    /// Iterate through a list of x-coordinates across the width of the bounds.
    /// The x-coordinates are in curve-space, but their distance apart should be
    /// at least `grid_size` in screen-space.
    ///
    /// This iterator indates where grid lines should be drawn to make a curve easier
    /// to read for the user.
    pub fn x_step_iter(&self, grid_size: f32) -> StepIterator {
        let zoom = self.scale;
        let step_size = grid_size / zoom.x.clamp(0.001, 1000.0);
        let screen_left = self.bounds.position.x;
        let screen_right = self.bounds.position.x + self.bounds.size.x;
        let left = self
            .screen_to_curve
            .transform_point(&Point2::new(screen_left, 0.0))
            .x;
        let right = self
            .screen_to_curve
            .transform_point(&Point2::new(screen_right, 0.0))
            .x;
        StepIterator::new(standardize_step(step_size), left, right)
    }
    /// Just like [CurveTransform::x_step_iter] but for y-coordinates.
    /// Iterate through a list of y-coordinates across the width of the bounds.
    /// The y-coordinates are in curve-space, but their distance apart should be
    /// at least `grid_size` in screen-space.
    ///
    /// This iterator indates where grid lines should be drawn to make a curve easier
    /// to read for the user.
    pub fn y_step_iter(&self, grid_size: f32) -> StepIterator {
        let zoom = self.scale;
        let step_size = grid_size / zoom.y.clamp(0.001, 1000.0);
        let screen_top = self.bounds.position.y;
        let screen_bottom = self.bounds.position.y + self.bounds.size.y;
        let start = self
            .screen_to_curve
            .transform_point(&Point2::new(0.0, screen_bottom))
            .y;
        let end = self
            .screen_to_curve
            .transform_point(&Point2::new(0.0, screen_top))
            .y;
        StepIterator::new(standardize_step(step_size), start, end)
    }
}

/// Iterate through f32 values stepping by `step_size`. Each value is
/// is a multiple of `step_size`.
#[derive(Debug, Clone)]
pub struct StepIterator {
    pub step_size: f32,
    index: isize,
    end: isize,
}

impl StepIterator {
    /// Construct an interator that starts at or before `start` and ends at or after `end`.
    /// The intention is to cover the whole range of `start` to `end` at least.
    pub fn new(step: f32, start: f32, end: f32) -> Self {
        Self {
            step_size: step,
            index: (start / step).floor() as isize,
            end: ((end / step).ceil() as isize).saturating_add(1),
        }
    }
}

impl Iterator for StepIterator {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.end {
            return None;
        }
        let value = (self.index as f32) * self.step_size;
        self.index += 1;
        Some(value)
    }
}

#[derive(Default, Clone, Visit, Reflect, Debug)]
pub struct CurvesContainer {
    curves: Vec<CurveKeyViewContainer>,
}

impl CurvesContainer {
    pub fn from_native(curves: &[Curve]) -> Self {
        Self {
            curves: curves
                .iter()
                .map(|curve| CurveKeyViewContainer::new(curve, BRUSH_BRIGHT))
                .collect::<Vec<_>>(),
        }
    }

    pub fn to_native(&self) -> Vec<Curve> {
        self.curves
            .iter()
            .map(|view| view.curve())
            .collect::<Vec<_>>()
    }

    pub fn container_of(&self, key_id: Uuid) -> Option<&CurveKeyViewContainer> {
        self.curves
            .iter()
            .find(|curve| curve.key_ref(key_id).is_some())
    }

    pub fn key_ref(&self, uuid: Uuid) -> Option<&CurveKeyView> {
        // TODO: This will be slow for curves with lots of keys.
        self.curves.iter().find_map(|keys| keys.key_ref(uuid))
    }

    pub fn key_mut(&mut self, uuid: Uuid) -> Option<&mut CurveKeyView> {
        // TODO: This will be slow for curves with lots of keys.
        self.curves.iter_mut().find_map(|keys| keys.key_mut(uuid))
    }

    pub fn remove(&mut self, uuid: Uuid) {
        for curve in self.curves.iter_mut() {
            curve.remove(uuid);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &CurveKeyViewContainer> {
        self.curves.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut CurveKeyViewContainer> {
        self.curves.iter_mut()
    }
}

#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct CurveEditor {
    widget: Widget,
    background_curves: CurvesContainer,
    curves: CurvesContainer,
    #[visit(skip)]
    #[reflect(hidden)]
    curve_transform: CurveTransformCell,
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
    key_id: Uuid,
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
        key_id: Uuid,
        left: bool,
    },
    BoxSelection {
        // In local coordinates.
        initial_mouse_pos: Vector2<f32>,
        min: Cell<Vector2<f32>>,
        max: Cell<Vector2<f32>>,
    },
}

impl OperationContext {
    fn is_dragging(&self) -> bool {
        matches!(
            self,
            OperationContext::DragKeys { .. } | OperationContext::DragTangent { .. }
        )
    }
}

#[derive(Clone, Debug)]
enum Selection {
    Keys { keys: FxHashSet<Uuid> },
    LeftTangent { key_id: Uuid },
    RightTangent { key_id: Uuid },
}

#[derive(Copy, Clone)]
enum PickResult {
    Key(Uuid),
    LeftTangent(Uuid),
    RightTangent(Uuid),
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
        self.curve_transform.set_bounds(self.screen_bounds());
        self.curve_transform.update_transform();
        self.draw_background(ctx);
        self.draw_highlight_zones(ctx);
        self.draw_grid(ctx);
        self.draw_curves(&self.background_curves, ctx);
        self.draw_curves(&self.curves, ctx);
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
                        let is_dragging = self
                            .operation_context
                            .as_ref()
                            .map_or(false, |ctx| ctx.is_dragging());
                        if self.pick(*pos).is_some() || is_dragging {
                            if self.cursor.is_none() {
                                ui.send_message(WidgetMessage::cursor(
                                    self.handle,
                                    MessageDirection::ToWidget,
                                    Some(if is_dragging {
                                        CursorIcon::Grabbing
                                    } else {
                                        CursorIcon::Grab
                                    }),
                                ));
                            }
                        } else if self.cursor.is_some() {
                            ui.send_message(WidgetMessage::cursor(
                                self.handle,
                                MessageDirection::ToWidget,
                                None,
                            ));
                        }

                        let curve_mouse_pos = self.screen_to_curve_space(*pos);
                        if let Some(operation_context) = self.operation_context.as_ref() {
                            match operation_context {
                                OperationContext::DragKeys {
                                    entries,
                                    initial_mouse_pos,
                                } => {
                                    let local_delta = curve_mouse_pos - initial_mouse_pos;
                                    for entry in entries {
                                        if let Some(key) = self.curves.key_mut(entry.key_id) {
                                            key.position = entry.initial_position + local_delta;
                                        }
                                    }
                                    self.sort_keys();
                                }
                                OperationContext::MoveView {
                                    initial_mouse_pos,
                                    initial_view_pos,
                                } => {
                                    let d = *pos - initial_mouse_pos;
                                    let zoom = self.curve_transform.scale();
                                    // Dragging left moves the position right. Dragging up moves the position down.
                                    // Remember: up is negative-y in screen space, and up is positive-y in curve space.
                                    let delta = Vector2::<f32>::new(-d.x / zoom.x, d.y / zoom.y);
                                    ui.send_message(CurveEditorMessage::view_position(
                                        self.handle,
                                        MessageDirection::ToWidget,
                                        initial_view_pos + delta,
                                    ));
                                }
                                OperationContext::DragTangent { key_id: key, left } => {
                                    if let Some(key) = self.curves.key_mut(*key) {
                                        let key_pos = key.position;

                                        let screen_key_pos = self
                                            .curve_transform
                                            .curve_to_screen()
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
                                    min.set(curve_mouse_pos.inf(initial_mouse_pos));
                                    max.set(curve_mouse_pos.sup(initial_mouse_pos));
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
                                                    key_id: *k,
                                                    initial_position: self
                                                        .curves
                                                        .key_ref(*k)
                                                        .map(|k| k.position)
                                                        .unwrap_or_default(),
                                                })
                                                .collect::<Vec<_>>(),
                                            initial_mouse_pos: curve_mouse_pos,
                                        });
                                    }
                                    Selection::LeftTangent { key_id: key } => {
                                        self.operation_context =
                                            Some(OperationContext::DragTangent {
                                                key_id: *key,
                                                left: true,
                                            })
                                    }
                                    Selection::RightTangent { key_id: key } => {
                                        self.operation_context =
                                            Some(OperationContext::DragTangent {
                                                key_id: *key,
                                                left: false,
                                            })
                                    }
                                }
                            } else {
                                self.operation_context = Some(OperationContext::BoxSelection {
                                    initial_mouse_pos: curve_mouse_pos,
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

                                    self.send_curves(ui);
                                }
                                OperationContext::BoxSelection { min, max, .. } => {
                                    let min = min.get();
                                    let max = max.get();

                                    let rect =
                                        Rect::new(min.x, min.y, max.x - min.x, max.y - min.y);

                                    let mut selection = FxHashSet::default();
                                    for curve in self.curves.iter() {
                                        for key in curve.keys() {
                                            if rect.contains(key.position) {
                                                selection.insert(key.id);
                                            }
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
                                        if let Some(picked_key_id) =
                                            self.curves.key_ref(picked_key).map(|key| key.id)
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
                                            Some(Selection::LeftTangent { key_id: picked_key }),
                                            ui,
                                        );
                                    }
                                    PickResult::RightTangent(picked_key) => {
                                        self.set_selection(
                                            Some(Selection::RightTangent { key_id: picked_key }),
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
                                initial_view_pos: self.curve_transform.position(),
                            });
                        }
                        _ => (),
                    },
                    WidgetMessage::MouseWheel { amount, .. } => {
                        let k = if *amount < 0.0 { 0.9 } else { 1.1 };

                        let zoom = self.curve_transform.scale();
                        let new_zoom = if ui.keyboard_modifiers().shift {
                            Vector2::new(zoom.x * k, zoom.y)
                        } else if ui.keyboard_modifiers.control {
                            Vector2::new(zoom.x, zoom.y * k)
                        } else {
                            zoom * k
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
                        CurveEditorMessage::SyncBackground(curves) => {
                            self.background_curves = CurvesContainer::from_native(curves);

                            for curve in self.background_curves.iter_mut() {
                                curve.brush = BRUSH_LIGHT;
                            }
                        }
                        CurveEditorMessage::Sync(curves) => {
                            let color_map = self
                                .curves
                                .iter()
                                .map(|curve| (curve.id(), curve.brush.clone()))
                                .collect::<Vec<_>>();

                            self.curves = CurvesContainer::from_native(curves);

                            self.colorize(&color_map);
                        }
                        CurveEditorMessage::Colorize(color_map) => {
                            self.colorize(color_map);
                        }
                        CurveEditorMessage::ViewPosition(view_position) => {
                            self.set_view_position(*view_position);
                            ui.send_message(message.reverse());
                        }
                        CurveEditorMessage::Zoom(zoom) => {
                            self.curve_transform
                                .set_scale(zoom.simd_clamp(self.min_zoom, self.max_zoom));
                            ui.send_message(message.reverse());
                        }
                        CurveEditorMessage::RemoveSelection => {
                            self.remove_selection(ui);
                        }
                        CurveEditorMessage::ChangeSelectedKeysKind(kind) => {
                            self.change_selected_keys_kind(kind.clone(), ui);
                        }
                        CurveEditorMessage::AddKey(screen_pos) => {
                            let local_pos = self.screen_to_curve_space(*screen_pos);
                            let dest_curve = if let Some(selection) = self.selection.as_ref() {
                                match selection {
                                    Selection::Keys { keys } => {
                                        self.curves.iter_mut().find(|curve| {
                                            for key in curve.keys() {
                                                if keys.contains(&key.id) {
                                                    return true;
                                                }
                                            }
                                            false
                                        })
                                    }
                                    Selection::LeftTangent { .. } => None,
                                    Selection::RightTangent { .. } => None,
                                }
                            } else {
                                self.curves.curves.first_mut()
                            };

                            if let Some(dest_curve) = dest_curve {
                                dest_curve.add(CurveKeyView {
                                    position: local_pos,
                                    kind: CurveKeyKind::Linear,
                                    id: Uuid::new_v4(),
                                });
                                self.set_selection(None, ui);
                                self.sort_keys();
                                self.send_curves(ui);
                            }
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
                            self.highlight_zones.clone_from(zones);
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
        self.curve_transform
            .set_position(self.view_bounds.map_or(position, |bounds| {
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
            }));
    }

    fn colorize(&mut self, color_map: &[(Uuid, Brush)]) {
        for (curve_id, brush) in color_map.iter() {
            if let Some(curve) = self.curves.iter_mut().find(|curve| &curve.id() == curve_id) {
                curve.brush = brush.clone();
            }
        }
    }

    fn zoom_to_fit(&mut self, sender: &Sender<UiMessage>) {
        let mut min = Vector2::repeat(f32::MAX);
        let mut max = Vector2::repeat(-f32::MAX);

        for curve in self.curves.iter() {
            let bounds = curve.curve().bounds();
            if bounds.position.x < min.x {
                min.x = bounds.position.x;
            }
            if bounds.position.y < min.y {
                min.y = bounds.position.y;
            }
            let local_max = bounds.position + bounds.size;
            if local_max.x > max.x {
                max.x = local_max.x;
            }
            if local_max.y > max.y {
                max.y = local_max.y;
            }
        }

        let mut bounds = Rect {
            position: min,
            size: max - min,
        };

        // Prevent division by zero.
        if bounds.size.x < 0.001 {
            bounds.size.x = 0.001;
        }
        if bounds.size.y < 0.001 {
            bounds.size.y = 0.001;
        }

        let center = bounds.center();

        sender
            .send(CurveEditorMessage::zoom(
                self.handle,
                MessageDirection::ToWidget,
                Vector2::new(
                    self.actual_local_size().x / bounds.w(),
                    self.actual_local_size().y / bounds.h(),
                ),
            ))
            .unwrap();

        sender
            .send(CurveEditorMessage::view_position(
                self.handle,
                MessageDirection::ToWidget,
                center,
            ))
            .unwrap();
    }

    /// Transforms a point to view space.
    pub fn point_to_view_space(&self, point: Vector2<f32>) -> Vector2<f32> {
        self.curve_transform
            .local_to_curve()
            .transform_point(&Point2::from(point))
            .coords
    }

    /// Transforms a point to screen space.
    pub fn point_to_screen_space(&self, point: Vector2<f32>) -> Vector2<f32> {
        self.curve_transform
            .curve_to_screen()
            .transform_point(&Point2::from(point))
            .coords
    }

    /// Transforms a vector to screen space.
    pub fn vector_to_screen_space(&self, vector: Vector2<f32>) -> Vector2<f32> {
        (*self.curve_transform.curve_to_screen() * Vector3::new(vector.x, vector.y, 0.0)).xy()
    }

    /// Transforms a screen position to a curve position.
    pub fn screen_to_curve_space(&self, point: Vector2<f32>) -> Vector2<f32> {
        self.curve_transform
            .screen_to_curve()
            .transform_point(&Point2::from(point))
            .coords
    }

    fn sort_keys(&mut self) {
        for curve in self.curves.iter_mut() {
            curve.sort_keys();
        }
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
                if let Some(key) = self.curves.key_ref(*first) {
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
                self.curves.remove(id);
            }

            self.set_selection(None, ui);

            // Send modified curve back to user.
            self.send_curves(ui);
        }
    }

    fn change_selected_keys_kind(&mut self, kind: CurveKeyKind, ui: &mut UserInterface) {
        if let Some(Selection::Keys { keys }) = self.selection.as_ref() {
            for key in keys {
                if let Some(key) = self.curves.key_mut(*key) {
                    key.kind = kind.clone();
                }
            }

            self.send_curves(ui);
        }
    }

    fn change_selected_keys_value(&mut self, value: f32, ui: &mut UserInterface) {
        if let Some(Selection::Keys { keys }) = self.selection.as_ref() {
            let mut modified = false;
            for key in keys {
                if let Some(key) = self.curves.key_mut(*key) {
                    let key_value = &mut key.position.y;
                    if (*key_value).ne(&value) {
                        *key_value = value;
                        modified = true;
                    }
                }
            }

            if modified {
                self.send_curves(ui);
            }
        }
    }

    fn change_selected_keys_location(&mut self, location: f32, ui: &mut UserInterface) {
        if let Some(Selection::Keys { keys }) = self.selection.as_ref() {
            let mut modified = false;
            for key in keys {
                if let Some(key) = self.curves.key_mut(*key) {
                    let key_location = &mut key.position.x;
                    if (*key_location).ne(&location) {
                        *key_location = location;
                        modified = true;
                    }
                }
            }

            if modified {
                self.send_curves(ui);
            }
        }
    }

    /// `pos` must be in screen space.
    fn pick(&self, pos: Vector2<f32>) -> Option<PickResult> {
        for curve in self.curves.iter() {
            // Linear search is fine here, having a curve with thousands of
            // points is insane anyway.
            for key in curve.keys().iter() {
                let screen_pos = self.point_to_screen_space(key.position);
                let bounds = Rect::new(
                    screen_pos.x - self.key_size * 0.5,
                    screen_pos.y - self.key_size * 0.5,
                    self.key_size,
                    self.key_size,
                );
                if bounds.contains(pos) {
                    return Some(PickResult::Key(key.id));
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
                        return Some(PickResult::LeftTangent(key.id));
                    }

                    let right_handle_pos = self
                        .tangent_screen_position(wrap_angle(right_tangent.atan()), key.position);

                    if (right_handle_pos - pos).norm() <= self.key_size * 0.5 {
                        return Some(PickResult::RightTangent(key.id));
                    }
                }
            }
        }
        None
    }

    fn tangent_screen_position(&self, angle: f32, key_position: Vector2<f32>) -> Vector2<f32> {
        self.point_to_screen_space(key_position)
            + Vector2::new(angle.cos(), angle.sin()).scale(self.handle_radius)
    }

    fn send_curves(&self, ui: &UserInterface) {
        ui.send_message(CurveEditorMessage::sync(
            self.handle,
            MessageDirection::FromWidget,
            self.curves.to_native(),
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

        let zoom = self.curve_transform.scale();
        let step_size_x = self.grid_size.x / zoom.x;
        let step_size_y = self.grid_size.y / zoom.y;

        let mut local_left_bottom = self.screen_to_curve_space(screen_bounds.left_top_corner());
        let local_left_bottom_n = local_left_bottom;
        local_left_bottom.x = round_to_step(local_left_bottom.x, step_size_x);
        local_left_bottom.y = round_to_step(local_left_bottom.y, step_size_y);

        let mut local_right_top = self.screen_to_curve_space(screen_bounds.right_bottom_corner());
        local_right_top.x = round_to_step(local_right_top.x, step_size_x);
        local_right_top.y = round_to_step(local_right_top.y, step_size_y);

        for y in self.curve_transform.y_step_iter(self.grid_size.y) {
            ctx.push_line(
                self.point_to_screen_space(Vector2::new(local_left_bottom.x - step_size_x, y)),
                self.point_to_screen_space(Vector2::new(local_right_top.x + step_size_x, y)),
                1.0,
            );
        }

        for x in self.curve_transform.x_step_iter(self.grid_size.x) {
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
            for y in self.curve_transform.y_step_iter(self.grid_size.y) {
                text.set_text(format!("{:.1}", y)).build();
                ctx.draw_text(
                    self.clip_bounds(),
                    self.point_to_screen_space(Vector2::new(local_left_bottom_n.x, y)),
                    &text,
                );
            }
        }

        if self.show_x_values {
            for x in self.curve_transform.x_step_iter(self.grid_size.x) {
                text.set_text(format!("{:.1}", x)).build();
                ctx.draw_text(
                    self.clip_bounds(),
                    self.point_to_screen_space(Vector2::new(x, local_left_bottom_n.y)),
                    &text,
                );
            }
        }
    }

    fn draw_curves(&self, curves: &CurvesContainer, ctx: &mut DrawingContext) {
        let screen_bounds = self.screen_bounds();

        for curve in curves.iter() {
            let draw_keys = curve.keys();

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
                curve.brush.clone(),
                CommandTexture::None,
                None,
            );
        }
    }

    fn draw_keys(&self, ctx: &mut DrawingContext) {
        for curve in self.curves.iter() {
            let keys_to_draw = curve.keys();

            for key in keys_to_draw.iter() {
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
                        Selection::LeftTangent { key_id } | Selection::RightTangent { key_id } => {
                            selected = key.id == *key_id;
                        }
                    }
                }

                // Show tangents for Cubic keys.
                if selected {
                    let (show_left, show_right) =
                        match self.curves.container_of(key.id).and_then(|container| {
                            container
                                .key_position(key.id)
                                .and_then(|i| keys_to_draw.get(i.wrapping_sub(1)))
                        }) {
                            Some(left) => match (&left.kind, &key.kind) {
                                (CurveKeyKind::Cubic { .. }, CurveKeyKind::Cubic { .. }) => {
                                    (true, true)
                                }
                                (CurveKeyKind::Linear, CurveKeyKind::Cubic { .. })
                                | (CurveKeyKind::Constant, CurveKeyKind::Cubic { .. }) => {
                                    (false, true)
                                }
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
    background_curves: Vec<Curve>,
    curves: Vec<Curve>,
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
            background_curves: Default::default(),
            curves: Default::default(),
            view_position: Default::default(),
            zoom: 1.0,
            view_bounds: None,
            show_x_values: true,
            show_y_values: true,
            grid_size: Vector2::new(STANDARD_GRID_SIZE, STANDARD_GRID_SIZE),
            min_zoom: Vector2::new(0.001, 0.001),
            max_zoom: Vector2::new(1000.0, 1000.0),
            highlight_zones: Default::default(),
        }
    }

    pub fn with_background_curves(mut self, curves: Vec<Curve>) -> Self {
        self.background_curves = curves;
        self
    }

    pub fn with_curves(mut self, curves: Vec<Curve>) -> Self {
        self.curves = curves;
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
        let mut background_curves = CurvesContainer::from_native(&self.curves);
        for curve in background_curves.iter_mut() {
            curve.brush = BRUSH_LIGHT;
        }

        let curves = CurvesContainer::from_native(&self.curves);

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
        let context_menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new()).with_content(
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
            ),
        )
        .build(ctx);
        let context_menu = RcUiNodeHandle::new(context_menu, ctx.sender());

        if self.widget_builder.foreground.is_none() {
            self.widget_builder.foreground = Some(BRUSH_BRIGHT)
        }

        let editor = CurveEditor {
            widget: self
                .widget_builder
                .with_context_menu(context_menu.clone())
                .with_preview_messages(true)
                .with_need_update(true)
                .build(),
            background_curves,
            curves,
            curve_transform: Default::default(),
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
