// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Grid widget is able to position children widgets into a grid of specifically sized rows and columns. See
//! [`Grid`] doc for more info and usage examples.

#![warn(missing_docs)]

use crate::{
    core::{
        algebra::Vector2, log::Log, math::Rect, pool::Handle, reflect::prelude::*,
        type_traits::prelude::*, uuid_provider, variable::InheritableVariable, visitor::prelude::*,
    },
    define_constructor,
    draw::{CommandTexture, Draw, DrawingContext},
    message::{MessageDirection, UiMessage},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UiNode, UserInterface,
};
use core::f32;

use fyrox_graph::constructor::{ConstructorProvider, GraphNodeConstructor};
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
    /// The desired size of this dimension must be provided in advance,
    /// and it will always be rendered with exactly that size, regardless of what nodes it contains.
    #[default]
    Strict,
    /// The desired size of this dimension is the maximum of the desired sizes of all the nodes when
    /// they are measured with infinite available size.
    Auto,
    /// The size of this dimension is determined by subtracting the desired size of the other rows/columns
    /// from the total available size, if the available size is finite.
    /// If the total available size is infinite, then Stretch is equivalent to Auto.
    Stretch,
}

uuid_provider!(SizeMode = "9c5dfbce-5df2-4a7f-8c57-c4473743a718");

/// Grid dimension defines sizing rules and constraints for [`Grid`]'s rows and columns.
#[derive(Clone, Copy, PartialEq, Debug, Reflect, Visit, Default)]
pub struct GridDimension {
    /// Current size mode of the dimension.
    pub size_mode: SizeMode,
    /// Desired size of the dimension. This must be supplied if [`SizeMode::Strict`],
    /// and it is automatically calculated if [`SizeMode::Auto`].
    /// If [`SizeMode::Stretch`]. this represents the size of the dimension before excess space is added.
    pub desired_size: f32,
    /// Measured size of the dimension. It could be considered as "output" parameter of the dimension
    /// that will be filled after measurement layout step. It is used to calculate the grid's desired size.
    pub actual_size: f32,
    /// Local position along the axis of the dimension after arrangement step.
    pub location: f32,
    /// The number of children in this dimension that still need to be measured before the size is known.
    /// For Auto rows and columns this is initially the number of nodes in that row or column,
    /// and then it is reduced as nodes are measured.
    /// This is zero for all non-Auto rows and columns.
    #[visit(skip)]
    #[reflect(hidden)]
    unmeasured_node_count: usize,
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
            unmeasured_node_count: 0,
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

