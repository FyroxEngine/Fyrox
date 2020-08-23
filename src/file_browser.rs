//! File browser is a tree view over file system. It allows to select file or folder.

use crate::{
    button::ButtonBuilder,
    core::{
        math::{vec2::Vec2, Rect},
        pool::Handle,
    },
    draw::DrawingContext,
    grid::{Column, GridBuilder, Row},
    message::{
        ButtonMessage, FileBrowserMessage, FileSelectorMessage, OsEvent, TextBoxMessage,
        TreeMessage, TreeRootMessage, UiMessage, UiMessageData, WindowMessage,
    },
    node::UINode,
    scroll_viewer::ScrollViewerBuilder,
    stack_panel::StackPanelBuilder,
    text::TextBuilder,
    text_box::TextBoxBuilder,
    tree::{TreeBuilder, TreeRootBuilder},
    widget::{Widget, WidgetBuilder},
    window::{Window, WindowBuilder, WindowTitle},
    BuildContext, Control, HorizontalAlignment, NodeHandleMapping, Orientation, Thickness,
    UserInterface,
};
use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    rc::Rc,
};

pub type Filter = dyn FnMut(&Path) -> bool;

pub struct FileBrowser<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    tree_root: Handle<UINode<M, C>>,
    path_text: Handle<UINode<M, C>>,
    path: PathBuf,
    filter: Option<Rc<RefCell<Filter>>>,
}

impl<M: 'static, C: 'static + Control<M, C>> Deref for FileBrowser<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for FileBrowser<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> Clone for FileBrowser<M, C> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.raw_copy(),
            tree_root: self.tree_root,
            path_text: self.path_text,
            path: self.path.clone(),
            filter: self.filter.clone(),
        }
    }
}

