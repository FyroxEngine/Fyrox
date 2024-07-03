//! Grid widget is able to position children widgets into a grid of specifically sized rows and columns. See
//! [`Grid`] doc for more info and usage examples.

#![warn(missing_docs)]

use crate::{
    core::{
        algebra::Vector2, math::Rect, pool::Handle, reflect::prelude::*, scope_profile,
        type_traits::prelude::*, uuid_provider, variable::InheritableVariable, visitor::prelude::*,
    },
    define_constructor,
    draw::{CommandTexture, Draw, DrawingContext},
    message::{MessageDirection, UiMessage},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UiNode, UserInterface,
};
use fyrox_graph::BaseSceneGraph;
use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
};
use strum_macros::{AsRefStr, EnumString, VariantNames};

/// A set of messages that can be used to modify [`Grid`] widget state.
#[derive(Debug, PartialEq, Clone)]
pub enum GridMessage {
    /// Sets new rows for the grid widget.
    Rows(Vec<Row>),
    /// Sets new columns for the grid widget.
    Columns(Vec<Column>),
    /// Sets whether the grid should draw its border or not.
    DrawBorder(bool),
    /// Sets new border thickness for the grid.
    BorderThickness(f32),
}

impl GridMessage {
    define_constructor!(
        /// Creates a new [`Self::Rows`] message.
        GridMessage:Rows => fn rows(Vec<Row>), layout: false
    );

    define_constructor!(
        /// Creates a new [`Self::Columns`] message.
        GridMessage:Columns => fn columns(Vec<Column>), layout: false
    );

    define_constructor!(
        /// Creates a new [`Self::DrawBorder`] message.
        GridMessage:DrawBorder => fn draw_border(bool), layout: false
    );

    define_constructor!(
        /// Creates a new [`Self::BorderThickness`] message.
        GridMessage:BorderThickness => fn border_thickness(f32), layout: false
    );
}

/// Size mode defines how grid's dimension (see [`GridDimension`]) will behave on layout step.
#[derive(
    Clone, Copy, PartialEq, Eq, Debug, Reflect, Visit, Default, AsRefStr, EnumString, VariantNames,
)]
pub enum SizeMode {
    /// Strict size of the dimension.
    #[default]
    Strict,
    /// Size of the dimension will match the size of the inner content.
    Auto,
    /// Size of the dimension will stretch to fit available bounds.
    Stretch,
}

uuid_provider!(SizeMode = "9c5dfbce-5df2-4a7f-8c57-c4473743a718");

/// Grid dimension defines sizing rules and constraints for [`Grid`]'s rows and columns.
#[derive(Clone, Copy, PartialEq, Debug, Reflect, Visit, Default)]
pub struct GridDimension {
    /// Current size mode of the dimension.
    pub size_mode: SizeMode,
    /// Desired size of the dimension. Makes sense only if size mode is [`SizeMode::Strict`].
    pub desired_size: f32,
    /// Measured size of the dimension. It could be considered as "output" parameter of the dimension
    /// that will be filled after measurement layout step.
    pub actual_size: f32,
    /// Local position along the axis of the dimension after arrangement step.
    pub location: f32,
}

uuid_provider!(GridDimension = "5e894900-c14a-4eb6-acb9-1636efead4b4");

impl GridDimension {
    /// Generic constructor for [`GridDimension`].
    pub fn generic(size_mode: SizeMode, desired_size: f32) -> Self {
        Self {
            size_mode,
            desired_size,
            actual_size: 0.0,
            location: 0.0,
        }
    }

    /// Creates new [`GridDimension`] with [`SizeMode::Strict`] and the specified size constraint.
    pub fn strict(desired_size: f32) -> Self {
        Self::generic(SizeMode::Strict, desired_size)
    }

    /// Creates new [`GridDimension`] with [`SizeMode::Stretch`].
    pub fn stretch() -> Self {
        Self::generic(SizeMode::Stretch, 0.0)
    }

    /// Creates new [`GridDimension`] with [`SizeMode::Auto`].
    pub fn auto() -> Self {
        Self::generic(SizeMode::Auto, 0.0)
    }
}

/// Type alias for grid columns.
pub type Column = GridDimension;

/// Type alias for grid rows.
pub type Row = GridDimension;

