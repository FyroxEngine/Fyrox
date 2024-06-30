use crate::{
    command::{Command, CommandTrait},
    fyrox::{
        core::{
            log::Log,
            pool::{ErasedHandle, Handle},
            uuid::Uuid,
        },
        gui::UiNode,
        material::MaterialResource,
        scene::{camera::Projection, mesh::surface::SurfaceResource, node::Node},
    },
    scene::Selection,
    SaveSceneConfirmationDialogAction,
};
use fyrox::scene::tilemap::tileset::TileSetResource;
use std::{path::PathBuf, sync::mpsc::Sender};

#[derive(Debug)]
pub enum Message {
    DoCommand(Command),
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
    OpenTileSetEditor(TileSetResource),
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
    ViewSurfaceData(SurfaceResource),
    SyncInteractionModes,
}

#[derive(Clone, Debug)]
pub struct MessageSender(pub Sender<Message>);

unsafe impl Send for MessageSender {}
unsafe impl Sync for MessageSender {}

impl MessageSender {
    pub fn do_command<C>(&self, cmd: C)
    where
        C: CommandTrait,
    {
        self.send(Message::DoCommand(Command::new(cmd)))
    }

    pub fn send(&self, message: Message) {
        Log::verify(self.0.send(message));
    }
}
