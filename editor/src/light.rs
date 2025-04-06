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

use crate::fyrox::{
    core::{log::Log, pool::Handle, reflect::prelude::*},
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        formatted_text::WrapMode,
        grid::{Column, GridBuilder, Row},
        inspector::{InspectorBuilder, InspectorContext, InspectorMessage, PropertyAction},
        message::{MessageDirection, UiMessage},
        progress_bar::{ProgressBarBuilder, ProgressBarMessage},
        scroll_viewer::ScrollViewerBuilder,
        text::TextBuilder,
        text::TextMessage,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, HorizontalAlignment, Thickness, UiNode, UserInterface, VerticalAlignment,
    },
    utils::lightmap::{
        CancellationToken, Lightmap, LightmapGenerationError, LightmapInputData, ProgressIndicator,
    },
};
use crate::plugins::inspector::editors::make_property_editors_container;
use crate::{message::MessageSender, scene::GameScene, Engine, MSG_SYNC_FLAG};
use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, Sender},
    sync::Arc,
};

#[derive(Reflect, Debug)]
struct LightmapperSettings {
    #[reflect(
        description = "Amount of texels per unit. It defines 'pixels density' per unit of area (square meters). The \
    more the value, the more detailed produced light map will be and vice versa. This value **directly** affects performance \
    in quadratic manner, which means that if you change it from 32 to 64, the time needed to generate the light map won't double, \
    but it will be 4 times more. Default value is 64 which is a good balance between quality and generation speed.",
        min_value = 1.0,
        max_value = 256.0
    )]
    texels_per_unit: u32,
    #[reflect(
        description = "Relative spacing between UV elements generated by the built-in UV mapper. The more the value, the \
    more the distance between the UV elements will be. This parameters is used to prevent seams from occurring, when rendering \
    meshes with bilinear filtration. Default value is 0.005, which is a good balance between size of the light maps and their \
    quality (lack of seams).",
        min_value = 0.0,
        max_value = 0.1,
        step = 0.001
    )]
    spacing: f32,
    #[reflect(
        description = "Path to the directory which will be used to save the generated light maps. Keep in mind, that \
    the lightmapper automatically generates names for the files."
    )]
    path: PathBuf,
}

impl Default for LightmapperSettings {
    fn default() -> Self {
        Self {
            texels_per_unit: 64,
            spacing: 0.005,
            path: Default::default(),
        }
    }
}

struct ProgressWindow {
    window: Handle<UiNode>,
    progress_bar: Handle<UiNode>,
    cancel: Handle<UiNode>,
    text: Handle<UiNode>,
    progress_indicator: ProgressIndicator,
    cancellation_token: CancellationToken,
}

impl ProgressWindow {
    pub fn new(
        ctx: &mut BuildContext,
        progress_indicator: ProgressIndicator,
        cancellation_token: CancellationToken,
    ) -> Self {
        let progress_bar;
        let cancel;
        let text;
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(400.0).with_height(120.0))
            .open(false)
            .with_title(WindowTitle::text("Progress"))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            TextBuilder::new(WidgetBuilder::new().on_row(0))
                                .with_text(
                                    "Please wait until light map is fully generated. It may \
                                take different amount of time depending on the settings.",
                                )
                                .with_wrap(WrapMode::Word)
                                .build(ctx),
                        )
                        .with_child({
                            progress_bar = ProgressBarBuilder::new(
                                WidgetBuilder::new().on_row(1).with_height(25.0),
                            )
                            .build(ctx);
                            progress_bar
                        })
                        .with_child({
                            text = TextBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(1)
                                    .with_horizontal_alignment(HorizontalAlignment::Center)
                                    .with_vertical_alignment(VerticalAlignment::Center),
                            )
                            .build(ctx);
                            text
                        })
                        .with_child({
                            cancel = ButtonBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(3)
                                    .with_width(100.0)
                                    .with_height(25.0)
                                    .with_horizontal_alignment(HorizontalAlignment::Right),
                            )
                            .with_text("Cancel")
                            .build(ctx);
                            cancel
                        }),
                )
                .add_row(Row::auto())
                .add_row(Row::auto())
                .add_row(Row::stretch())
                .add_row(Row::auto())
                .add_column(Column::stretch())
                .build(ctx),
            )
            .build(ctx);

        Self {
            window,
            progress_bar,
            cancel,
            text,
            progress_indicator,
            cancellation_token,
        }
    }

    pub fn show_progress(&self, ui: &UserInterface) {
        ui.send_message(ProgressBarMessage::progress(
            self.progress_bar,
            MessageDirection::ToWidget,
            self.progress_indicator.progress_percent() as f32 / 100.0,
        ));

        let stage = self.progress_indicator.stage();
        ui.send_message(TextMessage::text(
            self.text,
            MessageDirection::ToWidget,
            format!(
                "Stage {} out of 4: {}",
                stage as u32,
                self.progress_indicator.stage()
            ),
        ));
    }

    pub fn open(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::open_modal(
            self.window,
            MessageDirection::ToWidget,
            true,
            true,
        ));
    }

    pub fn close(&self, ui: &UserInterface) {
        ui.send_message(WidgetMessage::remove(
            self.window,
            MessageDirection::ToWidget,
        ));
    }
}

