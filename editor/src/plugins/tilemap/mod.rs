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

#![allow(clippy::collapsible_match)] // STFU

mod collider_editor;
mod colliders_tab;
mod commands;
mod handle_field;
mod interaction_mode;
mod misc;
pub mod palette;
pub mod panel;
mod panel_preview;
mod preview;
mod properties_tab;
mod tile_bounds_editor;
mod tile_editor;
mod tile_inspector;
mod tile_prop_editor;
pub mod tile_set_import;
pub mod tileset;

use collider_editor::*;
use colliders_tab::*;
use fyrox::gui::style::resource::StyleResourceExt;
use fyrox::gui::style::Style;
use fyrox::scene::tilemap::TileMapEditorDataRef;
use fyrox::{gui::message::KeyCode, scene::tilemap::TileMapEditorData};
use handle_field::*;
use interaction_mode::*;
use palette::PaletteWidget;
use panel::TileMapPanel;
use panel_preview::*;
use properties_tab::*;
use tile_bounds_editor::*;
use tile_editor::*;
use tile_inspector::*;
use tile_prop_editor::*;

use crate::fyrox::{
    asset::untyped::UntypedResource,
    core::{
        algebra::{Matrix4, Vector2, Vector3},
        color::Color,
        math::{plane::Plane, Matrix4Ext},
        parking_lot::{Mutex, MutexGuard},
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        visitor::prelude::*,
        Uuid,
    },
    engine::Engine,
    fxhash::FxHashSet,
    graph::{BaseSceneGraph, SceneGraph, SceneGraphNode},
    gui::{
        border::BorderBuilder,
        brush::Brush,
        button::ButtonBuilder,
        decorator::DecoratorBuilder,
        image::ImageBuilder,
        key::HotKey,
        message::{MessageDirection, UiMessage},
        utils::make_simple_tooltip,
        widget::{WidgetBuilder, WidgetMessage},
        BuildContext, Thickness, UiNode, UserInterface,
    },
    scene::{
        debug::Line,
        node::Node,
        tilemap::{
            brush::TileMapBrush,
            tileset::{TileSet, TileSetResource},
            RandomTileSource, Stamp, TileCollider, TileDefinitionHandle, TileMap, TilePaletteStage,
            TileResource, Tiles,
        },
        Scene,
    },
};
use crate::{
    interaction::{make_interaction_mode_button, InteractionMode},
    load_image,
    message::MessageSender,
    plugin::EditorPlugin,
    plugins::tilemap::{palette::PaletteMessage, preview::TileSetPreview, tileset::TileSetEditor},
    scene::{controller::SceneController, GameScene, Selection},
    settings::Settings,
    Editor, Message,
};
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::Arc,
};

lazy_static! {
    static ref VISIBLE_IMAGE: Option<UntypedResource> =
        load_image!("../../../resources/visible.png");
    static ref BRUSH_IMAGE: Option<UntypedResource> = load_image!("../../../resources/brush.png");
    static ref ERASER_IMAGE: Option<UntypedResource> = load_image!("../../../resources/eraser.png");
    static ref FILL_IMAGE: Option<UntypedResource> = load_image!("../../../resources/fill.png");
    static ref PICK_IMAGE: Option<UntypedResource> = load_image!("../../../resources/pipette.png");
    static ref RECT_FILL_IMAGE: Option<UntypedResource> =
        load_image!("../../../resources/rect_fill.png");
    static ref NINE_SLICE_IMAGE: Option<UntypedResource> =
        load_image!("../../../resources/nine_slice.png");
    static ref LINE_IMAGE: Option<UntypedResource> = load_image!("../../../resources/line.png");
    static ref TURN_LEFT_IMAGE: Option<UntypedResource> =
        load_image!("../../../resources/turn_left.png");
    static ref TURN_RIGHT_IMAGE: Option<UntypedResource> =
        load_image!("../../../resources/turn_right.png");
    static ref FLIP_X_IMAGE: Option<UntypedResource> = load_image!("../../../resources/flip_x.png");
    static ref FLIP_Y_IMAGE: Option<UntypedResource> = load_image!("../../../resources/flip_y.png");
    static ref RANDOM_IMAGE: Option<UntypedResource> = load_image!("../../../resources/die.png");
    static ref PALETTE_IMAGE: Option<UntypedResource> =
        load_image!("../../../resources/palette.png");
}

