use crate::{
    border::BorderBuilder,
    core::{inspect::Inspect, pool::Handle},
    decorator::DecoratorBuilder,
    define_constructor,
    dropdown_list::{DropdownList, DropdownListBuilder, DropdownListMessage},
    grid::{Column, GridBuilder, Row},
    inspector::{
        editors::{
            Layout, PropertyEditorBuildContext, PropertyEditorDefinition,
            PropertyEditorDefinitionContainer, PropertyEditorInstance,
            PropertyEditorMessageContext,
        },
        FieldKind, Inspector, InspectorBuilder, InspectorContext, InspectorEnvironment,
        InspectorError, InspectorMessage, PropertyChanged, HEADER_MARGIN, NAME_COLUMN_WIDTH,
    },
    message::{MessageDirection, UiMessage},
    text::TextBuilder,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, HorizontalAlignment, Thickness, UiNode, UserInterface,
    VerticalAlignment,
};
use std::{
    any::TypeId,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    rc::Rc,
};

const LOCAL_SYNC_FLAG: u64 = 0xFF;

pub trait InspectableEnum: Debug + Inspect + 'static {}

impl<T: Debug + Inspect + 'static> InspectableEnum for T {}

#[derive(Debug, Clone, PartialEq)]
pub enum EnumPropertyEditorMessage {
    Variant(usize),
    PropertyChanged(PropertyChanged),
}

impl EnumPropertyEditorMessage {
    define_constructor!(EnumPropertyEditorMessage:Variant => fn variant(usize), layout: false);
    define_constructor!(EnumPropertyEditorMessage:PropertyChanged => fn property_changed(PropertyChanged), layout: false);
}

pub struct EnumPropertyEditor<T: InspectableEnum> {
    widget: Widget,
    variant_selector: Handle<UiNode>,
    inspector: Handle<UiNode>,
    definition: EnumPropertyEditorDefinition<T>,
    definition_container: Rc<PropertyEditorDefinitionContainer>,
    environment: Option<Rc<dyn InspectorEnvironment>>,
    sync_flag: u64,
}

impl<T: InspectableEnum> Debug for EnumPropertyEditor<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "EnumPropertyEditor")
    }
}

impl<T: InspectableEnum> Clone for EnumPropertyEditor<T> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            variant_selector: self.variant_selector,
            inspector: self.inspector,
            definition: self.definition.clone(),
            definition_container: self.definition_container.clone(),
            environment: self.environment.clone(),
            sync_flag: self.sync_flag,
        }
    }
}

impl<T: InspectableEnum> Deref for EnumPropertyEditor<T> {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<T: InspectableEnum> DerefMut for EnumPropertyEditor<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<T: InspectableEnum> Control for EnumPropertyEditor<T> {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(EnumPropertyEditorMessage::Variant(variant)) =
            message.data::<EnumPropertyEditorMessage>()
        {
            if message.destination() == self.handle {
                let variant = (self.definition.variant_generator)(*variant);

                let ctx = InspectorContext::from_object(
                    &variant,
                    &mut ui.build_ctx(),
                    self.definition_container.clone(),
                    self.environment.clone(),
                    self.sync_flag,
                );

                ui.send_message(InspectorMessage::context(
                    self.inspector,
                    MessageDirection::ToWidget,
                    ctx,
                ));
            }
        } else if let Some(InspectorMessage::PropertyChanged(property_changed)) =
            message.data::<InspectorMessage>()
        {
            if message.destination() == self.inspector
                && message.direction() == MessageDirection::FromWidget
            {
                ui.send_message(EnumPropertyEditorMessage::property_changed(
                    self.handle,
                    MessageDirection::FromWidget,
                    property_changed.clone(),
                ))
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if message.direction() == MessageDirection::FromWidget
            && message.destination() == self.variant_selector
            && message.flags != LOCAL_SYNC_FLAG
        {
            if let Some(DropdownListMessage::SelectionChanged(Some(index))) =
                message.data::<DropdownListMessage>()
            {
                ui.send_message(EnumPropertyEditorMessage::variant(
                    self.handle,
                    MessageDirection::ToWidget,
                    *index,
                ));
            }
        }
    }
}

pub struct EnumPropertyEditorBuilder {
    widget_builder: WidgetBuilder,
    definition_container: Option<Rc<PropertyEditorDefinitionContainer>>,
    environment: Option<Rc<dyn InspectorEnvironment>>,
    sync_flag: u64,
    variant_selector: Handle<UiNode>,
}

impl EnumPropertyEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            definition_container: None,
            environment: None,
            sync_flag: 0,
            variant_selector: Handle::NONE,
        }
    }

    pub fn with_definition_container(
        mut self,
        definition_container: Rc<PropertyEditorDefinitionContainer>,
    ) -> Self {
        self.definition_container = Some(definition_container);
        self
    }

    pub fn with_sync_flag(mut self, sync_flag: u64) -> Self {
        self.sync_flag = sync_flag;
        self
    }

    pub fn with_environment(mut self, environment: Option<Rc<dyn InspectorEnvironment>>) -> Self {
        self.environment = environment;
        self
    }

    pub fn with_variant_selector(mut self, variant_selector: Handle<UiNode>) -> Self {
        self.variant_selector = variant_selector;
        self
    }

    pub fn build<T: InspectableEnum>(
        self,
        ctx: &mut BuildContext,
        definition: &EnumPropertyEditorDefinition<T>,
        value: &T,
    ) -> Handle<UiNode> {
        let definition_container = self
            .definition_container
            .unwrap_or_else(|| Rc::new(PropertyEditorDefinitionContainer::new()));

        let context = InspectorContext::from_object(
            value,
            ctx,
            definition_container.clone(),
            self.environment.clone(),
            self.sync_flag,
        );

        let inspector = InspectorBuilder::new(WidgetBuilder::new())
            .with_context(context)
            .build(ctx);

        let editor = EnumPropertyEditor {
            widget: self
                .widget_builder
                .with_preview_messages(true)
                .with_child(inspector)
                .build(),
            variant_selector: self.variant_selector,
            inspector,
            definition: definition.clone(),
            definition_container,
            environment: self.environment,
            sync_flag: self.sync_flag,
        };

        ctx.add_node(UiNode::new(editor))
    }
}

