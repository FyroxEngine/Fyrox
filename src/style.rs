use std::{
    any::Any,
    rc::Rc
};

#[derive(Debug)]
pub struct PropertySetter {
    name: String,
    value: Box<dyn Any>,
}

impl PropertySetter {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn value(&self) -> &dyn Any {
        self.value.as_ref()
    }
}

#[derive(Default, Debug)]
pub struct Style {
    base_style: Option<Rc<Style>>,
    setters: Vec<PropertySetter>,
}

impl Style {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn setters(&self) -> &[PropertySetter] {
        self.setters.as_slice()
    }

    pub fn base_style(&self) -> Option<Rc<Style>> {
        self.base_style.clone()
    }
}

pub struct StyleBuilder {
    base_style: Option<Rc<Style>>,
    setters: Vec<PropertySetter>,
}

impl Default for StyleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl StyleBuilder {
    pub fn new() -> Self {
        Self {
            base_style: None,
            setters: Default::default(),
        }
    }

    pub fn with_base_style(mut self, style: Rc<Style>) -> Self {
        self.base_style = Some(style);
        self
    }

    pub fn with_setter(mut self, name: &str, value: Box<dyn Any>) -> Self {
        self.setters.push(PropertySetter { name: name.to_owned(), value });
        self
    }

    pub fn build(self) -> Style {
        Style {
            base_style: self.base_style,
            setters: self.setters
        }
    }
}