use crate::{
    utils::pool::*,
    math::{
        vec2::*,
        vec3::*,
        quat::*,
    },
    scene::{
        node::*,
        *,
    },
    physics::Body,
    game::{
        weapon::{Weapon, WeaponKind},
        GameTime,
    },
    engine::State,
};

pub struct Controller {
    move_forward: bool,
    move_backward: bool,
    move_left: bool,
    move_right: bool,
    crouch: bool,
    jump: bool,
    run: bool,
    last_mouse_pos: Vec2,
    shoot: bool,
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
            shoot: false,
        }
    }
}

pub struct Player {
    camera: Handle<Node>,
    camera_pivot: Handle<Node>,
    pivot: Handle<Node>,
    body: Handle<Body>,
    weapon_pivot: Handle<Node>,
    controller: Controller,
    yaw: f32,
    dest_yaw: f32,
    pitch: f32,
    dest_pitch: f32,
    run_speed_multiplier: f32,
    stand_body_radius: f32,
    crouch_body_radius: f32,
    move_speed: f32,
    weapons: Vec<Weapon>,
    camera_offset: Vec3,
    camera_dest_offset: Vec3,
    current_weapon: usize,
}

impl Player {
    pub fn new(state: &mut State, scene: &mut Scene) -> Player {
        let camera_handle = scene.add_node(Node::new(NodeKind::Camera(Camera::default())));

        let mut camera_pivot = Node::new(NodeKind::Base);
        camera_pivot.set_local_position(Vec3 { x: 0.0, y: 1.0, z: 0.0 });
        let camera_pivot_handle = scene.add_node(camera_pivot);
        scene.link_nodes(&camera_handle, &camera_pivot_handle);

        let mut pivot = Node::new(NodeKind::Base);
        pivot.set_local_position(Vec3 { x: -1.0, y: 0.0, z: 1.0 });

        let stand_body_radius = 0.5;
        let mut body = Body::new();
        body.set_radius(stand_body_radius);
        let body_handle = scene.get_physics_mut().add_body(body);
        pivot.set_body(body_handle.clone());

        let pivot_handle = scene.add_node(pivot);
        scene.link_nodes(&camera_pivot_handle, &pivot_handle);

        let mut weapon_pivot = Node::new(NodeKind::Base);
        weapon_pivot.set_local_position(Vec3::make(-0.065, -0.052, 0.02));
        let weapon_pivot_handle = scene.add_node(weapon_pivot);
        scene.link_nodes(&weapon_pivot_handle, &camera_handle);

        let mut player = Player {
            camera: camera_handle,
            pivot: pivot_handle,
            camera_pivot: camera_pivot_handle,
            controller: Controller::default(),
            stand_body_radius,
            dest_pitch: 0.0,
            dest_yaw: 0.0,
            move_speed: 0.058,
            body: body_handle,
            run_speed_multiplier: 1.75,
            crouch_body_radius: 0.35,
            yaw: 0.0,
            pitch: 0.0,
            weapons: Vec::new(),
            camera_dest_offset: Vec3::new(),
            camera_offset: Vec3::new(),
            weapon_pivot: weapon_pivot_handle,
            current_weapon: 0,
        };

        let ak47 = Weapon::new(WeaponKind::Ak47, state, scene);
        player.add_weapon(scene, ak47);

        let m4 = Weapon::new(WeaponKind::M4, state, scene);
        player.add_weapon(scene, m4);

        player
    }

    pub fn add_weapon(&mut self, scene: &mut Scene, weapon: Weapon) {
        scene.link_nodes(&weapon.get_model(), &self.weapon_pivot);
        self.weapons.push(weapon);
    }

    pub fn next_weapon(&mut self) {
        if !self.weapons.is_empty() && self.current_weapon < self.weapons.len() - 1 {
            self.current_weapon += 1;
        }
    }

    pub fn prev_weapon(&mut self) {
        if self.current_weapon > 0 {
            self.current_weapon -= 1;
        }
    }

    pub fn has_ground_contact(&self, scene: &Scene) -> bool {
        if let Some(body) = scene.get_physics().borrow_body(&self.body) {
            for contact in body.get_contacts() {
                if contact.normal.y >= 0.7 {
                    return true;
                }
            }
        }
        false
    }

    pub fn update(&mut self, scene: &mut Scene, time: &GameTime) {
        let mut look = Vec3::zero();
        let mut side = Vec3::zero();

        if let Some(pivot_node) = scene.get_node(&self.pivot) {
            look = pivot_node.get_look_vector();
            side = pivot_node.get_side_vector();
        }

        let has_ground_contact = self.has_ground_contact(scene);

        let mut is_moving = false;
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

            if let Some(normalized_velocity) = velocity.normalized() {
                body.set_x_velocity(normalized_velocity.x * self.move_speed * speed_mult);
                body.set_z_velocity(normalized_velocity.z * self.move_speed * speed_mult);

                is_moving = true;
            }

            if self.controller.jump {
                if has_ground_contact {
                    body.set_y_velocity(0.07);
                }
                self.controller.jump = false;
            }
        }

        if has_ground_contact && is_moving {
            let k = (time.elapsed * 15.0) as f32;
            self.camera_dest_offset.x = 0.05 * (k * 0.5).cos();
            self.camera_dest_offset.y = 0.1 * k.sin();
        }

        self.camera_offset.x += (self.camera_dest_offset.x - self.camera_offset.x) * 0.1;
        self.camera_offset.y += (self.camera_dest_offset.y - self.camera_offset.y) * 0.1;
        self.camera_offset.z += (self.camera_dest_offset.z - self.camera_offset.z) * 0.1;

        if let Some(camera_node) = scene.get_node_mut(&self.camera) {
            camera_node.set_local_position(self.camera_offset);
        }

        for (i, weapon) in self.weapons.iter().enumerate() {
            if let Some(model) = scene.get_node_mut(&weapon.get_model()) {
                model.set_visibility(i == self.current_weapon);
            }
        }

        self.yaw += (self.dest_yaw - self.yaw) * 0.2;
        self.pitch += (self.dest_pitch - self.pitch) * 0.2;

        if let Some(pivot_node) = scene.get_node_mut(&self.pivot) {
            pivot_node.set_local_rotation(Quat::from_axis_angle(Vec3::up(), self.yaw.to_radians()));
        }

        if let Some(camera_pivot) = scene.get_node_mut(&self.camera_pivot) {
            camera_pivot.set_local_rotation(Quat::from_axis_angle(Vec3::right(), self.pitch.to_radians()));
        }

        if let Some(current_weapon) = self.weapons.get_mut(self.current_weapon) {
            if self.controller.shoot {
                current_weapon.shoot(time);
            }
            current_weapon.update(scene);
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

            WindowEvent::MouseInput { button, state, .. } => {
                match button {
                    MouseButton::Left => {
                        match state {
                            ElementState::Pressed => {
                                self.controller.shoot = true;
                            }
                            ElementState::Released => {
                                self.controller.shoot = false;
                            }
                        }
                    }
                    _ => ()
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                match delta {
                    MouseScrollDelta::LineDelta(_, y) => {
                        if *y < 0.0 {
                            self.prev_weapon();
                        } else if *y > 0.0 {
                            self.next_weapon();
                        }
                    }
                    _ => ()
                }
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
                                VirtualKeyCode::C => self.controller.crouch = true,
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
                                VirtualKeyCode::C => self.controller.crouch = false,
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

