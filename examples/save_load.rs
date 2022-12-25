//! Example 06. Save/load.
//!
//! Difficulty: Advanced.
//!
//! This example based on 3rd_person example, it uses lots of code from shared mod.
//!
//! Fyrox has powerful built-in serialization/deserialization which is used for various
//! purposes, one of them is to create or load save files in your game. It very easy
//! to use, all you need to do is to implement Visit trait on your game structures and
//! then create new instance of visitor and call your_struct.visit(...) on it. Check code
//! below for more info.
//!
//! # Important
//!
//! You should carefully read documentation of fyrox::core::Visitor to understand basic ideas
//! of how it works, otherwise Visit trait implementation might be confusing.

pub mod shared;

use crate::shared::{create_ui, fix_shadows_distance, Game, GameScene, LocomotionMachine, Player};
use fyrox::scene::SceneLoader;
use fyrox::{
    core::{
        algebra::Vector2,
        visitor::{Visit, VisitResult, Visitor},
    },
    event::{Event, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
    gui::{
        message::MessageDirection, progress_bar::ProgressBarMessage, text::TextMessage,
        widget::WidgetMessage,
    },
    renderer::QualitySettings,
    utils::{
        log::{Log, MessageKind},
        translate_event,
    },
};
use fyrox_core::futures::executor::block_on;
use std::path::Path;
use std::time::Instant;

// Start implementing Visit trait for simple types which are used by more complex.
// At first implement trait for LocomotionMachine.
impl Visit for LocomotionMachine {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        // Almost every implementation of visit should start with this line. It creates
        // new node in tree structure and makes it current so every later calls of visit
        // will write data into that node, of course inner calls can call enter_region -
        // visitor can manage trees of any depth.
        let mut region = visitor.enter_region(name)?;

        // Just call visit on every field, checking the result of operation.
        // For backwards compatibility you can ignore result.
        // There is a small pitfall that can be in your way - if you have Option, Rc, Arc, Mutex,
        // or some other generic type, inner type must implement at least Default trait plus
        // some types (Arc, Mutex) adds Send, Sync - if compiler tells you that .visit method is
        // not found then it is probably you missed some of required trait bounds.
        self.jump_animation.visit("JumpAnimation", &mut region)?;
        self.walk_animation.visit("WalkAnimation", &mut region)?;
        self.walk_state.visit("WalkState", &mut region)?;
        // Machine is an internal Fyrox type, however it has implementation of Visit and
        // can be serialized in one call.
        self.machine.visit("Machine", &mut region)?;

        Ok(())
    }
}

// Continue implementing Visit trait for Rest of game structures.
impl Visit for Player {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        self.model.visit("Model", &mut region)?;
        self.body.visit("Body", &mut region)?;
        self.camera_pivot.visit("CameraPivot", &mut region)?;
        self.camera_hinge.visit("CameraHinge", &mut region)?;
        self.camera.visit("Camera", &mut region)?;
        self.locomotion_machine
            .visit("LocomotionMachine", &mut region)?;
        self.model_yaw.visit("ModelYaw", &mut region)?;
        self.pivot.visit("Pivot", &mut region)?;
        // self.input_controller isn't visited because we don't care about its state -
        // it will be synced with keyboard state anyway.

        Ok(())
    }
}

impl Visit for GameScene {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        self.scene.visit("Scene", &mut region)?;
        self.player.visit("Player", &mut region)?;

        Ok(())
    }
}

// For simplicity we'll be save (or load) game from hardcoded path.
const SAVE_FILE: &str = "save.bin";

fn save(game: &mut Game) {
    if let Some(game_scene) = game.game_scene.as_mut() {
        let mut visitor = Visitor::new();

        // Serialize game scene first.
        game.engine.scenes[game_scene.scene]
            .save("Scene", &mut visitor)
            .unwrap();
        // Then serialize the game scene.
        game_scene.visit("GameScene", &mut visitor).unwrap();

        // And call save method to write everything to disk.
        visitor.save_binary(Path::new(SAVE_FILE)).unwrap();
    }
}

