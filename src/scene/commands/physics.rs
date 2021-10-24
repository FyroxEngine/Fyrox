use crate::{
    command::Command,
    physics::{Collider, Joint, RigidBody},
    scene::commands::SceneContext,
    Physics,
};
use rg3d::scene::graph::Graph;
use rg3d::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        pool::{ErasedHandle, Handle, Ticket},
    },
    physics3d::desc::{ColliderShapeDesc, JointParamsDesc, RigidBodyTypeDesc},
    scene::node::Node,
};

#[derive(Debug)]
pub struct AddJointCommand {
    ticket: Option<Ticket<Joint>>,
    handle: Handle<Joint>,
    joint: Option<Joint>,
}

impl AddJointCommand {
    pub fn new(node: Joint) -> Self {
        Self {
            ticket: None,
            handle: Default::default(),
            joint: Some(node),
        }
    }
}

impl Command for AddJointCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Add Joint".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        match self.ticket.take() {
            None => {
                self.handle = context
                    .editor_scene
                    .physics
                    .joints
                    .spawn(self.joint.take().unwrap());
            }
            Some(ticket) => {
                let handle = context
                    .editor_scene
                    .physics
                    .joints
                    .put_back(ticket, self.joint.take().unwrap());
                assert_eq!(handle, self.handle);
            }
        }
    }

    fn revert(&mut self, context: &mut SceneContext) {
        let (ticket, node) = context
            .editor_scene
            .physics
            .joints
            .take_reserve(self.handle);
        self.ticket = Some(ticket);
        self.joint = Some(node);
    }

    fn finalize(&mut self, context: &mut SceneContext) {
        if let Some(ticket) = self.ticket.take() {
            context.editor_scene.physics.joints.forget_ticket(ticket)
        }
    }
}

#[derive(Debug)]
pub struct DeleteJointCommand {
    handle: Handle<Joint>,
    ticket: Option<Ticket<Joint>>,
    node: Option<Joint>,
}

impl DeleteJointCommand {
    pub fn new(handle: Handle<Joint>) -> Self {
        Self {
            handle,
            ticket: None,
            node: None,
        }
    }
}

impl Command for DeleteJointCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Delete Joint".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        let (ticket, node) = context
            .editor_scene
            .physics
            .joints
            .take_reserve(self.handle);
        self.node = Some(node);
        self.ticket = Some(ticket);
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.handle = context
            .editor_scene
            .physics
            .joints
            .put_back(self.ticket.take().unwrap(), self.node.take().unwrap());
    }

    fn finalize(&mut self, context: &mut SceneContext) {
        if let Some(ticket) = self.ticket.take() {
            context.editor_scene.physics.joints.forget_ticket(ticket)
        }
    }
}

#[derive(Debug)]
pub struct LinkBodyCommand {
    pub node: Handle<Node>,
    pub handle: Handle<RigidBody>,
}

impl Command for LinkBodyCommand {
    fn name(&mut self, _: &SceneContext) -> String {
        "Link Body Command".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        assert!(context
            .editor_scene
            .physics
            .binder
            .insert(self.node, self.handle)
            .is_none());
    }

    fn revert(&mut self, context: &mut SceneContext) {
        assert!(context
            .editor_scene
            .physics
            .binder
            .remove_by_key(&self.node)
            .is_some())
    }
}

#[derive(Debug)]
pub struct UnlinkBodyCommand {
    pub node: Handle<Node>,
    pub handle: Handle<RigidBody>,
}

impl Command for UnlinkBodyCommand {
    fn name(&mut self, _: &SceneContext) -> String {
        "Unlink Body Command".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        assert!(context
            .editor_scene
            .physics
            .binder
            .remove_by_key(&self.node)
            .is_some())
    }

    fn revert(&mut self, context: &mut SceneContext) {
        assert!(context
            .editor_scene
            .physics
            .binder
            .insert(self.node, self.handle)
            .is_none());
    }
}

#[derive(Debug)]
pub struct SetBodyCommand {
    node: Handle<Node>,
    ticket: Option<Ticket<RigidBody>>,
    handle: Handle<RigidBody>,
    body: Option<RigidBody>,
}

