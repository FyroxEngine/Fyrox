use crate::{
    fyrox::{
        core::pool::Handle,
        engine::ApplicationLoopController,
        graph::SceneGraph,
        gui::{
            button::{ButtonBuilder, ButtonMessage},
            dropdown_list::{DropdownListBuilder, DropdownListMessage},
            grid::{Column, GridBuilder, Row},
            menu::{MenuItem, MenuItemMessage},
            message::UiMessage,
            scroll_viewer::ScrollViewerBuilder,
            stack_panel::StackPanelBuilder,
            text::{Text, TextBuilder, TextMessage},
            text_box::TextBoxBuilder,
            tree::{Tree, TreeBuilder, TreeRoot, TreeRootBuilder, TreeRootMessage},
            widget::{WidgetBuilder, WidgetMessage},
            window::{Window, WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, HorizontalAlignment, Thickness, UiNode,
        },
    },
    menu::create_menu_item,
    plugin::EditorPlugin,
    Editor,
};
use fyrox::core::uuid::{uuid, Uuid};
use fyrox_biosphere::{
    container_format::{
        CapsuleHeraldry, ContainerHeraldry, GenesisContainer,
        serialize_qgenesis,
    },
    domain::Domain,
    heraldry::CrestName,
};
use std::path::PathBuf;

pub struct QuantumGenesisPlugin {
    window: Handle<Window>,
    open_menu_item: Handle<MenuItem>,

    genesis: Option<GenesisContainer>,
    loaded_path: Option<PathBuf>,

    tree_root: Handle<TreeRoot>,
    status_text: Handle<Text>,

    new_genesis_btn: Handle<UiNode>,
    open_btn: Handle<UiNode>,
    save_btn: Handle<UiNode>,
    add_child_btn: Handle<UiNode>,
    validate_btn: Handle<UiNode>,
    seal_btn: Handle<UiNode>,
    activate_btn: Handle<UiNode>,

    name_field: Handle<UiNode>,
    domain_dropdown: Handle<UiNode>,
    heraldry_dropdown: Handle<UiNode>,
    crest_dropdown: Handle<UiNode>,

    // Locally tracked state from UI widgets
    domain_selection: usize,
    heraldry_selection: usize,
    crest_selection: usize,

    selected_node: SelectedNode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SelectedNode {
    None,
    Genesis,
    Mythos(usize),
    Container(usize, usize),
    Capsule(usize, usize, usize),
}

impl Default for QuantumGenesisPlugin {
    fn default() -> Self {
        Self {
            window: Handle::NONE,
            open_menu_item: Handle::NONE,
            genesis: None,
            loaded_path: None,
            tree_root: Handle::NONE,
            status_text: Handle::NONE,
            new_genesis_btn: Handle::NONE,
            open_btn: Handle::NONE,
            save_btn: Handle::NONE,
            add_child_btn: Handle::NONE,
            validate_btn: Handle::NONE,
            seal_btn: Handle::NONE,
            activate_btn: Handle::NONE,
            name_field: Handle::NONE,
            domain_dropdown: Handle::NONE,
            heraldry_dropdown: Handle::NONE,
            crest_dropdown: Handle::NONE,
            domain_selection: 0,
            heraldry_selection: 0,
            crest_selection: 0,
            selected_node: SelectedNode::None,
        }
    }
}

impl QuantumGenesisPlugin {
    pub const OPEN_QUANTUM_GENESIS: Uuid = uuid!("a7e3c1b0-5f2d-4a8e-b9c1-3d4e5f6a7b8c");

    fn build_window(&mut self, ctx: &mut BuildContext) -> Handle<Window> {
        let name_field = TextBoxBuilder::new(
            WidgetBuilder::new()
                .with_height(22.0)
                .with_margin(Thickness::uniform(2.0)),
        )
        .with_text("New Entity")
        .build(ctx);
        self.name_field = name_field.to_base::<UiNode>();

        let domain_items = make_text_items(
            ctx,
            &["Narrative", "Music", "Software", "Agent", "Visual"],
        );
        let domain_dd = DropdownListBuilder::new(
            WidgetBuilder::new()
                .with_height(22.0)
                .with_margin(Thickness::uniform(2.0)),
        )
        .with_items(domain_items)
        .with_selected(0)
        .build(ctx);
        self.domain_dropdown = domain_dd.to_base::<UiNode>();

        let heraldry_items = make_text_items(
            ctx,
            &[
                "Glyph", "Device", "Emblem", "Trait", "Mark", "Token", "Sigil",
            ],
        );
        let heraldry_dd = DropdownListBuilder::new(
            WidgetBuilder::new()
                .with_height(22.0)
                .with_margin(Thickness::uniform(2.0)),
        )
        .with_items(heraldry_items)
        .with_selected(0)
        .build(ctx);
        self.heraldry_dropdown = heraldry_dd.to_base::<UiNode>();

        let crest_items = make_text_items(
            ctx,
            &[
                "Core", "Atlas", "Vault", "Mythos", "Codex", "Loom", "Composer", "Forge",
                "Order", "Mind", "Soul",
            ],
        );
        let crest_dd = DropdownListBuilder::new(
            WidgetBuilder::new()
                .with_height(22.0)
                .with_margin(Thickness::uniform(2.0)),
        )
        .with_items(crest_items)
        .with_selected(0)
        .build(ctx);
        self.crest_dropdown = crest_dd.to_base::<UiNode>();

        self.new_genesis_btn = make_button(ctx, "New Genesis");
        self.open_btn = make_button(ctx, "Open .qgenesis");
        self.save_btn = make_button(ctx, "Save");
        self.add_child_btn = make_button(ctx, "Add Child");
        self.validate_btn = make_button(ctx, "Validate");
        self.seal_btn = make_button(ctx, "Seal");
        self.activate_btn = make_button(ctx, "Activate");

        let toolbar = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .with_child(self.new_genesis_btn)
                .with_child(self.open_btn)
                .with_child(self.save_btn)
                .with_child(self.validate_btn)
                .with_child(self.activate_btn)
                .with_child(self.seal_btn),
        )
        .build(ctx);

        let form_label = TextBuilder::new(
            WidgetBuilder::new()
                .with_height(18.0)
                .with_margin(Thickness::uniform(2.0)),
        )
        .with_text("--- Add Entity ---")
        .with_horizontal_text_alignment(HorizontalAlignment::Center)
        .build(ctx);

        let name_label = TextBuilder::new(
            WidgetBuilder::new()
                .with_height(18.0)
                .with_margin(Thickness::uniform(2.0)),
        )
        .with_text("Name:")
        .build(ctx);

        let domain_label = TextBuilder::new(
            WidgetBuilder::new()
                .with_height(18.0)
                .with_margin(Thickness::uniform(2.0)),
        )
        .with_text("Domain:")
        .build(ctx);

        let heraldry_label = TextBuilder::new(
            WidgetBuilder::new()
                .with_height(18.0)
                .with_margin(Thickness::uniform(2.0)),
        )
        .with_text("Heraldry:")
        .build(ctx);

        let crest_label = TextBuilder::new(
            WidgetBuilder::new()
                .with_height(18.0)
                .with_margin(Thickness::uniform(2.0)),
        )
        .with_text("Crest:")
        .build(ctx);

        let form = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .with_child(form_label)
                .with_child(name_label)
                .with_child(self.name_field)
                .with_child(domain_label)
                .with_child(self.domain_dropdown)
                .with_child(crest_label)
                .with_child(self.crest_dropdown)
                .with_child(heraldry_label)
                .with_child(self.heraldry_dropdown)
                .with_child(self.add_child_btn),
        )
        .build(ctx);

        let left_panel = StackPanelBuilder::new(
            WidgetBuilder::new()
                .on_column(0)
                .with_child(toolbar)
                .with_child(form),
        )
        .build(ctx);

        self.tree_root = TreeRootBuilder::new(WidgetBuilder::new()).build(ctx);

        let tree_scroll = ScrollViewerBuilder::new(
            WidgetBuilder::new()
                .on_column(1)
                .with_margin(Thickness::uniform(2.0)),
        )
        .with_content(self.tree_root)
        .build(ctx);

        self.status_text = TextBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .on_column(0)
                .with_margin(Thickness::uniform(4.0)),
        )
        .with_text("No Genesis Container loaded")
        .build(ctx);

        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(left_panel)
                .with_child(tree_scroll)
                .with_child(self.status_text),
        )
        .add_column(Column::strict(200.0))
        .add_column(Column::stretch())
        .add_row(Row::stretch())
        .add_row(Row::strict(24.0))
        .build(ctx);

        WindowBuilder::new(WidgetBuilder::new().with_width(700.0).with_height(500.0))
            .with_title(WindowTitle::text("Quantum Genesis"))
            .with_content(content)
            .open(false)
            .build(ctx)
    }

    fn refresh_tree(&self, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();

        let items = if let Some(genesis) = &self.genesis {
            let ctx = &mut ui.build_ctx();
            vec![build_genesis_tree(genesis, ctx)]
        } else {
            vec![]
        };

        ui.send(self.tree_root, TreeRootMessage::Items(items));
    }

    fn set_status(&self, editor: &mut Editor, msg: &str) {
        let ui = editor.engine.user_interfaces.first();
        ui.send(self.status_text, TextMessage::Text(msg.to_string()));
    }

    fn selected_domain(&self) -> Domain {
        match self.domain_selection {
            0 => Domain::Narrative,
            1 => Domain::Music,
            2 => Domain::Software,
            3 => Domain::Agent,
            4 => Domain::Visual,
            _ => Domain::Narrative,
        }
    }

    fn selected_crest(&self) -> CrestName {
        match self.crest_selection {
            0 => CrestName::Core,
            1 => CrestName::Atlas,
            2 => CrestName::Vault,
            3 => CrestName::Mythos,
            4 => CrestName::Codex,
            5 => CrestName::Loom,
            6 => CrestName::Composer,
            7 => CrestName::Forge,
            8 => CrestName::Order,
            9 => CrestName::Mind,
            10 => CrestName::Soul,
            _ => CrestName::Core,
        }
    }

    fn selected_container_heraldry(&self) -> ContainerHeraldry {
        match self.heraldry_selection {
            0 => ContainerHeraldry::Glyph,
            1 => ContainerHeraldry::Device,
            2 => ContainerHeraldry::Emblem,
            _ => ContainerHeraldry::Glyph,
        }
    }

    fn selected_capsule_heraldry(&self) -> CapsuleHeraldry {
        match self.heraldry_selection {
            3 => CapsuleHeraldry::Trait,
            4 => CapsuleHeraldry::Mark,
            5 => CapsuleHeraldry::Token,
            6 => CapsuleHeraldry::Sigil,
            _ => CapsuleHeraldry::Sigil,
        }
    }

    fn read_name_field(&self, editor: &Editor) -> String {
        let ui = editor.engine.user_interfaces.first();
        ui.node(self.name_field)
            .cast::<fyrox::gui::text_box::TextBox>()
            .map(|tb| tb.text())
            .unwrap_or_default()
    }

    fn handle_new_genesis(&mut self, editor: &mut Editor) {
        let name = self.read_name_field(editor);
        let domain = self.selected_domain();
        let genesis = GenesisContainer::new(name.clone(), domain.clone(), None);
        self.genesis = Some(genesis);
        self.loaded_path = None;
        self.selected_node = SelectedNode::Genesis;
        self.set_status(editor, &format!("Created new Genesis: {name} ({domain})"));
        self.refresh_tree(editor);
    }

    fn handle_add_child(&mut self, editor: &mut Editor) {
        let name = self.read_name_field(editor);
        if name.is_empty() {
            self.set_status(editor, "Name cannot be empty");
            return;
        }

        let selected = self.selected_node.clone();
        let crest = self.selected_crest();
        let container_heraldry = self.selected_container_heraldry();
        let capsule_heraldry = self.selected_capsule_heraldry();

        let genesis = match &mut self.genesis {
            Some(g) => g,
            None => {
                self.set_status(editor, "No Genesis Container — create one first");
                return;
            }
        };

        let status_msg = match &selected {
            SelectedNode::None => {
                "Select a node in the tree first".to_string()
            }
            SelectedNode::Genesis => {
                match genesis.add_mythos(name.clone(), crest.clone(), None) {
                    Ok(_) => format!(
                        "Added Mythos: {name} [Crest: {crest}] ({}/16)",
                        genesis.mythos.len()
                    ),
                    Err(e) => format!("Error: {e}"),
                }
            }
            SelectedNode::Mythos(mi) => {
                let mi = *mi;
                if mi >= genesis.mythos.len() {
                    "Invalid Mythos selection".to_string()
                } else {
                    match genesis.mythos[mi].add_container(name.clone(), container_heraldry, None) {
                        Ok(_) => format!(
                            "Added Container: {name} ({}/16)",
                            genesis.mythos[mi].containers.len()
                        ),
                        Err(e) => format!("Error: {e}"),
                    }
                }
            }
            SelectedNode::Container(mi, ci) => {
                let (mi, ci) = (*mi, *ci);
                if mi >= genesis.mythos.len() || ci >= genesis.mythos[mi].containers.len() {
                    "Invalid Container selection".to_string()
                } else {
                    match genesis.mythos[mi].containers[ci].add_capsule(
                        name.clone(),
                        capsule_heraldry,
                        fyrox_biosphere::wire::WireType::Data,
                        serde_json::json!({}),
                        None,
                    ) {
                        Ok(_) => format!(
                            "Added Capsule: {name} ({}/16)",
                            genesis.mythos[mi].containers[ci].capsules.len()
                        ),
                        Err(e) => format!("Error: {e}"),
                    }
                }
            }
            SelectedNode::Capsule(..) => {
                "Capsules are atomic — cannot add children".to_string()
            }
        };

        self.set_status(editor, &status_msg);
        self.refresh_tree(editor);
    }

    fn handle_validate(&mut self, editor: &mut Editor) {
        let genesis = match &self.genesis {
            Some(g) => g,
            None => {
                self.set_status(editor, "No Genesis Container to validate");
                return;
            }
        };

        let errors = genesis.validate();
        if errors.is_empty() {
            let capsule_count = genesis.total_capsule_count();
            self.set_status(
                editor,
                &format!(
                    "Valid! {} Mythos, {} total Capsules, lifecycle: {}",
                    genesis.mythos.len(),
                    capsule_count,
                    genesis.lifecycle
                ),
            );
        } else {
            let msg: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
            self.set_status(editor, &format!("Errors: {}", msg.join("; ")));
        }
    }

    fn handle_seal(&mut self, editor: &mut Editor) {
        let genesis = match &mut self.genesis {
            Some(g) => g,
            None => {
                self.set_status(editor, "No Genesis Container to seal");
                return;
            }
        };
        match genesis.seal() {
            Ok(()) => {
                self.set_status(editor, "Genesis Container sealed — hierarchy is now frozen");
                self.refresh_tree(editor);
            }
            Err(e) => {
                self.set_status(editor, &format!("Cannot seal: {e}"));
            }
        }
    }

    fn handle_activate(&mut self, editor: &mut Editor) {
        let genesis = match &mut self.genesis {
            Some(g) => g,
            None => {
                self.set_status(editor, "No Genesis Container to activate");
                return;
            }
        };
        match genesis.activate() {
            Ok(()) => {
                self.set_status(editor, "Genesis Container activated");
                self.refresh_tree(editor);
            }
            Err(e) => {
                self.set_status(editor, &format!("Cannot activate: {e}"));
            }
        }
    }

    fn handle_save(&mut self, editor: &mut Editor) {
        let genesis = match &self.genesis {
            Some(g) => g,
            None => {
                self.set_status(editor, "No Genesis Container to save");
                return;
            }
        };

        let path = self.loaded_path.clone().unwrap_or_else(|| {
            PathBuf::from(format!("{}.qgenesis", genesis.name.replace(' ', "_")))
        });

        match serialize_qgenesis(genesis) {
            Ok(data) => match std::fs::write(&path, &data) {
                Ok(()) => {
                    self.loaded_path = Some(path.clone());
                    self.set_status(editor, &format!("Saved to {}", path.display()));
                }
                Err(e) => {
                    self.set_status(editor, &format!("Write error: {e}"));
                }
            },
            Err(e) => {
                self.set_status(editor, &format!("Serialization error: {e}"));
            }
        }
    }

    fn resolve_tree_selection(
        &self,
        selected_handle: Handle<Tree>,
        editor: &Editor,
    ) -> SelectedNode {
        let ui = editor.engine.user_interfaces.first();
        let tree_root_base = self.tree_root.to_base::<UiNode>();

        let mut depth = 0;
        let mut indices = Vec::new();
        let mut current = selected_handle.to_base::<UiNode>();

        loop {
            let parent = ui.node(current).parent();
            if parent == tree_root_base || parent.is_none() {
                break;
            }

            let parent_node = ui.node(parent);
            let siblings: Vec<Handle<UiNode>> = parent_node
                .children()
                .iter()
                .copied()
                .filter(|&h| ui.node(h).cast::<Tree>().is_some())
                .collect();

            let idx = siblings.iter().position(|&h| h == current).unwrap_or(0);
            indices.push(idx);
            current = parent;
            depth += 1;
        }

        indices.reverse();

        match depth {
            0 => SelectedNode::Genesis,
            1 => SelectedNode::Mythos(indices.first().copied().unwrap_or(0)),
            2 => SelectedNode::Container(
                indices.first().copied().unwrap_or(0),
                indices.get(1).copied().unwrap_or(0),
            ),
            3 => SelectedNode::Capsule(
                indices.first().copied().unwrap_or(0),
                indices.get(1).copied().unwrap_or(0),
                indices.get(2).copied().unwrap_or(0),
            ),
            _ => SelectedNode::None,
        }
    }
}

