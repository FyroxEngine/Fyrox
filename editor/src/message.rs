use crate::{
    command::Command, scene::commands::SceneCommand, scene::Selection, BuildProfile,
    SaveSceneConfirmationDialogAction,
};
use fyrox::{
    core::{
        log::Log,
        pool::{ErasedHandle, Handle},
        uuid::Uuid,
    },
    gui::UiNode,
    material::MaterialResource,
    scene::{camera::Projection, node::Node},
};
use std::{any::TypeId, path::PathBuf, sync::mpsc::Sender};

#[derive(Debug)]
pub enum Message {
    DoSceneCommand(SceneCommand),
    UndoSceneCommand,
    RedoSceneCommand,
    ClearSceneCommandStack,
    SelectionChanged {
        old_selection: Selection,
    },
    SaveScene {
        id: Uuid,
        path: PathBuf,
    },
    LoadScene(PathBuf),
    CloseScene(Uuid),
    SetInteractionMode(Uuid),
    Configure {
        working_directory: PathBuf,
    },
    NewScene,
    Exit {
        force: bool,
    },
    OpenSettings,
    OpenAnimationEditor,
    OpenAbsmEditor,
    OpenMaterialEditor(MaterialResource),
    OpenNodeRemovalDialog,
    ShowInAssetBrowser(PathBuf),
    SetWorldViewerFilter(String),
    LocateObject {
        handle: ErasedHandle,
    },
    SelectObject {
        type_id: TypeId,
        handle: ErasedHandle,
    },
    SetCurrentScene(Uuid),
    FocusObject(Handle<Node>),
    SetEditorCameraProjection(Projection),
    SwitchToBuildMode,
    SwitchToEditMode,
    SwitchMode,
    OpenLoadSceneDialog,
    OpenSaveSceneDialog,
    OpenSaveSceneConfirmationDialog {
        id: Uuid,
        action: SaveSceneConfirmationDialogAction,
    },
    SetBuildProfile(BuildProfile),
    SaveSelectionAsPrefab(PathBuf),
    SyncNodeHandleName {
        view: Handle<UiNode>,
        handle: Handle<Node>,
    },
    ProvideSceneHierarchy {
        view: Handle<UiNode>,
    },
    ForceSync,
    ShowDocumentation(String),
    SaveLayout,
    LoadLayout,
}

#[derive(Clone, Debug)]
pub struct MessageSender(pub Sender<Message>);

impl MessageSender {
    pub fn do_scene_command<C>(&self, cmd: C)
    where
        C: Command,
    {
        self.send(Message::DoSceneCommand(SceneCommand::new(cmd)))
    }

    pub fn send(&self, message: Message) {
        Log::verify(self.0.send(message));
    }
}
