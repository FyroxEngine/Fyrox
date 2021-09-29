use crate::{
    core::{
        algebra::Scalar,
        inspect::PropertyInfo,
        math::Rect,
        num_traits::{cast::*, NumAssign},
        pool::Handle,
    },
    inspector::{
        editors::{
            Layout, PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
        },
        InspectorError,
    },
    message::{FieldKind, MessageDirection, PropertyChanged, UiMessage, UiMessageData},
    rect::{RectEditorBuilder, RectEditorMessage},
    widget::WidgetBuilder,
    UiNode,
};
use std::{any::TypeId, fmt::Debug, marker::PhantomData};

#[derive(Debug)]
pub struct RectPropertyEditorDefinition<T>
where
    T: NumAssign + Scalar + PartialOrd + Debug + Copy + Send + Sync + NumCast + 'static,
{
    phantom: PhantomData<T>,
}

impl<T> RectPropertyEditorDefinition<T>
where
    T: NumAssign + Scalar + PartialOrd + Debug + Copy + Send + Sync + NumCast + 'static,
{
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<T> PropertyEditorDefinition for RectPropertyEditorDefinition<T>
where
    T: NumAssign + Scalar + PartialOrd + Debug + Copy + Send + Sync + NumCast + 'static,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Rect<T>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<Rect<T>>()?;

        Ok(PropertyEditorInstance {
            title: Default::default(),
            editor: RectEditorBuilder::new(WidgetBuilder::new().with_height(36.0))
                .with_value(value.clone())
                .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        instance: Handle<UiNode>,
        property_info: &PropertyInfo,
    ) -> Result<UiMessage, InspectorError> {
        let value = property_info.cast_value::<Rect<T>>()?;
        Ok(UiMessage::user(
            instance,
            MessageDirection::ToWidget,
            Box::new(RectEditorMessage::Value(value.clone())),
        ))
    }

    fn translate_message(
        &self,
        name: &str,
        owner_type_id: TypeId,
        message: &UiMessage,
    ) -> Option<PropertyChanged> {
        if message.direction() == MessageDirection::FromWidget {
            if let UiMessageData::User(msg) = message.data() {
                if let Some(msg) = msg.cast::<RectEditorMessage<T>>() {
                    if let RectEditorMessage::Value(value) = msg {
                        return Some(PropertyChanged {
                            name: name.to_string(),
                            owner_type_id,
                            value: FieldKind::object(*value),
                        });
                    }
                }
            }
        }
        None
    }

    fn layout(&self) -> Layout {
        Layout::Vertical
    }
}
