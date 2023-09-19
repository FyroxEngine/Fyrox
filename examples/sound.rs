//! Example 07. Sound.
//!
//! Difficulty: Advanced.
//!
//! This example based on 3rd_person example.
//!
//!

pub mod shared;

use crate::shared::{create_ui, fix_shadows_distance, Game, GameScene};
use fyrox::{
    animation::AnimationSignal,
    core::{
        algebra::Vector2,
        log::{Log, MessageKind},
        uuid::{uuid, Uuid},
    },
    engine::GraphicsContext,
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
    gui::{
        message::MessageDirection, progress_bar::ProgressBarMessage, text::TextMessage,
        widget::WidgetMessage,
    },
    rand::Rng,
    renderer::QualitySettings,
    scene::{
        animation::{absm::AnimationBlendingStateMachine, AnimationPlayer},
        base::BaseBuilder,
        sound::{SoundBuilder, Status},
        transform::TransformBuilder,
    },
    utils::translate_event,
};
use fyrox_sound::buffer::SoundBuffer;
use fyrox_sound::bus::AudioBusGraph;
use std::time::Instant;
use winit::keyboard::KeyCode;

const FOOTSTEP_SIGNAL: Uuid = uuid!("3e536261-9edf-4436-bba0-11173e61c8e9");