fn make_button(
    title: &str,
    tooltip: &str,
    enabled: bool,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_enabled(enabled)
            .with_width(100.0)
            .with_height(24.0)
            .with_margin(Thickness::uniform(1.0))
            .with_tooltip(make_simple_tooltip(ctx, tooltip)),
    )
    .with_text(title)
    .build(ctx)
}

fn make_drawing_mode_button(
    ctx: &mut BuildContext,
    width: f32,
    height: f32,
    image: Option<UntypedResource>,
    tooltip: &str,
    tab_index: Option<usize>,
) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_tab_index(tab_index)
            .with_tooltip(make_simple_tooltip(ctx, tooltip))
            .with_margin(Thickness::uniform(1.0)),
    )
    .with_back(
        DecoratorBuilder::new(
            BorderBuilder::new(
                WidgetBuilder::new().with_foreground(ctx.style.property(Style::BRUSH_DARKER)),
            )
            .with_pad_by_corner_radius(false)
            .with_corner_radius((4.0).into())
            .with_stroke_thickness(Thickness::uniform(1.0).into()),
        )
        .with_selected_brush(ctx.style.property(Style::BRUSH_BRIGHT_BLUE))
        .with_normal_brush(ctx.style.property(Style::BRUSH_LIGHT))
        .with_hover_brush(ctx.style.property(Style::BRUSH_LIGHTER))
        .with_pressed_brush(ctx.style.property(Style::BRUSH_LIGHTEST))
        .build(ctx),
    )
    .with_content(
        ImageBuilder::new(
            WidgetBuilder::new()
                .with_background(Brush::Solid(Color::opaque(180, 180, 180)).into())
                .with_margin(Thickness::uniform(2.0))
                .with_width(width)
                .with_height(height),
        )
        .with_opt_texture(image)
        .build(ctx),
    )
    .build(ctx)
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Visit, Reflect)]
pub enum DrawingMode {
    #[default]
    Draw,
    Erase,
    FloodFill,
    Pick,
    RectFill,
    NineSlice,
    Line,
    Editor,
}

#[derive(Debug, PartialEq, Clone)]
struct OpenTilePanelMessage {
    resource: TileResource,
    center: Option<TileDefinitionHandle>,
}

impl OpenTilePanelMessage {
    fn message(resource: TileResource, center: Option<TileDefinitionHandle>) -> UiMessage {
        UiMessage::with_data(Self { resource, center })
    }
}

#[derive(Debug, PartialEq, Clone)]
struct DelayedMessage {
    delay_frames: usize,
    content: UiMessage,
}

impl DelayedMessage {
    fn message(delay_frames: usize, content: UiMessage) -> UiMessage {
        UiMessage::with_data(Self {
            delay_frames,
            content,
        })
    }
}

#[derive(Default)]
pub struct TileMapEditorPlugin {
    editor_data: Option<TileMapEditorDataRef>,
    state: TileDrawStateRef,
    tile_set_editor: Option<TileSetEditor>,
    panel: Option<TileMapPanel>,
    tile_map: Handle<Node>,
    delayed_messages: Vec<DelayedMessage>,
}