impl<M: 'static, C: 'static + Control<M, C>> Control<M, C> for FileBrowser<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::FileBrowser(self.clone())
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        self.tree_root = *node_map.get(&self.tree_root).unwrap();
        self.path_text = *node_map.get(&self.path_text).unwrap();
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        match &message.data {
            UiMessageData::FileBrowser(msg) => {
                if message.destination == self.handle() {
                    match msg {
                        FileBrowserMessage::Root(path) => {
                            // Rebuild tree.
                            ui.send_message(UiMessage {
                                handled: false,
                                data: UiMessageData::TreeRoot(TreeRootMessage::Items(vec![])),
                                destination: self.tree_root,
                            });
                            build_tree(self.tree_root, true, path, Path::new(""), ui);
                        }
                        FileBrowserMessage::Path(path) => {
                            if &self.path != path {
                                // TODO: Maybe its better to make new tree if new path exists in
                                //  file system?
                                let tree = find_tree(self.tree_root, path, ui);
                                if tree.is_some() {
                                    self.path = path.clone();
                                    ui.send_message(TextBoxMessage::text(
                                        self.path_text,
                                        path.to_string_lossy().to_string(),
                                    ));
                                    ui.send_message(TreeRootMessage::select(
                                        self.tree_root,
                                        vec![tree],
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            UiMessageData::TextBox(msg) => {
                if message.destination == self.path_text {
                    if let TextBoxMessage::Text(txt) = msg {
                        self.path = txt.into();
                        // Try to find tree corresponding to path.
                        let tree = find_tree(self.tree_root, txt, ui);
                        if tree.is_some() {
                            ui.send_message(TreeRootMessage::select(self.tree_root, vec![tree]));
                        }
                    }
                }
            }
            UiMessageData::Tree(msg) => {
                if let TreeMessage::Expand(expand) = *msg {
                    if expand {
                        // Look into internals of directory and build tree items.
                        if let UINode::Tree(tree) = ui.node(message.destination) {
                            let parent_path = tree.user_data_ref::<PathBuf>().clone();
                            if let Ok(dir_iter) = std::fs::read_dir(&parent_path) {
                                for p in dir_iter {
                                    if let Ok(entry) = p {
                                        let path = entry.path();
                                        let build = if let Some(filter) = self.filter.as_ref() {
                                            filter.deref().borrow_mut().deref_mut()(&path)
                                        } else {
                                            true
                                        };
                                        if build {
                                            build_tree(
                                                message.destination,
                                                false,
                                                &path,
                                                &parent_path,
                                                ui,
                                            );
                                        }
                                    }
                                }
                            }
                        } else {
                            panic!("must be tree");
                        }
                    } else {
                        // Nuke everything in collapsed item. This also will free some resources
                        // and will speed up layout pass.
                        ui.send_message(TreeMessage::set_items(message.destination, vec![]));
                    }
                }
            }
            UiMessageData::TreeRoot(msg) => {
                if message.destination == self.tree_root {
                    if let TreeRootMessage::Selected(selection) = msg {
                        if let Some(&first_selected) = selection.first() {
                            let path = ui.node(first_selected).user_data_ref::<PathBuf>();
                            if &self.path != path {
                                ui.send_message(FileBrowserMessage::path(
                                    self.handle,
                                    path.as_path().to_owned(),
                                ));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn remove_ref(&mut self, handle: Handle<UINode<M, C>>) {
        if self.tree_root == handle {
            self.tree_root = Handle::NONE;
        }
        if self.path_text == handle {
            self.path_text = Handle::NONE;
        }
    }
}

fn find_tree<M: 'static, C: 'static + Control<M, C>, P: AsRef<Path>>(
    node: Handle<UINode<M, C>>,
    path: &P,
    ui: &UserInterface<M, C>,
) -> Handle<UINode<M, C>> {
    let mut tree_handle = Handle::NONE;
    match ui.node(node) {
        UINode::Tree(tree) => {
            let tree_path = tree.user_data_ref::<PathBuf>();
            if tree_path
                .to_string_lossy()
                .starts_with(path.as_ref().to_string_lossy().deref())
            {
                tree_handle = node;
            }
            for &item in tree.items() {
                let tree = find_tree(item, path, ui);
                if tree.is_some() {
                    tree_handle = tree;
                    break;
                }
            }
        }
        UINode::TreeRoot(root) => {
            for &item in root.items() {
                let tree = find_tree(item, path, ui);
                if tree.is_some() {
                    tree_handle = tree;
                    break;
                }
            }
        }
        _ => unreachable!(),
    }
    tree_handle
}

fn build_tree_item<M: 'static, C: 'static + Control<M, C>>(
    path: &Path,
    parent_path: &Path,
    ctx: &mut BuildContext<M, C>,
) -> Handle<UINode<M, C>> {
    let is_dir_empty = path.read_dir().map_or(true, |mut f| f.next().is_none());
    TreeBuilder::new(WidgetBuilder::new().with_user_data(Rc::new(path.to_owned())))
        .with_expanded(false)
        .with_always_show_expander(!is_dir_empty)
        .with_content(
            TextBuilder::new(WidgetBuilder::new())
                .with_text(
                    path.to_string_lossy()
                        .replace(&parent_path.to_string_lossy().to_string(), ""),
                )
                .build(ctx),
        )
        .build(ctx)
}

fn build_tree<M: 'static, C: 'static + Control<M, C>>(
    parent: Handle<UINode<M, C>>,
    is_parent_root: bool,
    path: &Path,
    parent_path: &Path,
    ui: &mut UserInterface<M, C>,
) -> Handle<UINode<M, C>> {
    let tree = build_tree_item(path, parent_path, &mut ui.build_ctx());

    if is_parent_root {
        ui.send_message(TreeRootMessage::add_item(parent, tree));
    } else {
        ui.send_message(TreeMessage::add_item(parent, tree));
    }

    tree
}

pub struct FileBrowserBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    path: PathBuf,
    filter: Option<Rc<RefCell<Filter>>>,
}

impl<M: 'static, C: 'static + Control<M, C>> FileBrowserBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            path: Default::default(),
            filter: None,
        }
    }

    pub fn with_filter(mut self, filter: Rc<RefCell<Filter>>) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn with_opt_filter(mut self, filter: Option<Rc<RefCell<Filter>>>) -> Self {
        self.filter = filter;
        self
    }

    pub fn with_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.path = path.as_ref().to_owned();
        self
    }

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let path_text;
        let tree_root;
        let scroll_viewer = ScrollViewerBuilder::new(WidgetBuilder::new().on_row(1).on_column(0))
            .with_content({
                tree_root = TreeRootBuilder::new(WidgetBuilder::new())
                    .with_items(vec![build_tree_item(&self.path, Path::new(""), ctx)])
                    .build(ctx);
                tree_root
            })
            .build(ctx);

        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    path_text = TextBoxBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(0)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .with_text(self.path.to_string_lossy().as_ref())
                    .build(ctx);
                    path_text
                })
                .with_child(scroll_viewer),
        )
        .add_column(Column::auto())
        .add_row(Row::strict(30.0))
        .add_row(Row::stretch())
        .build(ctx);

        let browser = FileBrowser {
            widget: self.widget_builder.with_child(grid).build(),
            tree_root,
            path_text,
            path: self.path,
            filter: self.filter,
        };

        ctx.add_node(UINode::FileBrowser(browser))
    }
}

/// File selector is a modal window that allows you to select a file (or directory) and commit or
/// cancel selection.
pub struct FileSelector<M: 'static, C: 'static + Control<M, C>> {
    window: Window<M, C>,
    browser: Handle<UINode<M, C>>,
    ok: Handle<UINode<M, C>>,
    cancel: Handle<UINode<M, C>>,
}

impl<M: 'static, C: 'static + Control<M, C>> Deref for FileSelector<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for FileSelector<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.window
    }
}

impl<M: 'static, C: 'static + Control<M, C>> Clone for FileSelector<M, C> {
    fn clone(&self) -> Self {
        Self {
            window: self.window.clone(),
            browser: self.browser,
            ok: self.ok,
            cancel: self.cancel,
        }
    }
}

// File selector extends Window widget so it delegates most of calls
// to inner window.
impl<M: 'static, C: 'static + Control<M, C>> Control<M, C> for FileSelector<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::FileSelector(self.clone())
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        self.window.resolve(node_map);
        self.ok = *node_map.get(&self.ok).unwrap();
        self.cancel = *node_map.get(&self.cancel).unwrap();
    }

    fn measure_override(&self, ui: &UserInterface<M, C>, available_size: Vec2) -> Vec2 {
        self.window.arrange_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vec2) -> Vec2 {
        self.window.arrange_override(ui, final_size)
    }

    fn arrange(&self, ui: &UserInterface<M, C>, final_rect: &Rect<f32>) {
        self.window.arrange(ui, final_rect);
    }

    fn is_measure_valid(&self, ui: &UserInterface<M, C>) -> bool {
        self.window.is_measure_valid(ui)
    }

    fn is_arrange_valid(&self, ui: &UserInterface<M, C>) -> bool {
        self.window.is_arrange_valid(ui)
    }

    fn measure(&self, ui: &UserInterface<M, C>, available_size: Vec2) {
        self.window.measure(ui, available_size)
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        self.window.draw(drawing_context)
    }

    fn update(&mut self, dt: f32) {
        self.window.update(dt);
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.window.handle_routed_message(ui, message);

        match &message.data {
            UiMessageData::Button(msg) => {
                if let ButtonMessage::Click = msg {
                    if message.destination == self.ok {
                        let path = if let UINode::FileBrowser(browser) = ui.node(self.browser) {
                            &browser.path
                        } else {
                            unreachable!();
                        };
                        ui.send_message(FileSelectorMessage::commit(self.handle, path.clone()));
                    } else if message.destination == self.cancel {
                        ui.send_message(FileSelectorMessage::cancel(self.handle))
                    }
                }
            }
            UiMessageData::FileSelector(msg) => match msg {
                FileSelectorMessage::Commit(_) | FileSelectorMessage::Cancel => {
                    ui.send_message(WindowMessage::close(self.handle))
                }
                FileSelectorMessage::Path(path) => {
                    ui.send_message(FileBrowserMessage::root(self.browser, path.clone()))
                }
            },
            _ => {}
        }
    }

    fn preview_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.window.preview_message(ui, message);
    }

    fn handle_os_event(
        &mut self,
        self_handle: Handle<UINode<M, C>>,
        ui: &mut UserInterface<M, C>,
        event: &OsEvent,
    ) {
        self.window.handle_os_event(self_handle, ui, event);
    }

    fn remove_ref(&mut self, handle: Handle<UINode<M, C>>) {
        self.window.remove_ref(handle)
    }
}

