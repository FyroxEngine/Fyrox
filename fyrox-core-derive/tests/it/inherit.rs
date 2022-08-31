use fyrox::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        reflect::Reflect,
        variable::{InheritableVariable, TemplateVariable},
    },
    scene::{DirectlyInheritableEntity, Inherit},
};

fn as_dyn(x: &impl InheritableVariable) -> &dyn InheritableVariable {
    x
}

fn as_dyn_mut(x: &mut impl InheritableVariable) -> &mut dyn InheritableVariable {
    x
}

fn test<'a, 'b>(
    xs: impl IntoIterator<Item = &'a dyn InheritableVariable>,
    ys: impl IntoIterator<Item = &'b dyn InheritableVariable>,
) {
    assert_eq!(
        xs.into_iter().map(|x| x.as_any().type_id()).collect::<Vec<_>>(),
        ys.into_iter().map(|y| y.as_any().type_id()).collect::<Vec<_>>(),
    );
}

fn test_mut<'a, 'b>(
    xs: impl IntoIterator<Item = &'a mut dyn InheritableVariable>,
    ys: impl IntoIterator<Item = &'b mut dyn InheritableVariable>,
) {
    assert_eq!(
        xs.into_iter().map(|x| x.as_any().type_id()).collect::<Vec<_>>(),
        ys.into_iter().map(|y| y.as_any().type_id()).collect::<Vec<_>>(),
    );
}

#[derive(Default, Clone, Reflect, Inherit)]
struct Foo {
    #[inherit]
    #[reflect(hidden)]
    inheritable_field: TemplateVariable<Vector3<f32>>,
    other_field: String,
    #[inherit]
    #[reflect(hidden)]
    x: TemplateVariable<UnitQuaternion<f32>>,
}

#[test]
fn test_inherit() {
    let mut foo1 = Foo::default();
    let mut foo2 = Foo::default();

    test(
        foo1.inheritable_properties_ref(),
        [as_dyn(&foo2.inheritable_field), as_dyn(&foo2.x)],
    );

    test_mut(
        foo1.inheritable_properties_mut(),
        [as_dyn_mut(&mut foo2.inheritable_field), as_dyn_mut(&mut foo2.x)],
    );
}
