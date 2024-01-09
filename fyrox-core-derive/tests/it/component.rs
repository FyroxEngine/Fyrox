use fyrox_core::type_traits::prelude::*;

#[derive(ComponentProvider)]
pub struct Foo {
    #[component(include)]
    component: Component,
    #[component(include)]
    other_component: OtherComponent,
    #[component(
        include,
        path = "wrapper.some_other_component",
        dest_type = "SomeOtherComponent"
    )]
    wrapper: Wrapper,
    #[allow(dead_code)]
    non_component: String,
}

#[derive(PartialEq, Debug)]
pub struct Component {
    stuff: f32,
}

#[derive(PartialEq, Debug)]
pub struct OtherComponent {
    other_stuff: u32,
}

pub struct Wrapper {
    some_other_component: SomeOtherComponent,
}

#[derive(PartialEq, Debug)]
pub struct SomeOtherComponent {
    other_stuff: u8,
}

#[test]
fn test_component_provider() {
    #[allow(clippy::disallowed_names)] // Stupid fucking clippy
    let mut foo = Foo {
        component: Component { stuff: 123.321 },
        other_component: OtherComponent { other_stuff: 123 },
        wrapper: Wrapper {
            some_other_component: SomeOtherComponent { other_stuff: 77 },
        },
        non_component: Default::default(),
    };
    assert_eq!(
        (&foo as &dyn ComponentProvider).component_ref::<Component>(),
        Some(Component { stuff: 123.321 }).as_ref()
    );
    assert_eq!(
        (&foo as &dyn ComponentProvider).component_ref::<OtherComponent>(),
        Some(OtherComponent { other_stuff: 123 }).as_ref()
    );
    assert_eq!(
        (&foo as &dyn ComponentProvider).component_ref::<SomeOtherComponent>(),
        Some(SomeOtherComponent { other_stuff: 77 }).as_ref()
    );

    assert_eq!(
        (&mut foo as &mut dyn ComponentProvider).component_mut::<Component>(),
        Some(Component { stuff: 123.321 }).as_mut()
    );
    assert_eq!(
        (&mut foo as &mut dyn ComponentProvider).component_mut::<OtherComponent>(),
        Some(OtherComponent { other_stuff: 123 }).as_mut()
    );
    assert_eq!(
        (&mut foo as &mut dyn ComponentProvider).component_mut::<SomeOtherComponent>(),
        Some(SomeOtherComponent { other_stuff: 77 }).as_mut()
    );

    assert_eq!(
        (&foo as &dyn ComponentProvider).component_ref::<String>(),
        None
    );
}