/// Grids are one of several methods to position multiple widgets in relation to each other. A Grid widget, as the name
/// implies, is able to position children widgets into a grid of specifically sized rows and columns.
///
/// Here is a simple example that positions several text widgets into a 2 by 2 grid:
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     UiNode, core::pool::Handle,
/// #     BuildContext,
/// #     widget::WidgetBuilder,
/// #     text::TextBuilder,
/// #     grid::{GridBuilder, GridDimension},
/// # };
/// fn create_text_grid(ctx: &mut BuildContext) -> Handle<UiNode> {
///     GridBuilder::new(
///         WidgetBuilder::new()
///             .with_child(
///                 TextBuilder::new(WidgetBuilder::new())
///                     .with_text("top left ")
///                     .build(ctx),
///             )
///             .with_child(
///                 TextBuilder::new(WidgetBuilder::new().on_column(1))
///                     .with_text(" top right")
///                     .build(ctx),
///             )
///             .with_child(
///                 TextBuilder::new(WidgetBuilder::new().on_row(1))
///                     .with_text("bottom left ")
///                     .build(ctx),
///             )
///             .with_child(
///                 TextBuilder::new(WidgetBuilder::new().on_row(1).on_column(1))
///                     .with_text(" bottom right")
///                     .build(ctx),
///             ),
///     )
///     .add_row(GridDimension::auto())
///     .add_row(GridDimension::auto())
///     .add_column(GridDimension::auto())
///     .add_column(GridDimension::auto())
///     .build(ctx)
/// }
/// ```
///
/// As with other UI widgets, Grids are created via the [`GridBuilder`] struct. Each widget whose position should be controlled
/// by the Grid should be added as a child of the [`GridBuilder`]'s base widget.
///
/// You then need to tell each child what row and column it belongs to via the [`WidgetBuilder::on_column`] and [`WidgetBuilder::on_row`]
/// functions of their base widget. By default, all children will be placed into row 0, column 0.
///
/// After that you need to provide sizing constraints for each row and column to the [`GridBuilder`] by using the [`GridBuilder::add_row`]
/// and [`GridBuilder::add_column`] functions while providing a [`GridDimension`] instance to the call. [`GridDimension`] can be
/// constructed with the following functions:
///
/// * [`GridDimension::auto`] - Sizes the row or column so it's just large enough to fit the largest child's size.
/// * [`GridDimension::stretch`] - Stretches the row or column to fill the parent's available space, if multiple rows or
/// columns have this option the size is evenly distributed between them.
/// * [`GridDimension::strict`] - Sets the row or column to be exactly the given value of pixels long. So a row will only
/// be the given number of pixels wide, while a column will be that many pixels tall.
///
/// You can add any number of rows and columns to a grid widget, and each grid cell does **not** need to have a UI widget
/// in it to be valid. For example you can add a column and set it to a specific size via strict to provide spacing between
/// two other columns.
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct Grid {
    /// Base widget of the grid.
    pub widget: Widget,
    /// A set of rows of the grid.
    pub rows: InheritableVariable<RefCell<Vec<Row>>>,
    /// A set of columns of the grid.
    pub columns: InheritableVariable<RefCell<Vec<Column>>>,
    /// Defines whether to draw grid's border or not. It could be useful for debugging purposes.
    pub draw_border: InheritableVariable<bool>,
    /// Defines border thickness when `draw_border` is on.
    pub border_thickness: InheritableVariable<f32>,
    /// Current set of cells of the grid.
    #[visit(skip)]
    #[reflect(hidden)]
    pub cells: RefCell<Vec<Cell>>,
    /// A set of four groups, where each group contains cell indices. It is used for measurement
    /// purposes to group the cells in specific way, so it can be measured in the correct order
    /// later.
    #[visit(skip)]
    #[reflect(hidden)]
    pub groups: RefCell<[Vec<usize>; 4]>,
}

crate::define_widget_deref!(Grid);

