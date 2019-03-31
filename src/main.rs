use std::ffi::{CStr, CString, c_void};
use std::ops;
use std::rc::Rc;
use std::rc::Weak;
use std::cell::RefCell;
use std::any::Any;

mod utils;

use utils::pool::*;

mod math;

use math::vec2::*;
use math::vec3::*;
use math::vec4::*;
use math::mat4::*;
use math::quat::*;

mod scene;

use scene::node::*;
use scene::*;

mod renderer;

mod engine;

use engine::*;

pub struct Controller {
    move_forward: bool,
    move_backward: bool,
    move_left: bool,
    move_right: bool,
}

pub struct Player {
    camera: Handle<Node>,
    pivot: Handle<Node>,
    controller: Controller,
    yaw: f32,
    pitch: f32,
    last_mouse_pos: Vec2,
}

impl Player {
    pub fn new(scene: &mut Scene) -> Player {
        let mut camera = Node::new(NodeKind::Camera(Camera::default()));
        camera.set_local_position(Vec3 { x: 0.0, y: 2.0, z: 0.0 });

        let mut pivot = Node::new(NodeKind::Base);
        pivot.set_local_position(Vec3 { x: 0.0, y: 0.0, z: 20.0 });

        let camera_handle = scene.add_node(camera);
        let pivot_handle = scene.add_node(pivot);
        scene.link_nodes(&camera_handle, &pivot_handle);

        Player {
            camera: camera_handle,
            pivot: pivot_handle,
            controller: Controller {
                move_backward: false,
                move_forward: false,
                move_left: false,
                move_right: false,
            },
            yaw: 0.0,
            pitch: 0.0,
            last_mouse_pos: Vec2::new()
        }
    }

    pub fn update(&mut self, scene: &mut Scene) {
        if let Some(pivot_node) = scene.borrow_node_mut(&self.pivot) {
            let look = pivot_node.get_look_vector();
            let side = pivot_node.get_side_vector();

            let mut velocity = Vec3::new();
            if self.controller.move_forward {
                velocity -= look;
            }
            if self.controller.move_backward {
                velocity += look;
            }
            if self.controller.move_left {
                velocity -= side;
            }
            if self.controller.move_right {
                velocity += side;
            }

            if let Ok(normalized_velocity) = velocity.normalized() {
                pivot_node.offset(normalized_velocity);
            }

            pivot_node.set_local_rotation(Quat::from_axis_angle(Vec3::up(), self.yaw.to_radians()));
        }

        if let Some(camera_node) = scene.borrow_node_mut(&self.camera) {
            camera_node.set_local_rotation(Quat::from_axis_angle(Vec3::right(), self.pitch.to_radians()));
        }
    }

    pub fn process_event(&mut self, event: &glutin::WindowEvent) -> bool {
        use glutin::*;

        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let mouse_velocity = Vec2 {
                    x: position.x as f32 - self.last_mouse_pos.x,
                    y: position.y as f32 - self.last_mouse_pos.y,
                };

                self.pitch -= mouse_velocity.y;
                self.yaw -= mouse_velocity.x;

                self.last_mouse_pos = Vec2 {
                    x: position.x as f32,
                    y: position.y as f32
                };
            },

            WindowEvent::KeyboardInput { input, .. } => {
                match input.state {
                    ElementState::Pressed => {
                        if let Some(key) = input.virtual_keycode {
                            match key {
                                VirtualKeyCode::W => self.controller.move_forward = true,
                                VirtualKeyCode::S => self.controller.move_backward = true,
                                VirtualKeyCode::A => self.controller.move_left = true,
                                VirtualKeyCode::D => self.controller.move_right = true,
                                _ => ()
                            }
                        }
                    },
                    ElementState::Released => {
                        if let Some(key) = input.virtual_keycode {
                            match key {
                                VirtualKeyCode::W => self.controller.move_forward = false,
                                VirtualKeyCode::S => self.controller.move_backward = false,
                                VirtualKeyCode::A => self.controller.move_left = false,
                                VirtualKeyCode::D => self.controller.move_right = false,
                                _ => ()
                            }
                        }
                    }
                }
            }
            _ => ()
        }
        false
    }
}

pub struct Level {
    scene: Handle<Scene>,
    player: Player,

    // Test stuff
    cubes: Vec<Handle<Node>>,
    angle: f32,
}

impl Level {
    pub fn new(engine: &mut Engine) -> Level {
        let mut cubes: Vec<Handle<Node>> = Vec::new();

        // Create test scene
        let mut scene = Scene::new();

        // Create floor
        {
            let mut floor_mesh = Mesh::default();
            floor_mesh.make_cube();
            let mut floor_node = Node::new(NodeKind::Mesh(floor_mesh));
            floor_node.set_local_scale(Vec3 { x: 10.0, y: 0.1, z: 10.0 });
            scene.add_node(floor_node);
        }

        // Create cubes
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    let mut cube_mesh = Mesh::default();
                    cube_mesh.make_cube();
                    let mut cube_node = Node::new(NodeKind::Mesh(cube_mesh));
                    let pos = Vec3 {
                        x: i as f32 * 2.0,
                        y: j as f32 * 2.0,
                        z: k as f32 * 2.0,
                    };
                    cube_node.set_local_position(pos);
                    cubes.push(scene.add_node(cube_node));
                }
            }
        }

        let player = Player::new(&mut scene);

        Level {
            player,
            cubes,
            angle: 0.0,
            scene: engine.add_scene(scene),
        }
    }

    pub fn update(&mut self, engine: &mut Engine) {
        self.angle += 0.1;

        let rotation = Quat::from_axis_angle(Vec3 { x: 0.0, y: 1.0, z: 0.0 }, self.angle);
        if let Some(scene) = engine.borrow_scene_mut(&self.scene) {
            for node_handle in self.cubes.iter() {
                if let Some(node) = scene.borrow_node_mut(node_handle) {
                    node.set_local_rotation(rotation);
                }
            }

            self.player.update(scene);
        }
    }
}

pub struct Game {
    engine: Engine,
    level: Level,
}

impl Game {
    pub fn new() -> Game {
        let mut engine = Engine::new();
        let level = Level::new(&mut engine);
        Game {
            engine,
            level,
        }
    }

    pub fn update(&mut self) {
        self.level.update(&mut self.engine);
    }

    pub fn run(&mut self) {
        while self.engine.is_running() {
            self.engine.poll_events();
            loop {
                let event = self.engine.pop_event();

                match event {
                    Some(event) => {
                        if let glutin::Event::WindowEvent { event, .. } = event {
                            self.level.player.process_event(&event);

                            match event {
                                glutin::WindowEvent::CloseRequested => self.engine.stop(),
                                glutin::WindowEvent::KeyboardInput {
                                    input: glutin::KeyboardInput {
                                        virtual_keycode: Some(glutin::VirtualKeyCode::Escape),
                                        ..
                                    },
                                    ..
                                } => self.engine.stop(),
                                _ => ()
                            }
                        }
                    }
                    None => break
                }
            }
            self.update();
            self.engine.update();
            self.engine.render();
        }
    }
}

fn main() {
    Game::new().run();
}