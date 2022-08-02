#[macro_export]
macro_rules! define_universal_commands {
    ($name:ident, $command:ident, $command_wrapper:ty, $ctx:ty, $handle:ty, $ctx_ident:ident, $handle_ident:ident, $self:ident, $entity_getter:block) => {
        enum Action {
            Modify { value: Box<dyn fyrox::core::reflect::Reflect> },
            AddItem { value: Box<dyn fyrox::core::reflect::Reflect> },
            RemoveItem { index: usize },
        }

        impl Action {
            fn from_field_kind(field_kind: &fyrox::gui::inspector::FieldKind) -> Self {
                match field_kind {
                    fyrox::gui::inspector::FieldKind::Object(ref value) => Self::Modify {
                        value: value.clone().into_box_reflect(),
                    },
                    fyrox::gui::inspector::FieldKind::Collection(ref collection_changed) => match **collection_changed {
                        fyrox::gui::inspector::CollectionChanged::Add(ref value) => Self::AddItem {
                            value: value.clone().into_box_reflect(),
                        },
                        fyrox::gui::inspector::CollectionChanged::Remove(index) => Self::RemoveItem { index },
                        fyrox::gui::inspector::CollectionChanged::ItemChanged { ref property, .. } => {
                            Self::from_field_kind(&property.value)
                        }
                    },
                    fyrox::gui::inspector::FieldKind::Inspectable(ref inspectable) => {
                        Self::from_field_kind(&inspectable.value)
                    }
                }
            }
        }

        pub fn $name($handle_ident: $handle, property_changed: &fyrox::gui::inspector::PropertyChanged) -> $command_wrapper {
            match Action::from_field_kind(&property_changed.value) {
                Action::Modify { value } => <$command_wrapper>::new(SetPropertyCommand::new(
                    $handle_ident,
                    property_changed.path(),
                    value,
                )),
                Action::AddItem { value } => <$command_wrapper>::new(
                    AddCollectionItemCommand::new($handle_ident, property_changed.path(), value),
                ),
                Action::RemoveItem { index } => <$command_wrapper>::new(
                    RemoveCollectionItemCommand::new($handle_ident, property_changed.path(), index),
                ),
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
                let entity = $entity_getter;

                let mut components = fyrox::core::reflect::path_to_components(&$self.path);
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

                                return;
                            }
                            Ok(property) => property,
                        }
                    };

                    match parent_entity.set_field(field, $self.value.take().unwrap()) {
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
                } else {
                    fyrox::utils::log::Log::err(format!(
                        "Failed to set property {}! Invalid path!",
                        $self.path
                    ))
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
                let entity = $entity_getter;
                try_modify_property(entity, &$self.path, |field| {
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
                let entity = $entity_getter;
                try_modify_property(entity, &$self.path, |field| {
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
                let entity = $entity_getter;
                try_modify_property(entity, &$self.path, |field| {
                    if let Some(list) = field.as_list_mut() {
                        $self.value = list.reflect_remove($self.index);
                    } else {
                        fyrox::utils::log::Log::err(format!("Property {} is not a collection!", $self.path))
                    }
                })
            }

            fn revert(&mut $self, $ctx_ident: &mut $ctx) {
                let entity = $entity_getter;
                try_modify_property(entity, &$self.path, |field| {
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