pub struct FileSelectorBuilder<M: 'static, C: 'static + Control<M, C>> {
    window_builder: WindowBuilder<M, C>,
    filter: Option<Rc<RefCell<Filter>>>,
    path: PathBuf,
}

impl<M: 'static, C: 'static + Control<M, C>> FileSelectorBuilder<M, C> {
    pub fn new(window_builder: WindowBuilder<M, C>) -> Self {
        Self {
            window_builder,
            filter: None,
            path: Default::default(),
        }
    }

    pub fn with_filter(mut self, filter: Rc<RefCell<Filter>>) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn with_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.path = path.as_ref().to_owned();
        self
    }

    pub fn build(mut self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let browser;
        let ok;
        let cancel;

        if self.window_builder.title.is_none() {
            self.window_builder.title = Some(WindowTitle::text("Select File"));
        }

        let window = self
            .window_builder
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            browser = FileBrowserBuilder::new(
                                WidgetBuilder::new()
                                    .with_height(400.0)
                                    .on_column(0)
                                    .on_column(0),
                            )
                            .with_opt_filter(self.filter)
                            .with_path(self.path)
                            .build(ctx);
                            browser
                        })
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .with_horizontal_alignment(HorizontalAlignment::Right)
                                    .on_column(0)
                                    .on_row(1)
                                    .with_child({
                                        ok = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0))
                                                .with_width(100.0)
                                                .with_height(30.0),
                                        )
                                        .with_text("OK")
                                        .build(ctx);
                                        ok
                                    })
                                    .with_child({
                                        cancel = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0))
                                                .with_width(100.0)
                                                .with_height(30.0),
                                        )
                                        .with_text("Cancel")
                                        .build(ctx);
                                        cancel
                                    }),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        ),
                )
                .add_column(Column::auto())
                .add_row(Row::stretch())
                .add_row(Row::auto())
                .build(ctx),
            )
            .build_window(ctx);

        let file_selector = FileSelector {
            window,
            browser,
            ok,
            cancel,
        };

        ctx.add_node(UINode::FileSelector(file_selector))
    }
}
