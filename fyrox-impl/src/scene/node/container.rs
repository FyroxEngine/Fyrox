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

//! A wrapper for node pool record that allows to define custom visit method to have full
//! control over instantiation process at deserialization.
//!

// Since Pool<Node> uses NodeContainer exclusively,
// we just pass the Visit implementation to the Node through VisitAsOption trait.
// See impl VisitAsOption for Node in fyrpx-impl/src/scene/graph/mod.rs.
// (not fyrox-graph/src/lib.rs, which is just for testing)

// use crate::{
//     core::{
//         reflect::prelude::*,
//         uuid::Uuid,
//         visitor::{Visit, VisitResult, Visitor},
//     },
//     engine::SerializationContext,
//     scene::node::Node,
// };
// use fyrox_core::visitor::error::VisitError;

// /// A wrapper for node pool record that allows to define custom visit method to have full
// /// control over instantiation process at deserialization.
// #[derive(Debug, Clone, Default, Reflect)]
// pub struct NodeContainer(Option<Node>);

// impl Visit for NodeContainer {
//     fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
//         let mut region = visitor.enter_region(name)?;

//         let mut is_some = u8::from(self.is_some());
//         is_some.visit("IsSome", &mut region)?;

//         if is_some != 0 {
//             if region.is_reading() {
//                 *self = NodeContainer(Some(read_node("Data", &mut region)?));
//             } else {
//                 write_node("Data", self.0.as_mut().unwrap(), &mut region)?;
//             }
//         }

//         Ok(())
//     }
// }

// impl PayloadContainer for NodeContainer {
//     type Element = Node;

//     fn new_empty() -> Self {
//         Self(None)
//     }

//     fn new(element: Self::Element) -> Self {
//         Self(Some(element))
//     }

//     fn is_some(&self) -> bool {
//         self.0.is_some()
//     }

//     fn as_ref(&self) -> Option<&Self::Element> {
//         self.0.as_ref()
//     }

//     fn as_mut(&mut self) -> Option<&mut Self::Element> {
//         self.0.as_mut()
//     }

//     fn replace(&mut self, element: Self::Element) -> Option<Self::Element> {
//         self.0.replace(element)
//     }

//     fn take(&mut self) -> Option<Self::Element> {
//         self.0.take()
//     }
// }
