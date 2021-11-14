use crate::{
    core::inspect::Inspect,
    inspector::{
        editors::{
            Layout, PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext,
        },
        FieldKind, InspectorError, InspectorMessage, PropertyChanged,
    },
    inspector::{Inspector, InspectorBuilder, InspectorContext},
    message::{MessageDirection, UiMessage},
    widget::WidgetBuilder,
};
use std::{
    any::TypeId,
    fmt::{Debug, Formatter},
    marker::PhantomData,
};

pub struct InspectablePropertyEditorDefinition<T>
where
    T: Inspect + 'static,
{
    phantom: PhantomData<T>,
}

impl<T> InspectablePropertyEditorDefinition<T>
where
    T: Inspect + 'static,
{
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<T> Debug for InspectablePropertyEditorDefinition<T>
where
    T: Inspect + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "InspectablePropertyEditorDefinition")
    }
}

impl<T> PropertyEditorDefinition for InspectablePropertyEditorDefinition<T>
where
    T: Inspect + 'static,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<T>()?;

        let inspector_context = InspectorContext::from_object(
            value,
            ctx.build_context,
            ctx.definition_container.clone(),
            ctx.environment.clone(),
            ctx.sync_flag,
        );

        Ok(PropertyEditorInstance {
            title: Default::default(),
            editor: InspectorBuilder::new(WidgetBuilder::new())
                .with_context(inspector_context)
                .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<T>()?;

        let mut error_group = Vec::new();

        let inspector_context = ctx
            .ui
            .node(ctx.instance)
            .cast::<Inspector>()
            .expect("Must be Inspector!")
            .context()
            .clone();
        if let Err(e) = inspector_context.sync(value, ctx.ui) {
            error_group.extend(e.into_iter())
        }

        if error_group.is_empty() {
            Ok(None)
        } else {
            Err(InspectorError::Group(error_group))
        }
    }

    fn translate_message(
        &self,
        name: &str,
        owner_type_id: TypeId,
        message: &UiMessage,
    ) -> Option<PropertyChanged> {
        if let Some(InspectorMessage::PropertyChanged(msg)) = message.data::<InspectorMessage>() {
            if message.direction() == MessageDirection::FromWidget {
                return Some(PropertyChanged {
                    name: name.to_owned(),
                    owner_type_id,
                    value: FieldKind::Inspectable(Box::new(msg.clone())),
                });
            }
        }

        None
    }

    fn layout(&self) -> Layout {
        Layout::Vertical
    }
}