async fn load(game: &mut Game) {
    // Try to load saved game.
    if Path::new(SAVE_FILE).exists() {
        // Remove current scene first.
        if let Some(game_scene) = game.game_scene.take() {
            game.engine.scenes.remove(game_scene.scene);
        }

        let mut visitor = Visitor::load_binary(SAVE_FILE).await.unwrap();

        let scene = SceneLoader::load(
            "Scene",
            game.engine.serialization_context.clone(),
            &mut visitor,
        )
        .unwrap()
        .finish(game.engine.resource_manager.clone())
        .await;

        let mut game_scene = GameScene::default();
        game_scene.visit("GameScene", &mut visitor).unwrap();

        game_scene.scene = game.engine.scenes.add(scene);
        game.game_scene = Some(game_scene);
    }
}

fn main() {
    let (mut game, event_loop) = Game::new("Example 06 - Save/load");

    // Create simple user interface that will show some useful info.
    let window = game.engine.get_window();
    let screen_size = window.inner_size().to_logical(window.scale_factor());
    let interface = create_ui(
        &mut game.engine.user_interface.build_ctx(),
        Vector2::new(screen_size.width, screen_size.height),
    );

    let mut previous = Instant::now();
    let fixed_timestep = 1.0 / 60.0;
    let mut lag = 0.0;

    // Finally run our event loop which will respond to OS and window events and update
    // engine state accordingly.
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::MainEventsCleared => {
                // This is main game loop - it has fixed time step which means that game
                // code will run at fixed speed even if renderer can't give you desired
                // 60 fps.
                let elapsed = previous.elapsed();
                previous = Instant::now();
                lag += elapsed.as_secs_f32();
                while lag >= fixed_timestep {

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
                                reverb_effect: load_result.reverb_effect
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

                    let fps = game.engine.renderer.get_statistics().frames_per_second;
                    let debug_text = format!(
                        "Example 06 - Save/load\n[W][S][A][D] - walk, [SPACE] - jump.\nFPS: {}\nUse [1][2][3][4] to select graphics quality.\nUse F5 to save game, F9 to load.",
                        fps
                    );
                    game. engine.user_interface.send_message(TextMessage::text(
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

                    game.engine.update(fixed_timestep, control_flow, &mut lag, Default::default());

                    lag -= fixed_timestep;
                }

                // Rendering must be explicitly requested and handled after RedrawRequested event is received.
                game.engine.get_window().request_redraw();
            }
            Event::RedrawRequested(_) => {
                // Run renderer at max speed - it is not tied to game code.
                game.engine.render().unwrap();
            }
            Event::LoopDestroyed => {
                println!("{:?}", fyrox::core::profiler::print());
            }
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(size) => {
                        // It is very important to handle Resized event from window, because
                        // renderer knows nothing about window size - it must be notified
                        // directly when window size has changed.
                        if let Err(e) = game.engine.set_frame_size(size.into()) {
                            Log::writeln(MessageKind::Error, format!("Unable to set frame size: {:?}", e));
                        }

                        // Root UI node should be resized too, otherwise progress bar will stay
                        // in wrong position after resize.
                        let size = size.to_logical(game.engine.get_window().scale_factor());
                        game.engine.user_interface.send_message(WidgetMessage::width(
                            interface.root,
                            MessageDirection::ToWidget,
                            size.width,
                        ));
                        game.engine.user_interface.send_message(WidgetMessage::height(
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
                                _ => None
                            };

                            if let Some(settings) = settings {
                                game.engine
                                    .renderer
                                    .set_quality_settings(&fix_shadows_distance(settings))
                                    .unwrap();
                            }

                            // Prevent saving/loading while example is starting.
                            if game.game_scene.is_some() {
                                // Save/load bound to classic F5 and F9 keys.
                                match code {
                                    VirtualKeyCode::F5 => save(&mut game),
                                    VirtualKeyCode::F9 =>
                                        block_on(load(&mut game))
                                    ,
                                    _ => ()
                                };
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
                    game_scene.player.handle_device_event(&event, fixed_timestep);
                }
            }
            _ => *control_flow = ControlFlow::Poll,
        }
    });
}
