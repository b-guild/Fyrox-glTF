use crate::{
    animation::{
        command::{
            AddAnimationCommand, RemoveAnimationCommand, SetAnimationLengthCommand,
            SetAnimationNameCommand, SetAnimationSpeedCommand,
        },
        selection::AnimationSelection,
    },
    gui::make_dropdown_list_option_universal,
    load_image,
    scene::{
        commands::{ChangeSelectionCommand, CommandGroup, SceneCommand},
        EditorScene, Selection,
    },
    Message,
};
use fyrox::{
    animation::Animation,
    core::{algebra::Vector2, math::Rect, pool::Handle},
    gui::{
        border::BorderBuilder,
        button::{ButtonBuilder, ButtonMessage},
        check_box::{CheckBoxBuilder, CheckBoxMessage},
        dropdown_list::{DropdownList, DropdownListBuilder, DropdownListMessage},
        image::ImageBuilder,
        message::{MessageDirection, UiMessage},
        numeric::{NumericUpDownBuilder, NumericUpDownMessage},
        stack_panel::StackPanelBuilder,
        text::{TextBuilder, TextMessage},
        text_box::{TextBox, TextBoxBuilder},
        utils::{make_cross, make_simple_tooltip},
        vector_image::{Primitive, VectorImageBuilder},
        widget::{WidgetBuilder, WidgetMessage},
        BuildContext, Orientation, Thickness, UiNode, UserInterface, VerticalAlignment,
        BRUSH_BRIGHT, BRUSH_LIGHT,
    },
    scene::{animation::AnimationPlayer, node::Node},
};
use std::sync::mpsc::Sender;

pub struct Toolbar {
    pub panel: Handle<UiNode>,
    pub play_pause: Handle<UiNode>,
    pub stop: Handle<UiNode>,
    pub speed: Handle<UiNode>,
    pub animations: Handle<UiNode>,
    pub add_animation: Handle<UiNode>,
    pub remove_current_animation: Handle<UiNode>,
    pub rename_current_animation: Handle<UiNode>,
    pub clone_current_animation: Handle<UiNode>,
    pub animation_name: Handle<UiNode>,
    pub preview: Handle<UiNode>,
    pub length: Handle<UiNode>,
}

#[must_use]
pub enum ToolbarAction {
    None,
    EnterPreviewMode,
    LeavePreviewMode,
    SelectAnimation(Handle<Animation>),
    PlayPause,
    Stop,
}

