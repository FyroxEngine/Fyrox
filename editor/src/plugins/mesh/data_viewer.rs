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

use crate::{
    fyrox::{
        core::pool::Handle,
        engine::Engine,
        gui::{
            VerticalAlignment,
            grid::{Column, GridBuilder, Row},
            message::UiMessage,
            scroll_viewer::ScrollViewerBuilder,
            text::{Text, TextBuilder, TextMessage},
            widget::{WidgetBuilder, WidgetMessage},
            window::{Window, WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
        },
        scene::{
            base::BaseBuilder,
            mesh::{
                MeshBuilder,
                surface::{SurfaceBuilder, SurfaceData, SurfaceResource},
            },
        },
    },
    preview::PreviewPanel,
};

pub struct SurfaceDataViewer {
    pub window: Handle<Window>,
    info: Handle<Text>,
    preview_panel: PreviewPanel,
}

fn surface_data_statistics(surface_data: &SurfaceData) -> Result<String, std::fmt::Error> {
    use fyrox::core::math::TriangleDefinition;
    use std::fmt::Write;
    let mut stats = String::new();
    writeln!(
        &mut stats,
        "Vertices: {}\nVertex Size: {} bytes\nVertex Buffer Size: {} bytes",
        surface_data.vertex_buffer.vertex_count(),
        surface_data.vertex_buffer.vertex_size(),
        surface_data.vertex_buffer.raw_data().len()
    )?;
    for (i, attribute) in surface_data.vertex_buffer.layout().iter().enumerate() {
        writeln!(&mut stats, "[{i}]{attribute}")?;
    }
    let triangle_size = size_of::<TriangleDefinition>();
    writeln!(
        &mut stats,
        "Triangles: {}\nTriangle Size: {} bytes\nTriangle Buffer Size: {} bytes",
        surface_data.geometry_buffer.len(),
        triangle_size,
        surface_data.geometry_buffer.len() * triangle_size
    )?;
    Ok(stats)
}

impl SurfaceDataViewer {
    pub fn new(engine: &mut Engine) -> Self {
        let preview_panel = PreviewPanel::new(engine, 386, 386);

        let ctx = &mut engine.user_interfaces.first_mut().build_ctx();

        let info =
            TextBuilder::new(WidgetBuilder::new().with_vertical_alignment(VerticalAlignment::Top))
                .build(ctx);

        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(preview_panel.root)
                .with_child(
                    ScrollViewerBuilder::new(WidgetBuilder::new().on_row(0).on_column(1))
                        .with_content(info)
                        .build(ctx),
                ),
        )
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .add_column(Column::strict(220.0))
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(650.0).with_height(400.0))
            .open(false)
            .with_remove_on_close(true)
            .with_title(WindowTitle::text("Surface Data"))
            .with_content(content)
            .build(ctx);

        Self {
            window,
            info,
            preview_panel,
        }
    }

    pub fn open(&mut self, surface_data: SurfaceResource, engine: &mut Engine) {
        let guard = surface_data.data_ref();
        let ui = engine.user_interfaces.first();
        ui.send(
            self.info,
            TextMessage::Text(surface_data_statistics(&guard).unwrap_or_default()),
        );
        ui.send(
            self.window,
            WindowMessage::Open {
                alignment: WindowAlignment::Center,
                modal: true,
                focus_content: true,
            },
        );
        drop(guard);

        let graph = &mut engine.scenes[self.preview_panel.scene()].graph;
        let mesh = MeshBuilder::new(BaseBuilder::new())
            .with_surfaces(vec![SurfaceBuilder::new(surface_data).build()])
            .build(graph);

        self.preview_panel.set_model(mesh, engine);
    }

    pub fn destroy(self, engine: &mut Engine) {
        engine
            .user_interfaces
            .first()
            .send(self.window, WidgetMessage::Remove);
        self.preview_panel.destroy(engine);
    }

    pub fn handle_ui_message(mut self, message: &UiMessage, engine: &mut Engine) -> Option<Self> {
        self.preview_panel.handle_message(message, engine);

        if let Some(WindowMessage::Close) = message.data()
            && message.destination() == self.window
        {
            self.destroy(engine);
            return None;
        }

        Some(self)
    }

    pub fn update(&mut self, engine: &mut Engine) {
        self.preview_panel.update(engine)
    }
}
