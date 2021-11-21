use crate::{
    core::{algebra::Vector2, math::Rect, pool::Handle, scope_profile},
    draw::{CommandTexture, Draw, DrawingContext},
    message::UiMessage,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UiNode, UserInterface,
};
use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
};

#[derive(Clone, Copy, PartialEq)]
pub enum SizeMode {
    Strict,
    Auto,
    Stretch,
}

#[derive(Clone, Copy, PartialEq)]
pub struct GridDimension {
    size_mode: SizeMode,
    desired_size: f32,
    actual_size: f32,
    location: f32,
}

impl GridDimension {
    pub fn generic(size_mode: SizeMode, desired_size: f32) -> Self {
        Self {
            size_mode,
            desired_size,
            actual_size: 0.0,
            location: 0.0,
        }
    }

    pub fn strict(desired_size: f32) -> Self {
        Self::generic(SizeMode::Strict, desired_size)
    }

    pub fn stretch() -> Self {
        Self::generic(SizeMode::Stretch, 0.0)
    }

    pub fn auto() -> Self {
        Self::generic(SizeMode::Auto, 0.0)
    }
}

pub type Column = GridDimension;
pub type Row = GridDimension;

/// Automatically arranges children by rows and columns
#[derive(Clone)]
pub struct Grid {
    widget: Widget,
    rows: RefCell<Vec<Row>>,
    columns: RefCell<Vec<Column>>,
    draw_border: bool,
    border_thickness: f32,
    cells: RefCell<Vec<Cell>>,
    groups: RefCell<[Vec<usize>; 4]>,
}

crate::define_widget_deref!(Grid);

#[derive(Clone)]
struct Cell {
    nodes: Vec<Handle<UiNode>>,
    width_constraint: Option<f32>,
    height_constraint: Option<f32>,
    row_index: usize,
    column_index: usize,
}

fn group_index(row_size_mode: SizeMode, column_size_mode: SizeMode) -> usize {
    match (row_size_mode, column_size_mode) {
        (SizeMode::Strict, SizeMode::Strict)
        | (SizeMode::Strict, SizeMode::Auto)
        | (SizeMode::Auto, SizeMode::Strict)
        | (SizeMode::Auto, SizeMode::Auto) => 0,
        (SizeMode::Stretch, SizeMode::Auto) => 1,
        (SizeMode::Strict, SizeMode::Stretch) | (SizeMode::Auto, SizeMode::Stretch) => 2,
        (SizeMode::Stretch, SizeMode::Strict) | (SizeMode::Stretch, SizeMode::Stretch) => 3,
    }
}

fn choose_constraint(dimension: &GridDimension, available_size: f32) -> Option<f32> {
    match dimension.size_mode {
        SizeMode::Strict => Some(dimension.desired_size),
        SizeMode::Auto => Some(available_size),
        SizeMode::Stretch => None,
    }
}

fn choose_actual_size(
    dimension: &GridDimension,
    cell_size: f32,
    available_size: f32,
    stretch_size: f32,
) -> f32 {
    let current_actual_size = dimension.actual_size;
    match dimension.size_mode {
        SizeMode::Strict => dimension.desired_size,
        SizeMode::Auto => current_actual_size.max(cell_size),
        SizeMode::Stretch => current_actual_size.max(if available_size.is_infinite() {
            cell_size
        } else {
            stretch_size
        }),
    }
}

fn calc_total_size_of_non_stretch_dims(
    dims: &[GridDimension],
    children: &[Handle<UiNode>],
    ui: &UserInterface,
    desired_size_fetcher: fn(&UiNode, usize) -> Option<f32>,
) -> f32 {
    let mut preset_size = 0.0;

    for (i, dim) in dims.iter().enumerate() {
        if dim.size_mode == SizeMode::Strict {
            preset_size += dim.desired_size;
        } else if dim.size_mode == SizeMode::Auto {
            let mut dim_size = 0.0f32;
            for child_handle in children {
                let child = ui.nodes.borrow(*child_handle);
                if let Some(desired_size) = (desired_size_fetcher)(child, i) {
                    dim_size = dim_size.max(desired_size);
                }
            }
            preset_size += dim_size;
        }
    }

    preset_size
}

fn count_stretch_dims(dims: &[GridDimension]) -> usize {
    let mut stretch_sized_dims = 0;
    for dim in dims.iter() {
        if dim.size_mode == SizeMode::Stretch {
            stretch_sized_dims += 1;
        }
    }
    stretch_sized_dims
}

fn calc_avg_size_for_stretch_dim(
    dims: &[GridDimension],
    children: &[Handle<UiNode>],
    available_size: f32,
    ui: &UserInterface,
    desired_size_fetcher: fn(&UiNode, usize) -> Option<f32>,
) -> f32 {
    let preset_size = calc_total_size_of_non_stretch_dims(dims, children, ui, desired_size_fetcher);

    let rest_width = available_size - preset_size;

    let stretch_sized_dims = count_stretch_dims(dims);
    if stretch_sized_dims > 0 {
        rest_width / stretch_sized_dims as f32
    } else {
        0.0
    }
}

