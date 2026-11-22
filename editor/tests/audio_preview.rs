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

use fyrox::gui::message::MouseButton;
use fyrox::scene::graph::Graph;
use fyrox::scene::sound::Sound;
use fyroxed_base::asset::selector::{ASSET_SELECTOR_ITEM, AssetSelectorWindow};
use fyroxed_base::menu::file::FileMenu;
use fyroxed_base::plugins::audio::preview::AudioPreviewPanel;
use fyroxed_base::test::macros::Macro;
use fyroxed_base::test::utils;
use fyroxed_base::test::utils::{EditorTestingExtension, TestPlugin};
use fyroxed_base::utils::PICK_BUTTON;
use fyroxed_base::world::WorldViewer;
use fyroxed_base::world::create::EntityCreator;
use fyroxed_base::world::menu::SceneNodeContextMenu;

#[test]
fn test_audio_preview() {
    utils::run_editor_test_ex(
        "AudioPreview",
        |working_dir| {
            std::fs::create_dir_all(working_dir.join("data")).unwrap();
            std::fs::copy("resources/ak47.ogg", working_dir.join("data/ak47.ogg")).unwrap();
        },
        TestPlugin::new(
            Macro::begin()
                .lmb_click_at(FileMenu::FILE)
                .lmb_click_at(FileMenu::NEW_SCENE)
                .rmb_click_at_text(WorldViewer::WINDOW, Graph::SCENE_ROOT_NAME)
                .lmb_click_at(SceneNodeContextMenu::CREATE_CHILD)
                .lmb_click_at(EntityCreator::SEARCH_BAR)
                .type_text(Sound::DEFAULT_NAME)
                .lmb_click_at(EntityCreator::CREATE)
                .assert_has_scene_node::<Sound>(Sound::DEFAULT_NAME)
                .lmb_click_at_text(WorldViewer::WINDOW, Sound::DEFAULT_NAME)
                .scroll_inspector_to_property(Sound::BUFFER)
                .then_with_property(Sound::BUFFER, |editor, property_editor| {
                    editor.click_at_from(PICK_BUTTON, property_editor, MouseButton::Left);
                })
                .assert_visible(AssetSelectorWindow::WINDOW_ID)
                .lmb_click_at(ASSET_SELECTOR_ITEM)
                .lmb_click_at(AssetSelectorWindow::OK_ID)
                .lmb_click_at(AudioPreviewPanel::PREVIEW)
                .assert_in_preview_mode(),
        ),
    );
}
