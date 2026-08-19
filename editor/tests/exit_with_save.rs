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

use fyroxed_base::menu::file::FileMenu;
use fyroxed_base::test::macros::Macro;
use fyroxed_base::test::utils;
use fyroxed_base::test::utils::TestPlugin;

#[test]
fn test_exit_with_save() {
    utils::run_editor_test(
        "Menu/File/ExitWithSave",
        TestPlugin::new(
            Macro::begin()
                .click_at(FileMenu::FILE)
                .click_at(FileMenu::NEW_SCENE)
                .then(|editor| assert_eq!(editor.scenes.len(), 1))
                .click_at(FileMenu::FILE)
                .click_at(FileMenu::SAVE_SCENE)
                .click_at_text(FileMenu::SAVE_FILE_SELECTOR, "Save")
                .then(|editor| assert_eq!(editor.scenes.len(), 1)),
        ),
    );
}
