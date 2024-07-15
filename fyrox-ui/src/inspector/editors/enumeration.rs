use crate::{
    border::BorderBuilder,
    core::{
        combine_uuids,
        pool::Handle,
        reflect::prelude::*,
        reflect::Reflect,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
        TypeUuidProvider,
    },
    decorator::DecoratorBuilder,
    define_constructor,
    dropdown_list::{DropdownList, DropdownListBuilder, DropdownListMessage},
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition,
            PropertyEditorDefinitionContainer, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        make_expander_container, FieldKind, Inspector, InspectorBuilder, InspectorContext,
        InspectorEnvironment, InspectorError, InspectorMessage, PropertyChanged, PropertyFilter,
    },
    message::{MessageDirection, UiMessage},
    text::TextBuilder,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, HorizontalAlignment, Thickness, UiNode, UserInterface,
    VerticalAlignment,
};
use fyrox_core::ComponentProvider;
use fyrox_graph::BaseSceneGraph;
use std::{
    any::TypeId,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    str::FromStr,
    sync::Arc,
};
use strum::VariantNames;

const LOCAL_SYNC_FLAG: u64 = 0xFF;

pub trait InspectableEnum: Debug + Reflect + Clone + TypeUuidProvider + Send + 'static {}

impl<T: Debug + Reflect + Clone + TypeUuidProvider + Send + 'static> InspectableEnum for T {}

#[derive(Debug, Clone, PartialEq)]
pub enum EnumPropertyEditorMessage {
    Variant(usize),
    PropertyChanged(PropertyChanged),
}

impl EnumPropertyEditorMessage {
    define_constructor!(EnumPropertyEditorMessage:Variant => fn variant(usize), layout: false);
    define_constructor!(EnumPropertyEditorMessage:PropertyChanged => fn property_changed(PropertyChanged), layout: false);
}

#[derive(Visit, Reflect, ComponentProvider)]
pub struct EnumPropertyEditor<T: InspectableEnum> {
    pub widget: Widget,
    pub variant_selector: Handle<UiNode>,
    pub inspector: Handle<UiNode>,
    #[visit(skip)]
    #[reflect(hidden)]
    pub definition: EnumPropertyEditorDefinition<T>,
    #[visit(skip)]
    #[reflect(hidden)]
    pub definition_container: Arc<PropertyEditorDefinitionContainer>,
    #[visit(skip)]
    #[reflect(hidden)]
    pub environment: Option<Arc<dyn InspectorEnvironment>>,
    #[visit(skip)]
    #[reflect(hidden)]
    pub sync_flag: u64,
    #[visit(skip)]
    #[reflect(hidden)]
    pub layer_index: usize,
    #[visit(skip)]
    #[reflect(hidden)]
    pub generate_property_string_values: bool,
    #[visit(skip)]
    #[reflect(hidden)]
    pub filter: PropertyFilter,
    #[visit(skip)]
    #[reflect(hidden)]
    pub name_column_width: f32,
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
            layer_index: self.layer_index,
            generate_property_string_values: self.generate_property_string_values,
            filter: self.filter.clone(),
            name_column_width: self.name_column_width,
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

impl<T> TypeUuidProvider for EnumPropertyEditor<T>
where
    T: InspectableEnum,
{
    fn type_uuid() -> Uuid {
        combine_uuids(
            uuid!("0dbefddc-70fa-45a9-96f0-8fe25f6c1669"),
            T::type_uuid(),
        )
    }
}

impl<T: InspectableEnum> Control for EnumPropertyEditor<T> {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(EnumPropertyEditorMessage::Variant(variant)) =
            message.data::<EnumPropertyEditorMessage>()
        {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
            {
                let variant = (self.definition.variant_generator)(*variant);

                let ctx = InspectorContext::from_object(
                    &variant,
                    &mut ui.build_ctx(),
                    self.definition_container.clone(),
                    self.environment.clone(),
                    self.sync_flag,
                    self.layer_index,
                    self.generate_property_string_values,
                    self.filter.clone(),
                    self.name_column_width,
                );

                ui.send_message(InspectorMessage::context(
                    self.inspector,
                    MessageDirection::ToWidget,
                    ctx,
                ));

                ui.send_message(message.reverse());
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
    definition_container: Option<Arc<PropertyEditorDefinitionContainer>>,
    environment: Option<Arc<dyn InspectorEnvironment>>,
    sync_flag: u64,
    variant_selector: Handle<UiNode>,
    layer_index: usize,
    generate_property_string_values: bool,
    filter: PropertyFilter,
}

impl EnumPropertyEditorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            definition_container: None,
            environment: None,
            sync_flag: 0,
            variant_selector: Handle::NONE,
            layer_index: 0,
            generate_property_string_values: false,
            filter: Default::default(),
        }
    }

    pub fn with_definition_container(
        mut self,
        definition_container: Arc<PropertyEditorDefinitionContainer>,
    ) -> Self {
        self.definition_container = Some(definition_container);
        self
    }

    pub fn with_sync_flag(mut self, sync_flag: u64) -> Self {
        self.sync_flag = sync_flag;
        self
    }

    pub fn with_environment(mut self, environment: Option<Arc<dyn InspectorEnvironment>>) -> Self {
        self.environment = environment;
        self
    }

    pub fn with_variant_selector(mut self, variant_selector: Handle<UiNode>) -> Self {
        self.variant_selector = variant_selector;
        self
    }

    pub fn with_layer_index(mut self, layer_index: usize) -> Self {
        self.layer_index = layer_index;
        self
    }

    pub fn with_filter(mut self, filter: PropertyFilter) -> Self {
        self.filter = filter;
        self
    }

