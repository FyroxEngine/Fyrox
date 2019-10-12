use std::cell::RefCell;
use rg3d_core::{
    pool::Handle,
    math::{
        vec2::Vec2,
        Rect,
    },
};
use crate::gui::{
    EventSource,
    UserInterface,
    Visibility,
    node::{UINode, UINodeKind},
    builder::CommonBuilderFields,
    Layout,
    event::UIEvent,
};

#[derive(PartialEq)]
pub enum SizeMode {
    Strict,
    Auto,
    Stretch,
}

pub struct Column {
    size_mode: SizeMode,
    desired_width: f32,
    actual_width: f32,
    x: f32,
}

impl Column {
    pub fn generic(size_mode: SizeMode, desired_width: f32) -> Self {
        Column {
            size_mode,
            desired_width,
            actual_width: 0.0,
            x: 0.0,
        }
    }

    pub fn strict(desired_width: f32) -> Self {
        Self {
            size_mode: SizeMode::Strict,
            desired_width,
            actual_width: 0.0,
            x: 0.0,
        }
    }

    pub fn stretch() -> Self {
        Self {
            size_mode: SizeMode::Stretch,
            desired_width: 0.0,
            actual_width: 0.0,
            x: 0.0,
        }
    }

    pub fn auto() -> Self {
        Self {
            size_mode: SizeMode::Auto,
            desired_width: 0.0,
            actual_width: 0.0,
            x: 0.0,
        }
    }
}

pub struct Row {
    size_mode: SizeMode,
    desired_height: f32,
    actual_height: f32,
    y: f32,
}

impl Row {
    pub fn generic(size_mode: SizeMode, desired_height: f32) -> Self {
        Self {
            size_mode,
            desired_height,
            actual_height: 0.0,
            y: 0.0,
        }
    }

    pub fn strict(desired_height: f32) -> Self {
        Self {
            size_mode: SizeMode::Strict,
            desired_height,
            actual_height: 0.0,
            y: 0.0,
        }
    }

    pub fn stretch() -> Self {
        Self {
            size_mode: SizeMode::Stretch,
            desired_height: 0.0,
            actual_height: 0.0,
            y: 0.0,
        }
    }

    pub fn auto() -> Self {
        Self {
            size_mode: SizeMode::Auto,
            desired_height: 0.0,
            actual_height: 0.0,
            y: 0.0,
        }
    }
}

pub struct Grid {
    rows: RefCell<Vec<Row>>,
    columns: RefCell<Vec<Column>>,
}

impl Grid {
    fn new() -> Self {
        Self {
            rows: RefCell::new(Vec::new()),
            columns: RefCell::new(Vec::new()),
        }
    }
}

