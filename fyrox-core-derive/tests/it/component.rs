// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

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