    pub fn with_generate_property_string_values(
        mut self,
        generate_property_string_values: bool,
    ) -> Self {
        self.generate_property_string_values = generate_property_string_values;
        self
    }

    pub fn build<T: InspectableEnum>(
        self,
        ctx: &mut BuildContext,
        definition: &EnumPropertyEditorDefinition<T>,
        value: &T,
        name_column_width: f32,
    ) -> Handle<UiNode> {
        let definition_container = self
            .definition_container
            .unwrap_or_else(|| Arc::new(PropertyEditorDefinitionContainer::with_default_editors()));

        let context = InspectorContext::from_object(
            value,
            ctx,
            definition_container.clone(),
            self.environment.clone(),
            self.sync_flag,
            self.layer_index,
            self.generate_property_string_values,
            self.filter.clone(),
            name_column_width,
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
            layer_index: self.layer_index,
            generate_property_string_values: self.generate_property_string_values,
            filter: self.filter,
            name_column_width,
        };

        ctx.add_node(UiNode::new(editor))
    }
}

pub struct EnumPropertyEditorDefinition<T: InspectableEnum> {
    pub variant_generator: fn(usize) -> T,
    pub index_generator: fn(&T) -> usize,
    pub names_generator: fn() -> Vec<String>,
}

impl<T: InspectableEnum + Default> EnumPropertyEditorDefinition<T> {
    pub fn new_optional() -> EnumPropertyEditorDefinition<Option<T>> {
        EnumPropertyEditorDefinition {
            variant_generator: |i| match i {
                0 => None,
                1 => Some(Default::default()),
                _ => unreachable!(),
            },
            index_generator: |v| match v {
                None => 0,
                Some(_) => 1,
            },
            names_generator: || vec!["None".to_string(), "Some".to_string()],
        }
    }
}

impl<T, E: Debug> EnumPropertyEditorDefinition<T>
where
    T: InspectableEnum + VariantNames + AsRef<str> + FromStr<Err = E> + Debug,
{
    pub fn new() -> Self {
        Self {
            variant_generator: |i| T::from_str(T::VARIANTS[i]).unwrap(),
            index_generator: |in_var| {
                T::VARIANTS
                    .iter()
                    .position(|v| v == &in_var.as_ref())
                    .unwrap()
            },
            names_generator: || T::VARIANTS.iter().map(|v| v.to_string()).collect(),
        }
    }
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
                .with_margin(Thickness::top_bottom(1.0)),
        )
        .with_selected((self.index_generator)(value))
        .with_items(
            names
                .into_iter()
                .map(|name| {
                    DecoratorBuilder::new(
                        BorderBuilder::new(
                            WidgetBuilder::new().with_child(
                                TextBuilder::new(WidgetBuilder::new())
                                    .with_vertical_text_alignment(VerticalAlignment::Center)
                                    .with_horizontal_text_alignment(HorizontalAlignment::Center)
                                    .with_text(name)
                                    .build(ctx.build_context),
                            ),
                        )
                        .with_corner_radius(4.0)
                        .with_pad_by_corner_radius(false),
                    )
                    .build(ctx.build_context)
                })
                .collect::<Vec<_>>(),
        )
        .with_close_on_selection(true)
        .build(ctx.build_context);

        let editor;
        let container = make_expander_container(
            ctx.layer_index,
            ctx.property_info.display_name,
            ctx.property_info.description,
            variant_selector,
            {
                editor = EnumPropertyEditorBuilder::new(WidgetBuilder::new())
                    .with_variant_selector(variant_selector)
                    .with_layer_index(ctx.layer_index + 1)
                    .with_definition_container(ctx.definition_container.clone())
                    .with_environment(ctx.environment.clone())
                    .with_sync_flag(ctx.sync_flag)
                    .with_generate_property_string_values(ctx.generate_property_string_values)
                    .with_filter(ctx.filter)
                    .build(ctx.build_context, self, value, ctx.name_column_width);
                editor
            },
            ctx.name_column_width,
            ctx.build_context,
        );

        Ok(PropertyEditorInstance::Custom { container, editor })
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
        if Some(variant_index) != *variant_selector_ref.selection {
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
                ctx.layer_index + 1,
                ctx.generate_property_string_values,
                ctx.filter,
                ctx.name_column_width,
            );

            Ok(Some(InspectorMessage::context(
                inspector,
                MessageDirection::ToWidget,
                context,
            )))
        } else {
            let layer_index = ctx.layer_index;
            let inspector_ctx = ctx
                .ui
                .node(instance_ref.inspector)
                .cast::<Inspector>()
                .expect("Must be Inspector!")
                .context()
                .clone();

            if let Err(e) = inspector_ctx.sync(
                value,
                ctx.ui,
                layer_index + 1,
                ctx.generate_property_string_values,
                ctx.filter,
            ) {
                Err(InspectorError::Group(e))
            } else {
                Ok(None)
            }
        }
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if let Some(msg) = ctx.message.data::<EnumPropertyEditorMessage>() {
            return match msg {
                EnumPropertyEditorMessage::PropertyChanged(property_changed) => {
                    Some(PropertyChanged {
                        name: ctx.name.to_string(),
                        owner_type_id: ctx.owner_type_id,
                        value: FieldKind::Inspectable(Box::new(property_changed.clone())),
                    })
                }
                EnumPropertyEditorMessage::Variant(index) => Some(PropertyChanged {
                    name: ctx.name.to_string(),
                    owner_type_id: ctx.owner_type_id,
                    value: FieldKind::object((self.variant_generator)(*index)),
                }),
            };
        }

        None
    }
}