impl SetBodyCommand {
    pub fn new(node: Handle<Node>, body: RigidBody) -> Self {
        Self {
            node,
            ticket: None,
            handle: Default::default(),
            body: Some(body),
        }
    }
}

impl Command for SetBodyCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Set Node Body".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        match self.ticket.take() {
            None => {
                self.handle = context
                    .editor_scene
                    .physics
                    .bodies
                    .spawn(self.body.take().unwrap());
            }
            Some(ticket) => {
                context
                    .editor_scene
                    .physics
                    .bodies
                    .put_back(ticket, self.body.take().unwrap());
            }
        }
        context
            .editor_scene
            .physics
            .binder
            .insert(self.node, self.handle);
    }

    fn revert(&mut self, context: &mut SceneContext) {
        let (ticket, node) = context
            .editor_scene
            .physics
            .bodies
            .take_reserve(self.handle);
        self.ticket = Some(ticket);
        self.body = Some(node);
        context
            .editor_scene
            .physics
            .binder
            .remove_by_key(&self.node);
    }

    fn finalize(&mut self, context: &mut SceneContext) {
        if let Some(ticket) = self.ticket.take() {
            context.editor_scene.physics.bodies.forget_ticket(ticket);
            context
                .editor_scene
                .physics
                .binder
                .remove_by_key(&self.node);
        }
    }
}

#[derive(Debug)]
pub struct CreateRigidBodyCommand {
    ticket: Option<Ticket<RigidBody>>,
    handle: Handle<RigidBody>,
    body: Option<RigidBody>,
}

impl CreateRigidBodyCommand {
    pub fn new(body: RigidBody) -> Self {
        Self {
            ticket: None,
            handle: Default::default(),
            body: Some(body),
        }
    }
}

impl Command for CreateRigidBodyCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Create Rigid Body".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        match self.ticket.take() {
            None => {
                self.handle = context
                    .editor_scene
                    .physics
                    .bodies
                    .spawn(self.body.take().unwrap());
            }
            Some(ticket) => {
                context
                    .editor_scene
                    .physics
                    .bodies
                    .put_back(ticket, self.body.take().unwrap());
            }
        }
    }

    fn revert(&mut self, context: &mut SceneContext) {
        let (ticket, node) = context
            .editor_scene
            .physics
            .bodies
            .take_reserve(self.handle);
        self.ticket = Some(ticket);
        self.body = Some(node);
    }

    fn finalize(&mut self, context: &mut SceneContext) {
        if let Some(ticket) = self.ticket.take() {
            context.editor_scene.physics.bodies.forget_ticket(ticket);
        }
    }
}

#[derive(Debug)]
pub struct AddColliderCommand {
    body: Handle<RigidBody>,
    ticket: Option<Ticket<Collider>>,
    handle: Handle<Collider>,
    collider: Option<Collider>,
}

impl AddColliderCommand {
    pub fn new(body: Handle<RigidBody>, collider: Collider) -> Self {
        Self {
            body,
            ticket: None,
            handle: Default::default(),
            collider: Some(collider),
        }
    }
}

impl Command for AddColliderCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Add Collider".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        match self.ticket.take() {
            None => {
                self.handle = context
                    .editor_scene
                    .physics
                    .colliders
                    .spawn(self.collider.take().unwrap());
            }
            Some(ticket) => {
                context
                    .editor_scene
                    .physics
                    .colliders
                    .put_back(ticket, self.collider.take().unwrap());
            }
        }
        context.editor_scene.physics.colliders[self.handle].parent = self.body.into();
        context.editor_scene.physics.bodies[self.body]
            .colliders
            .push(self.handle.into());
    }

    fn revert(&mut self, context: &mut SceneContext) {
        let (ticket, mut collider) = context
            .editor_scene
            .physics
            .colliders
            .take_reserve(self.handle);
        collider.parent = Default::default();
        self.ticket = Some(ticket);
        self.collider = Some(collider);

        let body = &mut context.editor_scene.physics.bodies[self.body];
        body.colliders.remove(
            body.colliders
                .iter()
                .position(|&c| c == ErasedHandle::from(self.handle))
                .unwrap(),
        );
    }

    fn finalize(&mut self, context: &mut SceneContext) {
        if let Some(ticket) = self.ticket.take() {
            context.editor_scene.physics.colliders.forget_ticket(ticket);
        }
    }
}

