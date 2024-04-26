use crate::fyrox::core::math::curve::CurveKeyKind;
use crate::fyrox::core::pool::Handle;
use crate::fyrox::core::uuid::Uuid;
use crate::fyrox::generic_animation::value::ValueBinding;
use crate::fyrox::graph::BaseSceneGraph;
use crate::fyrox::gui::{
    button::ButtonBuilder, grid::Column, grid::GridBuilder, message::MessageDirection,
    numeric::NumericUpDownBuilder, numeric::NumericUpDownMessage, text::TextBuilder,
    text::TextMessage, widget::WidgetBuilder, widget::WidgetMessage, UiNode, UserInterface,
};
use crate::BuildContext;

use super::track::*;

const BUTTON_WIDTH: f32 = 16.0;

#[derive(Debug)]
pub struct CurveDataView {
    data: CurveData,
    label: Handle<UiNode>,
    model_value: Handle<UiNode>,
    value_key: Handle<UiNode>,
    key_field: Handle<UiNode>,
    key_kind: Handle<UiNode>,
    interp_text: Handle<UiNode>,
    left_tan: Handle<UiNode>,
    right_tan: Handle<UiNode>,
    next_key: Handle<UiNode>,
    prev_key: Handle<UiNode>,
    remove_key: Handle<UiNode>,
    set_model_to_value: Handle<UiNode>,
    set_value_to_model: Handle<UiNode>,
    set_value_to_interp: Handle<UiNode>,
}

#[derive(Debug, Clone)]
pub struct CurveData {
    pub id: Uuid,
    pub binding: ValueBinding,
    pub curve_index: usize,
    pub key_kind: Option<CurveKeyKind>,
    pub model_value: f32,
    pub value: f32,
    pub left_tan: f32,
    pub right_tan: f32,
    pub prev: f32,
    pub next: f32,
}

impl CurveData {
    pub fn name(&self) -> String {
        match self.binding {
            ValueBinding::Position => todo!(),
            ValueBinding::Scale => todo!(),
            ValueBinding::Rotation => todo!(),
            ValueBinding::Property { name, value_type } => todo!(),
        }
    }
    pub fn value_string(&self) -> String {
        format!("{:.3}", self.value)
    }
    pub fn prev_string(&self) -> String {
        format!("{:.2}s", self.prev)
    }
    pub fn next_string(&self) -> String {
        format!("{:.2}s", self.next)
    }
}

fn send_visible(handle: Handle<UiNode>, visible: bool, ui: &UserInterface) {
    ui.send_message(WidgetMessage::visibility(
        handle,
        MessageDirection::ToWidget,
        visible,
    ));
}

fn send_value(handle: Handle<UiNode>, value: f32, ui: &UserInterface) {
    ui.send_message(NumericUpDownMessage::value(
        handle,
        MessageDirection::ToWidget,
        value,
    ));
}

fn send_text(handle: Handle<UiNode>, text: String, ui: &UserInterface) {
    ui.send_message(TextMessage::text(handle, MessageDirection::ToWidget, text));
}

