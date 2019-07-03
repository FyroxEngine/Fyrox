#![allow(dead_code)]

// Textures
extern crate image;
// Window
extern crate glutin;
// Fast string -> number conversion
extern crate lexical;

// Serialization
extern crate serde;
extern crate serde_json;

use std::path::*;

mod utils;
mod math;
mod scene;
mod renderer;
mod engine;
mod resource;
mod physics;

use utils::pool::*;
use math::vec2::*;
use math::vec3::*;
use math::quat::*;
use scene::node::*;
use scene::*;
use engine::*;
use crate::physics::{Body, StaticGeometry, StaticTriangle};
use std::time::{Instant, Duration};
use std::fs::File;

pub struct Controller {
    move_forward: bool,
    move_backward: bool,
    move_left: bool,
    move_right: bool,
    crouch: bool,
    jump: bool,
    run: bool,
    last_mouse_pos: Vec2,
}

impl Default for Controller {
    fn default() -> Controller {
        Controller {
            move_backward: false,
            move_forward: false,
            move_left: false,
            move_right: false,
            crouch: false,
            jump: false,
            run: false,
            last_mouse_pos: Vec2::new(),
        }
    }
}

pub struct Player {
    camera: Handle<Node>,
    pivot: Handle<Node>,
    body: Handle<Body>,
    controller: Controller,
    yaw: f32,
    dest_yaw: f32,
    pitch: f32,
    dest_pitch: f32,
    stand_body_radius: f32,
    run_speed_multiplier: f32,
    move_speed: f32,
}

impl Player {
    pub fn new(scene: &mut Scene) -> Player {
        let mut camera = Node::new(NodeKind::Camera(Camera::default()));
        camera.set_local_position(Vec3 { x: 0.0, y: 1.0, z: 0.0 });

        let mut pivot = Node::new(NodeKind::Base);
        pivot.set_local_position(Vec3 { x: -1.0, y: 0.0, z: 1.0 });

        let stand_body_radius = 0.5;
        let mut body = Body::new();
        body.set_radius(stand_body_radius);
        let body_handle = scene.get_physics_mut().add_body(body);
        pivot.set_body(body_handle.clone());

        let camera_handle = scene.add_node(camera);
        let pivot_handle = scene.add_node(pivot);
        scene.link_nodes(&camera_handle, &pivot_handle);

        Player {
            camera: camera_handle,
            pivot: pivot_handle,
            controller: Controller::default(),
            stand_body_radius,
            dest_pitch: 0.0,
            dest_yaw: 0.0,
            move_speed: 0.058,
            body: body_handle,
            run_speed_multiplier: 1.75,
            yaw: 0.0,
            pitch: 0.0,
        }
    }

    pub fn update(&mut self, scene: &mut Scene) {
        let mut look = Vec3::zero();
        let mut side = Vec3::zero();

        if let Some(pivot_node) = scene.borrow_node(&self.pivot) {
            look = pivot_node.get_look_vector();
            side = pivot_node.get_side_vector();
        }

        if let Some(body) = scene.get_physics_mut().borrow_body_mut(&self.body) {
            let mut velocity = Vec3::new();
            if self.controller.move_forward {
                velocity += look;
            }
            if self.controller.move_backward {
                velocity -= look;
            }
            if self.controller.move_left {
                velocity += side;
            }
            if self.controller.move_right {
                velocity -= side;
            }

            let speed_mult =
                if self.controller.run {
                    self.run_speed_multiplier
                } else {
                    1.0
                };

            let mut has_ground_contact = false;
            for contact in body.get_contacts() {
                if contact.normal.y >= 0.7 {
                    has_ground_contact = true;
                    break;
                }
            }

            if has_ground_contact {
                if let Some(normalized_velocity) = velocity.normalized() {
                    body.set_x_velocity(normalized_velocity.x * self.move_speed * speed_mult);
                    body.set_z_velocity(normalized_velocity.z * self.move_speed * speed_mult);
                }
            }

            if self.controller.jump {
                if has_ground_contact {
                    body.set_y_velocity(0.07);
                }
                self.controller.jump = false;
            }
        }

        self.yaw += (self.dest_yaw - self.yaw) * 0.2;
        self.pitch += (self.dest_pitch - self.pitch) * 0.2;

        if let Some(pivot_node) = scene.borrow_node_mut(&self.pivot) {
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
                    x: position.x as f32 - self.controller.last_mouse_pos.x,
                    y: position.y as f32 - self.controller.last_mouse_pos.y,
                };

                let sens: f32 = 0.3;

                self.dest_pitch += mouse_velocity.y * sens;
                self.dest_yaw -= mouse_velocity.x * sens;

                if self.dest_pitch > 90.0 {
                    self.dest_pitch = 90.0;
                } else if self.dest_pitch < -90.0 {
                    self.dest_pitch = -90.0;
                }

                self.controller.last_mouse_pos = Vec2 {
                    x: position.x as f32,
                    y: position.y as f32,
                };
            }