#[derive(Debug)]
pub struct DeleteBodyCommand {
    handle: Handle<RigidBody>,
    ticket: Option<Ticket<RigidBody>>,
    body: Option<RigidBody>,
    node: Handle<Node>,
}

impl DeleteBodyCommand {
    pub fn new(handle: Handle<RigidBody>) -> Self {
        Self {
            handle,
            ticket: None,
            body: None,
            node: Handle::NONE,
        }
    }
}

impl Command for DeleteBodyCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Delete Body".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        let (ticket, node) = context
            .editor_scene
            .physics
            .bodies
            .take_reserve(self.handle);
        self.body = Some(node);
        self.ticket = Some(ticket);
        self.node = context.editor_scene.physics.unbind_by_body(self.handle);
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.handle = context
            .editor_scene
            .physics
            .bodies
            .put_back(self.ticket.take().unwrap(), self.body.take().unwrap());
        if self.node.is_some() {
            context
                .editor_scene
                .physics
                .binder
                .insert(self.node, self.handle);
        }
    }

    fn finalize(&mut self, context: &mut SceneContext) {
        if let Some(ticket) = self.ticket.take() {
            context.editor_scene.physics.bodies.forget_ticket(ticket)
        }
    }
}

#[derive(Debug)]
pub struct DeleteColliderCommand {
    handle: Handle<Collider>,
    ticket: Option<Ticket<Collider>>,
    collider: Option<Collider>,
    body: Handle<RigidBody>,
}

impl DeleteColliderCommand {
    pub fn new(handle: Handle<Collider>) -> Self {
        Self {
            handle,
            ticket: None,
            collider: None,
            body: Handle::NONE,
        }
    }
}

impl Command for DeleteColliderCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Delete Collider".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        let (ticket, collider) = context
            .editor_scene
            .physics
            .colliders
            .take_reserve(self.handle);
        self.body = collider.parent.into();
        self.collider = Some(collider);
        self.ticket = Some(ticket);

        let body = &mut context.editor_scene.physics.bodies[self.body];
        body.colliders.remove(
            body.colliders
                .iter()
                .position(|&c| c == ErasedHandle::from(self.handle))
                .unwrap(),
        );
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.handle = context
            .editor_scene
            .physics
            .colliders
            .put_back(self.ticket.take().unwrap(), self.collider.take().unwrap());

        let body = &mut context.editor_scene.physics.bodies[self.body];
        body.colliders.push(self.handle.into());
    }

    fn finalize(&mut self, context: &mut SceneContext) {
        if let Some(ticket) = self.ticket.take() {
            context.editor_scene.physics.colliders.forget_ticket(ticket)
        }
    }
}

macro_rules! define_physics_command {
    ($name:ident($human_readable_name:expr, $handle_type:ty, $value_type:ty) where fn swap($self:ident, $physics:ident) $apply_method:block ) => {
        #[derive(Debug)]
        pub struct $name {
            handle: Handle<$handle_type>,
            value: $value_type,
        }

        impl $name {
            pub fn new(handle: Handle<$handle_type>, value: $value_type) -> Self {
                Self { handle, value }
            }

            fn swap(&mut $self, $physics: &mut Physics) {
                 $apply_method
            }
        }

        impl Command for $name {


            fn name(&mut self, _context: &SceneContext) -> String {
                $human_readable_name.to_owned()
            }

            fn execute(&mut self, context: &mut SceneContext) {
                self.swap(&mut context.editor_scene.physics);
            }

            fn revert(&mut self, context: &mut SceneContext) {
                self.swap(&mut context.editor_scene.physics);
            }
        }
    };
}

