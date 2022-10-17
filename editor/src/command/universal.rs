use fyrox::core::reflect::{Reflect, ResolvePath};

pub fn set_entity_field(
    entity: &mut dyn Reflect,
    path: &str,
    value: Box<dyn Reflect>,
) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
    let mut components = fyrox::core::reflect::path_to_components(path);
    if let Some(fyrox::core::reflect::Component::Field(field)) = components.pop() {
        let mut parent_path = String::new();
        for component in components.into_iter() {
            match component {
                fyrox::core::reflect::Component::Field(s) => {
                    if !parent_path.is_empty() {
                        parent_path.push('.');
                    }
                    parent_path += s;
                }
                fyrox::core::reflect::Component::Index(s) => {
                    parent_path.push('[');
                    parent_path += s;
                    parent_path.push(']');
                }
            }
        }

        let parent_entity = if parent_path.is_empty() {
            entity
        } else {
            match entity.resolve_path_mut(&parent_path) {
                Err(e) => {
                    fyrox::utils::log::Log::err(format!(
                        "There is no such parent property {}! Reason: {:?}",
                        parent_path, e
                    ));

                    return Err(value);
                }
                Ok(property) => property,
            }
        };

        parent_entity.set_field(field, value)
    } else {
        Err(value)
    }
}