#[derive(Default, Clone, Visit)]
pub struct TileDrawState {
    /// True if the state has been changed and the change has not yet caused the UI to update.
    dirty: bool,
    /// The tile set that contains the definitions of the tiles that are being edited.
    tile_set: Option<TileSetResource>,
    /// The current stamp that the user uses when drawing tiles to a tile set, brush, or tile map.
    stamp: Stamp,
    /// The tool that the user has selected for editing tiles: Draw, Pick, Rectangle, Fill, etc.
    drawing_mode: DrawingMode,
    /// If the user is editing a tile set by drawing an editor value, then this is the editor.
    #[visit(skip)]
    active_editor: Option<TileEditorRef>,
    /// The UUIDs of the colliders that are currently visible to the user.
    #[visit(skip)]
    visible_colliders: FxHashSet<Uuid>,
    /// Does the user want tiles to be randomized?
    random_mode: bool,
    /// The currently selected tiles.
    selection: TileDrawSelection,
}

impl Debug for TileDrawState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TileDrawState")
            .field("dirty", &self.dirty)
            .field("tile_set", &self.tile_set)
            .field("stamp", &self.stamp)
            .field("drawing_mode", &self.drawing_mode)
            .field("random_mode", &self.random_mode)
            .field("selection", &self.selection)
            .finish()
    }
}

type TileEditorRef = Arc<Mutex<dyn TileEditor>>;

#[derive(Debug, Default, Clone)]
pub struct TileDrawStateRef(Arc<Mutex<TileDrawState>>);
pub struct TileDrawStateGuard<'a>(MutexGuard<'a, TileDrawState>);
pub struct TileDrawStateGuardMut<'a>(MutexGuard<'a, TileDrawState>);

impl Deref for TileDrawStateGuard<'_> {
    type Target = TileDrawState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for TileDrawStateGuardMut<'_> {
    type Target = TileDrawState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TileDrawStateGuardMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

const STATE_UPDATE_DEBUG: bool = false;

impl TileDrawStateRef {
    pub fn lock(&self) -> TileDrawStateGuard {
        TileDrawStateGuard(self.0.try_lock().expect("State lock failed."))
    }
    pub fn lock_mut(&self, reason: &str) -> TileDrawStateGuardMut {
        self.lock().into_mut(reason)
    }
    pub fn check_dirty(&self) -> bool {
        let mut state = self.0.lock();
        let dirty = state.dirty;
        state.dirty = false;
        dirty
    }
}

impl<'a> TileDrawStateGuard<'a> {
    pub fn into_mut(self, reason: &str) -> TileDrawStateGuardMut<'a> {
        if STATE_UPDATE_DEBUG {
            println!("State Update: {reason}");
        }
        let mut result = TileDrawStateGuardMut(self.0);
        result.dirty = true;
        result
    }
}

impl<'a> TileDrawStateGuardMut<'a> {
    pub fn into_const(self) -> TileDrawStateGuard<'a> {
        TileDrawStateGuard(self.0)
    }
}

#[derive(Default, Debug, Clone, Visit)]
struct TileDrawSelection {
    /// The selection either comes from a [`PaletteWidget`] or a tile map node.
    /// This field allows each object to check if it its tiles are selected.
    pub source: SelectionSource,
    /// The page of the currently selected tiles.
    pub page: Vector2<i32>,
    /// The currently selected cells.
    pub positions: FxHashSet<Vector2<i32>>,
}

