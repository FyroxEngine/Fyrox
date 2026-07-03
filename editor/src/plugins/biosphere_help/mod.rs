use crate::{
    fyrox::{
        core::pool::Handle,
        engine::ApplicationLoopController,
        gui::{menu::MenuItem, menu::MenuItemMessage, message::UiMessage},
    },
    menu::create_menu_item,
    plugin::EditorPlugin,
    utils::doc::DocWindow,
    Editor,
};
use fyrox::core::uuid::{uuid, Uuid};

pub struct BiosphereHelpPlugin {
    open_blueprint: Handle<MenuItem>,
    open_heraldry_ref: Handle<MenuItem>,
    open_wire_ref: Handle<MenuItem>,
    open_plugin_guide: Handle<MenuItem>,
    doc_window: Option<DocWindow>,
}

impl Default for BiosphereHelpPlugin {
    fn default() -> Self {
        Self {
            open_blueprint: Handle::NONE,
            open_heraldry_ref: Handle::NONE,
            open_wire_ref: Handle::NONE,
            open_plugin_guide: Handle::NONE,
            doc_window: None,
        }
    }
}

impl BiosphereHelpPlugin {
    pub const OPEN_BLUEPRINT: Uuid = uuid!("612bd42e-9347-42ab-af56-cbbdb6c06a17");
    pub const OPEN_HERALDRY: Uuid = uuid!("e3bb6939-4a14-493b-ad32-f79329d28644");
    pub const OPEN_WIRE_REF: Uuid = uuid!("29730a6e-23c7-48d2-8137-82356d71f9dc");
    pub const OPEN_PLUGIN_GUIDE: Uuid = uuid!("1434ceae-65fd-4bda-a898-3a1e01ef0dec");
}

impl EditorPlugin for BiosphereHelpPlugin {
    fn on_start(&mut self, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();
        let ctx = &mut ui.build_ctx();

        self.doc_window = Some(DocWindow::new(ctx));

        // Build BioSpark submenu items
        self.open_blueprint =
            create_menu_item("Project Blueprint", Self::OPEN_BLUEPRINT, vec![], ctx);
        self.open_heraldry_ref =
            create_menu_item("Heraldry Reference", Self::OPEN_HERALDRY, vec![], ctx);
        self.open_wire_ref =
            create_menu_item("Wire Types Reference", Self::OPEN_WIRE_REF, vec![], ctx);
        self.open_plugin_guide =
            create_menu_item("Plugin Dev Guide", Self::OPEN_PLUGIN_GUIDE, vec![], ctx);

        // Separator item label
        let biosphere_submenu = create_menu_item(
            "BioSpark Quantum Genesis",
            uuid!("7f27b28d-36e7-4c34-8008-c674e2875e5f"),
            vec![
                self.open_blueprint,
                self.open_heraldry_ref,
                self.open_wire_ref,
                self.open_plugin_guide,
            ],
            ctx,
        );

        ui.send(
            editor.menu.help_menu.menu,
            MenuItemMessage::AddItem(biosphere_submenu),
        );
    }

    fn on_ui_message(&mut self, message: &mut UiMessage, editor: &mut Editor) {
        if let Some(MenuItemMessage::Click) = message.data() {
            let dest = message.destination();
            let doc = if dest == self.open_blueprint {
                Some(BLUEPRINT_TEXT.to_string())
            } else if dest == self.open_heraldry_ref {
                Some(HERALDRY_REFERENCE.to_string())
            } else if dest == self.open_wire_ref {
                Some(WIRE_REFERENCE.to_string())
            } else if dest == self.open_plugin_guide {
                Some(PLUGIN_GUIDE.to_string())
            } else {
                None
            };

            if let Some(text) = doc {
                let ui = editor.engine.user_interfaces.first();
                if let Some(window) = &self.doc_window {
                    window.open(text, ui);
                }
            }
        }
    }

    fn on_update(&mut self, _editor: &mut Editor, _loop_controller: ApplicationLoopController) {}
}

// ── Embedded reference texts ────────────────────────────────────────────────

const BLUEPRINT_TEXT: &str = "\
BioSpark Quantum Genesis — Project Blueprint
============================================

LAYER 0 — Data Foundation (fyrox-biosphere)
  [DONE] Capacity Law (16-16-16) ......... fyrox-biosphere/capacity
  [DONE] Heraldry (20 types) ............. fyrox-biosphere/heraldry
  [DONE] Three-Way Alignment ............. fyrox-biosphere/alignment
  [DONE] B-DNA lineage & covenant ........ fyrox-biosphere/bdna
  [DONE] 16 Wire Types ................... fyrox-biosphere/wire
  [DONE] Domain mappings ................. fyrox-biosphere/domain
  [DONE] Container format (.qgenesis) .... fyrox-biosphere/container_format
  [    ] .qgcp format (data + assets)

LAYER 1 — Editor Integration
  [DONE] Quantum Genesis plugin .......... editor/plugins/quantum_genesis/
  [DONE] BioSpark Help section ........... editor/plugins/biosphere_help/
  [DONE] EditorPlugin template ........... biosphere-templates/
  [DONE] Biosphere crate template ........ biosphere-templates/
  [    ] Genesis Container wizard
  [    ] .qgcp import/export
  [    ] B-DNA lineage viewer
  [    ] Wire connection editor

LAYER 2 — Networking (fyrox-net)         [NOT STARTED]
LAYER 3 — Master Vault Server            [NOT STARTED]
LAYER 4 — Vault Client                   [NOT STARTED]
LAYER 5 — ACTOR System                   [NOT STARTED]

