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

use crate::message::MessageData;
use crate::{
    button::{Button, ButtonBuilder, ButtonMessage},
    control_trait_proxy_impls,
    core::{log::Log, pool::Handle, reflect::prelude::*, visitor::prelude::*},
    grid::{Column, GridBuilder, Row},
    inspector::{
        editors::{
            hashmap::HashMapKey, PropertyEditorBuildContext, PropertyEditorDefinitionContainer,
            PropertyEditorTranslationContext,
        },
        InspectorEnvironmentContainer, PropertyAction,
    },
    message::UiMessage,
    stack_panel::StackPanelBuilder,
    widget::{Widget, WidgetBuilder},
    window::{Window, WindowBuilder, WindowMessage},
    BuildContext, Control, Orientation, Thickness, UiNode, UserInterface,
};
use fxhash::FxHashSet;
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
    sync::Arc,
};

#[derive(Debug, PartialEq, Clone)]
pub enum SelectHashMapKeyDialogWindowMessage<K: HashMapKey> {
    Key(K),
}
impl<K: HashMapKey> MessageData for SelectHashMapKeyDialogWindowMessage<K> {}

#[derive(Default, Clone, Debug, Visit, PartialEq, Reflect)]
#[reflect(
    derived_type = "UiNode",
    type_uuid = "d917626c-d845-499d-99db-df6dd10d86bc"
)]
pub struct SelectHashMapKeyDialogWindow<K: HashMapKey> {
    #[visit(skip)]
    #[reflect(hidden)]
    existing_keys: FxHashSet<u64>,
    window: Window,
    #[visit(skip)]
    #[reflect(hidden)]
    property_editors: Arc<PropertyEditorDefinitionContainer>,
    key_editor: Handle<UiNode>,
    #[visit(skip)]
    #[reflect(hidden)]
    environment: Option<InspectorEnvironmentContainer>,
    #[visit(skip)]
    #[reflect(hidden)]
    key: K,
    ok: Handle<Button>,
    cancel: Handle<Button>,
}

impl<K: HashMapKey> Deref for SelectHashMapKeyDialogWindow<K> {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.window.widget
    }
}

impl<K: HashMapKey> DerefMut for SelectHashMapKeyDialogWindow<K> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.window.widget
    }
}

impl<K: HashMapKey> Control for SelectHashMapKeyDialogWindow<K> {
    control_trait_proxy_impls!(window);

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.window.handle_routed_message(ui, message);

        if message.destination() == self.key_editor {
            if let Some(definition) = self.property_editors.definitions().get(&TypeId::of::<K>()) {
                if let Some(change) =
                    definition
                        .property_editor
                        .translate_message(PropertyEditorTranslationContext {
                            environment: self.environment.clone(),
                            name: "",
                            message,
                            definition_container: self.property_editors.clone(),
                        })
                {
                    PropertyAction::from_field_action(&change.action).apply_on_result(
                        Ok(&mut self.key),
                        &mut |result| {
                            Log::verify(result);
                        },
                    );
                }
            }
        }

        if let Some(ButtonMessage::Click) = message.data_from(self.ok) {
            ui.post(
                self.handle(),
                SelectHashMapKeyDialogWindowMessage::Key(self.key.clone()),
            );
            ui.send(self.handle(), WindowMessage::Close);
        } else if let Some(ButtonMessage::Click) = message.data_from(self.cancel) {
            ui.send(self.handle(), WindowMessage::Close);
        }
    }
}

pub struct SelectHashMapKeyDialogWindowBuilder<K: HashMapKey> {
    window_builder: WindowBuilder,
    existing_keys: FxHashSet<u64>,
    property_editors: Arc<PropertyEditorDefinitionContainer>,
    environment: Option<InspectorEnvironmentContainer>,
    initial_value: K,
}

impl<K: HashMapKey> SelectHashMapKeyDialogWindowBuilder<K> {
    pub fn new(
        window_builder: WindowBuilder,
        property_editors: Arc<PropertyEditorDefinitionContainer>,
        initial_value: K,
    ) -> Self {
        Self {
            window_builder,
            existing_keys: Default::default(),
            property_editors,
            environment: None,
            initial_value,
        }
    }

    pub fn with_existing_keys(mut self, existing_keys: FxHashSet<u64>) -> Self {
        self.existing_keys = existing_keys;
        self
    }

    pub fn with_environment(mut self, environment: Option<InspectorEnvironmentContainer>) -> Self {
        self.environment = environment;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<SelectHashMapKeyDialogWindow<K>> {
        let key_info = FieldRef {
            metadata: &FieldMetadata {
                name: "",
                display_name: "",
                read_only: false,
                immutable_collection: false,
                min_value: Default::default(),
                max_value: Default::default(),
                step: Default::default(),
                precision: Default::default(),
                tag: "",
                doc: "",
            },
            value: &self.initial_value,
        };

        let key_editor = self
            .property_editors
            .definitions()
            .get(&TypeId::of::<K>())
            .and_then(|definition| {
                Some(
                    definition
                        .property_editor
                        .create_instance(PropertyEditorBuildContext {
                            build_context: ctx,
                            property_info: &key_info,
                            environment: self.environment.clone(),
                            definition_container: self.property_editors.clone(),
                            layer_index: 0,
                            generate_property_string_values: false,
                            filter: Default::default(),
                            name_column_width: 0.0,
                            hide_name_column: true,
                            base_path: "".to_string(),
                            has_parent_object: false,
                        })
                        .ok()?
                        .editor(),
                )
            })
            .unwrap_or_default();

        let ok = ButtonBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
            .with_text("OK")
            .build(ctx);
        let cancel = ButtonBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
            .with_text("Cancel")
            .build(ctx);
        let buttons = StackPanelBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .with_child(ok)
                .with_child(cancel),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(key_editor)
                .with_child(buttons),
        )
        .add_row(Row::stretch())
        .add_row(Row::auto())
        .add_column(Column::stretch())
        .build(ctx);

        let window = SelectHashMapKeyDialogWindow {
            existing_keys: self.existing_keys,
            window: self.window_builder.with_content(grid).build_window(ctx),
            property_editors: self.property_editors,
            key_editor,
            environment: self.environment,
            key: self.initial_value,
            ok,
            cancel,
        };

        ctx.add(window)
    }
}