impl CurveDataView {
    pub fn new(row: usize, data: CurveData, ctx: &mut BuildContext) -> Self {
        let label =
            TextBuilder::new(WidgetBuilder::new().on_column(NAME_COLUMN).on_row(row)).build(ctx);
        let model_value = NumericUpDownBuilder::<f32>::new(
            WidgetBuilder::new().on_column(MODEL_COLUMN).on_row(row),
        )
        .build(ctx);
        let set_value_to_model =
            ButtonBuilder::new(WidgetBuilder::new().on_column(VALUE_TO_MODEL).on_row(row))
                .build(ctx);
        let set_model_to_value =
            ButtonBuilder::new(WidgetBuilder::new().on_column(MODEL_TO_VALUE).on_row(row))
                .build(ctx);
        let key_field = NumericUpDownBuilder::<f32>::new(WidgetBuilder::new()).build(ctx);
        let key_kind = ButtonBuilder::new(WidgetBuilder::new().on_column(1)).build(ctx);
        let remove_key =
            ButtonBuilder::new(WidgetBuilder::new().on_column(INTERP_TO_KEY).on_row(row))
                .build(ctx);
        let value_key = GridBuilder::new(
            WidgetBuilder::new()
                .on_column(VALUE_COLUMN)
                .on_row(row)
                .with_child(key_field)
                .with_child(key_kind),
        )
        .add_column(Column::stretch())
        .add_column(Column::strict(BUTTON_WIDTH))
        .add_column(Column::strict(BUTTON_WIDTH))
        .build(ctx);
        let interp_text =
            TextBuilder::new(WidgetBuilder::new().on_column(VALUE_COLUMN).on_row(row)).build(ctx);
        let set_value_to_interp =
            ButtonBuilder::new(WidgetBuilder::new().on_column(INTERP_TO_KEY).on_row(row))
                .build(ctx);
        let remove_key =
            ButtonBuilder::new(WidgetBuilder::new().on_column(INTERP_TO_KEY).on_row(row))
                .build(ctx);
        let value_interp = GridBuilder::new(
            WidgetBuilder::new()
                .on_column(VALUE_COLUMN)
                .on_row(row)
                .with_child(interp_text),
        )
        .add_column(Column::stretch())
        .add_column(Column::strict(BUTTON_WIDTH))
        .build(ctx);
        let left_tan = NumericUpDownBuilder::<f32>::new(
            WidgetBuilder::new().on_column(LEFT_TAN_COLUMN).on_row(row),
        )
        .build(ctx);
        let right_tan = NumericUpDownBuilder::<f32>::new(
            WidgetBuilder::new().on_column(RIGHT_TAN_COLUMN).on_row(row),
        )
        .build(ctx);
        let prev_key =
            ButtonBuilder::new(WidgetBuilder::new().on_column(PREV_COLUMN).on_row(row)).build(ctx);
        let next_key =
            ButtonBuilder::new(WidgetBuilder::new().on_column(NEXT_COLUMN).on_row(row)).build(ctx);
        Self {
            data,
            label,
            model_value,
            value_key,
            key_field,
            key_kind,
            interp_text,
            set_value_to_interp,
            remove_key,
            left_tan,
            right_tan,
            prev_key,
            next_key,
            set_model_to_value,
            set_value_to_model,
        }
    }
    pub fn children(&self) -> impl Iterator<Item = Handle<UiNode>> {
        [
            self.label,
            self.model_value,
            self.value_key,
            self.interp_text,
            self.set_value_to_interp,
            self.remove_key,
            self.left_tan,
            self.right_tan,
            self.prev_key,
            self.next_key,
            self.set_model_to_value,
            self.set_value_to_model,
        ]
        .into_iter()
    }
    pub fn sync(&self, data: CurveData, ui: &UserInterface) {
        send_value(self.model_value, data.model_value, ui);
        if let Some(key_kind) = data.key_kind {
            send_value(self.key_field, data.value, ui);
            // TODO: Key kind
        } else {
            send_text(self.interp_text, data.value_string(), ui);
        }
        if ui.node(self.label).visibility() {
            let on_key = data.key_kind.is_some();
            send_visible(self.value_key, on_key, ui);
            send_visible(self.remove_key, on_key, ui);
            send_visible(self.interp_text, !on_key, ui);
            send_visible(self.set_value_to_interp, !on_key, ui);
        }
        send_value(self.left_tan, data.left_tan, ui);
        send_value(self.right_tan, data.right_tan, ui);
        send_text(self.prev_key, data.prev_string(), ui);
        send_text(self.next_key, data.next_string(), ui);
    }
    pub fn send_visibility(&self, visible: bool, ui: &UserInterface) {
        let on_key = self.data.key_kind.is_some();
        send_visible(self.label, visible, ui);
        send_visible(self.model_value, visible, ui);
        send_visible(self.value_key, visible && on_key, ui);
        send_visible(self.remove_key, visible && on_key, ui);
        send_visible(self.interp_text, visible && !on_key, ui);
        send_visible(self.set_value_to_interp, visible && !on_key, ui);
        send_visible(self.left_tan, visible, ui);
        send_visible(self.right_tan, visible, ui);
        send_visible(self.next_key, visible, ui);
        send_visible(self.prev_key, visible, ui);
        send_visible(self.set_model_to_value, visible, ui);
        send_visible(self.set_value_to_model, visible, ui);
    }
    pub fn sync_to_data(&self, data: CurveData, ui: &UserInterface) {}
}
