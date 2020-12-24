//! Example 07. Sound.
//!
//! Difficulty: Advanced.
//!
//! This example based on 3rd_person example.
//!
//!

extern crate rg3d;

pub mod shared;

use crate::shared::{create_ui, fix_shadows_distance, Game, GameScene};
use rg3d::core::algebra::Vector2;
use rg3d::{
    animation::AnimationSignal,
    event::{Event, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
    gui::message::{MessageDirection, ProgressBarMessage, TextMessage, WidgetMessage},
    rand::Rng,
    renderer::QualitySettings,
    sound::{
        effects::EffectInput,
        source::{generic::GenericSourceBuilder, spatial::SpatialSourceBuilder, Status},
    },
    utils::translate_event,
};

const FOOTSTEP_SIGNAL: u64 = 1;

fn main() {
    let (mut game, event_loop) = Game::new("Example 07 - Sound");

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

    // We'll use four footstep sounds to randomize sound and make it more natural.
    let footstep_paths = [
        "examples/data/sounds/FootStep_shoe_stone_step1.wav",
        "examples/data/sounds/FootStep_shoe_stone_step2.wav",
        "examples/data/sounds/FootStep_shoe_stone_step3.wav",
        "examples/data/sounds/FootStep_shoe_stone_step4.wav",
    ];

    // Request foot step sound buffer from resources directory.
    let footstep_buffers = rg3d::futures::executor::block_on(rg3d::futures::future::join_all(
        footstep_paths.iter().map(|&path| {
            game.engine
                .resource_manager
                .request_sound_buffer(path, false)
        }),
    ))
    .into_iter()
    .map(|r| r.unwrap())
    .collect::<Vec<_>>();

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
                        if let Some(mut load_result) = load_context.scene_data.take() {
                            // Once scene is fully loaded, add some signals to walking animation.
                            load_result.scene
                                .animations
                                .get_mut(load_result.player.locomotion_machine.walk_animation)
                                // Add signals to the walk animation timeline, we'll use signals to emit foot step
                                // sounds.
                                .add_signal(AnimationSignal::new(FOOTSTEP_SIGNAL, 0.2))
                                .add_signal(AnimationSignal::new(FOOTSTEP_SIGNAL, 0.95));

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

                        let mut ctx = scene.sound_context.state();

                        while let Some(event) = scene.animations.get_mut(game_scene.player.locomotion_machine.walk_animation).pop_event() {
                            // We must play sound only if it was foot step signal and player was in walking state.
                            if event.signal_id != FOOTSTEP_SIGNAL
                                || game_scene.player.locomotion_machine.machine.active_state() != game_scene.player.locomotion_machine.walk_state {
                                continue;
                            }

                            // We'll emit sounds on player's feet.
                            let mut position = scene.graph[game_scene.player.pivot].global_position();
                            position.y -= 0.5;

                            let foot_step = footstep_buffers[rg3d::rand::thread_rng().gen_range(0.. footstep_buffers.len())].clone();

                            // Create new temporary foot step sound source.
                            let source = ctx
                                .add_source(
                                    SpatialSourceBuilder::new(
                                        GenericSourceBuilder::new(foot_step.into())
                                            // rg3d-sound provides built-in way to create temporary sounds that will die immediately
                                            // after first play. This is very useful for foot step sounds.
                                            .with_play_once(true)
                                            // Every sound source must be explicity set to Playing status, otherwise it will be stopped.
                                            .with_status(Status::Playing)
                                            .build()
                                            .unwrap()
                                    ).with_position(position).build_source());

                            // Once foot step sound source was created, it must be attached to reverb effect, otherwise no reverb
                            // will be added to the source.
                            ctx
                                .effect_mut(game_scene.reverb_effect)
                                .add_input(EffectInput::direct(source));
                        }

                        // Final, and very important step - sync sound listener with active camera.
                        let camera = &scene.graph[game_scene.player.camera];
                        let listener = ctx.listener_mut();
                        listener.set_position(camera.global_position());
                        listener.set_orientation_lh(camera.look_vector(), camera.up_vector());
                    }

                    let fps = game.engine.renderer.get_statistics().frames_per_second;
                    let debug_text = format!(
                        "Example 07 - Sound\n[W][S][A][D] - walk, [SPACE] - jump.\nFPS: {}\nUse [1][2][3][4] to select graphics quality.",
                        fps
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