impl EditorPlugin for QuantumGenesisPlugin {
    fn on_start(&mut self, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();
        let ctx = &mut ui.build_ctx();
        self.open_menu_item = create_menu_item(
            "Quantum Genesis",
            Self::OPEN_QUANTUM_GENESIS,
            vec![],
            ctx,
        );
        ui.send(
            editor.menu.utils_menu.menu,
            MenuItemMessage::AddItem(self.open_menu_item),
        );
    }

    fn on_ui_message(&mut self, message: &mut UiMessage, editor: &mut Editor) {
        // Menu click to open window
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.open_menu_item && self.window.is_none() {
                let ui = editor.engine.user_interfaces.first_mut();
                let ctx = &mut ui.build_ctx();
                self.window = self.build_window(ctx);

                ui.send(
                    self.window,
                    WindowMessage::Open {
                        alignment: WindowAlignment::Center,
                        modal: false,
                        focus_content: true,
                    },
                );
            }
        }

        // Window close
        if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.window {
                let ui = editor.engine.user_interfaces.first_mut();
                ui.send(self.window, WidgetMessage::Remove);
                self.window = Handle::NONE;
            }
        }

        // Track dropdown selections
        if let Some(DropdownListMessage::Selection(Some(idx))) = message.data() {
            let dest = message.destination();
            if dest == self.domain_dropdown {
                self.domain_selection = *idx;
            } else if dest == self.heraldry_dropdown {
                self.heraldry_selection = *idx;
            } else if dest == self.crest_dropdown {
                self.crest_selection = *idx;
            }
        }

        // Button clicks
        if let Some(ButtonMessage::Click) = message.data() {
            let dest = message.destination();
            if dest == self.new_genesis_btn {
                self.handle_new_genesis(editor);
            } else if dest == self.add_child_btn {
                self.handle_add_child(editor);
            } else if dest == self.validate_btn {
                self.handle_validate(editor);
            } else if dest == self.seal_btn {
                self.handle_seal(editor);
            } else if dest == self.activate_btn {
                self.handle_activate(editor);
            } else if dest == self.save_btn {
                self.handle_save(editor);
            }
        }

        // Tree selection
        if let Some(TreeRootMessage::Select(selected)) = message.data_from(self.tree_root) {
            self.selected_node = if selected.is_empty() {
                SelectedNode::None
            } else {
                self.resolve_tree_selection(selected[0], editor)
            };

            let level_str = match &self.selected_node {
                SelectedNode::None => "None".to_string(),
                SelectedNode::Genesis => "Genesis (Level 1 — Seal)".to_string(),
                SelectedNode::Mythos(i) => {
                    if let Some(g) = &self.genesis {
                        if let Some(m) = g.mythos.get(*i) {
                            format!("Mythos: {} [Crest: {}]", m.name, m.crest)
                        } else {
                            "Mythos (invalid)".to_string()
                        }
                    } else {
                        "Mythos".to_string()
                    }
                }
                SelectedNode::Container(mi, ci) => {
                    if let Some(g) = &self.genesis {
                        if let Some(c) =
                            g.mythos.get(*mi).and_then(|m| m.containers.get(*ci))
                        {
                            format!("Container: {}", c.name)
                        } else {
                            "Container (invalid)".to_string()
                        }
                    } else {
                        "Container".to_string()
                    }
                }
                SelectedNode::Capsule(mi, ci, cai) => {
                    if let Some(g) = &self.genesis {
                        if let Some(cap) = g
                            .mythos
                            .get(*mi)
                            .and_then(|m| m.containers.get(*ci))
                            .and_then(|c| c.capsules.get(*cai))
                        {
                            format!("Capsule: {} [B-DNA gen:{}]", cap.name, cap.bdna.generation)
                        } else {
                            "Capsule (invalid)".to_string()
                        }
                    } else {
                        "Capsule".to_string()
                    }
                }
            };
            self.set_status(editor, &format!("Selected: {level_str}"));
        }
    }

    fn on_update(&mut self, _editor: &mut Editor, _loop_controller: ApplicationLoopController) {}
}