macro_rules! define_body_command {
    ($name:ident($human_readable_name:expr, $value_type:ty) where fn swap($self:ident, $physics: ident, $body:ident) $apply_method:block ) => {
        define_physics_command!($name($human_readable_name, RigidBody, $value_type) where fn swap($self, $physics) {
            let $body = &mut $physics.bodies[$self.handle];
            $apply_method
        });
    };
}

macro_rules! define_collider_command {
    ($name:ident($human_readable_name:expr, $value_type:ty) where fn swap($self:ident, $physics:ident, $collider:ident) $apply_method:block ) => {
        define_physics_command!($name($human_readable_name, Collider, $value_type) where fn swap($self, $physics) {
            let $collider = &mut $physics.colliders[$self.handle];
            $apply_method
        });
    };
}

macro_rules! define_joint_command {
    ($name:ident($human_readable_name:expr, $value_type:ty) where fn swap($self:ident, $physics:ident, $joint:ident) $apply_method:block ) => {
        define_physics_command!($name($human_readable_name, Joint, $value_type) where fn swap($self, $physics) {
            let $joint = &mut $physics.joints[$self.handle];
            $apply_method
        });
    };
}

macro_rules! define_joint_variant_command {
    ($name:ident($human_readable_name:expr, $value_type:ty) where fn swap($self:ident, $physics:ident, $variant:ident, $var:ident) $apply_method:block ) => {
        define_physics_command!($name($human_readable_name, Joint, $value_type) where fn swap($self, $physics) {
            let joint = &mut $physics.joints[$self.handle];
            if let JointParamsDesc::$variant($var) = &mut joint.params {
                $apply_method
            } else {
                unreachable!();
            }
        });
    };
}

macro_rules! define_collider_variant_command {
    ($name:ident($human_readable_name:expr, $value_type:ty) where fn swap($self:ident, $physics:ident, $variant:ident, $var:ident) $apply_method:block ) => {
        define_physics_command!($name($human_readable_name, Collider, $value_type) where fn swap($self, $physics) {
            let collider = &mut $physics.colliders[$self.handle];
            if let ColliderShapeDesc::$variant($var) = &mut collider.shape {
                $apply_method
            } else {
                unreachable!();
            }
        });
    };
}

define_body_command!(SetBodyMassCommand("Set Body Mass", f32) where fn swap(self, physics, body) {
    std::mem::swap(&mut body.mass, &mut self.value);
});

define_body_command!(SetBodyPositionCommand("Set Body Position", Vector3<f32>) where fn swap(self, physics, body) {
    std::mem::swap(&mut body.position, &mut self.value);
});

define_body_command!(SetBodyRotationCommand("Set Body Rotation", UnitQuaternion<f32>) where fn swap(self, physics, body) {
    std::mem::swap(&mut body.rotation, &mut self.value);
});

define_body_command!(SetBodyLinVelCommand("Set Body Linear Velocity", Vector3<f32>) where fn swap(self, physics, body) {
    std::mem::swap(&mut body.lin_vel, &mut self.value);
});

define_body_command!(SetBodyAngVelCommand("Set Body Angular Velocity", Vector3<f32>) where fn swap(self, physics, body) {
    std::mem::swap(&mut body.ang_vel, &mut self.value);
});

define_body_command!(SetBodyStatusCommand("Set Body Status", RigidBodyTypeDesc) where fn swap(self, physics, body) {
    std::mem::swap(&mut body.status, &mut self.value);
});

define_body_command!(SetBodyXRotationLockedCommand("Set Body X Rotation Locked", bool) where fn swap(self, physics, body) {
    std::mem::swap(&mut body.x_rotation_locked, &mut self.value);
});

define_body_command!(SetBodyYRotationLockedCommand("Set Body Y Rotation Locked", bool) where fn swap(self, physics, body) {
    std::mem::swap(&mut body.y_rotation_locked, &mut self.value);
});

define_body_command!(SetBodyZRotationLockedCommand("Set Body Z Rotation Locked", bool) where fn swap(self, physics, body) {
    std::mem::swap(&mut body.z_rotation_locked, &mut self.value);
});