impl TileDrawState {
    /// True if the given editor is the active editor.
    #[inline]
    pub fn is_active_editor(&self, editor: &TileEditorRef) -> bool {
        if let Some(active) = &self.active_editor {
            Arc::ptr_eq(editor, active)
        } else {
            false
        }
    }
    /// Set whether the given collider is visible.
    pub fn set_visible_collider(&mut self, uuid: Uuid, visible: bool) {
        if visible {
            let _ = self.visible_colliders.insert(uuid);
        } else {
            let _ = self.visible_colliders.remove(&uuid);
        }
    }
    /// True if the current selection is not empty
    #[inline]
    pub fn has_selection(&self) -> bool {
        !self.selection.positions.is_empty()
    }
    /// The handle of the palette widget that is currently being used to select tiles, or else Handle::NONE.
    #[inline]
    pub fn selection_palette(&self) -> Handle<UiNode> {
        match self.selection.source {
            SelectionSource::Widget(h) => h,
            _ => Handle::NONE,
        }
    }
    /// The handle of the tile map node that is currently being used to select tiles, or else Handle::NONE.
    #[inline]
    pub fn selection_node(&self) -> Handle<Node> {
        match self.selection.source {
            SelectionSource::Node(h) => h,
            _ => Handle::NONE,
        }
    }
    #[inline]
    pub fn set_palette(&mut self, handle: Handle<UiNode>) {
        self.selection.source = SelectionSource::Widget(handle);
    }
    #[inline]
    pub fn set_node(&mut self, handle: Handle<Node>) {
        self.selection.source = SelectionSource::Node(handle);
    }
    #[inline]
    pub fn selection_positions(&self) -> &FxHashSet<Vector2<i32>> {
        &self.selection.positions
    }
    #[inline]
    pub fn selection_positions_mut(&mut self) -> &mut FxHashSet<Vector2<i32>> {
        &mut self.selection.positions
    }
    #[inline]
    pub fn clear_selection(&mut self) {
        self.stamp.clear();
        self.selection.positions.clear();
        self.selection.source = SelectionSource::None;
    }
    #[inline]
    pub fn update_stamp<F>(&mut self, tile_set: Option<TileSetResource>, tile_handle: F)
    where
        F: Fn(Vector2<i32>) -> Option<TileDefinitionHandle>,
    {
        self.tile_set = tile_set;
        self.stamp.build(
            self.selection
                .positions
                .iter()
                .copied()
                .filter_map(|p| Some((p, tile_handle(p)?))),
        );
    }
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Visit)]
pub enum SelectionSource {
    #[default]
    None,
    Widget(Handle<UiNode>),
    Node(Handle<Node>),
}

impl TileMapEditorPlugin {
    fn get_tile_map_mut<'a>(&self, editor: &'a mut Editor) -> Option<&'a mut TileMap> {
        let entry = editor.scenes.current_scene_entry_mut()?;
        let game_scene = entry.controller.downcast_mut::<GameScene>()?;
        let scene = &mut editor.engine.scenes[game_scene.scene];
        let node = scene.graph.try_get_mut(self.tile_map)?;
        node.component_mut::<TileMap>()
    }
    fn open_panel_for_tile_set(
        &mut self,
        resource: TileResource,
        center: Option<TileDefinitionHandle>,
        ui: &mut UserInterface,
        sender: &MessageSender,
    ) {
        if let Some(panel) = &mut self.panel {
            panel.to_top(ui);
        } else if let Some(editor) = &self.tile_set_editor {
            let panel = TileMapPanel::new(&mut ui.build_ctx(), self.state.clone(), sender.clone());
            panel.align(editor.window, ui);
            self.panel = Some(panel);
        }
        if let Some(panel) = &mut self.panel {
            panel.set_resource(resource, ui);
            if let Some(focus) = center {
                panel.set_focus(focus, ui);
            }
        }
    }
    fn open_panel_for_tile_map(&mut self, editor: &mut Editor) {
        let resource = if let Some(tile_map) = self.get_tile_map_mut(editor) {
            if let Some(brush) = tile_map.active_brush() {
                TileResource::Brush(brush.clone())
            } else if let Some(tile_set) = tile_map.tile_set() {
                TileResource::TileSet(tile_set.clone())
            } else {
                TileResource::Empty
            }
        } else {
            return;
        };

        let ui = editor.engine.user_interfaces.first_mut();

        if let Some(panel) = &mut self.panel {
            panel.to_top(ui);
            panel.set_resource(resource, ui);
        } else {
            let mut panel = TileMapPanel::new(
                &mut ui.build_ctx(),
                self.state.clone(),
                editor.message_sender.clone(),
            );
            panel.align(editor.scene_viewer.frame(), ui);
            panel.set_resource(resource, ui);
            self.panel = Some(panel);
        }
    }
    fn update_state(&mut self) {
        let state = self.state.lock();
        if match state.drawing_mode {
            DrawingMode::Pick => false,
            DrawingMode::Editor => self.tile_set_editor.is_none(),
            _ => self.panel.is_none(),
        } {
            let mut state = state.into_mut("update_state");
            state.drawing_mode = DrawingMode::Pick;
            state.active_editor = None;
        } else if state.drawing_mode != DrawingMode::Editor && state.active_editor.is_some() {
            state
                .into_mut("update_state: drawing_mode != Editor")
                .active_editor = None;
        }
    }
    fn send_delayed_messages(&mut self, ui: &mut UserInterface) {
        let msgs = &mut self.delayed_messages;
        for dm in msgs.iter_mut() {
            dm.delay_frames = dm.delay_frames.saturating_sub(1);
        }
        let mut i = 0;
        while i < msgs.len() {
            if msgs[i].delay_frames == 0 {
                let m = msgs.swap_remove(i);
                ui.send_message(m.content);
            } else {
                i += 1;
            }
        }
    }
    fn on_tile_map_selected(&mut self, handle: Handle<Node>, editor: &mut Editor) {
        // Set the new tile map as the currently edited tile map.
        self.tile_map = handle;
        // Create new editor data and add it to the tile map, so the tile map node
        // will now render itself as being edited.
        let Some(tile_map) = self.get_tile_map_mut(editor) else {
            return;
        };
        let editor_data = Arc::new(Mutex::new(TileMapEditorData::default()));
        tile_map.editor_data = Some(editor_data.clone());
        self.editor_data = Some(editor_data.clone());
        // Prepare the tile map interaction mode.
        let Some(entry) = editor.scenes.current_scene_entry_mut() else {
            return;
        };
        entry.interaction_modes.add(TileMapInteractionMode::new(
            handle,
            self.state.clone(),
            editor.message_sender.clone(),
            editor_data,
        ));
    }
}

