/// Collection of default templates for all widgets. You can modify these before create any widgets in your app
/// to get your unique style.

use crate::ControlTemplate;
use crate::button::Button;

pub struct TemplatesCollection {
    button: ControlTemplate
}

impl TemplatesCollection {
    pub fn new() -> Self {
        Self {
            button: Button::template()
        }
    }
}
