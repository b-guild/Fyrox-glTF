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

//! A widget that shows numeric value as a set of individual bits allowing switching separate bits.

use crate::{
    check_box::{CheckBoxBuilder, CheckBoxMessage},
    core::{
        num_traits::{NumCast, One, Zero},
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid::uuid,
        visitor::prelude::*,
    },
    define_constructor,
    message::UiMessage,
    widget::{Widget, WidgetBuilder},
    wrap_panel::WrapPanelBuilder,
    BuildContext, Control, MessageDirection, MouseButton, Orientation, Thickness, UiNode,
    UserInterface, WidgetMessage,
};
use fyrox_graph::constructor::{ConstructorProvider, GraphNodeConstructor};
use fyrox_graph::BaseSceneGraph;
use std::{
    fmt::Debug,
    mem,
    ops::{BitAnd, BitOr, Deref, DerefMut, Not, Shl},
};

pub trait BitContainer:
    BitAnd<Output = Self>
    + BitOr<Output = Self>
    + Clone
    + Copy
    + Default
    + One
    + Shl<Output = Self>
    + NumCast
    + Not<Output = Self>
    + Zero
    + PartialEq
    + Debug
    + Reflect
    + Visit
    + Send
    + TypeUuidProvider
    + 'static
{
}

impl<T> BitContainer for T where
    T: BitAnd<Output = Self>
        + BitOr<Output = Self>
        + Clone
        + Copy
        + Default
        + One
        + Shl<Output = Self>
        + NumCast
        + Not<Output = Self>
        + Zero
        + PartialEq
        + Debug
        + Reflect
        + Visit
        + Send
        + TypeUuidProvider
        + 'static
{
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BitFieldMessage<T: BitContainer> {
    Value(T),
}

impl<T: BitContainer> BitFieldMessage<T> {
    define_constructor!(BitFieldMessage:Value => fn value(T), layout: false);
}

impl<T: BitContainer> ConstructorProvider<UiNode, UserInterface> for BitField<T> {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>()
            .with_variant(format!("Bit Field<{}>", std::any::type_name::<T>()), |ui| {
                BitFieldBuilder::<T>::new(WidgetBuilder::new())
                    .build(&mut ui.build_ctx())
                    .into()
            })
            .with_group("Bit")
    }
}

#[derive(Default, Clone, Reflect, Visit, Debug, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct BitField<T>
where
    T: BitContainer,
{
    pub widget: Widget,
    pub value: T,
    pub bit_switches: Vec<Handle<UiNode>>,
}

impl<T> Deref for BitField<T>
where
    T: BitContainer,
{
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<T> DerefMut for BitField<T>
where
    T: BitContainer,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

#[must_use]
fn set_bit<T: BitContainer>(value: T, index: usize) -> T {
    value | (T::one() << T::from(index).unwrap_or_default())
}

#[must_use]
fn reset_bit<T: BitContainer>(value: T, index: usize) -> T {
    value & !(T::one() << T::from(index).unwrap_or_default())
}

#[must_use]
fn is_bit_set<T: BitContainer>(value: T, index: usize) -> bool {
    value & (T::one() << T::from(index).unwrap_or_default()) != T::zero()
}

impl<T> TypeUuidProvider for BitField<T>
where
    T: BitContainer,
{
    fn type_uuid() -> Uuid {
        combine_uuids(
            uuid!("6c19b266-18be-46d2-bfd3-f1dc9cb3f36c"),
            T::type_uuid(),
        )
    }
}

impl<T> Control for BitField<T>
where
    T: BitContainer,
{
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(CheckBoxMessage::Check(Some(value))) = message.data() {
            if message.direction() == MessageDirection::FromWidget {
                if let Some(bit_index) = self
                    .bit_switches
                    .iter()
                    .position(|s| *s == message.destination())
                {
                    let new_value = if *value {
                        set_bit(self.value, bit_index)
                    } else {
                        reset_bit(self.value, bit_index)
                    };

                    ui.send_message(BitFieldMessage::value(
                        self.handle,
                        MessageDirection::ToWidget,
                        new_value,
                    ));
                }
            }
        } else if let Some(BitFieldMessage::Value(value)) = message.data() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
                && *value != self.value
            {
                self.value = *value;
                self.sync_switches(ui);
                ui.send_message(message.reverse());
            }
        } else if let Some(WidgetMessage::MouseDown { button, .. }) = message.data() {
            if *button == MouseButton::Right {
                for (index, bit) in self.bit_switches.iter().cloned().enumerate() {
                    if ui.node(bit).has_descendant(message.destination(), ui) {
                        let new_value = if is_bit_set(self.value, index) {
                            !(T::one() << T::from(index).unwrap_or_default())
                        } else {
                            T::one() << T::from(index).unwrap_or_default()
                        };

                        ui.send_message(BitFieldMessage::value(
                            self.handle,
                            MessageDirection::ToWidget,
                            new_value,
                        ));
                    }
                }
            }
        }
    }
}

impl<T> BitField<T>
where
    T: BitContainer,
{
    fn sync_switches(&self, ui: &UserInterface) {
        for (i, handle) in self.bit_switches.iter().cloned().enumerate() {
            ui.send_message(CheckBoxMessage::checked(
                handle,
                MessageDirection::ToWidget,
                Some(is_bit_set(self.value, i)),
            ));
        }
    }
}

pub struct BitFieldBuilder<T>
where
    T: BitContainer,
{
    widget_builder: WidgetBuilder,
    value: T,
}

impl<T> BitFieldBuilder<T>
where
    T: BitContainer,
{
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            value: T::default(),
        }
    }

    pub fn with_value(mut self, value: T) -> Self {
        self.value = value;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let bit_switches = (0..(mem::size_of::<T>() * 8))
            .map(|i| {
                CheckBoxBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                    .checked(Some(is_bit_set(self.value, i)))
                    .build(ctx)
            })
            .collect::<Vec<_>>();

        let panel =
            WrapPanelBuilder::new(WidgetBuilder::new().with_children(bit_switches.iter().cloned()))
                .with_orientation(Orientation::Horizontal)
                .build(ctx);

        let canvas = BitField {
            widget: self.widget_builder.with_child(panel).build(ctx),
            value: self.value,
            bit_switches,
        };
        ctx.add_node(UiNode::new(canvas))
    }
}

#[cfg(test)]
mod test {
    use crate::bit::BitFieldBuilder;
    use crate::{test::test_widget_deletion, widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| BitFieldBuilder::<usize>::new(WidgetBuilder::new()).build(ctx));
    }
}
