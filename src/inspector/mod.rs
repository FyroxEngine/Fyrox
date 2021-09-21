use crate::inspector::editors::texture::TexturePropertyEditorDefinition;
use crate::{
    gui::{BuildContext, EditorUiMessage, EditorUiNode, UiNode},
    scene::{EditorScene, Selection},
    GameEngine,
};
use rg3d::engine::resource_manager::ResourceManager;
use rg3d::gui::inspector::InspectorEnvironment;
use rg3d::{
    core::pool::Handle,
    gui::{
        inspector::{
            editors::PropertyEditorDefinitionContainer, InspectorBuilder, InspectorContext,
        },
        message::{InspectorMessage, MessageDirection},
        scroll_viewer::ScrollViewerBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
    },
};
use std::any::Any;
use std::sync::Arc;

pub mod editors;

pub struct EditorEnvironment {
    resource_manager: ResourceManager,
}

impl InspectorEnvironment for EditorEnvironment {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct Inspector {
    pub window: Handle<UiNode>,
    inspector: Handle<UiNode>,
    property_editors: Arc<PropertyEditorDefinitionContainer<EditorUiMessage, EditorUiNode>>,
}

impl Inspector {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let mut container = PropertyEditorDefinitionContainer::new();
        container.insert(Arc::new(TexturePropertyEditorDefinition));
        let property_editors = Arc::new(container);

        let inspector;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_title(WindowTitle::text("Inspector"))
            .with_content(
                ScrollViewerBuilder::new(WidgetBuilder::new())
                    .with_content({
                        inspector = InspectorBuilder::new(WidgetBuilder::new())
                            .with_property_editor_definitions(property_editors.clone())
                            .build(ctx);
                        inspector
                    })
                    .build(ctx),
            )
            .build(ctx);

        Self {
            window,
            inspector,
            property_editors,
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let scene = &engine.scenes[editor_scene.scene];

        if let Selection::Graph(selection) = &editor_scene.selection {
            if selection.is_single_selection() {
                let node_handle = selection.nodes()[0];
                if scene.graph.is_valid_handle(node_handle) {
                    let node = &scene.graph[node_handle];

                    let environment = Arc::new(EditorEnvironment {
                        resource_manager: engine.resource_manager.clone(),
                    });

                    let context = InspectorContext::from_object(
                        node,
                        &mut engine.user_interface.build_ctx(),
                        &*self.property_editors,
                        Some(environment),
                    );

                    engine
                        .user_interface
                        .send_message(InspectorMessage::context(
                            self.inspector,
                            MessageDirection::ToWidget,
                            context,
                        ));
                }
            }
        }
    }
}
