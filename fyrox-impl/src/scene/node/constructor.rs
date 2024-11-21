// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! A special container that is able to create nodes by their type UUID.

use crate::scene::graph::Graph;
use crate::scene::{
    self,
    animation::{absm::AnimationBlendingStateMachine, AnimationPlayer},
    camera::Camera,
    decal::Decal,
    dim2::{self, rectangle::Rectangle},
    light::{directional::DirectionalLight, point::PointLight, spot::SpotLight},
    mesh::Mesh,
    navmesh::NavigationalMesh,
    node::Node,
    particle_system::ParticleSystem,
    pivot::Pivot,
    ragdoll::Ragdoll,
    sound::{listener::Listener, Sound},
    sprite::Sprite,
    terrain::Terrain,
    tilemap::TileMap,
};
use fyrox_graph::constructor::{GraphNodeConstructor, GraphNodeConstructorContainer};

/// Node constructor creates scene nodes in various states.
pub type NodeConstructor = GraphNodeConstructor<Node, Graph>;

/// A special container that is able to create nodes by their type UUID.
pub type NodeConstructorContainer = GraphNodeConstructorContainer<Node, Graph>;

/// Creates default node constructor container with constructors for built-in engine nodes.
pub fn new_node_constructor_container() -> NodeConstructorContainer {
    let container = NodeConstructorContainer::default();

    container.add::<dim2::collider::Collider>();
    container.add::<dim2::joint::Joint>();
    container.add::<Rectangle>();
    container.add::<dim2::rigidbody::RigidBody>();
    container.add::<DirectionalLight>();
    container.add::<PointLight>();
    container.add::<SpotLight>();
    container.add::<Mesh>();
    container.add::<ParticleSystem>();
    container.add::<Sound>();
    container.add::<Listener>();
    container.add::<Camera>();
    container.add::<scene::collider::Collider>();
    container.add::<Decal>();
    container.add::<scene::joint::Joint>();
    container.add::<Pivot>();
    container.add::<scene::rigidbody::RigidBody>();
    container.add::<Sprite>();
    container.add::<Terrain>();
    container.add::<AnimationPlayer>();
    container.add::<AnimationBlendingStateMachine>();
    container.add::<NavigationalMesh>();
    container.add::<Ragdoll>();
    container.add::<TileMap>();

    container
}
