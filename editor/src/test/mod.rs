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

#![allow(dead_code)]

//! Automated tests for the entire editor.
//! WARNING: This is experimental functionality and currently in development.

mod macros;
mod utils;

use crate::{
    menu::file::FileMenu,
    test::{macros::Macro, utils::TestPlugin},
    SaveSceneConfirmationDialog,
};

#[test]
fn test_open() {
    utils::run_editor_test(
        "Menu/File",
        TestPlugin::new(
            Macro::begin()
                .click_at(FileMenu::FILE)
                .click_at(FileMenu::NEW_SCENE)
                .then(|editor| assert_eq!(editor.scenes.len(), 1)),
        ),
    );
}

#[test]
fn test_close() {
    utils::run_editor_test(
        "Menu/Close",
        TestPlugin::new(
            Macro::begin()
                .click_at(FileMenu::FILE)
                .click_at(FileMenu::NEW_SCENE)
                .then(|editor| assert_eq!(editor.scenes.len(), 1))
                .click_at(FileMenu::FILE)
                .click_at(FileMenu::CLOSE_SCENE)
                .click_at_text(SaveSceneConfirmationDialog::DIALOG_ID, "No")
                .then(|editor| assert_eq!(editor.scenes.len(), 0)),
        ),
    );
}
