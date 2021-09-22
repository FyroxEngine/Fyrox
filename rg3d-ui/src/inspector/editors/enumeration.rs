use crate::{
    border::BorderBuilder,
    core::{inspect::PropertyInfo, pool::Handle},
    decorator::DecoratorBuilder,
    dropdown_list::DropdownListBuilder,
    inspector::{
        editors::{PropertyEditorBuildContext, PropertyEditorDefinition},
        InspectorError,
    },
    message::{
        DropdownListMessage, MessageData, MessageDirection, PropertyChanged, UiMessage,
        UiMessageData,
    },
    node::UINode,
    text::TextBuilder,
    widget::WidgetBuilder,
    Control, HorizontalAlignment, Thickness, VerticalAlignment,
};
use std::{
    any::TypeId,
    fmt::{Debug, Formatter},
    sync::Arc,
};

pub struct EnumPropertyEditorDefinition<T: Debug> {
    pub variant_generator: fn(usize) -> T,
    pub index_generator: fn(&T) -> usize,
    pub names_generator: fn() -> Vec<String>,
}

impl<T> Debug for EnumPropertyEditorDefinition<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "EnumPropertyEditorDefinition")
    }
}

impl<T, M, C> PropertyEditorDefinition<M, C> for EnumPropertyEditorDefinition<T>
where
    T: Debug + Send + Sync + 'static,
    M: MessageData,
    C: Control<M, C>,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext<M, C>,
    ) -> Result<Handle<UINode<M, C>>, InspectorError> {
        let value = ctx.property_info.cast_value::<T>()?;
        let names = (self.names_generator)();

        Ok(DropdownListBuilder::new(
            WidgetBuilder::new()
                .on_row(ctx.row)
                .on_column(ctx.column)
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
        .build(ctx.build_context))
    }

    fn create_message(
        &self,
        instance: Handle<UINode<M, C>>,
        property_info: &PropertyInfo,
    ) -> Result<UiMessage<M, C>, InspectorError> {
        let value = property_info.cast_value::<T>()?;
        Ok(DropdownListMessage::selection(
            instance,
            MessageDirection::ToWidget,
            Some((self.index_generator)(value)),
        ))
    }

    fn translate_message(
        &self,
        name: &str,
        owner_type_id: TypeId,
        message: &UiMessage<M, C>,
    ) -> Option<PropertyChanged> {
        if message.direction() == MessageDirection::FromWidget {
            if let UiMessageData::DropdownList(DropdownListMessage::SelectionChanged(Some(index))) =
                message.data()
            {
                return Some(PropertyChanged {
                    name: name.to_string(),
                    owner_type_id,
                    value: Arc::new((self.variant_generator)(*index)),
                });
            }
        }

        None
    }
}