/// Cell of the grid, that contains additional information for layout purposes. It does not have any
/// particular use outside of grid's internals.
#[derive(Clone, Debug)]
pub struct Cell {
    /// A set of nodes of the cell.
    pub nodes: Vec<Handle<UiNode>>,
    /// Current width constraint of the cell.
    pub width_constraint: Option<f32>,
    /// Current height constraint of the cell.
    pub height_constraint: Option<f32>,
    /// Vertical location of the cell (row number).
    pub row_index: usize,
    /// Horizontal location of the cell (column number).
    pub column_index: usize,
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

uuid_provider!(Grid = "98ce15e2-bd62-497d-a37b-9b1cb4a1918c");

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
        if *self.draw_border {
            let bounds = self.widget.bounding_rect();

            let left_top = Vector2::new(bounds.x(), bounds.y());
            let right_top = Vector2::new(bounds.x() + bounds.w(), bounds.y());
            let right_bottom = Vector2::new(bounds.x() + bounds.w(), bounds.y() + bounds.h());
            let left_bottom = Vector2::new(bounds.x(), bounds.y() + bounds.h());

            drawing_context.push_line(left_top, right_top, *self.border_thickness);
            drawing_context.push_line(right_top, right_bottom, *self.border_thickness);
            drawing_context.push_line(right_bottom, left_bottom, *self.border_thickness);
            drawing_context.push_line(left_bottom, left_top, *self.border_thickness);

            for column in self.columns.borrow().iter() {
                let a = Vector2::new(bounds.x() + column.location, bounds.y());
                let b = Vector2::new(bounds.x() + column.location, bounds.y() + bounds.h());
                drawing_context.push_line(a, b, *self.border_thickness);
            }
            for row in self.rows.borrow().iter() {
                let a = Vector2::new(bounds.x(), bounds.y() + row.location);
                let b = Vector2::new(bounds.x() + bounds.w(), bounds.y() + row.location);
                drawing_context.push_line(a, b, *self.border_thickness);
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

        if let Some(msg) = message.data::<GridMessage>() {
            if message.direction() == MessageDirection::ToWidget
                && message.destination() == self.handle
            {
                match msg {
                    GridMessage::Rows(rows) => {
                        if &*self.rows.borrow() != rows {
                            self.rows
                                .set_value_and_mark_modified(RefCell::new(rows.clone()));
                            self.invalidate_layout();
                        }
                    }
                    GridMessage::Columns(columns) => {
                        if &*self.columns.borrow() != columns {
                            self.columns
                                .set_value_and_mark_modified(RefCell::new(columns.clone()));
                            self.invalidate_layout();
                        }
                    }
                    GridMessage::DrawBorder(draw_border) => {
                        self.draw_border.set_value_and_mark_modified(*draw_border);
                    }
                    GridMessage::BorderThickness(border_thickness) => {
                        self.border_thickness
                            .set_value_and_mark_modified(*border_thickness);
                    }
                }
            }
        }
    }
}

/// Grid builder creates [`Grid`] instances and adds it to the user interface.
pub struct GridBuilder {
    widget_builder: WidgetBuilder,
    rows: Vec<Row>,
    columns: Vec<Column>,
    draw_border: bool,
    border_thickness: f32,
}

impl GridBuilder {
    /// Creates new grid builder with the base widget builder.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            rows: Vec::new(),
            columns: Vec::new(),
            draw_border: false,
            border_thickness: 1.0,
        }
    }

    /// Adds a new row to the grid builder. Number of rows is unlimited.
    pub fn add_row(mut self, row: Row) -> Self {
        self.rows.push(row);
        self
    }

    /// Adds a new column to the grid builder. Number of columns is unlimited.
    pub fn add_column(mut self, column: Column) -> Self {
        self.columns.push(column);
        self
    }

    /// Adds a set of rows to the grid builder. Number of rows is unlimited.
    pub fn add_rows(mut self, mut rows: Vec<Row>) -> Self {
        self.rows.append(&mut rows);
        self
    }

    /// Adds a set of columnds to the grid builder. Number of columnds is unlimited.
    pub fn add_columns(mut self, mut columns: Vec<Column>) -> Self {
        self.columns.append(&mut columns);
        self
    }

    /// Specifies whether the grid should draw its border or not.
    pub fn draw_border(mut self, value: bool) -> Self {
        self.draw_border = value;
        self
    }

    /// Specifies grid's border thickness.
    pub fn with_border_thickness(mut self, value: f32) -> Self {
        self.border_thickness = value;
        self
    }

    /// Creates new [`Grid`] widget instance and adds it to the user interface.
    pub fn build(self, ui: &mut BuildContext) -> Handle<UiNode> {
        let grid = Grid {
            widget: self.widget_builder.build(),
            rows: RefCell::new(self.rows).into(),
            columns: RefCell::new(self.columns).into(),
            draw_border: self.draw_border.into(),
            border_thickness: self.border_thickness.into(),
            cells: Default::default(),
            groups: Default::default(),
        };
        ui.add_node(UiNode::new(grid))
    }
}
