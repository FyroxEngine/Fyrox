//! A wrapper for node pool record that allows to define custom visit method to have full
//! control over instantiation process at deserialization.

use crate::{
    core::{
        pool::PayloadContainer,
        uuid::Uuid,
        visitor::{Visit, VisitError, VisitResult, Visitor},
    },
    scene::{
        self,
        camera::Camera,
        decal::Decal,
        dim2::{self, rectangle::Rectangle},
        light::{directional::DirectionalLight, point::PointLight, spot::SpotLight},
        mesh::Mesh,
        node::{constructor::NodeConstructorContainer, Node},
        particle_system::ParticleSystem,
        pivot::Pivot,
        sound::{listener::Listener, Sound},
        sprite::Sprite,
        terrain::Terrain,
    },
};

/// A wrapper for node pool record that allows to define custom visit method to have full
/// control over instantiation process at deserialization.
#[derive(Debug, Default)]
pub struct NodeContainer(Option<Node>);

fn read_node(name: &str, visitor: &mut Visitor) -> Result<Node, VisitError> {
    let node = {
        // Handle legacy nodes.
        let mut kind_id = 0u8;
        if kind_id.visit("KindId", visitor).is_ok() {
            let mut node = match kind_id {
                0 => Node::new(Pivot::default()),
                1 => {
                    visitor.enter_region(name)?;

                    let mut light_id = 0u32;
                    light_id.visit("KindId", visitor)?;

                    let mut light_node = match light_id {
                        0 => Node::new(SpotLight::default()),
                        1 => Node::new(PointLight::default()),
                        2 => Node::new(DirectionalLight::default()),
                        _ => {
                            return Err(VisitError::User(format!(
                                "Invalid legacy light kind {}",
                                light_id
                            )))
                        }
                    };

                    light_node.visit("Data", visitor)?;

                    visitor.leave_region()?;

                    return Ok(light_node);
                }
                2 => Node::new(Camera::default()),
                3 => Node::new(Mesh::default()),
                4 => Node::new(Sprite::default()),
                5 => Node::new(ParticleSystem::default()),
                6 => Node::new(Terrain::default()),
                7 => Node::new(Decal::default()),
                8 => Node::new(scene::rigidbody::RigidBody::default()),
                9 => Node::new(scene::collider::Collider::default()),
                10 => Node::new(scene::joint::Joint::default()),
                11 => Node::new(Rectangle::default()),
                12 => Node::new(dim2::rigidbody::RigidBody::default()),
                13 => Node::new(dim2::collider::Collider::default()),
                14 => Node::new(dim2::joint::Joint::default()),
                15 => Node::new(Sound::default()),
                16 => Node::new(Listener::default()),
                _ => {
                    return Err(VisitError::User(format!(
                        "Invalid legacy node kind {}",
                        kind_id
                    )))
                }
            };

            node.visit(name, visitor)?;

            node
        } else {
            // Latest version
            visitor.enter_region(name)?;

            let mut id = Uuid::default();
            id.visit("TypeUuid", visitor)?;

            let mut node = NodeConstructorContainer::instance()
                .try_create(&id)
                .ok_or_else(|| VisitError::User(format!("Unknown node type uuid {}!", id)))?;

            node.visit("NodeData", visitor)?;

            visitor.leave_region()?;

            node
        }
    };

    Ok(node)
}

fn write_node(name: &str, node: &mut Node, visitor: &mut Visitor) -> VisitResult {
    visitor.enter_region(name)?;

    let mut id = node.id();
    id.visit("TypeUuid", visitor)?;

    node.visit("NodeData", visitor)?;

    visitor.leave_region()
}

impl Visit for NodeContainer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut is_some = if self.is_some() { 1u8 } else { 0u8 };
        is_some.visit("IsSome", visitor)?;

        if is_some != 0 {
            if visitor.is_reading() {
                *self = NodeContainer(Some(read_node("Data", visitor)?));
            } else {
                write_node("Data", self.0.as_mut().unwrap(), visitor)?;
            }
        }

        visitor.leave_region()?;
        Ok(())
    }
}

impl PayloadContainer for NodeContainer {
    type Element = Node;

    fn new_empty() -> Self {
        Self(None)
    }

    fn new(element: Self::Element) -> Self {
        Self(Some(element))
    }

    fn is_some(&self) -> bool {
        self.0.is_some()
    }

    fn as_ref(&self) -> Option<&Self::Element> {
        self.0.as_ref()
    }

    fn as_mut(&mut self) -> Option<&mut Self::Element> {
        self.0.as_mut()
    }

    fn replace(&mut self, element: Self::Element) -> Option<Self::Element> {
        self.0.replace(element)
    }

    fn take(&mut self) -> Option<Self::Element> {
        self.0.take()
    }
}