    fn update_size(&mut self, node_size: f32, available_size: f32) {
        match self.size_mode {
            SizeMode::Strict => (),
            SizeMode::Auto => {
                self.desired_size = self.desired_size.max(node_size);
                self.actual_size = self.desired_size;
            }
            SizeMode::Stretch => {
                if available_size.is_finite() {
                    self.actual_size = self.desired_size + available_size;
                } else {
                    self.actual_size = node_size;
                }
            }
        }
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
#[reflect(derived_type = "UiNode")]
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

impl ConstructorProvider<UiNode, UserInterface> for Grid {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>()
            .with_variant("Grid", |ui| {
                GridBuilder::new(WidgetBuilder::new().with_name("Grid"))
                    .build(&mut ui.build_ctx())
                    .into()
            })
            .with_group("Layout")
    }
}

crate::define_widget_deref!(Grid);

/// Cell of the grid, that contains additional information for layout purposes. It does not have any
/// particular use outside of grid's internals.
#[derive(Clone, Debug)]
pub struct Cell {
    /// A set of nodes of the cell.
    pub nodes: Vec<Handle<UiNode>>,
    /// Vertical location of the cell (row number).
    pub row_index: usize,
    /// Horizontal location of the cell (column number).
    pub column_index: usize,
}

/// ```text
///                Strict   Auto   Stretch
///               +-------+-------+-------+
///               |       |       |       |
///        Strict |   0   |   0   |   2   |
///               |       |       |       |
///               +-------+-------+-------+
///               |       |       |       |
///          Auto |   0   |   0   |   2   |
///               |       |       |       |
///               +-------+-------+-------+
///               |       |       |       |
///       Stretch |   3   |   1   |   3   |
///               |       |       |       |
///               +-------+-------+-------+
/// ```
/// Group 0 represents all nodes with no stretch. They can be measured without needing any
/// desired size information from other nodes, and so they are always measured first.
///
/// Group 1 is special because it contains all the remaining auto-width nodes
/// after group 0 has been measured, and group 1 may blocked from being measured
/// due to group 2 not yet being measured to provide the desired size of the
/// remaining auto rows.
///
/// In order to allow measurement to proceed in that situation, group 1 may be forced
/// to measure despite not yet knowing its true vertical available size.
/// The width information gained from the measurement of group 1 makes it possible to
/// measure group 2, and then group 1 will be measured a second time to get its
/// correct desired height. Group 1 is the only group that is ever measured twice.
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

fn choose_constraint(dimension: &GridDimension, available_size: f32) -> f32 {
    match dimension.size_mode {
        // Strict always has a constraint of its desired size.
        SizeMode::Strict => dimension.desired_size,
        // For Stretch rows and columns, the available size is whatever size is not used up
        // by the other rows and columns.
        // First we give the node the desired size, which is most likely zero for a Stretch row/column,
        // then we expand it to include the available size.
        SizeMode::Stretch => dimension.desired_size + available_size,
        // Auto means being free to choose whatever size the widget pleases.
        // If the constraint were set to `available_size` then the widget might choose
        // to use all of that size and crowd out all other cells of the grid.
        // A constraint of infinity encourages the node to pick a more reasonable size.
        SizeMode::Auto => f32::INFINITY,
    }
}

fn calc_total_size_of_non_stretch_dims(dims: &[GridDimension]) -> Option<f32> {
    if dims.iter().all(|d| d.size_mode != SizeMode::Stretch) {
        // If there are no stretch rows/columns, then the value we return will never be used.
        Some(0.0) // Arbitrarily choose 0.0, but it should not matter.
    } else if dims.iter().all(|d| d.unmeasured_node_count == 0) {
        // We have at least one stretch, so seriously calculate the size
        // This requires that all the autos be already measured.
        Some(dims.iter().map(|d| d.desired_size).sum())
    } else {
        // We have at least one stretch but not all the autos are measured
        // so we fail.
        None
    }
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
    dims: &RefCell<Vec<GridDimension>>,
    available_size: f32,
) -> Option<f32> {
    if available_size.is_infinite() {
        // If we have limitless available size, then short-circuit to avoid the possibility
        // of returning None due to missing Auto measurements. Measuring Auto nodes does not matter
        // when available_size is infinite, and returning None might force an unnecessary double-measure.
        return Some(available_size);
    }
    let dims = dims.borrow();
    let stretch_sized_dims = count_stretch_dims(&dims);
    if stretch_sized_dims > 0 {
        let rest_size = available_size - calc_total_size_of_non_stretch_dims(&dims)?;
        Some(rest_size / stretch_sized_dims as f32)
    } else {
        // If there are no stretch nodes in this row/column, then this result will never be used.
        Some(0.0) // Choose 0.0 arbitrarily.
    }
}

fn arrange_dims(dims: &mut [GridDimension], final_size: f32) {
    // Every row/column has a desired size, so summing all the desired sizes is correct.
    // Strict rows/columns have their desired size set when building the grid.
    // Auto rows/columns are calculated in the measure step.
    // Stretch rows/columns default to zero.
    let preset_width: f32 = dims.iter().map(|d| d.desired_size).sum();

    let stretch_count = count_stretch_dims(dims);
    let avg_stretch = if stretch_count > 0 {
        (final_size - preset_width) / stretch_count as f32
    } else {
        // Since stretch_count is zero, this value will never be used.
        0.0
    };

    let mut location = 0.0;
    for dim in dims.iter_mut() {
        dim.location = location;
        dim.actual_size = match dim.size_mode {
            SizeMode::Strict | SizeMode::Auto => dim.desired_size,
            SizeMode::Stretch => dim.desired_size + avg_stretch,
        };
        location += dim.actual_size;
    }
}

uuid_provider!(Grid = "98ce15e2-bd62-497d-a37b-9b1cb4a1918c");

impl Grid {
    fn initialize_measure(&self, ui: &UserInterface) {
        self.calc_needed_measurements(ui);

        let mut groups = self.groups.borrow_mut();
        for group in groups.iter_mut() {
            group.clear();
        }

        let mut cells = self.cells.borrow_mut();
        cells.clear();

        let rows = self.rows.borrow();
        let columns = self.columns.borrow();
        for (column_index, column) in columns.iter().enumerate() {
            for (row_index, row) in rows.iter().enumerate() {
                groups[group_index(row.size_mode, column.size_mode)].push(cells.len());

                cells.push(Cell {
                    nodes: self
                        .children()
                        .iter()
                        .copied()
                        .filter(|&c| {
                            let Some(child_ref) = ui.try_get(c) else {
                                return false;
                            };
                            child_ref.row() == row_index && child_ref.column() == column_index
                        })
                        .collect(),
                    row_index,
                    column_index,
                })
            }
        }
    }
    fn calc_needed_measurements(&self, ui: &UserInterface) {
        let mut rows = self.rows.borrow_mut();
        let mut cols = self.columns.borrow_mut();
        for dim in rows.iter_mut().chain(cols.iter_mut()) {
            dim.unmeasured_node_count = 0;
            match dim.size_mode {
                SizeMode::Auto => dim.desired_size = 0.0,
                SizeMode::Strict => dim.actual_size = dim.desired_size,
                SizeMode::Stretch => (),
            }
        }
        for handle in self.children() {
            let Some(node) = ui.try_get(*handle) else {
                continue;
            };
            let Some(row) = rows.get_mut(node.row()) else {
                Log::err(format!(
                    "Node row out of bounds: {} row:{}, column:{}",
                    Reflect::type_name(node),
                    node.row(),
                    node.column()
                ));
                continue;
            };
            let Some(col) = cols.get_mut(node.column()) else {
                Log::err(format!(
                    "Node column out of bounds: {} row:{}, column:{}",
                    Reflect::type_name(node),
                    node.row(),
                    node.column()
                ));
                continue;
            };
            if col.size_mode == SizeMode::Auto {
                col.unmeasured_node_count += 1
            }
            if row.size_mode == SizeMode::Auto {
                row.unmeasured_node_count += 1
            }
        }
    }
    fn measure_width_and_height(
        &self,
        child: Handle<UiNode>,
        ui: &UserInterface,
        available_size: Vector2<f32>,
        measure_width: bool,
        measure_height: bool,
    ) {
        let Some(node) = ui.try_get(child) else {
            return;
        };
        let mut rows = self.rows.borrow_mut();
        let mut cols = self.columns.borrow_mut();
        let Some(row) = rows.get_mut(node.row()) else {
            return;
        };
        let Some(col) = cols.get_mut(node.column()) else {
            return;
        };
        let constraint = Vector2::new(
            choose_constraint(col, available_size.x),
            choose_constraint(row, available_size.y),
        );
        ui.measure_node(child, constraint);
        if measure_width {
            col.update_size(node.desired_size().x, available_size.x);
            if col.size_mode == SizeMode::Auto {
                col.unmeasured_node_count -= 1;
            }
        }
        if measure_height {
            row.update_size(node.desired_size().y, available_size.y);
            if row.size_mode == SizeMode::Auto {
                row.unmeasured_node_count -= 1;
            }
        }
    }
    fn measure_group_width(
        &self,
        group: &[usize],
        ui: &UserInterface,
        available_size: Vector2<f32>,
    ) {
        let cells = self.cells.borrow();
        for cell in group.iter().map(|&i| &cells[i]) {
            for n in cell.nodes.iter() {
                self.measure_width_and_height(*n, ui, available_size, true, false);
            }
        }
    }
    fn measure_group_height(
        &self,
        group: &[usize],
        ui: &UserInterface,
        available_size: Vector2<f32>,
    ) {
        let cells = self.cells.borrow();
        for cell in group.iter().map(|&i| &cells[i]) {
            for n in cell.nodes.iter() {
                self.measure_width_and_height(*n, ui, available_size, false, true);
            }
        }
    }
    fn measure_group(&self, group: &[usize], ui: &UserInterface, available_size: Vector2<f32>) {
        let cells = self.cells.borrow();
        for cell in group.iter().map(|&i| &cells[i]) {
            for n in cell.nodes.iter() {
                self.measure_width_and_height(*n, ui, available_size, true, true);
            }
        }
    }
}

impl Control for Grid {
    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        // In case of no rows or columns, grid acts like default panel.
        if self.columns.borrow().is_empty() || self.rows.borrow().is_empty() {
            return self.widget.measure_override(ui, available_size);
        }

