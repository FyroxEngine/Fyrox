use crate::command::CommandContext;
use crate::fyrox::scene::sound::{
    context::SoundContext, DistanceModel, HrirSphereResource, Renderer,
};
use crate::{CommandTrait, GameSceneContext};

macro_rules! define_sound_context_command {
    ($($name:ident($human_readable_name:expr, $value_type:ty, $get:ident, $set:ident); )*) => {
        $(
            #[derive(Debug)]
            pub struct $name {
                value: $value_type,
            }

            impl $name {
                pub fn new(value: $value_type) -> Self {
                    Self { value }
                }

                fn swap(&mut self, sound_context: &mut SoundContext) {
                    let old = sound_context.state().$get();
                    sound_context.state().$set(self.value.clone());
                    self.value = old;
                }
            }

            impl CommandTrait for $name {
                fn name(&mut self, _context: &dyn CommandContext) -> String {
                    $human_readable_name.to_owned()
                }

                fn execute(&mut self, context: &mut dyn CommandContext) {
                    let context = context.get_mut::<GameSceneContext>();
                    self.swap(&mut context.scene.graph.sound_context);
                }

                fn revert(&mut self, context: &mut dyn CommandContext) {
                    let context = context.get_mut::<GameSceneContext>();
                    self.swap(&mut context.scene.graph.sound_context);
                }
            }
        )*
    };
}

define_sound_context_command! {
    SetDistanceModelCommand("Set Distance Model", DistanceModel, distance_model, set_distance_model);
    SetRendererCommand("Set Renderer", Renderer, renderer, set_renderer);
}

#[derive(Debug)]
pub struct SetHrtfRendererHrirSphereResource {
    value: Option<HrirSphereResource>,
}

impl SetHrtfRendererHrirSphereResource {
    pub fn new(value: Option<HrirSphereResource>) -> Self {
        Self { value }
    }

    fn swap(&mut self, sound_context: &mut SoundContext) {
        if let Renderer::HrtfRenderer(hrtf) = sound_context.state().renderer_ref_mut() {
            let old = hrtf.hrir_sphere_resource();
            hrtf.set_hrir_sphere_resource(self.value.clone());
            self.value = old;
        }
    }
}

impl CommandTrait for SetHrtfRendererHrirSphereResource {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Set Hrtf Renderer Hrir Sphere Resource".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        self.swap(&mut context.scene.graph.sound_context);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        self.swap(&mut context.scene.graph.sound_context);
    }
}
