use std::{
    cell::RefCell,
    collections::HashMap,
};
use crate::{
    core::{
        pool::Handle,
        math::{
            vec2::Vec2,
            Rect,
        },
    },
    Visibility,
    UserInterface,
    widget::{
        WidgetBuilder,
        Widget,
    },
    Control,
    UINode,
    ControlTemplate,
    UINodeContainer,
    Builder,
};
use crate::draw::{DrawingContext, CommandKind, CommandTexture};

#[derive(Clone, Copy, PartialEq)]
pub enum SizeMode {
    Strict,
    Auto,
    Stretch,
}

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy)]
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

/// Automatically arranges children by rows and columns
pub struct Grid {
    widget: Widget,
    rows: RefCell<Vec<Row>>,
    columns: RefCell<Vec<Column>>,
    draw_border: bool,
    border_thickness: f32
}

impl Control for Grid {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }

    fn raw_copy(&self) -> Box<dyn Control> {
        Box::new(Self {
            widget: *self.widget.raw_copy().downcast::<Widget>().unwrap_or_else(|_| panic!()),
            rows: self.rows.clone(),
            columns: self.columns.clone(),
            draw_border: self.draw_border,
            border_thickness: self.border_thickness
        })
    }

    fn resolve(&mut self, _: &ControlTemplate, _: &HashMap<Handle<UINode>, Handle<UINode>>) {}

    fn measure_override(&self, ui: &UserInterface, available_size: Vec2) -> Vec2 {
        // In case of no rows or columns, grid acts like default panel.
        if self.columns.borrow().is_empty() || self.rows.borrow().is_empty() {
            return self.widget.measure_override(ui, available_size);
        }

        let mut desired_size = Vec2::ZERO;
        // Step 1. Measure every children with relaxed constraints (size of grid).
        for child_handle in self.widget.children.iter() {
            ui.node(*child_handle).measure(ui, available_size);
        }

        // Step 2. Calculate width of columns and heights of rows.
        let preset_width = self.calculate_preset_width(ui);
        let preset_height = self.calculate_preset_height(ui);

        self.fit_stretch_sized_columns(ui, available_size, preset_width);
        self.fit_stretch_sized_rows(ui, available_size, preset_height);

        self.arrange_rows();
        self.arrange_columns();

        // Step 3. Re-measure children with new constraints.
        for child_handle in self.widget.children.iter() {
            let size_for_child = {
                let child = ui.nodes.borrow(*child_handle).widget();
                Vec2 {
                    x: self.columns.borrow()[child.column()].actual_width,
                    y: self.rows.borrow()[child.row()].actual_height,
                }
            };
            ui.node(*child_handle).measure(ui, size_for_child);
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

    fn arrange_override(&self, ui: &UserInterface, final_size: Vec2) -> Vec2 {
        if self.columns.borrow().is_empty() || self.rows.borrow().is_empty() {
            let rect = Rect::new(0.0, 0.0, final_size.x, final_size.y);
            for child_handle in self.widget.children.iter() {
                ui.node(*child_handle).arrange(ui, &rect);
            }
            return final_size;
        }

        for child_handle in self.widget.children.iter() {
            let mut final_rect = None;

            let child = ui.nodes.borrow(*child_handle).widget();
            if let Some(column) = self.columns.borrow().get(child.column()) {
                if let Some(row) = self.rows.borrow().get(child.row()) {
                    final_rect = Some(Rect::new(
                        column.x,
                        row.y,
                        column.actual_width,
                        row.actual_height,
                    ));
                }
            }

            if let Some(rect) = final_rect {
                ui.nodes.borrow(*child_handle).arrange(ui, &rect);
            }
        }

        final_size
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        if self.draw_border {
            let bounds = self.widget.get_screen_bounds();

            let left_top = Vec2::new(bounds.x, bounds.y);
            let right_top = Vec2::new(bounds.x + bounds.w, bounds.y);
            let right_bottom = Vec2::new(bounds.x + bounds.w, bounds.y + bounds.h);
            let left_bottom = Vec2::new(bounds.x, bounds.y + bounds.h);

            drawing_context.push_line(left_top, right_top, self.border_thickness);
            drawing_context.push_line(right_top, right_bottom, self.border_thickness);
            drawing_context.push_line(right_bottom, left_bottom, self.border_thickness);
            drawing_context.push_line(left_bottom, left_top, self.border_thickness);

            for column in self.columns.borrow().iter() {
                let a = Vec2::new(bounds.x + column.x, bounds.y);
                let b = Vec2::new(bounds.x + column.x, bounds.y + bounds.h);
                drawing_context.push_line(a, b, self.border_thickness);
            }
            for row in self.rows.borrow().iter() {
                let a = Vec2::new(bounds.x, bounds.y + row.y);
                let b = Vec2::new(bounds.x + bounds.w, bounds.y + row.y);
                drawing_context.push_line(a, b, self.border_thickness);
            }

            drawing_context.commit(CommandKind::Geometry, self.widget.foreground(), CommandTexture::None);
        }
    }
}

pub struct GridBuilder {
    widget_builder: WidgetBuilder,
    rows: Vec<Row>,
    columns: Vec<Column>,
    draw_border: bool,
    border_thickness: f32
}

impl GridBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        GridBuilder {
            widget_builder,
            rows: Vec::new(),
            columns: Vec::new(),
            draw_border: false,
            border_thickness: 1.0
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
}

impl Builder for GridBuilder {
    fn build(self, ui: &mut dyn UINodeContainer) -> Handle<UINode> {
        ui.add_node(Box::new(Grid {
            widget: self.widget_builder.build(),
            rows: RefCell::new(self.rows),
            columns: RefCell::new(self.columns),
            draw_border: self.draw_border,
            border_thickness: self.border_thickness,
        }))
    }
}

impl Grid {
    pub fn new(widget: Widget) -> Self {
        Self {
            widget,
            rows: Default::default(),
            columns: Default::default(),
            draw_border: false,
            border_thickness: 1.0
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

    fn calculate_preset_width(&self, ui: &UserInterface) -> f32 {
        let mut preset_width = 0.0;

        // Calculate size of strict-sized and auto-sized columns.
        for (i, col) in self.columns.borrow_mut().iter_mut().enumerate() {
            if col.size_mode == SizeMode::Strict {
                col.actual_width = col.desired_width;
                preset_width += col.actual_width;
            } else if col.size_mode == SizeMode::Auto {
                col.actual_width = col.desired_width;
                for child_handle in self.widget.children.iter() {
                    let child = ui.nodes.borrow(*child_handle).widget();
                    if child.column() == i && child.visibility == Visibility::Visible && child.desired_size.get().x > col.actual_width {
                        col.actual_width = child.desired_size.get().x;
                    }
                }
                preset_width += col.actual_width;
            }
        }

        preset_width
    }

    fn calculate_preset_height(&self, ui: &UserInterface) -> f32 {
        let mut preset_height = 0.0;

        // Calculate size of strict-sized and auto-sized rows.
        for (i, row) in self.rows.borrow_mut().iter_mut().enumerate() {
            if row.size_mode == SizeMode::Strict {
                row.actual_height = row.desired_height;
                preset_height += row.actual_height;
            } else if row.size_mode == SizeMode::Auto {
                row.actual_height = row.desired_height;
                for child_handle in self.widget.children.iter() {
                    let child = ui.nodes.borrow(*child_handle).widget();
                    if child.row() == i && child.visibility == Visibility::Visible && child.desired_size.get().y > row.actual_height {
                        row.actual_height = child.desired_size.get().y;
                    }
                }
                preset_height += row.actual_height;
            }
        }

        preset_height
    }

    fn fit_stretch_sized_columns(&self, ui: &UserInterface, available_size: Vec2, preset_width: f32) {
        let mut rest_width = 0.0;
        if available_size.x.is_infinite() {
            for child_handle in self.widget.children.iter() {
                let child = ui.nodes.borrow(*child_handle).widget();
                if let Some(column) = self.columns.borrow().get(child.column()) {
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
    }

    fn fit_stretch_sized_rows(&self, ui: &UserInterface, available_size: Vec2, preset_height: f32) {
        let mut stretch_sized_rows = 0;
        let mut rest_height = 0.0;
        if available_size.y.is_infinite() {
            for child_handle in self.widget.children.iter() {
                let child = ui.nodes.borrow(*child_handle).widget();
                if let Some(row) = self.rows.borrow().get(child.row()) {
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
    }

    fn arrange_rows(&self) {
        let mut y = 0.0;
        for row in self.rows.borrow_mut().iter_mut() {
            row.y = y;
            y += row.actual_height;
        }
    }

    fn arrange_columns(&self) {
        let mut x = 0.0;
        for column in self.columns.borrow_mut().iter_mut() {
            column.x = x;
            x += column.actual_width;
        }
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