fn build_genesis_tree(genesis: &GenesisContainer, ctx: &mut BuildContext) -> Handle<Tree> {
    let seal_type = match genesis.seal_type {
        fyrox_biosphere::capacity::SealType::Lesser => "Lesser Seal",
        fyrox_biosphere::capacity::SealType::Greater => "Greater Seal",
    };
    let label = format!(
        "{} [{}] ({}) [{}]",
        genesis.name, seal_type, genesis.domain, genesis.lifecycle
    );

    let mythos_trees: Vec<Handle<Tree>> = genesis
        .mythos
        .iter()
        .map(|m| build_mythos_tree(m, ctx))
        .collect();

    let content = TextBuilder::new(
        WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
    )
    .with_text(label)
    .build(ctx);

    TreeBuilder::new(WidgetBuilder::new())
        .with_content(content)
        .with_items(mythos_trees)
        .build(ctx)
}

fn build_mythos_tree(
    mythos: &fyrox_biosphere::container_format::MythosContainer,
    ctx: &mut BuildContext,
) -> Handle<Tree> {
    let label = format!(
        "{} [Crest: {}] ({}) [{}/16]",
        mythos.name,
        mythos.crest,
        mythos.lifecycle,
        mythos.containers.len()
    );

    let container_trees: Vec<Handle<Tree>> = mythos
        .containers
        .iter()
        .map(|c| build_container_tree(c, ctx))
        .collect();

    let content = TextBuilder::new(
        WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
    )
    .with_text(label)
    .build(ctx);

    TreeBuilder::new(WidgetBuilder::new())
        .with_content(content)
        .with_items(container_trees)
        .build(ctx)
}