fn fetch_width(child: &UiNode, i: usize) -> Option<f32> {
    if child.column() == i && child.visibility() {
        Some(child.desired_size().x)
    } else {
        None
    }
}

fn fetch_height(child: &UiNode, i: usize) -> Option<f32> {
    if child.row() == i && child.visibility() {
        Some(child.desired_size().y)
    } else {
        None
    }
}

fn arrange_dims(dims: &mut [GridDimension], final_size: f32) {
    let mut preset_width = 0.0;
    for dim in dims.iter() {
        if dim.size_mode == SizeMode::Auto || dim.size_mode == SizeMode::Strict {
            preset_width += dim.actual_size;
        }
    }

    let stretch_count = count_stretch_dims(dims);
    let avg_size = if stretch_count > 0 {
        (final_size - preset_width) / stretch_count as f32
    } else {
        0.0
    };

    let mut location = 0.0;
    for dim in dims.iter_mut() {
        dim.location = location;
        location += match dim.size_mode {
            SizeMode::Strict | SizeMode::Auto => dim.actual_size,
            SizeMode::Stretch => avg_size,
        };
    }
}

impl Control for Grid {
    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        scope_profile!();

        let mut rows = self.rows.borrow_mut();
        let mut columns = self.columns.borrow_mut();
        let mut groups = self.groups.borrow_mut();
        let mut cells = self.cells.borrow_mut();

        // In case of no rows or columns, grid acts like default panel.
        if columns.is_empty() || rows.is_empty() {
            return self.widget.measure_override(ui, available_size);
        }

        for row in rows.iter_mut() {
            row.actual_size = 0.0;
        }
        for column in columns.iter_mut() {
            column.actual_size = 0.0;
        }
        for group in groups.iter_mut() {
            group.clear();
        }
        cells.clear();

        for (column_index, column) in columns.iter().enumerate() {
            for (row_index, row) in rows.iter().enumerate() {
                groups[group_index(row.size_mode, column.size_mode)].push(cells.len());

                cells.push(Cell {
                    nodes: self
                        .children()
                        .iter()
                        .filter_map(|&c| {
                            let child_ref = ui.node(c);
                            if child_ref.row() == row_index && child_ref.column() == column_index {
                                Some(c)
                            } else {
                                None
                            }
                        })
                        .collect(),
                    width_constraint: choose_constraint(column, available_size.x),
                    height_constraint: choose_constraint(row, available_size.y),
                    row_index,
                    column_index,
                })
            }
        }

        for group in groups.iter() {
            for &cell_index in group.iter() {
                let cell = &cells[cell_index];

                let stretch_sized_width = calc_avg_size_for_stretch_dim(
                    &columns,
                    self.children(),
                    available_size.x,
                    ui,
                    fetch_width,
                );

                let stretch_sized_height = calc_avg_size_for_stretch_dim(
                    &rows,
                    self.children(),
                    available_size.y,
                    ui,
                    fetch_height,
                );

                let child_constraint = Vector2::new(
                    cell.width_constraint.unwrap_or(stretch_sized_width),
                    cell.height_constraint.unwrap_or(stretch_sized_height),
                );

                let mut cell_size = Vector2::<f32>::default();
                for &node in cell.nodes.iter() {
                    ui.measure_node(node, child_constraint);
                    let node_ref = ui.node(node);
                    let desired_size = node_ref.desired_size();
                    cell_size.x = cell_size.x.max(desired_size.x);
                    cell_size.y = cell_size.y.max(desired_size.y);
                }

                let column = &mut columns[cell.column_index];
                column.actual_size =
                    choose_actual_size(column, cell_size.x, available_size.x, stretch_sized_width);

                let row = &mut rows[cell.row_index];
                row.actual_size =
                    choose_actual_size(row, cell_size.y, available_size.y, stretch_sized_height);
            }
        }

        let mut desired_size = Vector2::default();
        // Step 4. Calculate desired size of grid.
        for column in columns.iter() {
            desired_size.x += column.actual_size;
        }
        for row in rows.iter() {
            desired_size.y += row.actual_size;
        }
        desired_size
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        scope_profile!();

        let mut columns = self.columns.borrow_mut();
        let mut rows = self.rows.borrow_mut();

        if columns.is_empty() || rows.is_empty() {
            let rect = Rect::new(0.0, 0.0, final_size.x, final_size.y);
            for child_handle in self.widget.children() {
                ui.arrange_node(*child_handle, &rect);
            }
            return final_size;
        }

