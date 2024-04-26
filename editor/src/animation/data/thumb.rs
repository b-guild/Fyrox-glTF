use fyrox::gui::button::{ButtonBuilder, ButtonMessage};
use fyrox::gui::image::ImageBuilder;
use fyrox::gui::numeric::{NumericUpDown, NumericUpDownBuilder, NumericUpDownMessage};
use fyrox::gui::stack_panel::StackPanelBuilder;
use fyrox::gui::utils::make_simple_tooltip;
use fyrox::gui::widget::WidgetBuilder;
use fyrox::gui::{BuildContext, Thickness, BRUSH_BRIGHT};

use crate::animation::*;
use crate::fyrox::core::pool::Handle;
use crate::fyrox::gui::UiNode;
use crate::load_image;

use crate::animation::command::SetAnimationTimeSliceCommand;

#[derive(Debug, Default)]
pub struct ThumbDataView {
    content: Handle<UiNode>,
    prev: f32,
    next: f32,
    position_box: Handle<UiNode>,
    next_key: Handle<UiNode>,
    prev_key: Handle<UiNode>,
    goto_start: Handle<UiNode>,
    goto_end: Handle<UiNode>,
    start_box: Handle<UiNode>,
    end_box: Handle<UiNode>,
    value_to_model: Handle<UiNode>,
    value_to_interp: Handle<UiNode>,
    model_to_value: Handle<UiNode>,
    remove_key: Handle<UiNode>,
}

fn new_icon(data: &'static [u8], ctx: &mut BuildContext) -> Handle<UiNode> {
    ImageBuilder::new(
        WidgetBuilder::new()
            .with_width(18.0)
            .with_height(18.0)
            .with_margin(Thickness::uniform(1.0))
            .with_background(BRUSH_BRIGHT),
    )
    .with_opt_texture(load_image(data))
    .build(ctx)
}

fn new_time_box(tooltip: &'static str, ctx: &mut BuildContext) -> Handle<UiNode> {
    NumericUpDownBuilder::<f32>::new(
        WidgetBuilder::new()
            .with_enabled(false)
            .with_width(50.0)
            .with_margin(Thickness::uniform(1.0))
            .with_tooltip(make_simple_tooltip(ctx, tooltip)),
    )
    .with_min_value(0.0)
    .with_value(0.0)
    .build(ctx)
}

fn new_button(tooltip: &'static str, text: &'static str, ctx: &mut BuildContext) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_width(50.0)
            .with_tooltip(make_simple_tooltip(ctx, tooltip)),
    )
    .with_text(text)
    .build(ctx)
}

fn new_time_button(tooltip: &'static str, ctx: &mut BuildContext) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_width(50.0)
            .with_tooltip(make_simple_tooltip(ctx, tooltip)),
    )
    .with_text("0s")
    .build(ctx)
}
fn new_text_button(text: &'static str, ctx: &mut BuildContext) -> Handle<UiNode> {
    ButtonBuilder::new(WidgetBuilder::new())
        .with_text(text)
        .build(ctx)
}
fn new_icon_button(
    tooltip: &'static str,
    data: &'static [u8],
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_width(50.0)
            .with_tooltip(make_simple_tooltip(ctx, tooltip)),
    )
    .with_content(new_icon(data, ctx))
    .build(ctx)
}

fn set_thumb<N>(
    animations: &AnimationContainer<Handle<N>>,
    selection: &AnimationSelection<N>,
    value: f32,
) {
    if let Some(animation) = animations.try_get_mut(selection.animation) {
        animation.set_time_position(value);
    }
}

impl ThumbDataView {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let position_box = new_time_box("Current Time within Animation", ctx);
        let value_to_model = new_icon_button("Key from model for all tracks", KEY_ICON, ctx);
        let value_to_interp =
            new_icon_button("Key from interpolation for all tracks", KEY_ICON, ctx);
        let model_to_value = new_button("Model to value for all tracks", "<", ctx);
        let remove_key = new_icon_button("Remove key for all tracks", CLEAR_ICON, ctx);
        let goto_start = new_text_button("Start", ctx);
        let goto_end = new_text_button("End", ctx);
        let start_box = new_time_box("Start Time of Animation", ctx);
        let end_box = new_time_box("End Time of Animation", ctx);
        let prev_key = new_time_button("Go to previous key", ctx);
        let next_key = new_time_button("Go to next key", ctx);
        let content = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(new_icon(TIME_ICON, ctx))
                .with_child(position_box)
                .with_child(value_to_model)
                .with_child(model_to_value)
                .with_child(value_to_interp)
                .with_child(remove_key)
                .with_child(goto_start)
                .with_child(start_box)
                .with_child(goto_end)
                .with_child(end_box)
                .with_child(prev_key)
                .with_child(next_key),
        )
        .build(ctx);
        Self {
            next: 0.0,
            prev: 0.0,
            content,
            position_box,
            value_to_model,
            model_to_value,
            value_to_interp,
            remove_key,
            goto_start,
            start_box,
            goto_end,
            end_box,
            prev_key,
            next_key,
        }
    }
    pub fn position(&self, ui: &UserInterface) -> f32 {
        *ui.node(self.position_box)
            .cast::<NumericUpDown<f32>>()
            .unwrap()
            .value
    }
    pub fn update_thumb(&self, position: f32, ui: &UserInterface) -> bool {
        if self.position(ui) != position {
            ui.send_message(NumericUpDownMessage::value(
                self.position_box,
                MessageDirection::ToWidget,
                position,
            ));
            true
        } else {
            false
        }
    }
    pub fn handle_ui_message<G, N>(
        &mut self,
        message: &UiMessage,
        sender: &MessageSender,
        graph: &G,
        ui: &mut UserInterface,
        animation_player_handle: Handle<N>,
        animations: &AnimationContainer<Handle<N>>,
        root: Handle<N>,
        selection: &AnimationSelection<N>,
    ) where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode<SceneGraph = G>,
    {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.goto_start {
                todo!();
            } else if message.destination() == self.goto_end {
                todo!();
            } else if message.destination() == self.model_to_value {
                todo!();
            } else if message.destination() == self.value_to_model {
                todo!();
            } else if message.destination() == self.value_to_interp {
                todo!();
            } else if message.destination() == self.remove_key {
                todo!();
            }
        } else if let Some(NumericUpDownMessage::<f32>::Value(value)) = message.data() {
            if message.direction() == MessageDirection::FromWidget {
                if message.destination() == self.position_box {
                    set_thumb(animations, selection, *value);
                } else if message.destination() == self.start_box {
                    let mut time_slice = animations[selection.animation].time_slice();
                    time_slice.start = value.min(time_slice.end);
                    sender.do_command(SetAnimationTimeSliceCommand {
                        node_handle: animation_player_handle,
                        animation_handle: selection.animation,
                        value: time_slice,
                    });
                } else if message.destination() == self.end_box {
                    let mut time_slice = animations[selection.animation].time_slice();
                    time_slice.end = value.max(time_slice.start);
                    sender.do_command(SetAnimationTimeSliceCommand {
                        node_handle: animation_player_handle,
                        animation_handle: selection.animation,
                        value: time_slice,
                    });
                }
            }
        }
    }
}
