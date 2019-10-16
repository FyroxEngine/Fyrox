use crate::scene::node::{CommonNodeData};
use rg3d_core::visitor::{Visit, Visitor, VisitResult};

#[derive(Clone)]
pub struct Pivot {
    common: CommonNodeData
}

impl Default for Pivot {
    fn default() -> Self {
        Self {
            common: Default::default()
        }
    }
}

impl Visit for Pivot {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.common.visit("Common", visitor)?;

        visitor.leave_region()
    }
}

impl_node_trait!(Pivot);
impl_node_trait_private!(Pivot);