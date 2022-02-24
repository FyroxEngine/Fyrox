use fyrox::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        color::Hsv,
        inspect::{Inspect, PropertyInfo},
        sstorage::ImmutableString,
        uuid::Uuid,
        visitor::prelude::*,
    },
    material::PropertyValue,
    plugin::{Plugin, PluginContext},
    scene::mesh::Mesh,
    script::{ScriptContext, ScriptDefinition, ScriptDefinitionStorage, ScriptTrait},
};
use std::{str::FromStr, sync::Arc};

#[derive(Visit, Inspect, Default)]
struct GamePlugin {
    #[visit(skip)]
    #[inspect(skip)]
    script_storage: Arc<ScriptDefinitionStorage>,
}

impl GamePlugin {
    fn type_uuid() -> Uuid {
        Uuid::from_str("a9507fb2-0945-4fc1-91ce-115ae7c8a615").unwrap()
    }

    pub fn new() -> Self {
        let mut script_storage = ScriptDefinitionStorage::new();

        script_storage.add(ScriptDefinition {
            name: "TestScript".to_string(),
            type_uuid: TestScript::type_uuid(),
            constructor: Box::new(|| Box::new(TestScript::default())),
        });

        Self {
            script_storage: Arc::new(script_storage),
        }
    }
}

impl Plugin for GamePlugin {
    fn on_init(&mut self, _engine: &mut PluginContext) {
        println!("Hello, world!");
    }

    fn on_unload(&mut self, _context: &mut PluginContext) {}

    fn update(&mut self, _context: &mut PluginContext) {}

    fn script_definition_storage(&self) -> Arc<ScriptDefinitionStorage> {
        self.script_storage.clone()
    }

    fn type_uuid(&self) -> Uuid {
        Self::type_uuid()
    }
}

#[derive(Visit, Inspect, Debug, Clone)]
struct TestScript {
    foo: String,

    hue: f32,
}

impl Default for TestScript {
    fn default() -> Self {
        Self {
            foo: "Test String".to_string(),
            hue: 0.0,
        }
    }
}

impl TestScript {
    fn type_uuid() -> Uuid {
        Uuid::from_str("4aa165aa-011b-479f-bc10-b90b2c4b5060").unwrap()
    }
}

impl ScriptTrait for TestScript {
    fn on_init(&mut self, _context: &mut ScriptContext) {}

    fn on_update(&mut self, context: &mut ScriptContext) {
        let transform = context.node.local_transform_mut();
        let new_rotation = **transform.rotation()
            * UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 1.0f32.to_radians());
        transform.set_rotation(new_rotation);

        if let Some(mesh) = context.node.cast_mut::<Mesh>() {
            for surface in mesh.surfaces_mut() {
                surface
                    .material()
                    .lock()
                    .set_property(
                        &ImmutableString::new("diffuseColor"),
                        PropertyValue::Color(Hsv::new(self.hue, 100.0, 100.0).into()),
                    )
                    .unwrap();
            }
        }
        self.hue = (self.hue + 0.2) % 360.0;
    }

    fn type_uuid(&self) -> Uuid {
        Self::type_uuid()
    }

    fn plugin_uuid(&self) -> Uuid {
        GamePlugin::type_uuid()
    }
}

// Script entry point.
#[no_mangle]
pub extern "C" fn fyrox_main() -> Box<Box<dyn Plugin>> {
    Box::new(Box::new(GamePlugin::new()))
}
