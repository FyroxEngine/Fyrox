use crate::{
    command::Command, interaction::InteractionModeKind, scene::commands::SceneCommand,
    scene::Selection, BuildProfile, SaveSceneConfirmationDialogAction,
};
use fyrox::{
    core::pool::{ErasedHandle, Handle},
    gui::UiNode,
    material::SharedMaterial,
    scene::{camera::Projection, node::Node},
};
use std::{any::TypeId, path::PathBuf};

#[derive(Debug)]
pub enum Message {
    DoSceneCommand(SceneCommand),
    UndoSceneCommand,
    RedoSceneCommand,
    ClearSceneCommandStack,
    SelectionChanged {
        old_selection: Selection,
    },
    SaveScene(PathBuf),
    LoadScene(PathBuf),
    CloseScene,
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
    OpenMaterialEditor(SharedMaterial),
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
    SetEditorCameraProjection(Projection),
    SwitchToBuildMode,
    SwitchToEditMode,
    SwitchMode,
    OpenLoadSceneDialog,
    OpenSaveSceneDialog,
    OpenSaveSceneConfirmationDialog(SaveSceneConfirmationDialogAction),
    SetBuildProfile(BuildProfile),
    SaveSelectionAsPrefab(PathBuf),
    SyncNodeHandleName {
        view: Handle<UiNode>,
        handle: Handle<Node>,
    },
    ForceSync,
    ShowDocumentation(String),
}

impl Message {
    pub fn do_scene_command<C>(cmd: C) -> Self
    where
        C: Command,
    {
        Self::DoSceneCommand(SceneCommand::new(cmd))
    }
}
