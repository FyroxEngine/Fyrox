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

pub mod container;

use crate::controller::SceneController;
use crate::{
    fyrox::{
        core::{algebra::Vector2, define_as_any_trait, pool::Handle, reflect::Reflect, uuid::Uuid},
        engine::Engine,
        gui::{
            BuildContext, Thickness,
            border::BorderBuilder,
            button::{Button, ButtonBuilder},
            decorator::DecoratorBuilder,
            image::ImageBuilder,
            key::HotKey,
            message::{KeyCode, UiMessage},
            style::{Style, resource::StyleResourceExt},
            utils::make_simple_tooltip,
            widget::WidgetBuilder,
        },
    },
    game_scene::Selection,
    load_image_internal,
    settings::Settings,
};

define_as_any_trait!(InteractionModeAsAny => InteractionMode);

pub trait InteractionMode: Reflect + InteractionModeAsAny {
    fn on_left_mouse_button_down(
        &mut self,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
        settings: &Settings,
    );

    fn on_left_mouse_button_up(
        &mut self,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
        settings: &Settings,
    );

    fn on_mouse_move(
        &mut self,
        mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        frame_size: Vector2<f32>,
        settings: &Settings,
    );

    /// Called when the mouse enters the scene viewer while this interaction mode is active.
    #[allow(unused_variables)]
    fn on_mouse_enter(
        &mut self,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        frame_size: Vector2<f32>,
        settings: &Settings,
    ) {
    }

    /// Called when the mouse leaves the scene viewer while this interaction mode is active.
    /// - `mouse_position`: The position of the mouse relative to the scene viewer, with (0,0) being the left-top corner.
    /// - `editor_selection`: The currently selected object in the editor.
    #[allow(unused_variables)]
    fn on_mouse_leave(
        &mut self,
        mouse_position: Vector2<f32>,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        frame_size: Vector2<f32>,
        settings: &Settings,
    ) {
    }

    fn update(
        &mut self,
        #[allow(unused_variables)] editor_selection: &Selection,
        #[allow(unused_variables)] controller: &mut dyn SceneController,
        #[allow(unused_variables)] engine: &mut Engine,
        #[allow(unused_variables)] settings: &Settings,
    ) {
    }

    fn activate(
        &mut self,
        #[allow(unused_variables)] controller: &dyn SceneController,
        #[allow(unused_variables)] engine: &mut Engine,
    ) {
    }

    fn deactivate(
        &mut self,
        #[allow(unused_variables)] controller: &dyn SceneController,
        #[allow(unused_variables)] engine: &mut Engine,
    ) {
    }

    /// Should return `true` if the `key` was handled in any way, otherwise you may mess up
    /// keyboard message routing. Return `false` if the `key` is unhandled.
    fn on_key_down(
        &mut self,
        #[allow(unused_variables)] key: KeyCode,
        #[allow(unused_variables)] editor_selection: &Selection,
        #[allow(unused_variables)] controller: &mut dyn SceneController,
        #[allow(unused_variables)] engine: &mut Engine,
    ) -> bool {
        false
    }

    /// Should return `true` if the `key` was handled in any way, otherwise you may mess up
    /// keyboard message routing. Return `false` if the `key` is unhandled.
    fn on_key_up(
        &mut self,
        #[allow(unused_variables)] key: KeyCode,
        #[allow(unused_variables)] controller: &mut dyn SceneController,
        #[allow(unused_variables)] engine: &mut Engine,
    ) -> bool {
        false
    }

    fn handle_ui_message(
        &mut self,
        #[allow(unused_variables)] message: &UiMessage,
        #[allow(unused_variables)] editor_selection: &Selection,
        #[allow(unused_variables)] controller: &mut dyn SceneController,
        #[allow(unused_variables)] engine: &mut Engine,
    ) {
    }

    fn on_drop(&mut self, _engine: &mut Engine) {}

    fn on_hot_key_pressed(
        &mut self,
        #[allow(unused_variables)] hotkey: &HotKey,
        #[allow(unused_variables)] controller: &mut dyn SceneController,
        #[allow(unused_variables)] engine: &mut Engine,
        #[allow(unused_variables)] settings: &Settings,
    ) -> bool {
        false
    }

    fn on_hot_key_released(
        &mut self,
        #[allow(unused_variables)] hotkey: &HotKey,
        #[allow(unused_variables)] controller: &mut dyn SceneController,
        #[allow(unused_variables)] engine: &mut Engine,
        #[allow(unused_variables)] settings: &Settings,
    ) -> bool {
        false
    }

    fn make_button(&mut self, ctx: &mut BuildContext, selected: bool) -> Handle<Button>;

    fn uuid(&self) -> Uuid;
}

pub fn make_interaction_mode_button(
    ctx: &mut BuildContext,
    image: &[u8],
    tooltip: &str,
    selected: bool,
) -> Handle<Button> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_tooltip(make_simple_tooltip(ctx, tooltip))
            .with_margin(Thickness {
                left: 1.0,
                top: 0.0,
                right: 1.0,
                bottom: 1.0,
            }),
    )
    .with_back(
        DecoratorBuilder::new(
            BorderBuilder::new(
                WidgetBuilder::new().with_foreground(ctx.style.property(Style::BRUSH_DARKER)),
            )
            .with_pad_by_corner_radius(false)
            .with_corner_radius(4.0f32.into())
            .with_stroke_thickness(Thickness::uniform(1.0).into()),
        )
        .with_normal_brush(ctx.style.property(Style::BRUSH_LIGHT))
        .with_hover_brush(ctx.style.property(Style::BRUSH_LIGHTER))
        .with_pressed_brush(ctx.style.property(Style::BRUSH_LIGHTEST))
        .with_selected_brush(ctx.style.property(Style::BRUSH_BRIGHT_BLUE))
        .with_selected(selected)
        .build(ctx),
    )
    .with_content(
        ImageBuilder::new(
            WidgetBuilder::new()
                .with_background(ctx.style.property(Style::BRUSH_TEXT))
                .with_margin(Thickness::uniform(4.0))
                .with_width(19.0)
                .with_height(19.0),
        )
        .with_opt_texture(load_image_internal(image).into())
        .build(ctx),
    )
    .build(ctx)
}
