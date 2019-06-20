#![allow(dead_code)]

extern crate image;
extern crate glutin;
extern crate lexical;

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

pub struct Controller {
    move_forward: bool,
    move_backward: bool,
    move_left: bool,
    move_right: bool,
}

pub struct Player {
    camera: Handle<Node>,
    pivot: Handle<Node>,
    body: Handle<Body>,
    controller: Controller,
    yaw: f32,
    pitch: f32,
    last_mouse_pos: Vec2,
    stand_body_radius: f32,
    move_speed: f32,
}

impl Player {
    pub fn new(scene: &mut Scene) -> Player {
        let mut camera = Node::new(NodeKind::Camera(Camera::default()));
        camera.set_local_position(Vec3 { x: 0.0, y: 2.0, z: 0.0 });

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
            controller: Controller {
                move_backward: false,
                move_forward: false,
                move_left: false,
                move_right: false,
            },
            stand_body_radius,
            move_speed: 0.028,
            body: body_handle,
            yaw: 0.0,
            pitch: 0.0,
            last_mouse_pos: Vec2::new(),
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

            if let Some(normalized_velocity) = velocity.normalized() {
                body.move_by(normalized_velocity.scale(self.move_speed));
            }
        }

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
                    x: position.x as f32 - self.last_mouse_pos.x,
                    y: position.y as f32 - self.last_mouse_pos.y,
                };

                let sens: f32 = 0.3;

                self.pitch += mouse_velocity.y * sens;
                self.yaw -= mouse_velocity.x * sens;

                if self.pitch > 90.0 {
                    self.pitch = 90.0;
                } else if self.pitch < -90.0 {
                    self.pitch = -90.0;
                }

                self.last_mouse_pos = Vec2 {
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

        // Load map
        match resource::fbx::load_to_scene(&mut scene, engine.get_resource_manager(), Path::new("data/models/map.fbx")) {
            Ok(root_handle) => {
                let mut static_geometry = StaticGeometry::new();

                if let Some(polygon_handle) = scene.find_node_by_name(&root_handle, "Polygon") {
                    if let Some(polygon) = scene.borrow_node(&polygon_handle) {

                        let global_transform = polygon.global_transform.clone();

                        if let NodeKind::Mesh(mesh) = polygon.borrow_kind() {
                            for surface in mesh.get_surfaces() {
                                let shared_data = surface.data.borrow();

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
            self.update();
            self.engine.update();
            self.engine.render();
        }
    }
}

fn main() {
    Game::new().run();
}