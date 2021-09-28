use crate::{
    core::{inspect::PropertyInfo, pool::Handle},
    inspector::{
        editors::{
            Layout, PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
        },
        InspectorError,
    },
    message::{
        FieldKind, MessageDirection, NumericUpDownMessage, PropertyChanged, UiMessage,
        UiMessageData,
    },
    numeric::NumericUpDownBuilder,
    widget::WidgetBuilder,
    Thickness, UiNode,
};
use std::any::TypeId;

macro_rules! define_integer_property_editor {
    ($name:ident, $value:ty, $min:expr) => {
        #[derive(Debug)]
        pub struct $name;

        impl PropertyEditorDefinition for $name {
            fn value_type_id(&self) -> TypeId {
                TypeId::of::<$value>()
            }

            fn create_instance(
                &self,
                ctx: PropertyEditorBuildContext,
            ) -> Result<PropertyEditorInstance, InspectorError> {
                let value = ctx.property_info.cast_value::<$value>()?;
                Ok(PropertyEditorInstance {
                    title: Default::default(),
                    editor: NumericUpDownBuilder::new(
                        WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                    )
                    .with_precision(0)
                    .with_step(1.0)
                    .with_min_value($min as f32)
                    .with_max_value(<$value>::MAX as f32)
                    .with_value(*value as f32)
                    .build(ctx.build_context),
                })
            }

            fn create_message(
                &self,
                instance: Handle<UiNode>,
                property_info: &PropertyInfo,
            ) -> Result<UiMessage, InspectorError> {
                let value = property_info.cast_value::<$value>()?;
                Ok(NumericUpDownMessage::value(
                    instance,
                    MessageDirection::ToWidget,
                    *value as f32,
                ))
            }

            fn translate_message(
                &self,
                name: &str,
                owner_type_id: TypeId,
                message: &UiMessage,
            ) -> Option<PropertyChanged> {
                if message.direction() == MessageDirection::FromWidget {
                    if let UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value)) =
                        message.data()
                    {
                        return Some(PropertyChanged {
                            name: name.to_string(),
                            owner_type_id,
                            value: FieldKind::object(*value as $value),
                        });
                    }
                }
                None
            }

            fn layout(&self) -> Layout {
                Layout::Horizontal
            }
        }
    };
}

define_integer_property_editor!(I8PropertyEditorDefinition, i8, -i8::MAX);
define_integer_property_editor!(U8PropertyEditorDefinition, u8, 0);
define_integer_property_editor!(I16PropertyEditorDefinition, i16, -i16::MAX);
define_integer_property_editor!(U16PropertyEditorDefinition, u16, 0);
define_integer_property_editor!(I32PropertyEditorDefinition, i32, -i32::MAX);
define_integer_property_editor!(U32PropertyEditorDefinition, u32, 0);
define_integer_property_editor!(I64PropertyEditorDefinition, i64, -i64::MAX);
define_integer_property_editor!(U64PropertyEditorDefinition, u64, 0);
