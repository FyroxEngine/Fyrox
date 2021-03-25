//! Example 03. 3rd person walk simulator.
//!
//! Difficulty: Advanced.
//!
//! This example based on async example, because it requires to load decent amount of
//! resources which might be slow on some machines.
//!
//! In this example we'll create simple 3rd person game with character that can idle,
//! walk, or jump.
//!
//! Also this example demonstrates the power of animation blending machines. Animation
//! blending machines are used in all modern games to create complex animations from set
//! of simple ones.
//!
//! TODO: Improve explanations. Some places can be explained better.
//!
//! Known bugs: Sometimes character will jump, but jumping animations is not playing.
//!
//! Possible improvements:
//!  - Smart camera - camera which will not penetrate walls.
//!  - Separate animation machines for upper and lower body - upper machine might be
//!    for combat, lower - for locomotion.
//!  - Tons of them, this is simple example after all.

extern crate rg3d;

pub mod shared;

use crate::shared::{create_ui, fix_shadows_distance, Game, GameScene};
use rg3d::core::algebra::Vector2;
use rg3d::{
    event::{Event, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
    gui::message::{MessageDirection, ProgressBarMessage, TextMessage, WidgetMessage},
    renderer::QualitySettings,
    utils::translate_event,
};

fn main() {
    let (mut game, event_loop) = Game::new("Example 03 - 3rd person");

    // Create simple user interface that will show some useful info.
    let window = game.engine.get_window();
    let screen_size = window.inner_size().to_logical(window.scale_factor());
    let interface = create_ui(
        &mut game.engine.user_interface.build_ctx(),
        Vector2::new(screen_size.width, screen_size.height),
    );

    let clock = std::time::Instant::now();
    let fixed_timestep = 1.0 / 60.0;
    let mut elapsed_time = 0.0;

    // Finally run our event loop which will respond to OS and window events and update
    // engine state accordingly.
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::MainEventsCleared => {
                // This is main game loop - it has fixed time step which means that game
                // code will run at fixed speed even if renderer can't give you desired
                // 60 fps.
                let mut dt = clock.elapsed().as_secs_f32() - elapsed_time;
                while dt >= fixed_timestep {
                    dt -= fixed_timestep;
                    elapsed_time += fixed_timestep;

                    // ************************
                    // Put your game logic here.
                    // ************************

                    // Check each frame if our scene is created - here we just trying to lock context
                    // without blocking, it is important for main thread to be functional while other
                    // thread still loading data.
                    if let Ok(mut load_context) = game.load_context.as_ref().unwrap().try_lock() {
                        if let Some(load_result) = load_context.scene_data.take() {
                            // Add scene to engine - engine will take ownership over scene and will return
                            // you a handle to scene which can be used later on to borrow it and do some
                            // actions you need.
                            game.game_scene = Some(GameScene {
                                scene: game.engine.scenes.add(load_result.scene),
                                player: load_result.player,
                                reverb_effect: load_result.reverb_effect,
                            });

                            // Once scene is loaded, we should hide progress bar and text.
                            game.engine
                                .user_interface
                                .send_message(WidgetMessage::visibility(
                                    interface.progress_bar,
                                    MessageDirection::ToWidget,
                                    false,
                                ));
                            game.engine
                                .user_interface
                                .send_message(WidgetMessage::visibility(
                                    interface.progress_text,
                                    MessageDirection::ToWidget,
                                    false,
                                ));
                        }

                        // Report progress in UI.
                        game.engine
                            .user_interface
                            .send_message(ProgressBarMessage::progress(
                                interface.progress_bar,
                                MessageDirection::ToWidget,
                                load_context.progress,
                            ));
                        game.engine.user_interface.send_message(TextMessage::text(
                            interface.progress_text,
                            MessageDirection::ToWidget,
                            format!(
                                "Loading scene: {}%\n{}",
                                load_context.progress * 100.0,
                                load_context.message
                            ),
                        ));
                    }

                    // Update scene only if it is loaded.
                    if let Some(game_scene) = game.game_scene.as_mut() {
                        // Use stored scene handle to borrow a mutable reference of scene in
                        // engine.
                        let scene = &mut game.engine.scenes[game_scene.scene];
                        game_scene.player.update(scene, fixed_timestep);
                    }

                    let debug_text = format!(
                        "Example 03 - 3rd Person\n\
                        [W][S][A][D] - walk, [SPACE] - jump.\n\
                        Use [1][2][3][4] to select graphics quality.\n\
                        {}",
                        game.engine.renderer.get_statistics()
                    );
                    game.engine.user_interface.send_message(TextMessage::text(
                        interface.debug_text,
                        MessageDirection::ToWidget,
                        debug_text,
                    ));

                    // It is very important to "pump" messages from UI. Even if don't need to
                    // respond to such message, you should call this method, otherwise UI
                    // might behave very weird.
                    while let Some(_ui_event) = game.engine.user_interface.poll_message() {
                        // ************************
                        // Put your data model synchronization code here. It should
                        // take message and update data in your game according to
                        // changes in UI.
                        // ************************
                    }

                    game.engine.update(fixed_timestep);
                }

                // Rendering must be explicitly requested and handled after RedrawRequested event is received.
                game.engine.get_window().request_redraw();
            }
            Event::RedrawRequested(_) => {
                // Run renderer at max speed - it is not tied to game code.
                game.engine.render(fixed_timestep).unwrap();
            }
            Event::LoopDestroyed => {
                rg3d::core::profiler::print();
            }
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(size) => {
                        // It is very important to handle Resized event from window, because
                        // renderer knows nothing about window size - it must be notified
                        // directly when window size has changed.
                        game.engine.renderer.set_frame_size(size.into());

                        // Root UI node should be resized too, otherwise progress bar will stay
                        // in wrong position after resize.
                        let size = size.to_logical(game.engine.get_window().scale_factor());
                        game.engine
                            .user_interface
                            .send_message(WidgetMessage::width(
                                interface.root,
                                MessageDirection::ToWidget,
                                size.width,
                            ));
                        game.engine
                            .user_interface
                            .send_message(WidgetMessage::height(
                                interface.root,
                                MessageDirection::ToWidget,
                                size.height,
                            ));
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        if let Some(code) = input.virtual_keycode {
                            // Handle key input events via `WindowEvent`, not via `DeviceEvent` (#32)
                            if let Some(game_scene) = game.game_scene.as_mut() {
                                game_scene.player.handle_key_event(&input, fixed_timestep);
                            }

                            let settings = match code {
                                VirtualKeyCode::Key1 => Some(QualitySettings::ultra()),
                                VirtualKeyCode::Key2 => Some(QualitySettings::high()),
                                VirtualKeyCode::Key3 => Some(QualitySettings::medium()),
                                VirtualKeyCode::Key4 => Some(QualitySettings::low()),
                                _ => None,
                            };

                            if let Some(settings) = settings {
                                game.engine
                                    .renderer
                                    .set_quality_settings(&fix_shadows_distance(settings))
                                    .unwrap();
                            }
                        }
                    }
                    _ => (),
                }

                // It is very important to "feed" user interface (UI) with events coming
                // from main window, otherwise UI won't respond to mouse, keyboard, or any
                // other event.
                if let Some(os_event) = translate_event(&event) {
                    game.engine.user_interface.process_os_event(&os_event);
                }
            }
            Event::DeviceEvent { event, .. } => {
                if let Some(game_scene) = game.game_scene.as_mut() {
                    game_scene
                        .player
                        .handle_device_event(&event, fixed_timestep);
                }
            }
            _ => *control_flow = ControlFlow::Poll,
        }
    });
}