define_body_command!(SetBodyTranslationLockedCommand("Set Body Translation Locked", bool) where fn swap(self, physics, body) {
    std::mem::swap(&mut body.translation_locked, &mut self.value);
});

define_collider_command!(SetColliderFrictionCommand("Set Collider Friction", f32) where fn swap(self, physics, collider) {
    std::mem::swap(&mut collider.friction, &mut self.value);
});

define_collider_command!(SetColliderRestitutionCommand("Set Collider Restitution", f32) where fn swap(self, physics, collider) {
    std::mem::swap(&mut collider.restitution, &mut self.value);
});

define_collider_command!(SetColliderPositionCommand("Set Collider Position", Vector3<f32>) where fn swap(self, physics, collider) {
    std::mem::swap(&mut collider.translation, &mut self.value);
});

define_collider_command!(SetColliderRotationCommand("Set Collider Rotation", UnitQuaternion<f32>) where fn swap(self, physics, collider) {
    std::mem::swap(&mut collider.rotation, &mut self.value);
});

define_collider_command!(SetColliderIsSensorCommand("Set Collider Is Sensor", bool) where fn swap(self, physics, collider) {
    std::mem::swap(&mut collider.is_sensor, &mut self.value);
});

define_collider_command!(SetColliderDensityCommand("Set Collider Density", Option<f32>) where fn swap(self, physics, collider) {
    std::mem::swap(&mut collider.density, &mut self.value);
});

define_collider_command!(SetColliderCollisionGroupsMembershipsCommand("Set Collider Collision Groups Memberships", u32) where fn swap(self, physics, collider) {
    std::mem::swap(&mut collider.collision_groups.memberships, &mut self.value);
});

define_collider_command!(SetColliderCollisionGroupsFilterCommand("Set Collider Collision Groups Filter", u32) where fn swap(self, physics, collider) {
    std::mem::swap(&mut collider.collision_groups.filter, &mut self.value);
});

define_collider_command!(SetColliderSolverGroupsMembershipsCommand("Set Collider Solver Groups Memberships", u32) where fn swap(self, physics, collider) {
    std::mem::swap(&mut collider.solver_groups.memberships, &mut self.value);
});

define_collider_command!(SetColliderSolverGroupsFilterCommand("Set Collider Solver Groups Filter", u32) where fn swap(self, physics, collider) {
    std::mem::swap(&mut collider.solver_groups.filter, &mut self.value);
});

define_collider_variant_command!(SetCylinderHalfHeightCommand("Set Cylinder Half Height", f32) where fn swap(self, physics, Cylinder, cylinder) {
    std::mem::swap(&mut cylinder.half_height, &mut self.value);
});

define_collider_variant_command!(SetCylinderRadiusCommand("Set Cylinder Radius", f32) where fn swap(self, physics, Cylinder, cylinder) {
    std::mem::swap(&mut cylinder.radius, &mut self.value);
});

define_collider_variant_command!(SetRoundCylinderHalfHeightCommand("Set Cylinder Half Height", f32) where fn swap(self, physics, RoundCylinder, round_cylinder) {
    std::mem::swap(&mut round_cylinder.half_height, &mut self.value);
});

define_collider_variant_command!(SetRoundCylinderRadiusCommand("Set Round Cylinder Radius", f32) where fn swap(self, physics, RoundCylinder, round_cylinder) {
    std::mem::swap(&mut round_cylinder.radius, &mut self.value);
});

define_collider_variant_command!(SetRoundCylinderBorderRadiusCommand("Set Round Cylinder Border Radius", f32) where fn swap(self, physics, RoundCylinder, round_cylinder) {
    std::mem::swap(&mut round_cylinder.border_radius, &mut self.value);
});

define_collider_variant_command!(SetConeHalfHeightCommand("Set Cone Half Height", f32) where fn swap(self, physics, Cone, cone) {
    std::mem::swap(&mut cone.half_height, &mut self.value);
});

define_collider_variant_command!(SetConeRadiusCommand("Set Cone Radius", f32) where fn swap(self, physics, Cone, cone) {
    std::mem::swap(&mut cone.radius, &mut self.value);
});