pub struct EnumPropertyEditorDefinition<T: InspectableEnum> {
    pub variant_generator: fn(usize) -> T,
    pub index_generator: fn(&T) -> usize,
    pub names_generator: fn() -> Vec<String>,
}

impl<T: InspectableEnum> Clone for EnumPropertyEditorDefinition<T> {
    fn clone(&self) -> Self {
        Self {
            variant_generator: self.variant_generator,
            index_generator: self.index_generator,
            names_generator: self.names_generator,
        }
    }
}

impl<T> Debug for EnumPropertyEditorDefinition<T>
where
    T: InspectableEnum,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "EnumPropertyEditorDefinition")
    }
}

impl<T> PropertyEditorDefinition for EnumPropertyEditorDefinition<T>
where
    T: InspectableEnum,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<T>()?;
        let names = (self.names_generator)();

        let variant_selector = DropdownListBuilder::new(
            WidgetBuilder::new()
                .on_column(1)
                .with_margin(Thickness::uniform(1.0)),
        )
        .with_selected((self.index_generator)(value))
        .with_items(
            names
                .into_iter()
                .map(|name| {
                    DecoratorBuilder::new(BorderBuilder::new(
                        WidgetBuilder::new().with_height(26.0).with_child(
                            TextBuilder::new(WidgetBuilder::new())
                                .with_vertical_text_alignment(VerticalAlignment::Center)
                                .with_horizontal_text_alignment(HorizontalAlignment::Center)
                                .with_text(name)
                                .build(ctx.build_context),
                        ),
                    ))
                    .build(ctx.build_context)
                })
                .collect::<Vec<_>>(),
        )
        .with_close_on_selection(true)
        .build(ctx.build_context);

        Ok(PropertyEditorInstance {
            title: GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(
                        TextBuilder::new(WidgetBuilder::new().with_margin(HEADER_MARGIN))
                            .with_text(ctx.property_info.display_name)
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ctx.build_context),
                    )
                    .with_child(variant_selector),
            )
            .add_column(Column::strict(NAME_COLUMN_WIDTH))
            .add_column(Column::stretch())
            .add_row(Row::strict(26.0))
            .build(ctx.build_context),
            editor: EnumPropertyEditorBuilder::new(WidgetBuilder::new())
                .with_variant_selector(variant_selector)
                .with_definition_container(ctx.definition_container.clone())
                .with_environment(ctx.environment.clone())
                .with_sync_flag(ctx.sync_flag)
                .build(ctx.build_context, self, value),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<T>()?;

        let instance_ref = ctx
            .ui
            .node(ctx.instance)
            .cast::<EnumPropertyEditor<T>>()
            .expect("Must be EnumPropertyEditor!");

        let variant_selector_ref = ctx
            .ui
            .node(instance_ref.variant_selector)
            .cast::<DropdownList>()
            .expect("Must be a DropDownList");

        let variant_index = (self.index_generator)(value);
        if Some(variant_index) != variant_selector_ref.selection() {
            let environment = ctx
                .ui
                .node(instance_ref.inspector)
                .cast::<Inspector>()
                .expect("Must be Inspector!")
                .context()
                .environment
                .clone();

            let mut selection_message = DropdownListMessage::selection(
                instance_ref.variant_selector,
                MessageDirection::ToWidget,
                Some(variant_index),
            );
            selection_message.flags = LOCAL_SYNC_FLAG;
            ctx.ui.send_message(selection_message);

            let inspector = instance_ref.inspector;

            let context = InspectorContext::from_object(
                value,
                &mut ctx.ui.build_ctx(),
                ctx.definition_container.clone(),
                environment,
                ctx.sync_flag,
            );

            Ok(Some(InspectorMessage::context(
                inspector,
                MessageDirection::ToWidget,
                context,
            )))
        } else {
            let inspector_ctx = ctx
                .ui
                .node(instance_ref.inspector)
                .cast::<Inspector>()
                .expect("Must be Inspector!")
                .context()
                .clone();

            if let Err(e) = inspector_ctx.sync(value, ctx.ui) {
                Err(InspectorError::Group(e))
            } else {
                Ok(None)
            }
        }
    }

    fn translate_message(
        &self,
        name: &str,
        owner_type_id: TypeId,
        message: &UiMessage,
    ) -> Option<PropertyChanged> {
        if let Some(msg) = message.data::<EnumPropertyEditorMessage>() {
            return match msg {
                EnumPropertyEditorMessage::PropertyChanged(property_changed) => {
                    Some(PropertyChanged {
                        name: name.to_string(),
                        owner_type_id,
                        value: FieldKind::Inspectable(Box::new(property_changed.clone())),
                    })
                }
                EnumPropertyEditorMessage::Variant(index) => Some(PropertyChanged {
                    name: name.to_string(),
                    owner_type_id,
                    value: FieldKind::object((self.variant_generator)(*index)),
                }),
            };
        }

        None
    }

    fn layout(&self) -> Layout {
        Layout::Vertical
    }
}