            WindowEvent::KeyboardInput { input, .. } => {
                match input.state {
                    ElementState::Pressed => {
                        if let Some(key) = input.virtual_keycode {
                            match key {
                                VirtualKeyCode::W => self.controller.move_forward = true,
                                VirtualKeyCode::S => self.controller.move_backward = true,
                                VirtualKeyCode::A => self.controller.move_left = true,
                                VirtualKeyCode::D => self.controller.move_right = true,
                                VirtualKeyCode::Space => self.controller.jump = true,
                                VirtualKeyCode::LShift => self.controller.run = true,
                                _ => ()
                            }
                        }
                    }
                    ElementState::Released => {
                        if let Some(key) = input.virtual_keycode {
                            match key {
                                VirtualKeyCode::W => self.controller.move_forward = false,
                                VirtualKeyCode::S => self.controller.move_backward = false,
                                VirtualKeyCode::A => self.controller.move_left = false,
                                VirtualKeyCode::D => self.controller.move_right = false,
                                VirtualKeyCode::LShift => self.controller.run = false,
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
}

impl Level {
    pub fn new(engine: &mut Engine) -> Level {
        // Create test scene
        let mut scene = Scene::new();

        // Load map
        match resource::fbx::load_to_scene(&mut scene, engine.get_resource_manager(), Path::new("data/models/map.fbx")) {
            Ok(root_handle) => {
                let mut static_geometry = StaticGeometry::new();

                if let Some(polygon_handle) = scene.find_node_by_name(&root_handle, "Polygon") {
                    if let Some(polygon) = scene.borrow_node(&polygon_handle) {
                        let global_transform = polygon.global_transform.clone();

                        if let NodeKind::Mesh(mesh) = polygon.borrow_kind() {
                            for surface in mesh.get_surfaces() {
                                let data_storage = scene.get_surface_data_storage();
                                let shared_data = data_storage.borrow(surface.get_data_handle()).unwrap();

                                let vertices = shared_data.get_vertices();
                                let indices = shared_data.get_indices();

                                let last = indices.len() - indices.len() % 3;
                                let mut i: usize = 0;
                                while i < last {
                                    let a = global_transform.transform_vector(vertices[indices[i] as usize].position);
                                    let b = global_transform.transform_vector(vertices[indices[i + 1] as usize].position);
                                    let c = global_transform.transform_vector(vertices[indices[i + 2] as usize].position);

                                    if let Some(triangle) = StaticTriangle::from_points(a, b, c) {
                                        static_geometry.add_triangle(triangle);
                                    } else {
                                        println!("degenerated triangle!");
                                    }

                                    i += 3;
                                }
                            }
                        }
                    }
                }

                scene.get_physics_mut().add_static_geometry(static_geometry);
            }
            Err(err_msg) => println!("{}", err_msg)
        }

        // Load test models
        match resource::fbx::load_to_scene(&mut scene, engine.get_resource_manager(), Path::new("data/models/ripper.fbx")) {
            Ok(node_handle) => {
                if let Some(node) = scene.borrow_node_mut(&node_handle) {
                    node.set_local_position(Vec3::make(-1.0, 0.0, -1.0));
                }
            }
            Err(err_msg) => println!("{}", err_msg)
        }

        let player = Player::new(&mut scene);

        Level {
            player,
            scene: engine.add_scene(scene),
        }
    }

    pub fn update(&mut self, engine: &mut Engine, dt: f64) {
        if let Some(scene) = engine.borrow_scene_mut(&self.scene) {
            self.player.update(scene);
        }
    }
}

pub struct Game {
    engine: Engine,
    level: Level,
}

fn duration_to_seconds_f64(duration: Duration) -> f64 {
    duration.as_secs() as f64 + duration.subsec_nanos() as f64 / 1_000_000_000.0
}

impl Game {
    pub fn new() -> Game {
        let mut engine = Engine::new();
        let level = Level::new(&mut engine);

        /*
        match File::create(Path::new("test.json")) {
            Err(reason) => println!("unable to create a save"),
            Ok(file) => {
                serde_json::to_writer_pretty(file, engine.get_state()).unwrap();
            }
        }*/

        Game {
            engine,
            level,
        }
    }

    pub fn update(&mut self, dt: f64) {
        self.level.update(&mut self.engine, dt);
    }

    pub fn run(&mut self) {
        let fixed_fps = 60.0;
        let fixed_timestep = 1.0 / fixed_fps;
        let clock = Instant::now();
        let mut game_time = 0.0;
        while self.engine.is_running() {
            let mut dt = duration_to_seconds_f64(clock.elapsed()) - game_time;
            while dt >= fixed_timestep {
                dt -= fixed_timestep;
                game_time += fixed_timestep;
                self.engine.poll_events();
                while let Some(event) = self.engine.pop_event() {
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
                self.update(fixed_timestep);
                self.engine.update(fixed_timestep);
            }
            // Render at max speed
            self.engine.render();
        }
    }
}

fn main() {
    Game::new().run();
}