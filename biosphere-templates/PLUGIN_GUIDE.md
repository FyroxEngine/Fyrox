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