define_collider_variant_command!(SetCuboidHalfExtentsCommand("Set Cuboid Half Extents", Vector3<f32>) where fn swap(self, physics, Cuboid, cuboid) {
    std::mem::swap(&mut cuboid.half_extents, &mut self.value);
});

define_collider_variant_command!(SetCapsuleRadiusCommand("Set Capsule Radius", f32) where fn swap(self, physics, Capsule, capsule) {
    std::mem::swap(&mut capsule.radius, &mut self.value);
});

define_collider_variant_command!(SetCapsuleBeginCommand("Set Capsule Begin", Vector3<f32>) where fn swap(self, physics, Capsule, capsule) {
    std::mem::swap(&mut capsule.begin, &mut self.value);
});

define_collider_variant_command!(SetCapsuleEndCommand("Set Capsule End", Vector3<f32>) where fn swap(self, physics, Capsule, capsule) {
    std::mem::swap(&mut capsule.end, &mut self.value);
});

define_collider_variant_command!(SetSegmentBeginCommand("Set Segment Begin", Vector3<f32>) where fn swap(self, physics, Segment, segment) {
    std::mem::swap(&mut segment.begin, &mut self.value);
});

define_collider_variant_command!(SetSegmentEndCommand("Set Segment End", Vector3<f32>) where fn swap(self, physics, Segment, segment) {
    std::mem::swap(&mut segment.end, &mut self.value);
});

define_collider_variant_command!(SetTriangleACommand("Set Triangle A", Vector3<f32>) where fn swap(self, physics, Triangle, triangle) {
    std::mem::swap(&mut triangle.a, &mut self.value);
});

define_collider_variant_command!(SetTriangleBCommand("Set Triangle B", Vector3<f32>) where fn swap(self, physics, Triangle, triangle) {
    std::mem::swap(&mut triangle.b, &mut self.value);
});

define_collider_variant_command!(SetTriangleCCommand("Set Triangle C", Vector3<f32>) where fn swap(self, physics, Triangle, triangle) {
    std::mem::swap(&mut triangle.c, &mut self.value);
});

define_collider_variant_command!(SetBallRadiusCommand("Set Ball Radius", f32) where fn swap(self, physics, Ball, ball) {
    std::mem::swap(&mut ball.radius, &mut self.value);
});

define_joint_variant_command!(SetBallJointAnchor1Command("Set Ball Joint Anchor 1", Vector3<f32>) where fn swap(self, physics, BallJoint, ball) {
    std::mem::swap(&mut ball.local_anchor1, &mut self.value);
});

define_joint_variant_command!(SetBallJointAnchor2Command("Set Ball Joint Anchor 2", Vector3<f32>) where fn swap(self, physics, BallJoint, ball) {
    std::mem::swap(&mut ball.local_anchor2, &mut self.value);
});

define_joint_variant_command!(SetFixedJointAnchor1TranslationCommand("Set Fixed Joint Anchor 1 Translation", Vector3<f32>) where fn swap(self, physics, FixedJoint, fixed) {
    std::mem::swap(&mut fixed.local_anchor1_translation, &mut self.value);
});

define_joint_variant_command!(SetFixedJointAnchor2TranslationCommand("Set Fixed Joint Anchor 2 Translation", Vector3<f32>) where fn swap(self, physics, FixedJoint, fixed) {
    std::mem::swap(&mut fixed.local_anchor2_translation, &mut self.value);
});

define_joint_variant_command!(SetFixedJointAnchor1RotationCommand("Set Fixed Joint Anchor 1 Rotation", UnitQuaternion<f32>) where fn swap(self, physics, FixedJoint, fixed) {
    std::mem::swap(&mut fixed.local_anchor1_rotation, &mut self.value);
});

define_joint_variant_command!(SetFixedJointAnchor2RotationCommand("Set Fixed Joint Anchor 2 Rotation", UnitQuaternion<f32>) where fn swap(self, physics, FixedJoint, fixed) {
    std::mem::swap(&mut fixed.local_anchor2_rotation, &mut self.value);
});