        arrange_dims(&mut columns, final_size.x);
        arrange_dims(&mut rows, final_size.y);

        for child_handle in self.widget.children() {
            let child = ui.nodes.borrow(*child_handle);
            if let Some(column) = columns.get(child.column()) {
                if let Some(row) = rows.get(child.row()) {
                    ui.arrange_node(
                        *child_handle,
                        &Rect::new(
                            column.location,
                            row.location,
                            column.actual_size,
                            row.actual_size,
                        ),
                    );
                }
            }
        }

        final_size
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        if self.draw_border {
            let bounds = self.widget.screen_bounds();

            let left_top = Vector2::new(bounds.x(), bounds.y());
            let right_top = Vector2::new(bounds.x() + bounds.w(), bounds.y());
            let right_bottom = Vector2::new(bounds.x() + bounds.w(), bounds.y() + bounds.h());
            let left_bottom = Vector2::new(bounds.x(), bounds.y() + bounds.h());

            drawing_context.push_line(left_top, right_top, self.border_thickness);
            drawing_context.push_line(right_top, right_bottom, self.border_thickness);
            drawing_context.push_line(right_bottom, left_bottom, self.border_thickness);
            drawing_context.push_line(left_bottom, left_top, self.border_thickness);

            for column in self.columns.borrow().iter() {
                let a = Vector2::new(bounds.x() + column.location, bounds.y());
                let b = Vector2::new(bounds.x() + column.location, bounds.y() + bounds.h());
                drawing_context.push_line(a, b, self.border_thickness);
            }
            for row in self.rows.borrow().iter() {
                let a = Vector2::new(bounds.x(), bounds.y() + row.location);
                let b = Vector2::new(bounds.x() + bounds.w(), bounds.y() + row.location);
                drawing_context.push_line(a, b, self.border_thickness);
            }

            drawing_context.commit(
                self.clip_bounds(),
                self.widget.foreground(),
                CommandTexture::None,
                None,
            );
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);
    }
}

pub struct GridBuilder {
    widget_builder: WidgetBuilder,
    rows: Vec<Row>,
    columns: Vec<Column>,
    draw_border: bool,
    border_thickness: f32,
}

impl GridBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            rows: Vec::new(),
            columns: Vec::new(),
            draw_border: false,
            border_thickness: 1.0,
        }
    }

    pub fn add_row(mut self, row: Row) -> Self {
        self.rows.push(row);
        self
    }

    pub fn add_column(mut self, column: Column) -> Self {
        self.columns.push(column);
        self
    }

    pub fn add_rows(mut self, mut rows: Vec<Row>) -> Self {
        self.rows.append(&mut rows);
        self
    }

    pub fn add_columns(mut self, mut columns: Vec<Column>) -> Self {
        self.columns.append(&mut columns);
        self
    }

    pub fn draw_border(mut self, value: bool) -> Self {
        self.draw_border = value;
        self
    }

    pub fn with_border_thickness(mut self, value: f32) -> Self {
        self.border_thickness = value;
        self
    }

    pub fn build(self, ui: &mut BuildContext) -> Handle<UiNode> {
        let grid = Grid {
            widget: self.widget_builder.build(),
            rows: RefCell::new(self.rows),
            columns: RefCell::new(self.columns),
            draw_border: self.draw_border,
            border_thickness: self.border_thickness,
            cells: Default::default(),
            groups: Default::default(),
        };
        ui.add_node(UiNode::new(grid))
    }
}

impl Grid {
    pub fn new(widget: Widget) -> Self {
        Self {
            widget,
            rows: Default::default(),
            columns: Default::default(),
            draw_border: false,
            border_thickness: 1.0,
            cells: Default::default(),
            groups: Default::default(),
        }
    }

    pub fn add_row(&mut self, row: Row) -> &mut Self {
        self.rows.borrow_mut().push(row);
        self
    }

    pub fn add_column(&mut self, column: Column) -> &mut Self {
        self.columns.borrow_mut().push(column);
        self
    }

    pub fn clear_columns(&mut self) {
        self.columns.borrow_mut().clear();
    }

    pub fn clear_rows(&mut self) {
        self.rows.borrow_mut().clear();
    }

    pub fn set_columns(&mut self, columns: Vec<Column>) {
        self.columns = RefCell::new(columns);
    }

    pub fn set_rows(&mut self, rows: Vec<Row>) {
        self.rows = RefCell::new(rows);
    }

    pub fn set_draw_border(&mut self, value: bool) -> &mut Self {
        self.draw_border = value;
        self
    }

    pub fn is_draw_border(&self) -> bool {
        self.draw_border
    }

    pub fn set_border_thickness(&mut self, value: f32) -> &mut Self {
        self.border_thickness = value;
        self
    }

    pub fn border_thickness(&self) -> f32 {
        self.border_thickness
    }
}
