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

use crate::game_scene::GameScene;
use crate::make_color_material;
use fyrox::core::algebra::Matrix4;
use fyrox::core::color::Color;
use fyrox::core::pool::Handle;
use fyrox::engine::Engine;
use fyrox::graph::SceneGraph;
use fyrox::scene::base::BaseBuilder;
use fyrox::scene::graph::Graph;
use fyrox::scene::mesh::surface::{SurfaceBuilder, SurfaceData, SurfaceResource};
use fyrox::scene::mesh::{Mesh, MeshBuilder, RenderPath};

#[derive(PartialEq)]
pub struct BrushGizmo {
    pub brush: Handle<Mesh>,
}

impl BrushGizmo {
    pub fn new(game_scene: &GameScene, engine: &mut Engine) -> Self {
        let scene = &mut engine.scenes[game_scene.scene];
        let graph = &mut scene.graph;

        let brush = MeshBuilder::new(
            BaseBuilder::new()
                .with_cast_shadows(false)
                .with_name("Brush")
                .with_visibility(false),
        )
        .with_render_path(RenderPath::Forward)
        .with_surfaces(vec![
            SurfaceBuilder::new(SurfaceResource::new_embedded(SurfaceData::make_quad(
                &Matrix4::identity(),
            )))
            .with_material(make_color_material(Color::from_rgba(0, 255, 0, 130)))
            .build(),
        ])
        .build(graph);

        graph.link_nodes(brush, game_scene.editor_objects_root);

        Self { brush }
    }

    pub fn set_visible(&self, graph: &mut Graph, visibility: bool) {
        graph[self.brush].set_visibility(visibility);
    }

    pub fn destroy(self, graph: &mut Graph) {
        graph.remove_node(self.brush);
    }
}