impl EditorPlugin for TileMapEditorPlugin {
    fn on_start(&mut self, editor: &mut Editor) {
        editor
            .asset_browser
            .preview_generators
            .add(TileSet::type_uuid(), TileSetPreview);
    }

    fn on_exit(&mut self, _editor: &mut Editor) {
        if let Some(tile_set_editor) = self.tile_set_editor.as_mut() {
            tile_set_editor.try_save();
        }
    }

    fn on_suspended(&mut self, _editor: &mut Editor) {}

    fn on_mode_changed(&mut self, _editor: &mut Editor) {
        if let Some(tile_set_editor) = self.tile_set_editor.as_mut() {
            tile_set_editor.try_save();
        }
    }

    fn on_sync_to_model(&mut self, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();

        let palette = self.state.lock().selection_palette();
        if let Some(palette) = ui
            .try_get_mut(palette)
            .and_then(|p| p.cast_mut::<PaletteWidget>())
        {
            palette.sync_selection_to_model();
        }

        if let Some(tile_set_editor) = self.tile_set_editor.as_mut() {
            tile_set_editor.sync_to_model(ui);
        }
        if let Some(panel) = self.panel.as_mut() {
            panel.sync_to_model(ui);
        }
    }

    fn on_ui_message(&mut self, message: &mut UiMessage, editor: &mut Editor) {
        if let Some(delayed_message) = message.data::<DelayedMessage>() {
            self.delayed_messages.push(delayed_message.clone());
            return;
        }

        let ui = editor.engine.user_interfaces.first_mut();

        if let Some(tile_set_editor) = self.tile_set_editor.take() {
            self.tile_set_editor = tile_set_editor.handle_ui_message(
                message,
                ui,
                &editor.engine.resource_manager,
                &editor.message_sender,
                editor.engine.serialization_context.clone(),
            );
        }

        if let Some(OpenTilePanelMessage { resource, center }) = message.data() {
            self.open_panel_for_tile_set(resource.clone(), *center, ui, &editor.message_sender);
        }

        if let Some(panel) = self.panel.take() {
            let editor_scene_entry = editor.scenes.current_scene_entry_mut();

            let tile_map = editor_scene_entry
                .as_ref()
                .and_then(|entry| entry.controller.downcast_ref::<GameScene>())
                .and_then(|scene| {
                    editor.engine.scenes[scene.scene]
                        .graph
                        .try_get_of_type::<TileMap>(self.tile_map)
                });

            self.panel = panel.handle_ui_message(
                message,
                ui,
                self.tile_map,
                tile_map,
                &editor.message_sender,
                editor_scene_entry,
            );
        }
    }