pub struct LightPanel {
    pub window: Handle<UiNode>,
    inspector: Handle<UiNode>,
    generate: Handle<UiNode>,
    settings: LightmapperSettings,
    progress_window: Option<ProgressWindow>,
    sender: Sender<Result<Lightmap, LightmapGenerationError>>,
    receiver: Receiver<Result<Lightmap, LightmapGenerationError>>,
}

impl LightPanel {
    pub fn new(engine: &mut Engine, sender: MessageSender) -> Self {
        let settings = LightmapperSettings::default();
        let container = Arc::new(make_property_editors_container(
            sender,
            engine.resource_manager.clone(),
        ));

        let generate;
        let inspector;
        let ctx = &mut engine.user_interfaces.first_mut().build_ctx();
        let window = WindowBuilder::new(
            WidgetBuilder::new()
                .with_name("LightPanel")
                .with_width(300.0)
                .with_height(400.0),
        )
        .with_title(WindowTitle::text("Light Settings"))
        .open(false)
        .with_content(
            GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(
                        ScrollViewerBuilder::new(
                            WidgetBuilder::new()
                                .with_margin(Thickness::uniform(1.0))
                                .on_row(0),
                        )
                        .with_content({
                            inspector = InspectorBuilder::new(
                                WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                            )
                            .with_context(InspectorContext::from_object(
                                &settings,
                                ctx,
                                container,
                                None,
                                MSG_SYNC_FLAG,
                                0,
                                true,
                                Default::default(),
                                150.0,
                            ))
                            .build(ctx);
                            inspector
                        })
                        .build(ctx),
                    )
                    .with_child({
                        generate = ButtonBuilder::new(
                            WidgetBuilder::new()
                                .on_row(1)
                                .on_column(0)
                                .with_margin(Thickness::uniform(1.0)),
                        )
                        .with_text("Generate Lightmap")
                        .build(ctx);
                        generate
                    }),
            )
            .add_column(Column::stretch())
            .add_row(Row::stretch())
            .add_row(Row::strict(25.0))
            .build(ctx),
        )
        .build(ctx);

        let (sender, receiver) = std::sync::mpsc::channel();

        Self {
            window,
            inspector,
            generate,
            settings,
            progress_window: None,
            sender,
            receiver,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        game_scene: &GameScene,
        engine: &mut Engine,
    ) {
        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.generate {
                let scene = &mut engine.scenes[game_scene.scene];

                let progress_indicator = ProgressIndicator::new();
                let cancellation_token = CancellationToken::new();

                let progress_window = ProgressWindow::new(
                    &mut engine.user_interfaces.first_mut().build_ctx(),
                    progress_indicator.clone(),
                    cancellation_token.clone(),
                );
                progress_window.open(engine.user_interfaces.first());
                self.progress_window = Some(progress_window);

                if let Ok(input_data) = LightmapInputData::from_scene(
                    scene,
                    |handle, _| handle != game_scene.editor_objects_root,
                    cancellation_token.clone(),
                    progress_indicator.clone(),
                ) {
                    let sender = self.sender.clone();
                    let texels_per_unit = self.settings.texels_per_unit;
                    let spacing = self.settings.spacing;
                    let path = self.settings.path.clone();
                    let resource_manager = engine.resource_manager.clone();

                    if let Err(e) = std::thread::Builder::new()
                        .name("LightmapGenerationThread".to_string())
                        .spawn(move || {
                            match Lightmap::new(
                                input_data,
                                texels_per_unit,
                                spacing,
                                cancellation_token,
                                progress_indicator,
                            ) {
                                Ok(lightmap) => {
                                    if lightmap.save_textures(path, resource_manager).is_err() {
                                        sender
                                            .send(Err(LightmapGenerationError::Cancelled))
                                            .unwrap();
                                    } else {
                                        sender.send(Ok(lightmap)).unwrap();
                                    }
                                }
                                Err(err) => {
                                    sender.send(Err(err)).unwrap();
                                }
                            }
                        })
                    {
                        Log::err(format!(
                            "Failed to create a new lightmap generation thread. Reason: {e}"
                        ))
                    }
                }
            }

            if let Some(progress_window) = self.progress_window.as_ref() {
                if message.destination() == progress_window.cancel {
                    progress_window.cancellation_token.cancel();
                }
            }
        } else if let Some(InspectorMessage::PropertyChanged(args)) = message.data() {
            if message.destination() == self.inspector
                && message.direction() == MessageDirection::FromWidget
            {
                PropertyAction::from_field_kind(&args.value).apply(
                    &args.path(),
                    &mut self.settings,
                    &mut |result| {
                        Log::verify(result);
                    },
                );
            }
        }
    }

    pub fn update(&mut self, game_scene: &GameScene, engine: &mut Engine) {
        if let Some(progress_window) = self.progress_window.as_ref() {
            progress_window.show_progress(engine.user_interfaces.first());
        }

        if let Ok(result) = self.receiver.try_recv() {
            let scene = &mut engine.scenes[game_scene.scene];
            match result {
                Ok(lightmap) => {
                    if let Err(err) = scene.graph.set_lightmap(lightmap) {
                        Log::err(format!("Failed to set generated lightmap. Reason: {err}"));
                    }
                }
                Err(err) => {
                    Log::err(format!("Failed to generated a lightmap. Reason: {err}"));
                }
            }

            if let Some(progress_window) = self.progress_window.take() {
                progress_window.close(engine.user_interfaces.first());
            }
        }
    }

    pub fn is_in_preview_mode(&self) -> bool {
        self.progress_window.is_some()
    }
}
