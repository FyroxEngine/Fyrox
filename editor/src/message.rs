// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::{
    command::{Command, CommandGroup, CommandTrait},
    fyrox::{
        core::{
            log::Log,
            pool::{ErasedHandle, Handle},
            uuid::Uuid,
        },
        gui::{inspector::PropertyChanged, UiNode},
        material::MaterialResource,
        scene::{
            camera::Projection,
            mesh::surface::SurfaceResource,
            node::Node,
            tilemap::{brush::TileMapBrushResource, tileset::TileSetResource},
            Scene,
        },
    },
    scene::Selection,
    SaveSceneConfirmationDialogAction,
};
use std::{path::PathBuf, sync::mpsc::channel, sync::mpsc::Sender};

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
    SaveAllScenes,
    AddScene {
        scene: Scene,
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
    OpenAnimationEditor,
    OpenAbsmEditor,
    OpenMaterialEditor(MaterialResource),
    OpenTileSetEditor(TileSetResource),
    OpenTileMapBrushEditor(TileMapBrushResource),
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
    SwitchToBuildMode {
        play_after_build: bool,
    },
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
    SetAssetBrowserCurrentDir(PathBuf),
}

#[derive(Clone, Debug)]
pub struct MessageSender(pub Sender<Message>);

impl Default for MessageSender {
    fn default() -> Self {
        let (rx, _) = channel();
        Self(rx)
    }
}

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

    pub fn send_command(&self, command: Command) {
        self.send(Message::DoCommand(command))
    }

    pub fn do_command_group(&self, group: Vec<Command>) {
        if group.len() == 1 {
            self.send(Message::DoCommand(group.into_iter().next().unwrap()))
        } else {
            self.do_command(CommandGroup::from(group));
        }
    }

    pub fn do_command_group_with_inheritance(&self, group: Vec<Command>, args: &PropertyChanged) {
        if group.is_empty() {
            if !args.is_inheritable() {
                Log::err(format!("Failed to handle a property {}", args.path()))
            }
        } else {
            self.do_command_group(group);
        }
    }
}