        self.initialize_measure(ui);

        let groups = self.groups.borrow_mut();

        // Start by measuring all the nodes with no stretch in either dimension: group 0
        self.measure_group(&groups[0], ui, available_size);

        if let Some(space_y) = calc_avg_size_for_stretch_dim(&self.rows, available_size.y) {
            // Measuring group 0 was enough to allow us to calculate the needed stretch along the height of the grid,
            // so use that stretch to measure group 1 (auto width, stretch height).
            self.measure_group(&groups[1], ui, Vector2::new(available_size.x, space_y));
            // Measuring group 0 and group 1 guarantees that we have measured all the auto-width nodes, so this is safe to unwrap.
            let space_x = calc_avg_size_for_stretch_dim(&self.columns, available_size.x).unwrap();
            // Use the calculated horizontal stretch to measure all the remaining nodes.
            self.measure_group(&groups[2], ui, Vector2::new(space_x, available_size.y));
            self.measure_group(&groups[3], ui, Vector2::new(space_x, space_y));
        } else if let Some(space_x) = calc_avg_size_for_stretch_dim(&self.columns, available_size.x)
        {
            // We were unable to calculate the vertical stretch, but we can calculate the horizontal stretch,
            // so use the horizontal stretch to measure group 2 (stretch width, strict/auto height).
            // We know that group 1 is empty, since group 1 has auto width and we have not yet measured group 1.
            self.measure_group(&groups[2], ui, Vector2::new(space_x, available_size.y));
            // Measuring group 0 and group 2 guarantees that we have measured all the auto-height nodes, so this is safe to unwrap.
            let space_y = calc_avg_size_for_stretch_dim(&self.rows, available_size.y).unwrap();
            // Use the calculated vertical stretch to measure the remaining nodes.
            self.measure_group(&groups[3], ui, Vector2::new(space_x, space_y));
        } else {
            // We could not calculate either the vertical stretch or the horizontal stretch.
            // The only horizontal autos we have not measured are in group 1 (auto width, stretch height),
            // so we are forced to measure group 1 as it if had auto height, just so it can provide its width to its column.
            // The desired height provided by this measurement is ignored.
            self.measure_group_width(&groups[1], ui, Vector2::new(f32::INFINITY, f32::INFINITY));
            // Measuring group 0 and group 1 guarantees that we have measured all the auto-width nodes, so this is safe to unwrap.
            let space_x = calc_avg_size_for_stretch_dim(&self.columns, available_size.x).unwrap();
            // Use the calculated horizontal stretch to measure group 2 (stretch width, strict/auto height).
            self.measure_group(&groups[2], ui, Vector2::new(space_x, available_size.y));
            // Measuring group 0 and group 2 guarantees that we have measured all the auto-height nodes, so this is safe to unwrap.
            let space_y = calc_avg_size_for_stretch_dim(&self.rows, available_size.y).unwrap();
            // Now that we finally have the vertical stretch amount, we can properly measure group 1 (auto width, stretch height).
            // This is the only time we measure a node twice. The first time was just to discover the width.
            // This measurement is just for height, now that we can give the node the true available veritical size.
            self.measure_group_height(&groups[1], ui, Vector2::new(available_size.x, space_y));
            self.measure_group(&groups[3], ui, Vector2::new(space_x, space_y));
        }

        let desired_size = Vector2::<f32>::new(
            self.columns.borrow().iter().map(|c| c.actual_size).sum(),
            self.rows.borrow().iter().map(|r| r.actual_size).sum(),
        );
        desired_size
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
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
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let grid = Grid {
            widget: self.widget_builder.build(ctx),
            rows: RefCell::new(self.rows).into(),
            columns: RefCell::new(self.columns).into(),
            draw_border: self.draw_border.into(),
            border_thickness: self.border_thickness.into(),
            cells: Default::default(),
            groups: Default::default(),
        };
        ctx.add_node(UiNode::new(grid))
    }
}

#[cfg(test)]
mod test {
    use crate::grid::GridBuilder;
    use crate::{test::test_widget_deletion, widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| GridBuilder::new(WidgetBuilder::new()).build(ctx));
    }
}