impl Toolbar {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let play_pause;
        let stop;
        let speed;
        let animations;
        let add_animation;
        let remove_current_animation;
        let rename_current_animation;
        let clone_current_animation;
        let animation_name;
        let preview;
        let length;
        let panel = BorderBuilder::new(
            WidgetBuilder::new()
                .on_row(0)
                .with_foreground(BRUSH_LIGHT)
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_child({
                                animation_name = TextBoxBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(100.0)
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .with_vertical_text_alignment(VerticalAlignment::Center)
                                .with_text("New Animation")
                                .build(ctx);
                                animation_name
                            })
                            .with_child({
                                add_animation = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(20.0)
                                        .with_height(20.0)
                                        .with_vertical_alignment(VerticalAlignment::Center)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tooltip(make_simple_tooltip(
                                            ctx,
                                            "Add New Animation",
                                        )),
                                )
                                .with_text("+")
                                .build(ctx);
                                add_animation
                            })
                            .with_child({
                                rename_current_animation = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_enabled(false)
                                        .with_width(20.0)
                                        .with_height(20.0)
                                        .with_vertical_alignment(VerticalAlignment::Center)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tooltip(make_simple_tooltip(
                                            ctx,
                                            "Rename Selected Animation",
                                        )),
                                )
                                .with_content(
                                    ImageBuilder::new(
                                        WidgetBuilder::new()
                                            .with_width(18.0)
                                            .with_height(18.0)
                                            .with_margin(Thickness::uniform(1.0))
                                            .with_background(BRUSH_BRIGHT),
                                    )
                                    .with_opt_texture(load_image(include_bytes!(
                                        "../../resources/embed/rename.png"
                                    )))
                                    .build(ctx),
                                )
                                .build(ctx);
                                rename_current_animation
                            })
                            .with_child({
                                animations = DropdownListBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(120.0)
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .build(ctx);
                                animations
                            })
                            .with_child({
                                remove_current_animation = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_enabled(false)
                                        .with_width(20.0)
                                        .with_height(20.0)
                                        .with_vertical_alignment(VerticalAlignment::Center)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tooltip(make_simple_tooltip(
                                            ctx,
                                            "Remove Selected Animation",
                                        )),
                                )
                                .with_content(make_cross(ctx, 14.0, 2.0))
                                .build(ctx);
                                remove_current_animation
                            })
                            .with_child({
                                clone_current_animation = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_enabled(false)
                                        .with_width(20.0)
                                        .with_height(20.0)
                                        .with_vertical_alignment(VerticalAlignment::Center)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tooltip(make_simple_tooltip(
                                            ctx,
                                            "Clone Selected Animation",
                                        )),
                                )
                                .with_content(
                                    ImageBuilder::new(
                                        WidgetBuilder::new()
                                            .with_width(18.0)
                                            .with_height(18.0)
                                            .with_margin(Thickness::uniform(1.0))
                                            .with_background(BRUSH_BRIGHT),
                                    )
                                    .with_opt_texture(load_image(include_bytes!(
                                        "../../resources/embed/copy.png"
                                    )))
                                    .build(ctx),
                                )
                                .build(ctx);
                                clone_current_animation
                            })
                            .with_child(
                                ImageBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(18.0)
                                        .with_height(18.0)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_background(BRUSH_BRIGHT),
                                )
                                .with_opt_texture(load_image(include_bytes!(
                                    "../../resources/embed/speed.png"
                                )))
                                .build(ctx),
                            )
                            .with_child({
                                speed = NumericUpDownBuilder::<f32>::new(
                                    WidgetBuilder::new()
                                        .with_enabled(false)
                                        .with_width(60.0)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tooltip(make_simple_tooltip(
                                            ctx,
                                            "Animation Playback Speed",
                                        )),
                                )
                                .with_min_value(0.0)
                                .with_value(1.0)
                                .build(ctx);
                                speed
                            })
                            .with_child(
                                ImageBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(18.0)
                                        .with_height(18.0)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_background(BRUSH_BRIGHT),
                                )
                                .with_opt_texture(load_image(include_bytes!(
                                    "../../resources/embed/time.png"
                                )))
                                .build(ctx),
                            )
                            .with_child({
                                length = NumericUpDownBuilder::<f32>::new(
                                    WidgetBuilder::new()
                                        .with_enabled(false)
                                        .with_width(60.0)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tooltip(make_simple_tooltip(
                                            ctx,
                                            "Animation Length in Seconds",
                                        )),
                                )
                                .with_min_value(0.0)
                                .with_value(1.0)
                                .build(ctx);
                                length
                            })
                            .with_child({
                                preview = CheckBoxBuilder::new(
                                    WidgetBuilder::new().with_enabled(false).with_margin(
                                        Thickness {
                                            left: 10.0,
                                            top: 1.0,
                                            right: 5.0,
                                            bottom: 1.0,
                                        },
                                    ),
                                )
                                .with_content(
                                    TextBuilder::new(
                                        WidgetBuilder::new()
                                            .with_vertical_alignment(VerticalAlignment::Center),
                                    )
                                    .with_text("Preview")
                                    .build(ctx),
                                )
                                .checked(Some(false))
                                .build(ctx);
                                preview
                            })
                            .with_child({
                                play_pause = ButtonBuilder::new(
                                    WidgetBuilder::new().with_enabled(false).with_margin(
                                        Thickness {
                                            left: 1.0,
                                            top: 1.0,
                                            right: 1.0,
                                            bottom: 1.0,
                                        },
                                    ),
                                )
                                .with_content(
                                    VectorImageBuilder::new(
                                        WidgetBuilder::new()
                                            .with_foreground(BRUSH_BRIGHT)
                                            .with_tooltip(make_simple_tooltip(ctx, "Play/Pause")),
                                    )
                                    .with_primitives(vec![
                                        Primitive::Triangle {
                                            points: [
                                                Vector2::new(0.0, 0.0),
                                                Vector2::new(8.0, 8.0),
                                                Vector2::new(0.0, 16.0),
                                            ],
                                        },
                                        Primitive::RectangleFilled {
                                            rect: Rect::new(10.0, 0.0, 4.0, 16.0),
                                        },
                                        Primitive::RectangleFilled {
                                            rect: Rect::new(15.0, 0.0, 4.0, 16.0),
                                        },
                                    ])
                                    .build(ctx),
                                )
                                .build(ctx);
                                play_pause
                            })
                            .with_child({
                                stop = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_enabled(false)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tooltip(make_simple_tooltip(ctx, "Stop Playback")),
                                )
                                .with_content(
                                    VectorImageBuilder::new(
                                        WidgetBuilder::new().with_foreground(BRUSH_BRIGHT),
                                    )
                                    .with_primitives(vec![Primitive::RectangleFilled {
                                        rect: Rect::new(0.0, 0.0, 16.0, 16.0),
                                    }])
                                    .build(ctx),
                                )
                                .build(ctx);
                                stop
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                ),
        )
        .with_stroke_thickness(Thickness::uniform(1.0))
        .build(ctx);

        Self {
            panel,
            play_pause,
            stop,
            speed,
            animations,
            add_animation,
            rename_current_animation,
            remove_current_animation,
            animation_name,
            preview,
            length,
            clone_current_animation,
        }
    }

    pub fn handle_ui_message(
        &self,
        message: &UiMessage,
        sender: &Sender<Message>,
        ui: &UserInterface,
        animation_player_handle: Handle<Node>,
        animation_player: &mut AnimationPlayer,
        editor_scene: &EditorScene,
        selection: &AnimationSelection,
    ) -> ToolbarAction {
        if let Some(DropdownListMessage::SelectionChanged(Some(index))) = message.data() {
            if message.destination() == self.animations
                && message.direction() == MessageDirection::FromWidget
            {
                let item = ui
                    .node(self.animations)
                    .query_component::<DropdownList>()
                    .unwrap()
                    .items()[*index];
                let animation = ui.node(item).user_data_ref::<Handle<Animation>>().unwrap();
                sender
                    .send(Message::do_scene_command(ChangeSelectionCommand::new(
                        Selection::Animation(AnimationSelection {
                            animation_player: animation_player_handle,
                            animation: *animation,
                            entities: vec![],
                        }),
                        editor_scene.selection.clone(),
                    )))
                    .unwrap();
                return ToolbarAction::SelectAnimation(*animation);
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.play_pause {
                return ToolbarAction::PlayPause;
            } else if message.destination() == self.stop {
                return ToolbarAction::Stop;
            } else if message.destination() == self.remove_current_animation {
                if animation_player
                    .animations()
                    .try_get(selection.animation)
                    .is_some()
                {
                    let group = vec![
                        SceneCommand::new(ChangeSelectionCommand::new(
                            Selection::Animation(AnimationSelection {
                                animation_player: animation_player_handle,
                                animation: Default::default(),
                                entities: vec![],
                            }),
                            editor_scene.selection.clone(),
                        )),
                        SceneCommand::new(RemoveAnimationCommand::new(
                            animation_player_handle,
                            selection.animation,
                        )),
                    ];

                    sender
                        .send(Message::do_scene_command(CommandGroup::from(group)))
                        .unwrap();
                }
            } else if message.destination() == self.rename_current_animation {
                sender
                    .send(Message::do_scene_command(SetAnimationNameCommand {
                        node_handle: animation_player_handle,
                        animation_handle: selection.animation,
                        value: ui
                            .node(self.animation_name)
                            .query_component::<TextBox>()
                            .unwrap()
                            .text(),
                    }))
                    .unwrap();
            } else if message.destination() == self.add_animation {
                let mut animation = Animation::default();
                animation.set_name(
                    ui.node(self.animation_name)
                        .query_component::<TextBox>()
                        .unwrap()
                        .text(),
                );
                sender
                    .send(Message::do_scene_command(AddAnimationCommand::new(
                        animation_player_handle,
                        animation,
                    )))
                    .unwrap();
            } else if message.destination() == self.clone_current_animation {
                if let Some(animation) = animation_player.animations().try_get(selection.animation)
                {
                    let mut animation_clone = animation.clone();

                    animation_clone.set_name(format!("{} Copy", animation.name()));

                    sender
                        .send(Message::do_scene_command(AddAnimationCommand::new(
                            animation_player_handle,
                            animation_clone,
                        )))
                        .unwrap();
                }
            }
        } else if let Some(CheckBoxMessage::Check(Some(checked))) = message.data() {
            if message.destination() == self.preview
                && message.direction() == MessageDirection::FromWidget
            {
                return if *checked {
                    ToolbarAction::EnterPreviewMode
                } else {
                    ToolbarAction::LeavePreviewMode
                };
            }
        } else if let Some(NumericUpDownMessage::Value(value)) = message.data() {
            if message.direction() == MessageDirection::FromWidget {
                if message.destination() == self.length {
                    sender
                        .send(Message::do_scene_command(SetAnimationLengthCommand {
                            node_handle: animation_player_handle,
                            animation_handle: selection.animation,
                            value: *value,
                        }))
                        .unwrap();
                } else if message.destination() == self.speed {
                    sender
                        .send(Message::do_scene_command(SetAnimationSpeedCommand {
                            node_handle: animation_player_handle,
                            animation_handle: selection.animation,
                            value: *value,
                        }))
                        .unwrap();
                }
            }
        }

        ToolbarAction::None
    }

    pub fn clear(&mut self, ui: &UserInterface) {
        ui.send_message(DropdownListMessage::items(
            self.animations,
            MessageDirection::ToWidget,
            vec![],
        ));
    }

    pub fn on_preview_mode_changed(&self, ui: &UserInterface, in_preview_mode: bool) {
        for widget in [self.play_pause, self.stop] {
            ui.send_message(WidgetMessage::enabled(
                widget,
                MessageDirection::ToWidget,
                in_preview_mode,
            ));
        }
    }

    pub fn sync_to_model(
        &self,
        animation_player: &AnimationPlayer,
        selection: &AnimationSelection,
        ui: &mut UserInterface,
        in_preview_mode: bool,
    ) {
        let new_items = animation_player
            .animations()
            .pair_iter()
            .map(|(h, a)| {
                make_dropdown_list_option_universal(&mut ui.build_ctx(), a.name(), 22.0, h)
            })
            .collect();

        ui.send_message(DropdownListMessage::items(
            self.animations,
            MessageDirection::ToWidget,
            new_items,
        ));

        let mut selected_animation_valid = false;
        if let Some(animation) = animation_player.animations().try_get(selection.animation) {
            selected_animation_valid = true;
            ui.send_message(TextMessage::text(
                self.animation_name,
                MessageDirection::ToWidget,
                animation.name().to_string(),
            ));

            ui.send_message(NumericUpDownMessage::value(
                self.length,
                MessageDirection::ToWidget,
                animation.length(),
            ));

            ui.send_message(NumericUpDownMessage::value(
                self.speed,
                MessageDirection::ToWidget,
                animation.speed(),
            ));
        }

        for widget in [
            self.preview,
            self.speed,
            self.rename_current_animation,
            self.remove_current_animation,
            self.length,
            self.clone_current_animation,
        ] {
            ui.send_message(WidgetMessage::enabled(
                widget,
                MessageDirection::ToWidget,
                selected_animation_valid,
            ));
        }

        for widget in [self.play_pause, self.stop] {
            ui.send_message(WidgetMessage::enabled(
                widget,
                MessageDirection::ToWidget,
                selected_animation_valid && in_preview_mode,
            ));
        }
    }
}