define_joint_variant_command!(SetRevoluteJointAnchor1Command("Set Revolute Joint Anchor 1", Vector3<f32>) where fn swap(self, physics, RevoluteJoint, revolute) {
    std::mem::swap(&mut revolute.local_anchor1, &mut self.value);
});

define_joint_variant_command!(SetRevoluteJointAxis1Command("Set Revolute Joint Axis 1", Vector3<f32>) where fn swap(self, physics, RevoluteJoint, revolute) {
    std::mem::swap(&mut revolute.local_axis1, &mut self.value);
});

define_joint_variant_command!(SetRevoluteJointAnchor2Command("Set Revolute Joint Anchor 2", Vector3<f32>) where fn swap(self, physics, RevoluteJoint, revolute) {
    std::mem::swap(&mut revolute.local_anchor2, &mut self.value);
});

define_joint_variant_command!(SetRevoluteJointAxis2Command("Set Prismatic Joint Axis 2", Vector3<f32>) where fn swap(self, physics, RevoluteJoint, revolute) {
    std::mem::swap(&mut revolute.local_axis2, &mut self.value);
});

define_joint_variant_command!(SetPrismaticJointAnchor1Command("Set Prismatic Joint Anchor 1", Vector3<f32>) where fn swap(self, physics, PrismaticJoint, prismatic) {
    std::mem::swap(&mut prismatic.local_anchor1, &mut self.value);
});

define_joint_variant_command!(SetPrismaticJointAxis1Command("Set Prismatic Joint Axis 1", Vector3<f32>) where fn swap(self, physics, PrismaticJoint, prismatic) {
    std::mem::swap(&mut prismatic.local_axis1, &mut self.value);
});

define_joint_variant_command!(SetPrismaticJointAnchor2Command("Set Prismatic Joint Anchor 2", Vector3<f32>) where fn swap(self, physics, PrismaticJoint, prismatic) {
    std::mem::swap(&mut prismatic.local_anchor2, &mut self.value);
});

define_joint_variant_command!(SetPrismaticJointAxis2Command("Set Prismatic Joint Axis 2", Vector3<f32>) where fn swap(self, physics, PrismaticJoint, prismatic) {
    std::mem::swap(&mut prismatic.local_axis2, &mut self.value);
});

define_joint_command!(SetJointBody1Command("Set Joint Body 1", ErasedHandle) where fn swap(self, physics, joint) {
    std::mem::swap(&mut joint.body1, &mut self.value);
});

define_joint_command!(SetJointBody2Command("Set Joint Body 2", ErasedHandle) where fn swap(self, physics, joint) {
    std::mem::swap(&mut joint.body2, &mut self.value);
});

#[derive(Debug)]
pub struct MoveRigidBodyCommand {
    rigid_body: Handle<RigidBody>,
    old_position: Vector3<f32>,
    new_position: Vector3<f32>,
}

impl MoveRigidBodyCommand {
    pub fn new(
        rigid_body: Handle<RigidBody>,
        old_position: Vector3<f32>,
        new_position: Vector3<f32>,
    ) -> Self {
        Self {
            rigid_body,
            old_position,
            new_position,
        }
    }

    fn swap(&mut self) -> Vector3<f32> {
        let position = self.new_position;
        std::mem::swap(&mut self.new_position, &mut self.old_position);
        position
    }

    fn set_position(&self, graph: &mut Graph, physics: &mut Physics, position: Vector3<f32>) {
        physics.bodies[self.rigid_body].position = position;
        if let Some(&node) = physics.binder.key_of(&self.rigid_body) {
            graph[node].local_transform_mut().set_position(position);
        }
    }
}

impl Command for MoveRigidBodyCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Move Rigid Body".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        let position = self.swap();
        self.set_position(
            &mut context.scene.graph,
            &mut context.editor_scene.physics,
            position,
        );
    }

    fn revert(&mut self, context: &mut SceneContext) {
        let position = self.swap();
        self.set_position(
            &mut context.scene.graph,
            &mut context.editor_scene.physics,
            position,
        );
    }
}
