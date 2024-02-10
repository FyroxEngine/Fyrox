use crate::command::{Command, CommandTrait};
use crate::{
    command::GameSceneCommandTrait,
    scene::{commands::GameSceneCommand, Selection},
    BuildProfile, SaveSceneConfirmationDialogAction,
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
use std::{path::PathBuf, sync::mpsc::Sender};

#[derive(Debug)]
pub enum Message {
    DoGameSceneCommand(GameSceneCommand),
    DoUiSceneCommand(Command),
    UndoCurrentSceneCommand,
    RedoCurrentSceneCommand,
    ClearCurrentSceneCommandStack,
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
    NewUiScene,
    Exit {
        force: bool,
    },
    OpenSettings,
    OpenAnimationEditor,
    OpenAbsmEditor,
    OpenMaterialEditor(MaterialResource),
    OpenNodeRemovalDialog,
    ShowInAssetBrowser(PathBuf),
    LocateObject {
        handle: ErasedHandle,
    },
    SelectObject {
        handle: ErasedHandle,
    },
    SetCurrentScene(Uuid),
    FocusObject(Handle<Node>),
    SetEditorCameraProjection(Projection),
    SwitchToBuildMode,
    SwitchToEditMode,
    SwitchMode,
    OpenLoadSceneDialog,
    OpenSaveSceneDialog {
        default_file_name: PathBuf,
    },
    OpenSaveSceneConfirmationDialog {
        id: Uuid,
        action: SaveSceneConfirmationDialogAction,
    },
    SetBuildProfile(BuildProfile),
    SaveSelectionAsPrefab(PathBuf),
    SyncNodeHandleName {
        view: Handle<UiNode>,
        handle: ErasedHandle,
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

unsafe impl Send for MessageSender {}
unsafe impl Sync for MessageSender {}

impl MessageSender {
    pub fn do_scene_command<C>(&self, cmd: C)
    where
        C: GameSceneCommandTrait,
    {
        self.send(Message::DoGameSceneCommand(GameSceneCommand::new(cmd)))
    }

    pub fn do_ui_scene_command<C>(&self, cmd: C)
    where
        C: CommandTrait,
    {
        self.send(Message::DoUiSceneCommand(Command::new(cmd)))
    }

    pub fn send(&self, message: Message) {
        Log::verify(self.0.send(message));
    }
}
