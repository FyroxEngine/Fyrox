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

//! A set of useful scripts that can be used to in your game.

use crate::camera::FlyingCameraController;
use fyrox::script::constructor::ScriptConstructorContainer;

pub mod camera;

/// Registers every script from the crate in the given constructor container. Use it, if you want to register all
/// available scripts at once. Typical usage could be like this:
///
/// ```rust,no_run
/// # use fyrox::{
/// #     core::pool::Handle, core::visitor::prelude::*, core::reflect::prelude::*,
/// #     plugin::{Plugin, PluginContext, PluginRegistrationContext},
/// #     scene::Scene,
/// # };
/// #
/// # #[derive(Visit, Reflect, Debug)]
/// # struct Game;
/// #
/// # impl Plugin for Game {
///   // This is PluginConstructor::register method of your GameConstructor.
///   fn register(&self, context: PluginRegistrationContext) {
///       fyrox_scripts::register(&context.serialization_context.script_constructors)
///   }
/// # }
/// ```
pub fn register(container: &ScriptConstructorContainer) {
    container.add::<FlyingCameraController>("Fyrox Flying Camera Controller");
}