    fn on_update(&mut self, editor: &mut Editor) {
        if let Some(tile_set_editor) = self.tile_set_editor.as_mut() {
            tile_set_editor.update();
        }

        self.send_delayed_messages(editor.engine.user_interfaces.first_mut());

        self.update_state();

        if self.state.check_dirty() {
            if let Some(tile_set_editor) = self.tile_set_editor.as_mut() {
                tile_set_editor.sync_to_state(editor.engine.user_interfaces.first_mut());
            }
            if let Some(panel) = self.panel.as_mut() {
                panel.sync_to_state(editor.engine.user_interfaces.first_mut());
            }
            if let Some(interaction_mode) = editor
                .scenes
                .current_scene_entry_mut()
                .and_then(|s| s.interaction_modes.of_type_mut::<TileMapInteractionMode>())
            {
                interaction_mode.sync_to_state();
            }
        }
    }

    fn on_message(&mut self, message: &Message, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();

        let tile_resource: Option<TileResource> =
            if let Message::OpenTileSetEditor(tile_set) = message {
                Some(TileResource::TileSet(tile_set.clone()))
            } else if let Message::OpenTileMapBrushEditor(brush) = message {
                Some(TileResource::Brush(brush.clone()))
            } else {
                None
            };

        if let Some(tile_resource) = tile_resource {
            if self.tile_set_editor.is_none() {
                let mut tile_set_editor = TileSetEditor::new(
                    tile_resource.clone(),
                    self.state.clone(),
                    editor.message_sender.clone(),
                    editor.engine.resource_manager.clone(),
                    &mut ui.build_ctx(),
                );
                tile_set_editor.set_tile_resource(tile_resource, ui);
                self.tile_set_editor = Some(tile_set_editor);
            } else if let Some(editor) = &mut self.tile_set_editor {
                editor.set_tile_resource(tile_resource.clone(), ui);
            }
        }

        if let Message::SetInteractionMode(uuid) = message {
            if *uuid == TileMapInteractionMode::type_uuid() && self.panel.is_none() {
                if let Some(tile_map) = self.get_tile_map_mut(editor) {
                    let resource = if let Some(brush) = tile_map.active_brush() {
                        TileResource::Brush(brush.clone())
                    } else if let Some(tile_set) = tile_map.tile_set() {
                        TileResource::TileSet(tile_set.clone())
                    } else {
                        TileResource::Empty
                    };
                    if !resource.is_empty() {
                        self.open_panel_for_tile_map(editor);
                    }
                }
            }
        }

        let Some(entry) = editor.scenes.current_scene_entry_mut() else {
            return;
        };

        let Some(selection) = entry.selection.as_graph() else {
            return;
        };

        let Some(game_scene) = entry.controller.downcast_mut::<GameScene>() else {
            return;
        };

        if let Message::SelectionChanged { .. } = message {
            let scene = &mut editor.engine.scenes[game_scene.scene];
            entry
                .interaction_modes
                .remove_typed::<TileMapInteractionMode>();

            // Remove the editor data from the currently selected tile map, so it will render as normal.
            if let Some(tile_map) = scene
                .graph
                .try_get_mut(self.tile_map)
                .and_then(|n| n.component_mut::<TileMap>())
            {
                tile_map.editor_data = None;
            }

            if let Some(handle) = selection.nodes().iter().copied().find(|h| {
                scene
                    .graph
                    .try_get(*h)
                    .and_then(|n| n.component_ref::<TileMap>())
                    .is_some()
            }) {
                self.on_tile_map_selected(handle, editor);
            }
        }
    }
}
