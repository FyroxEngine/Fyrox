use crate::{
    core::{
        algebra::Matrix3,
        pool::Handle,
        visitor::{Visit, VisitResult, Visitor},
    },
    scene2d::{node::Node, transform::Transform},
};

#[derive(Default)]
pub struct Base {
    transform: Transform,
    global_transform: Matrix3<f32>,
    pub(in crate) parent: Handle<Node>,
    pub(in crate) children: Vec<Handle<Node>>,
    name: String,
}

impl Visit for Base {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.transform.visit("Transform", visitor)?;
        self.global_transform.visit("GlobalTransform", visitor)?;
        self.parent.visit("Parent", visitor)?;
        self.children.visit("Children", visitor)?;

        visitor.leave_region()
    }
}

impl Base {
    pub fn set_name<S: AsRef<str>>(&mut self, name: S) {
        self.name = name.as_ref().to_owned();
    }

    pub fn children(&self) -> &[Handle<Node>] {
        &self.children
    }

    pub fn local_transform(&self) -> &Transform {
        &self.transform
    }

    pub fn local_transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }
}