impl Layout for Grid {
    fn measure_override(&self, self_handle: Handle<UINode>, ui: &UserInterface, available_size: Vec2) -> Vec2 {
        // In case of no rows or columns, grid acts like default panel.
        if self.columns.borrow().is_empty() || self.rows.borrow().is_empty() {
            return ui.default_measure_override(self_handle, available_size);
        }

        let mut desired_size = Vec2::zero();
        let node = ui.nodes.borrow(self_handle);
        // Step 1. Measure every children with relaxed constraints (size of grid).
        for child_handle in node.children.iter() {
            ui.measure(*child_handle, available_size);
        }

        // Step 2. Calculate width of columns and heights of rows.
        let mut preset_width = 0.0;
        let mut preset_height = 0.0;

        // Step 2.1. Calculate size of strict-sized and auto-sized columns.
        for (i, col) in self.columns.borrow_mut().iter_mut().enumerate() {
            if col.size_mode == SizeMode::Strict {
                col.actual_width = col.desired_width;
                preset_width += col.actual_width;
            } else if col.size_mode == SizeMode::Auto {
                col.actual_width = col.desired_width;
                for child_handle in node.children.iter() {
                    let child = ui.nodes.borrow(*child_handle);
                    if child.column == i && child.visibility == Visibility::Visible && child.desired_size.get().x > col.actual_width {
                        col.actual_width = child.desired_size.get().x;
                    }
                }
                preset_width += col.actual_width;
            }
        }

        // Step 2.2. Calculate size of strict-sized and auto-sized rows.
        for (i, row) in self.rows.borrow_mut().iter_mut().enumerate() {
            if row.size_mode == SizeMode::Strict {
                row.actual_height = row.desired_height;
                preset_height += row.actual_height;
            } else if row.size_mode == SizeMode::Auto {
                row.actual_height = row.desired_height;
                for child_handle in node.children.iter() {
                    let child = ui.nodes.borrow(*child_handle);
                    if child.row == i && child.visibility == Visibility::Visible && child.desired_size.get().y > row.actual_height {
                        row.actual_height = child.desired_size.get().y;
                    }
                }
                preset_height += row.actual_height;
            }
        }

        // Step 2.3. Fit stretch-sized columns
        let mut rest_width = 0.0;
        if available_size.x.is_infinite() {
            for child_handle in node.children.iter() {
                let child = ui.nodes.borrow(*child_handle);
                if let Some(column) = self.columns.borrow().get(child.column) {
                    if column.size_mode == SizeMode::Stretch {
                        rest_width += child.desired_size.get().x;
                    }
                }
            }
        } else {
            rest_width = available_size.x - preset_width;
        }

        // count columns first
        let mut stretch_sized_columns = 0;
        for column in self.columns.borrow().iter() {
            if column.size_mode == SizeMode::Stretch {
                stretch_sized_columns += 1;
            }
        }
        if stretch_sized_columns > 0 {
            let width_per_col = rest_width / stretch_sized_columns as f32;
            for column in self.columns.borrow_mut().iter_mut() {
                if column.size_mode == SizeMode::Stretch {
                    column.actual_width = width_per_col;
                }
            }
        }

        // Step 2.4. Fit stretch-sized rows.
        let mut stretch_sized_rows = 0;
        let mut rest_height = 0.0;
        if available_size.y.is_infinite() {
            for child_handle in node.children.iter() {
                let child = ui.nodes.borrow(*child_handle);
                if let Some(row) = self.rows.borrow().get(child.row) {
                    if row.size_mode == SizeMode::Stretch {
                        rest_height += child.desired_size.get().y;
                    }
                }
            }
        } else {
            rest_height = available_size.y - preset_height;
        }
        // count rows first
        for row in self.rows.borrow().iter() {
            if row.size_mode == SizeMode::Stretch {
                stretch_sized_rows += 1;
            }
        }
        if stretch_sized_rows > 0 {
            let height_per_row = rest_height / stretch_sized_rows as f32;
            for row in self.rows.borrow_mut().iter_mut() {
                if row.size_mode == SizeMode::Stretch {
                    row.actual_height = height_per_row;
                }
            }
        }

        // Step 2.5. Calculate positions of each column.
        let mut y = 0.0;
        for row in self.rows.borrow_mut().iter_mut() {
            row.y = y;
            y += row.actual_height;
        }

        // Step 2.6. Calculate positions of each row.
        let mut x = 0.0;
        for column in self.columns.borrow_mut().iter_mut() {
            column.x = x;
            x += column.actual_width;
        }

        // Step 3. Re-measure children with new constraints.
        for child_handle in node.children.iter() {
            let size_for_child = {
                let child = ui.nodes.borrow(*child_handle);
                Vec2 {
                    x: self.columns.borrow()[child.column].actual_width,
                    y: self.rows.borrow()[child.row].actual_height,
                }
            };
            ui.measure(*child_handle, size_for_child);
        }

        // Step 4. Calculate desired size of grid.
        for column in self.columns.borrow().iter() {
            desired_size.x += column.actual_width;
        }
        for row in self.rows.borrow().iter() {
            desired_size.y += row.actual_height;
        }

        desired_size
    }

    fn arrange_override(&self, self_handle: Handle<UINode>, ui: &UserInterface, final_size: Vec2) -> Vec2 {
        let node = ui.nodes.borrow(self_handle);
        if self.columns.borrow().is_empty() || self.rows.borrow().is_empty() {
            let rect = Rect::new(0.0, 0.0, final_size.x, final_size.y);
            for child_handle in node.children.iter() {
                ui.arrange(*child_handle, &rect);
            }
            return final_size;
        }

        for child_handle in node.children.iter() {
            let mut final_rect = None;

            let child = ui.nodes.borrow(*child_handle);
            if let Some(column) = self.columns.borrow().get(child.column) {
                if let Some(row) = self.rows.borrow().get(child.row) {
                    final_rect = Some(Rect::new(
                        column.x,
                        row.y,
                        column.actual_width,
                        row.actual_height,
                    ));
                }
            }

            if let Some(rect) = final_rect {
                ui.arrange(*child_handle, &rect);
            }
        }

        final_size
    }
}

pub struct GridBuilder {
    rows: Vec<Row>,
    columns: Vec<Column>,
    common: CommonBuilderFields,
}

impl Default for GridBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl GridBuilder {
    pub fn new() -> Self {
        GridBuilder {
            rows: Vec::new(),
            columns: Vec::new(),
            common: CommonBuilderFields::new(),
        }
    }

    impl_default_builder_methods!();

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

    pub fn build(mut self, ui: &mut UserInterface) -> Handle<UINode> {
        let mut grid = Grid::new();
        grid.columns = RefCell::new(self.columns);
        grid.rows = RefCell::new(self.rows);

        let node = UINode::new(UINodeKind::Grid(grid));

        let handle = ui.add_node(node);
        self.common.apply(ui, handle);
        handle
    }
}

impl Grid {
    pub fn add_row(&mut self, row: Row) -> &mut Self {
        self.rows.borrow_mut().push(row);
        self
    }

    pub fn add_column(&mut self, column: Column) -> &mut Self {
        self.columns.borrow_mut().push(column);
        self
    }
}

impl EventSource for Grid {
    fn emit_event(&mut self) -> Option<UIEvent> {
        None
    }
}