use crate::{
    command::Command, interaction::InteractionModeKind, scene::commands::SceneCommand,
    scene::Selection, BuildProfile, SaveSceneConfirmationDialogAction,
};
use fyrox::{
    core::{
        log::Log,
        pool::{ErasedHandle, Handle},
    },
    gui::UiNode,
    material::MaterialResource,
    scene::{camera::Projection, node::Node, Scene},
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
        scene: Handle<Scene>,
        path: PathBuf,
    },
    LoadScene(PathBuf),
    CloseScene(Handle<Scene>),
    SetInteractionMode(InteractionModeKind),
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
        type_id: TypeId,
        handle: ErasedHandle,
    },
    SelectObject {
        type_id: TypeId,
        handle: ErasedHandle,
    },
    SetCurrentScene(Handle<Scene>),
    FocusObject(Handle<Node>),
    SetEditorCameraProjection(Projection),
    SwitchToBuildMode,
    SwitchToEditMode,
    SwitchMode,
    OpenLoadSceneDialog,
    OpenSaveSceneDialog,
    OpenSaveSceneConfirmationDialog {
        scene: Handle<Scene>,
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