fn main() {
    let (mut game, event_loop) = Game::new("Example 07 - Sound");

    // Create simple user interface that will show some useful info.
    let interface = create_ui(
        &mut game.engine.user_interface.build_ctx(),
        Vector2::new(100.0, 100.0),
    );

    let mut previous = Instant::now();
    let fixed_timestep = 1.0 / 60.0;
    let mut lag = 0.0;

    // We'll use four footstep sounds to randomize sound and make it more natural.
    let footstep_paths = [
        "examples/data/sounds/FootStep_shoe_stone_step1.wav",
        "examples/data/sounds/FootStep_shoe_stone_step2.wav",
        "examples/data/sounds/FootStep_shoe_stone_step3.wav",
        "examples/data/sounds/FootStep_shoe_stone_step4.wav",
    ];

    // Request foot step sound buffer from resources directory.
    let footstep_buffers =
        fyrox::core::futures::executor::block_on(fyrox::core::futures::future::join_all(
            footstep_paths
                .iter()
                .map(|&path| game.engine.resource_manager.request::<SoundBuffer, _>(path)),
        ))
        .into_iter()
        .map(|r| r.unwrap())
        .collect::<Vec<_>>();

    // Finally run our event loop which will respond to OS and window events and update
    // engine state accordingly.
    event_loop
        .run(move |event, window_target, control_flow| {
            match event {
                Event::AboutToWait => {
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
                        if let Ok(mut load_context) = game.load_context.as_ref().unwrap().try_lock()
                        {
                            if let Some(mut load_result) = load_context.scene_data.take() {
                                let animation_player = load_result
                                    .scene
                                    .graph
                                    .find(load_result.player.animation_player, &mut |n| {
                                        n.query_component_ref::<AnimationPlayer>().is_some()
                                    })
                                    .unwrap()
                                    .0;

                                // Once scene is fully loaded, add some signals to walking animation.
                                (**load_result.scene.graph[animation_player]
                                    .query_component_mut::<AnimationPlayer>()
                                    .unwrap()
                                    .animations_mut())
                                .get_mut(load_result.player.locomotion_machine.walk_animation)
                                // Add signals to the walk animation timeline, we'll use signals to emit foot step
                                // sounds.
                                .add_signal(AnimationSignal {
                                    id: FOOTSTEP_SIGNAL,
                                    time: 0.2,
                                    name: "Footstep".to_string(),
                                    enabled: true,
                                })
                                .add_signal(AnimationSignal {
                                    id: FOOTSTEP_SIGNAL,
                                    time: 0.95,
                                    name: "Footstep".to_string(),
                                    enabled: true,
                                });

                                // Add scene to engine - engine will take ownership over scene and will return
                                // you a handle to scene which can be used later on to borrow it and do some
                                // actions you need.
                                game.game_scene = Some(GameScene {
                                    scene: game.engine.scenes.add(load_result.scene),
                                    player: load_result.player,
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

                            let active_state = scene.graph
                                [game_scene.player.locomotion_machine.machine]
                                .query_component_ref::<AnimationBlendingStateMachine>()
                                .unwrap()
                                .machine()
                                .layers()[0]
                                .active_state();

                            let animation_player = scene.graph[game_scene.player.animation_player]
                                .query_component_mut::<AnimationPlayer>()
                                .unwrap();

                            let mut events = Vec::new();
                            while let Some(event) = (**animation_player.animations_mut())
                                .get_mut(game_scene.player.locomotion_machine.walk_animation)
                                .pop_event()
                            {
                                events.push(event);
                            }

                            while let Some(event) = events.pop() {
                                // We must play sound only if it was foot step signal and player was in walking state.
                                if event.signal_id != FOOTSTEP_SIGNAL
                                    || active_state
                                        != game_scene.player.locomotion_machine.walk_state
                                {
                                    continue;
                                }

                                // We'll emit sounds on player's feet.
                                let mut position =
                                    scene.graph[game_scene.player.pivot].global_position();
                                position.y -= 0.5;

                                let foot_step = footstep_buffers[fyrox::rand::thread_rng()
                                    .gen_range(0..footstep_buffers.len())]
                                .clone();

                                // Create new temporary foot step sound source.
                                SoundBuilder::new(
                                    BaseBuilder::new().with_local_transform(
                                        TransformBuilder::new()
                                            .with_local_position(position)
                                            .build(),
                                    ),
                                )
                                // Specify the name of the effect to which the sound will attach to.
                                .with_audio_bus(AudioBusGraph::PRIMARY_BUS.to_string())
                                // Fyrox provides built-in way to create temporary sounds that will die immediately
                                // after first play. This is very useful for foot step sounds.
                                .with_play_once(true)
                                .with_buffer(Some(foot_step))
                                // Every sound source must be explicitly set to Playing status, otherwise it will be stopped.
                                .with_status(Status::Playing)
                                .build(&mut scene.graph);
                            }
                        }

                        if let GraphicsContext::Initialized(ref ctx) = game.engine.graphics_context
                        {
                            let fps = ctx.renderer.get_statistics().frames_per_second;
                            let debug_text = format!(
                                "Example 07 - Sound\n[W][S][A][D] - walk, [SPACE] - jump.\n\
                        FPS: {}\nUse [1][2][3][4] to select graphics quality.",
                                fps
                            );
                            game.engine.user_interface.send_message(TextMessage::text(
                                interface.debug_text,
                                MessageDirection::ToWidget,
                                debug_text,
                            ));
                        }

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

                        game.engine.update(
                            fixed_timestep,
                            control_flow,
                            &mut lag,
                            Default::default(),
                        );

                        lag -= fixed_timestep;
                    }

                    // Rendering must be explicitly requested and handled after RedrawRequested event is received.
                    if let GraphicsContext::Initialized(ref ctx) = game.engine.graphics_context {
                        ctx.window.request_redraw();
                    }
                }
                Event::Resumed => {
                    game.engine
                        .initialize_graphics_context(window_target)
                        .unwrap();
                }
                Event::Suspended => {
                    game.engine.destroy_graphics_context().unwrap();
                }
                Event::RedrawRequested(_) => {
                    // Run renderer at max speed - it is not tied to game code.
                    game.engine.render().unwrap();
                }
                Event::LoopExiting => {
                    println!("{:?}", fyrox::core::profiler::print());
                }
                Event::WindowEvent { event, .. } => {
                    match &event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(size) => {
                            // It is very important to handle Resized event from window, because
                            // renderer knows nothing about window size - it must be notified
                            // directly when window size has changed.
                            if let Err(e) = game.engine.set_frame_size((*size).into()) {
                                Log::writeln(
                                    MessageKind::Error,
                                    format!("Unable to set frame size: {:?}", e),
                                );
                            }

                            // Root UI node should be resized too, otherwise progress bar will stay
                            // in wrong position after resize.
                            if let GraphicsContext::Initialized(ref ctx) =
                                game.engine.graphics_context
                            {
                                let size = size.to_logical(ctx.window.scale_factor());
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
                        }
                        WindowEvent::KeyboardInput { event: input, .. } => {
                            // Handle key input events via `WindowEvent`, not via `DeviceEvent` (#32)
                            if let Some(game_scene) = game.game_scene.as_mut() {
                                game_scene.player.handle_key_event(input, fixed_timestep);
                            }

                            let settings = match input.physical_key {
                                KeyCode::Digit1 => Some(QualitySettings::ultra()),
                                KeyCode::Digit2 => Some(QualitySettings::high()),
                                KeyCode::Digit3 => Some(QualitySettings::medium()),
                                KeyCode::Digit4 => Some(QualitySettings::low()),
                                _ => None,
                            };

                            if let Some(settings) = settings {
                                if let GraphicsContext::Initialized(ref mut ctx) =
                                    game.engine.graphics_context
                                {
                                    ctx.renderer
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
        })
        .unwrap();
}