fn build_container_tree(
    container: &fyrox_biosphere::container_format::Container,
    ctx: &mut BuildContext,
) -> Handle<Tree> {
    let heraldry_str = match &container.heraldry {
        ContainerHeraldry::Glyph => "Glyph",
        ContainerHeraldry::Device => "Device",
        ContainerHeraldry::Emblem => "Emblem",
    };
    let label = format!(
        "{} [{}] ({}) [{}/16]",
        container.name,
        heraldry_str,
        container.lifecycle,
        container.capsules.len()
    );

    let capsule_trees: Vec<Handle<Tree>> = container
        .capsules
        .iter()
        .map(|c| build_capsule_tree(c, ctx))
        .collect();

    let content = TextBuilder::new(
        WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
    )
    .with_text(label)
    .build(ctx);

    TreeBuilder::new(WidgetBuilder::new())
        .with_content(content)
        .with_items(capsule_trees)
        .build(ctx)
}

fn build_capsule_tree(
    capsule: &fyrox_biosphere::container_format::Capsule,
    ctx: &mut BuildContext,
) -> Handle<Tree> {
    let heraldry_str = match &capsule.heraldic_current {
        CapsuleHeraldry::Trait => "Trait",
        CapsuleHeraldry::Mark => "Mark",
        CapsuleHeraldry::Token => "Token",
        CapsuleHeraldry::Sigil => "Sigil",
    };
    let label = format!(
        "{} [{}] (gen:{})",
        capsule.name, heraldry_str, capsule.bdna.generation
    );

    let content = TextBuilder::new(
        WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
    )
    .with_text(label)
    .build(ctx);

    TreeBuilder::new(WidgetBuilder::new())
        .with_content(content)
        .build(ctx)
}

fn make_button(ctx: &mut BuildContext, text: &str) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_height(26.0)
            .with_margin(Thickness::uniform(2.0)),
    )
    .with_text(text)
    .build(ctx)
    .to_base::<UiNode>()
}

fn make_text_items(ctx: &mut BuildContext, items: &[&str]) -> Vec<Handle<UiNode>> {
    items
        .iter()
        .map(|text| {
            TextBuilder::new(
                WidgetBuilder::new()
                    .with_height(22.0)
                    .with_margin(Thickness::uniform(2.0)),
            )
            .with_text(*text)
            .build(ctx)
            .to_base::<UiNode>()
        })
        .collect()
}
