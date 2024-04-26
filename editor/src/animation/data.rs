use super::*;
mod curve;
mod thumb;
mod track;
use curve::*;
use thumb::*;
use track::*;

pub struct AnimationDataEditor {
    pub window: Handle<UiNode>,
    track_list: TrackDataList,
    toolbar: Toolbar,
    content: Handle<UiNode>,
    thumb: ThumbDataView,
}

impl AnimationDataEditor {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let thumb = ThumbDataView::new(ctx);

        let track_list = TrackDataList::new(ctx);
        let toolbar = Toolbar::new(ctx);

        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(toolbar.panel)
                .with_child(track_list.content),
        )
        .add_row(Row::strict(26.0))
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let window = WindowBuilder::new(
            WidgetBuilder::new()
                .with_name("AnimationData")
                .with_width(600.0)
                .with_height(500.0),
        )
        .with_content(content)
        .open(false)
        .with_title(WindowTitle::text("Animation Data"))
        .build(ctx);

        Self {
            window,
            track_list,
            toolbar,
            content,
            thumb,
        }
    }

    pub fn open(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::open(
            self.window,
            MessageDirection::ToWidget,
            true,
            true,
        ));
    }

    pub fn handle_ui_message<P, G, N>(
        &mut self,
        message: &UiMessage,
        editor_selection: &Selection,
        graph: &mut G,
        root: Handle<N>,
        ui: &mut UserInterface,
        resource_manager: &ResourceManager,
        sender: &MessageSender,
        node_overrides: &mut FxHashSet<Handle<N>>,
    ) where
        P: PrefabData<Graph = G> + AnimationSource<Node = N, SceneGraph = G, Prefab = P>,
        G: SceneGraph<Node = N, Prefab = P>,
        N: SceneGraphNode<SceneGraph = G, ResourceData = P>,
    {
        let selection = fetch_selection(editor_selection);

        if let Some(container) = animation_container_ref(graph, selection.animation_player) {
            let toolbar_action = self.toolbar.handle_ui_message(
                message,
                sender,
                graph,
                ui,
                selection.animation_player,
                container,
                root,
                &selection,
            );

            let animations = animation_container(graph, selection.animation_player).unwrap();

            match toolbar_action {
                ToolbarAction::None => {}
                ToolbarAction::EnterPreviewMode => {
                    assert!(node_overrides.insert(selection.animation_player));

                    let animation_player_node =
                        graph.try_get_mut(selection.animation_player).unwrap();

                    // HACK. This is unreliable to just use `bool` here. It should be wrapped into
                    // newtype or something.
                    if let Some(auto_apply) = animation_player_node.component_mut::<bool>() {
                        *auto_apply = true;
                    } else {
                        Log::warn("No `auto_apply` component in animation player!")
                    }

                    // Save state of animation player first.
                    let initial_animation_player_handle = selection.animation_player;
                    let initial_animation_player = animation_player_node.clone();

                    // Now we can freely modify the state of the animation player in the scene - all
                    // changes will be reverted at the exit of the preview mode.
                    let animations =
                        animation_container(graph, selection.animation_player).unwrap();

                    // Disable every animation, except preview one.
                    for (handle, animation) in animations.pair_iter_mut() {
                        animation.set_enabled(handle == selection.animation);
                    }

                    if let Some(animation) = animations.try_get_mut(selection.animation) {
                        animation.rewind();

                        let animation_targets = animation
                            .tracks()
                            .iter()
                            .map(|t| t.target())
                            .collect::<FxHashSet<_>>();

                        self.enter_preview_mode(
                            initial_animation_player_handle,
                            initial_animation_player,
                            animation_targets,
                            graph,
                            ui,
                            node_overrides,
                        );
                    }
                }
                ToolbarAction::LeavePreviewMode => {
                    if self.preview_mode_data.is_some() {
                        self.leave_preview_mode(graph, ui, node_overrides);
                    }
                }
                ToolbarAction::SelectAnimation(animation) => (),
                ToolbarAction::PlayPause => {
                    if self.preview_mode_data.is_some() {
                        if let Some(animation) = animations.try_get_mut(selection.animation) {
                            animation.set_enabled(!animation.is_enabled());
                        }
                    }
                }
                ToolbarAction::Stop => {
                    if self.preview_mode_data.is_some() {
                        if let Some(animation) = animations.try_get_mut(selection.animation) {
                            animation.rewind();
                            animation.set_enabled(false);
                        }
                    }
                }
            }

            self.track_list
                .handle_ui_message(message, &selection, root, sender, ui, graph);
        }

        self.toolbar.post_handle_ui_message(
            message,
            sender,
            ui,
            selection.animation_player,
            graph,
            root,
            editor_selection,
            resource_manager,
        );
    }

    fn enter_preview_mode<G, N>(
        &mut self,
        initial_animation_player_handle: Handle<N>,
        initial_animation_player: N,
        animation_targets: FxHashSet<Handle<N>>,
        graph: &G,
        ui: &UserInterface,
        node_overrides: &mut FxHashSet<Handle<N>>,
    ) where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode,
    {
        assert!(self.preview_mode_data.is_none());

        self.toolbar.on_preview_mode_changed(ui, true);

        for &target in &animation_targets {
            assert!(node_overrides.insert(target));
        }

        let mut data = PreviewModeData {
            nodes: animation_targets
                .into_iter()
                .map(|t| (t, graph.node(t).clone()))
                .collect(),
        };

        data.nodes
            .push((initial_animation_player_handle, initial_animation_player));

        // Save state of affected nodes.
        self.preview_mode_data = Some(Box::new(data));
    }

    fn leave_preview_mode<G, N>(
        &mut self,
        graph: &mut G,
        ui: &UserInterface,
        node_overrides: &mut FxHashSet<Handle<N>>,
    ) where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode<SceneGraph = G>,
    {
        self.toolbar.on_preview_mode_changed(ui, false);

        let preview_data = self
            .preview_mode_data
            .take()
            .expect("Unable to leave animation preview mode!");

        // Revert state of nodes.
        for (handle, node) in preview_data.downcast::<PreviewModeData<N>>().unwrap().nodes {
            node_overrides.remove(&handle);
            *graph.node_mut(handle) = node;
        }
    }

    pub fn try_leave_preview_mode<G, N>(
        &mut self,
        graph: &mut G,
        ui: &UserInterface,
        node_overrides: &mut FxHashSet<Handle<N>>,
    ) where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode<SceneGraph = G>,
    {
        if self.preview_mode_data.is_some() {
            self.leave_preview_mode(graph, ui, node_overrides);
        }
    }

    pub fn is_in_preview_mode(&self) -> bool {
        self.preview_mode_data.is_some()
    }

    pub fn handle_message<G, N>(
        &mut self,
        message: &Message,
        graph: &mut G,
        ui: &UserInterface,
        node_overrides: &mut FxHashSet<Handle<N>>,
    ) where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode<SceneGraph = G>,
    {
        // Leave preview mode before execution of any scene command.
        if let Message::DoCommand(_)
        | Message::UndoCurrentSceneCommand
        | Message::RedoCurrentSceneCommand = message
        {
            self.try_leave_preview_mode(graph, ui, node_overrides);
        }
    }

    pub fn clear(&mut self, ui: &UserInterface) {
        self.toolbar.clear(ui);
        self.track_list.clear(ui);
    }

    pub fn update<G, N>(&mut self, editor_selection: &Selection, ui: &UserInterface, graph: &G)
    where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode<SceneGraph = G>,
    {
        let selection = fetch_selection(editor_selection);

        if let Some(container) = animation_container_ref(graph, selection.animation_player) {
            if let Some(animation) = container.try_get(selection.animation) {
                if self.thumb.update_thumb(animation.time_position(), ui) {
                    self.update_values(editor_selection, ui, graph);
                }
            }
        }
    }

    fn update_values<G, N>(&mut self, editor_selection: &Selection, ui: &UserInterface, graph: &G)
    where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode<SceneGraph = G>,
    {
        todo!()
    }

    pub fn sync_to_model<G, N>(
        &mut self,
        editor_selection: &Selection,
        ui: &mut UserInterface,
        graph: &G,
    ) where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode<SceneGraph = G>,
    {
        let selection = fetch_selection(editor_selection);

        let mut is_animation_player_selected = false;
        let mut is_animation_selected = false;
        let mut is_curve_selected = false;

        if let Some(animations) = animation_container_ref(graph, selection.animation_player) {
            self.toolbar.sync_to_model(
                animations,
                &selection,
                graph,
                ui,
                self.preview_mode_data.is_some(),
            );

            if let Some(animation) = animations.try_get(selection.animation) {
                self.track_list
                    .sync_to_model(animation, graph, &selection, ui);
                self.update_values(editor_selection, ui, graph);

                is_animation_selected = true;
            }
            is_animation_player_selected = true;
        }

        if !is_animation_selected || !is_animation_player_selected {
            self.track_list.clear(ui);

            send_sync_message(
                ui,
                CurveEditorMessage::zoom(
                    self.curve_editor,
                    MessageDirection::ToWidget,
                    Vector2::new(1.0, 1.0),
                ),
            );
            send_sync_message(
                ui,
                CurveEditorMessage::view_position(
                    self.curve_editor,
                    MessageDirection::ToWidget,
                    Vector2::default(),
                ),
            );
        }

        if !is_animation_selected || !is_animation_player_selected || !is_curve_selected {
            send_sync_message(
                ui,
                CurveEditorMessage::sync(
                    self.curve_editor,
                    MessageDirection::ToWidget,
                    Default::default(),
                ),
            );
        }

        if !is_animation_player_selected {
            self.toolbar.clear(ui);
        }

        send_sync_message(
            ui,
            WidgetMessage::visibility(
                self.content,
                MessageDirection::ToWidget,
                is_animation_player_selected,
            ),
        );
        send_sync_message(
            ui,
            WidgetMessage::enabled(
                self.track_list.panel,
                MessageDirection::ToWidget,
                is_animation_selected,
            ),
        );
        send_sync_message(
            ui,
            CheckBoxMessage::checked(
                self.toolbar.preview,
                MessageDirection::ToWidget,
                Some(self.preview_mode_data.is_some()),
            ),
        );
        send_sync_message(
            ui,
            WidgetMessage::enabled(
                self.curve_editor,
                MessageDirection::ToWidget,
                is_curve_selected,
            ),
        );
    }
}