DEFERRED (design not complete):
  Sub-vaults / shard-verses, Order of the Quantum Quill,
  MIDI/musical interaction, Five Factions, 16 Quantum Modules,
  Heraldry-based world manipulation, ACTOR HUD, Human/AGENT interface
";

const HERALDRY_REFERENCE: &str = "\
Quantum Genesis — Heraldry Reference
=====================================

CONTAINER HIERARCHY
  Genesis  (Level 1) — Seal
    Mythos  (Level 2) — Crest
      Container  (Level 3) — Glyph | Device | Emblem
        Capsule  (Level 4) — Trait | Mark | Token | Sigil

CAPACITY LAW
  Each level holds at most 16 children.
  Overflow requires splitting into a sibling, never exceeding.
  Max atomic entities per Genesis: 16^3 = 4,096

SEAL (Genesis level)
  Greater Seal — primary Genesis Container
  Lesser Seal  — grouping of up to 16 sealed Genesis Containers

CRESTS (Mythos level) — 11 known, up to 5 custom
  Core     Atlas    Vault    Mythos   Codex
  Loom     Composer Forge    Order    Mind    Soul

CONTAINER HERALDRY (Level 3)
  Glyph   — composable capability unit
  Device  — standalone functional unit
  Emblem  — thematic grouping

CAPSULE HERALDRY (Level 4)
  Trait   — semi-permanent measurable attribute
  Mark    — variable tag or category
  Token   — temporary transactional element
  Sigil   — semi-permanent unique personal binding (ACTOR identity)

LIFECYCLE
  Seeding -> Active -> Sealed -> Archived -> Deprecated
  Sealed: hierarchy frozen, payload updates still allowed

THREE-WAY ALIGNMENT
  Every entity must align on three dimensions simultaneously:
    Structural — container level (Genesis/Mythos/Container/Capsule)
    Functional — ecosystem role (Engine/MajorSystem/Addon/Entity)
    Symbolic   — heraldic type (Seal/Crest/Glyph.../Trait...)
  Misalignment is rejected by validate_alignment().
";

const WIRE_REFERENCE: &str = "\
Quantum Genesis — 16 Wire Types
================================

DAT — Data         Universal fallback. Connects to any type.
CTL — Control      Boolean / gate / trigger signals.
AUD — Audio        Waveform and sample streams.
NAR — Narrative    Story / text / lore content.
TMP — Temporal     Time / tick / clock signals.
AGT — Agent        Agent instruction and state packets.
VIS — Visual       Image / render / shader data.
SPA — Spatial      3D / voxel / coordinate data.
BHV — Behavioral   Emotion / drive / decision signals.
SOC — Social       Relationship / faction / reputation.
ENR — Energy       Power and resource flow.
IDN — Identity     B-DNA / lineage / covenant payloads.
EVT — Event        Cosmic bus events.
AST — Asset        File / binary / media references.
MET — Meta         Schema / type / structure definitions.
LGC — Logic        Boolean expression / rule streams.

COMPATIBILITY RULES
  source.wire_type must match target.wire_type, OR
  source.wire_type is DAT (universal fallback).
  No other cross-type connections are permitted.

B-DNA flows through IDN wires as read-only payload.
MIDI/musical data flows through AUD + CTL combinations.
ACTOR identity and covenants flow through IDN wires.
";

const PLUGIN_GUIDE: &str = "\
BioSpark — EditorPlugin Development Guide
==========================================

1. CREATE THE MODULE
   editor/src/plugins/YOUR_NAME/mod.rs
   Copy from: biosphere-templates/editor-plugin-template/src/lib.rs

2. REGISTER THE MODULE
   editor/src/plugins/mod.rs:
     pub mod YOUR_NAME;

   editor/src/lib.rs — add to import block:
     plugins::YOUR_NAME::YourPlugin,

   editor/src/lib.rs — add to plugin chain (~line 1063):
     .with(YourPlugin::default())

3. PLUGIN STRUCT RULES
   - Store UI handles (Handle<Window>, Handle<UiNode>, etc.)
   - All handles default to Handle::NONE
   - Track dropdown/textbox state via messages (not node queries)
   - Build windows lazily in on_ui_message, not on_start

4. KEY API PATTERNS

   Add menu item to Utils:
     ui.send(editor.menu.utils_menu.menu,
             MenuItemMessage::AddItem(handle));

   Add menu item to Help:
     ui.send(editor.menu.help_menu.menu,
             MenuItemMessage::AddItem(handle));

   Open window:
     ui.send(window, WindowMessage::Open {
         alignment: WindowAlignment::Center,
         modal: false,
         focus_content: true,
     });

   Send text update:
     ui.send(text_handle, TextMessage::Text(string));

   Listen for tree selection:
     if let Some(TreeRootMessage::Select(sel)) = message.data_from(tree_root) { ... }

5. TYPED HANDLE CONVERSIONS
   ButtonBuilder returns Handle<Button> — convert with .to_base::<UiNode>()
   TextBuilder returns Handle<Text>
   TreeBuilder returns Handle<Tree>
   TreeRootBuilder returns Handle<TreeRoot>
   DropdownListBuilder returns Handle<DropdownList>

   with_child() accepts Handle<impl ObjectOrVariant<UiNode>> — no conversion needed.
   ui.node() requires Handle<UiNode> — use .to_base::<UiNode>() to convert.
   ui.node(h).cast::<Tree>() to downcast.

6. COMMON MISTAKES
   - TreeRootMessage::Select not Selected
   - TreeBuilder::with_items() not with_child() for tree children
   - TextBoxMessage has no Text variant — read text via node().cast::<TextBox>()
   - to_base() needs explicit type: .to_base::<UiNode>() not .to_base()
   - Don't build windows in on_start (no UI context yet that renders correctly)
";