#[macro_export]
macro_rules! define_universal_commands {
    ($name:ident, $command:ident, $command_wrapper:ty, $ctx:ty, $handle:ty, $ctx_ident:ident, $handle_ident:ident, $self:ident, $entity_getter:block) => {
        pub fn $name($handle_ident: $handle, property_changed: &fyrox::gui::inspector::PropertyChanged) -> Option<$command_wrapper> {
            match fyrox::gui::inspector::PropertyAction::from_field_kind(&property_changed.value) {
                fyrox::gui::inspector::PropertyAction::Modify { value } => Some(<$command_wrapper>::new(SetPropertyCommand::new(
                    $handle_ident,
                    property_changed.path(),
                    value,
                ))),
                fyrox::gui::inspector::PropertyAction::AddItem { value } => Some(<$command_wrapper>::new(
                    AddCollectionItemCommand::new($handle_ident, property_changed.path(), value),
                )),
                fyrox::gui::inspector::PropertyAction::RemoveItem { index } => Some(<$command_wrapper>::new(
                    RemoveCollectionItemCommand::new($handle_ident, property_changed.path(), index),
                )),
                // Must be handled outside, there is not enough context and it near to impossible to create universal reversion
                // for InheritableVariable<T>.
                fyrox::gui::inspector::PropertyAction::Revert => None
            }
        }

        fn try_modify_property<F: FnOnce(&mut dyn fyrox::core::reflect::Reflect)>(
            entity: &mut dyn fyrox::core::reflect::Reflect,
            path: &str,
            func: F,
        ) {
            match entity.resolve_path_mut(path) {
                Ok(field) => func(field),
                Err(e) => fyrox::utils::log::Log::err(format!(
                    "There is no such property {}! Reason: {:?}",
                    path, e
                )),
            }
        }

        #[derive(Debug)]
        pub struct SetPropertyCommand {
            #[allow(dead_code)]
            $handle_ident: $handle,
            value: Option<Box<dyn fyrox::core::reflect::Reflect>>,
            path: String,
        }

        impl SetPropertyCommand {
            pub fn new($handle_ident: $handle, path: String, value: Box<dyn fyrox::core::reflect::Reflect>) -> Self {
                Self {
                    $handle_ident,
                    value: Some(value),
                    path,
                }
            }

            fn swap(&mut $self, $ctx_ident: &mut $ctx) {
                match $crate::command::universal::set_entity_field($entity_getter, &$self.path, $self.value.take().unwrap()) {
                    Ok(old_value) => {
                        $self.value = Some(old_value);
                    }
                    Err(current_value) => {
                        $self.value = Some(current_value);
                        fyrox::utils::log::Log::err(format!(
                            "Failed to set property {}! Incompatible types!",
                            $self.path
                        ))
                    }
                }
            }
        }

        impl $command for SetPropertyCommand {
            fn name(&mut $self, _: &$ctx) -> String {
                format!("Set {} property", $self.path)
            }

            fn execute(&mut $self, $ctx_ident: &mut $ctx) {
                $self.swap($ctx_ident);
            }

            fn revert(&mut $self, $ctx_ident: &mut $ctx) {
                $self.swap($ctx_ident);
            }
        }

        #[derive(Debug)]
        pub struct AddCollectionItemCommand {
            #[allow(dead_code)]
            $handle_ident: $handle,
            path: String,
            item: Option<Box<dyn fyrox::core::reflect::Reflect>>,
        }

        impl AddCollectionItemCommand {
            pub fn new($handle_ident: $handle, path: String, item: Box<dyn fyrox::core::reflect::Reflect>) -> Self {
                Self {
                    $handle_ident,
                    path,
                    item: Some(item),
                }
            }
        }

        impl $command for AddCollectionItemCommand {
            fn name(&mut $self, _: &$ctx) -> String {
                format!("Add item to {} collection", $self.path)
            }

            fn execute(&mut $self, $ctx_ident: &mut $ctx) {
                try_modify_property($entity_getter, &$self.path, |field| {
                    if let Some(list) = field.as_list_mut() {
                        if let Err(item) = list.reflect_push($self.item.take().unwrap()) {
                            $self.item = Some(item);
                            fyrox::utils::log::Log::err(format!(
                                "Failed to push item to {} collection. Type mismatch!",
                                $self.path
                            ))
                        }
                    } else {
                        fyrox::utils::log::Log::err(format!("Property {} is not a collection!", $self.path))
                    }
                })
            }

            fn revert(&mut $self, $ctx_ident: &mut $ctx) {
                try_modify_property($entity_getter, &$self.path, |field| {
                    if let Some(list) = field.as_list_mut() {
                        if let Some(item) = list.reflect_pop() {
                            $self.item = Some(item);
                        } else {
                            fyrox::utils::log::Log::err(format!("Failed to pop item from {} collection!", $self.path))
                        }
                    } else {
                        fyrox::utils::log::Log::err(format!("Property {} is not a collection!", $self.path))
                    }
                })
            }
        }

        #[derive(Debug)]
        pub struct RemoveCollectionItemCommand {
            #[allow(dead_code)]
            $handle_ident: $handle,
            path: String,
            index: usize,
            value: Option<Box<dyn fyrox::core::reflect::Reflect>>,
        }

        impl RemoveCollectionItemCommand {
            pub fn new($handle_ident: $handle, path: String, index: usize) -> Self {
                Self {
                    $handle_ident,
                    path,
                    index,
                    value: None,
                }
            }
        }

        impl $command for RemoveCollectionItemCommand {
            fn name(&mut $self, _: &$ctx) -> String {
                format!("Remove collection {} item {}", $self.path, $self.index)
            }

            fn execute(&mut $self, $ctx_ident: &mut $ctx) {
                try_modify_property($entity_getter, &$self.path, |field| {
                    if let Some(list) = field.as_list_mut() {
                        $self.value = list.reflect_remove($self.index);
                    } else {
                        fyrox::utils::log::Log::err(format!("Property {} is not a collection!", $self.path))
                    }
                })
            }

            fn revert(&mut $self, $ctx_ident: &mut $ctx) {
                try_modify_property($entity_getter, &$self.path, |field| {
                    if let Some(list) = field.as_list_mut() {
                        if let Err(item) =
                            list.reflect_insert($self.index, $self.value.take().unwrap())
                        {
                            $self.value = Some(item);
                        } else {
                            fyrox::utils::log::Log::err(format!(
                                "Failed to insert item to {} collection. Type mismatch!",
                                $self.path
                            ))
                        }
                    } else {
                        fyrox::utils::log::Log::err(format!("Property {} is not a collection!", $self.path))
                    }
                })
            }
        }
    };
}
