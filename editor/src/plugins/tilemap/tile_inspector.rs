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

use std::fmt::Debug;

use crate::{
    command::{Command, CommandGroup},
    plugins::material::editor::{MaterialFieldEditorBuilder, MaterialFieldMessage},
    send_sync_message, MSG_SYNC_FLAG,
};
use brush::TileMapBrushPage;
use fyrox::{
    asset::{manager::ResourceManager, ResourceDataRef},
    core::{
        algebra::Vector2, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        visitor::prelude::*,
    },
    gui::{
        button::{Button, ButtonMessage},
        decorator::DecoratorMessage,
        expander::ExpanderBuilder,
        grid::{Column, GridBuilder, Row},
        message::UiMessage,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        vec::{Vec2EditorBuilder, Vec2EditorMessage},
        widget::WidgetBuilder,
        BuildContext, UiNode, UserInterface,
    },
    material::{MaterialResource, MaterialResourceExtension},
    scene::tilemap::{tileset::*, *},
};

use super::*;
use commands::*;
use palette::*;

pub const FIELD_LABEL_WIDTH: f32 = 100.0;

struct OptionIterator<I>(Option<I>);

impl<I: Iterator> Iterator for OptionIterator<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.as_mut()?.next()
    }
}

pub struct TileEditorStateRef {
    pub page: Option<Vector2<i32>>,
    pub pages_palette: Handle<UiNode>,
    pub tiles_palette: Handle<UiNode>,
    pub state: TileDrawStateRef,
    pub tile_resource: TileResource,
}

impl TileEditorStateRef {
    pub fn lock(&self) -> TileEditorState {
        TileEditorState {
            page: self.page,
            pages_palette: self.pages_palette,
            tiles_palette: self.tiles_palette,
            state: Some(self.state.lock()),
            data: TileResourceData::new(&self.tile_resource),
        }
    }
}

pub struct TileEditorState<'a> {
    page: Option<Vector2<i32>>,
    pages_palette: Handle<UiNode>,
    tiles_palette: Handle<UiNode>,
    state: Option<TileDrawStateGuard<'a>>,
    data: TileResourceData<'a>,
}

enum TileResourceData<'a> {
    Empty,
    TileSet(ResourceDataRef<'a, TileSet>),
    Brush(ResourceDataRef<'a, TileMapBrush>),
}

impl Debug for TileResourceData<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "Empty"),
            Self::TileSet(_) => write!(f, "TileSet(..)"),
            Self::Brush(_) => write!(f, "Brush(..)"),
        }
    }
}

impl<'a> TileResourceData<'a> {
    fn new(tile_resource: &'a TileResource) -> Self {
        match tile_resource {
            TileResource::Empty => Self::Empty,
            TileResource::TileSet(resource) => Self::TileSet(resource.data_ref()),
            TileResource::Brush(resource) => Self::Brush(resource.data_ref()),
        }
    }
    fn tile_set(&self) -> Option<&ResourceDataRef<'a, TileSet>> {
        if let Self::TileSet(v) = self {
            Some(v)
        } else {
            None
        }
    }
    fn brush(&self) -> Option<&ResourceDataRef<'a, TileMapBrush>> {
        if let Self::Brush(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl<'a> TileEditorState<'a> {
    fn is_tile_set(&self) -> bool {
        self.tile_set().is_some()
    }
    fn is_brush(&self) -> bool {
        self.brush().is_some()
    }
    fn state(&self) -> &TileDrawStateGuard<'a> {
        self.state.as_ref().unwrap()
    }
    pub fn is_active_editor(&self, editor: &TileEditorRef) -> bool {
        self.state().is_active_editor(editor)
    }
    pub fn is_visible_collider(&self, uuid: Uuid) -> bool {
        self.state().visible_colliders.contains(&uuid)
    }
    pub fn visible_colliders(&self) -> impl Iterator<Item = &Uuid> {
        self.state().visible_colliders.iter()
    }
    pub fn drawing_mode(&self) -> DrawingMode {
        self.state().drawing_mode
    }
    /// Force the UI to update itself as if the state had changed.
    pub fn touch(&mut self) {
        let state = self.state.take().unwrap().into_mut("touch");
        self.state = Some(state.into_const());
    }
    pub fn set_active_editor(&mut self, editor: Option<TileEditorRef>) {
        let mut state = self.state.take().unwrap().into_mut("set_active_editor");
        state.active_editor = editor;
        self.state = Some(state.into_const());
    }
    pub fn set_drawing_mode(&mut self, mode: DrawingMode) {
        let mut state = self.state.take().unwrap().into_mut("set_drawing_mode");
        state.drawing_mode = mode;
        self.state = Some(state.into_const());
    }
    pub fn set_visible_collider(&mut self, uuid: Uuid, visible: bool) {
        let mut state = self.state.take().unwrap().into_mut("set_visible_collider");
        state.set_visible_collider(uuid, visible);
        self.state = Some(state.into_const());
    }
    pub fn tile_set(&self) -> Option<&ResourceDataRef<'a, TileSet>> {
        self.data.tile_set()
    }
    pub fn brush(&self) -> Option<&ResourceDataRef<'a, TileMapBrush>> {
        self.data.brush()
    }
    pub fn page(&self) -> Option<Vector2<i32>> {
        self.page
    }
    pub fn has_pages(&self) -> bool {
        self.state().selection_palette() == self.pages_palette && self.state().has_selection()
    }
    pub fn has_tiles(&self) -> bool {
        self.state().selection_palette() == self.tiles_palette && self.state().has_selection()
    }
    pub fn tiles_count(&self) -> usize {
        if self.state().selection_palette() == self.tiles_palette {
            self.state().selection_positions().len()
        } else {
            0
        }
    }
    pub fn pages_count(&self) -> usize {
        if self.state().selection_palette() == self.pages_palette {
            self.state().selection_positions().len()
        } else {
            0
        }
    }
    pub fn selected_positions(&self) -> impl Iterator<Item = Vector2<i32>> + '_ {
        self.state().selection_positions().iter().copied()
    }
    pub fn find_property(&self, property_id: Uuid) -> Option<&TileSetPropertyLayer> {
        self.tile_set()?.find_property(property_id)
    }
    pub fn find_collider(&self, collider_id: Uuid) -> Option<&TileSetColliderLayer> {
        self.tile_set()?.find_collider(collider_id)
    }
    pub fn properties(&self) -> impl Iterator<Item = &TileSetPropertyLayer> {
        OptionIterator(self.tile_set().map(|d| d.properties.iter()))
    }
    pub fn colliders(&self) -> impl Iterator<Item = &TileSetColliderLayer> {
        OptionIterator(self.tile_set().map(|d| d.colliders.iter()))
    }
    pub fn page_positions(&self) -> impl Iterator<Item = Vector2<i32>> + '_ {
        if self.state().selection_palette() == self.pages_palette {
            OptionIterator(Some(self.state().selection_positions().iter().copied()))
        } else {
            OptionIterator(None)
        }
    }
    pub fn empty_page_positions(&self) -> impl Iterator<Item = Vector2<i32>> + '_ {
        if self.state().selection_palette() == self.pages_palette {
            OptionIterator(Some(
                self.state()
                    .selection_positions()
                    .iter()
                    .copied()
                    .filter(|p| {
                        if let Some(tile_set) = self.tile_set() {
                            !tile_set.pages.contains_key(p)
                        } else if let Some(brush) = self.brush() {
                            !brush.pages.contains_key(p)
                        } else {
                            false
                        }
                    }),
            ))
        } else {
            OptionIterator(None)
        }
    }
    pub fn tile_set_pages(&self) -> impl Iterator<Item = (Vector2<i32>, &TileSetPage)> {
        if self.state().selection_palette() == self.pages_palette {
            OptionIterator(Some(
                self.state()
                    .selection_positions()
                    .iter()
                    .copied()
                    .filter_map(|p| Some((p, self.tile_set()?.pages.get(&p)?))),
            ))
        } else {
            OptionIterator(None)
        }
    }
    pub fn brush_pages(&self) -> impl Iterator<Item = (Vector2<i32>, &TileMapBrushPage)> {
        if self.state().selection_palette() == self.pages_palette {
            OptionIterator(Some(
                self.state()
                    .selection_positions()
                    .iter()
                    .copied()
                    .filter_map(|p| Some((p, self.brush()?.pages.get(&p)?))),
            ))
        } else {
            OptionIterator(None)
        }
    }
    pub fn material_page(&self) -> Option<(Vector2<i32>, &TileMaterial)> {
        let mut pages = self.tile_set_pages();
        let result = pages.next();
        if pages.next().is_some() {
            return None;
        }
        let (position, page) = result?;
        if let TileSetPageSource::Material(m) = &page.source {
            Some((position, m))
        } else {
            None
        }
    }
    pub fn is_material_page(&self, position: Vector2<i32>) -> bool {
        match &self.data {
            TileResourceData::Empty => false,
            TileResourceData::TileSet(tile_set) => {
                if let Some(page) = tile_set.pages.get(&position) {
                    page.is_material()
                } else {
                    false
                }
            }
            TileResourceData::Brush(_) => false,
        }
    }
    pub fn is_freeform_page(&self, position: Vector2<i32>) -> bool {
        match &self.data {
            TileResourceData::Empty => false,
            TileResourceData::TileSet(tile_set) => {
                if let Some(page) = tile_set.pages.get(&position) {
                    page.is_freeform()
                } else {
                    false
                }
            }
            TileResourceData::Brush(_) => false,
        }
    }
    pub fn is_transform_page(&self, position: Vector2<i32>) -> bool {
        match &self.data {
            TileResourceData::Empty => false,
            TileResourceData::TileSet(tile_set) => {
                if let Some(page) = tile_set.pages.get(&position) {
                    page.is_transform_set()
                } else {
                    false
                }
            }
            TileResourceData::Brush(_) => false,
        }
    }
    pub fn is_brush_page(&self, position: Vector2<i32>) -> bool {
        match &self.data {
            TileResourceData::Empty => false,
            TileResourceData::TileSet(_) => false,
            TileResourceData::Brush(brush) => brush.pages.contains_key(&position),
        }
    }
    pub fn tile_handles(&self) -> impl Iterator<Item = TileDefinitionHandle> + '_ {
        let page = self.page;
        self.state()
            .selection_positions()
            .iter()
            .copied()
            .filter_map(move |p| TileDefinitionHandle::try_new(page?, p))
    }
    pub fn empty_tiles(&self) -> impl Iterator<Item = TileDefinitionHandle> + '_ {
        let page = self.page;
        self.state()
            .selection_positions()
            .iter()
            .copied()
            .filter_map(move |p| TileDefinitionHandle::try_new(page?, p))
            .filter(|h| {
                let Some(tile_set) = self.tile_set() else {
                    return false;
                };
                tile_set.is_free_at(TilePaletteStage::Tiles, h.page(), h.tile())
            })
    }
    pub fn tile_material_bounds(
        &self,
    ) -> impl Iterator<Item = (TileDefinitionHandle, &TileMaterialBounds)> {
        let page = self.page;
        self.state()
            .selection_positions()
            .iter()
            .copied()
            .filter_map(move |p| {
                let handle = TileDefinitionHandle::try_new(page?, p)?;
                Some((handle, self.tile_set()?.tile_bounds(handle)?))
            })
    }
    pub fn tile_data(&self) -> impl Iterator<Item = (TileDefinitionHandle, &TileData)> {
        let page = self.page;
        self.state()
            .selection_positions()
            .iter()
            .copied()
            .filter_map(move |p| {
                let handle = TileDefinitionHandle::try_new(page?, p)?;
                Some((handle, self.tile_set()?.tile_data(handle)?))
            })
    }
    pub fn tile_redirect(
        &self,
    ) -> impl Iterator<Item = (TileDefinitionHandle, TileDefinitionHandle)> + '_ {
        let page = self.page;
        self.state()
            .selection_positions()
            .iter()
            .copied()
            .filter_map(move |p| {
                let handle = TileDefinitionHandle::try_new(page?, p)?;
                if let Some(tile_set) = self.tile_set() {
                    Some((handle, tile_set.tile_redirect(handle)?))
                } else {
                    Some((handle, self.brush()?.tile_redirect(handle)?))
                }
            })
    }
}

fn make_button(
    title: &str,
    tooltip: &str,
    row: usize,
    column: usize,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .on_row(row)
            .on_column(column)
            .with_height(24.0)
            .with_margin(Thickness::uniform(1.0))
            .with_tooltip(make_simple_tooltip(ctx, tooltip)),
    )
    .with_text(title)
    .build(ctx)
}

fn make_label(name: &str, ctx: &mut BuildContext) -> Handle<UiNode> {
    TextBuilder::new(WidgetBuilder::new())
        .with_text(name)
        .build(ctx)
}

fn highlight_tool_button(button: Handle<UiNode>, highlight: bool, ui: &UserInterface) {
    if button.is_none() {
        return;
    }
    let decorator = *ui.try_get_of_type::<Button>(button).unwrap().decorator;
    ui.send_message(DecoratorMessage::select(
        decorator,
        MessageDirection::ToWidget,
        highlight,
    ));
}

fn send_visibility(ui: &UserInterface, destination: Handle<UiNode>, visible: bool) {
    ui.send_message(WidgetMessage::visibility(
        destination,
        MessageDirection::ToWidget,
        visible,
    ));
}

fn make_property_editors(
    state: &TileEditorState,
    editors: &mut Vec<(Uuid, TileEditorRef)>,
    ctx: &mut BuildContext,
) {
    editors.clear();
    for prop_layer in state.properties() {
        editors.push((
            prop_layer.uuid,
            Arc::new(Mutex::new(TilePropertyEditor::new(
                prop_layer,
                &find_property_value(prop_layer, state),
                ctx,
            ))),
        ));
    }
}

fn make_collider_editors(
    state: &TileEditorState,
    editors: &mut Vec<(Uuid, TileEditorRef)>,
    ctx: &mut BuildContext,
) {
    editors.clear();
    editors.clear();
    for collider_layer in state.colliders() {
        editors.push((
            collider_layer.uuid,
            Arc::new(Mutex::new(TileColliderEditor::new(
                collider_layer,
                find_collider_value(collider_layer, state),
                ctx,
            ))),
        ));
    }
}

fn find_property_value(
    prop_layer: &TileSetPropertyLayer,
    state: &TileEditorState,
) -> TileSetPropertyOptionValue {
    let mut result = prop_layer.prop_type.default_option_value();
    let default_value = prop_layer.prop_type.default_value();
    for (_, data) in state.tile_data() {
        let value = data
            .properties
            .get(&prop_layer.uuid)
            .unwrap_or(&default_value);
        result.intersect(value);
    }
    result
}

fn find_collider_value(
    collider_layer: &TileSetColliderLayer,
    state: &TileEditorState,
) -> TileCollider {
    let uuid = &collider_layer.uuid;
    let mut iter = state
        .tile_data()
        .map(|d| d.1)
        .map(|d| d.colliders.get(uuid));
    iter.next()
        .map(|c| c.cloned().unwrap_or_default())
        .unwrap_or_default()
}

#[derive(Clone, Default, Debug, Visit, Reflect)]
struct InspectorField {
    handle: Handle<UiNode>,
    field: Handle<UiNode>,
}

impl InspectorField {
    fn new(label: &str, field: Handle<UiNode>, ctx: &mut BuildContext) -> Self {
        let label = make_label(label, ctx);
        Self {
            handle: GridBuilder::new(WidgetBuilder::new().with_child(label).with_child(field))
                .add_row(Row::auto())
                .add_column(Column::strict(FIELD_LABEL_WIDTH))
                .add_column(Column::stretch())
                .build(ctx),
            field,
        }
    }
}

#[derive(Clone, Default, Visit, Reflect)]
struct PropertyEditors {
    handle: Handle<UiNode>,
    content: Handle<UiNode>,
    #[visit(skip)]
    #[reflect(hidden)]
    editors: Vec<(Uuid, TileEditorRef)>,
}

impl Debug for PropertyEditors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PropertyEditors")
            .field("handle", &self.handle)
            .field("content", &self.content)
            .finish()
    }
}

impl PropertyEditors {
    fn new(state: &TileEditorState, ctx: &mut BuildContext<'_>) -> Self {
        let mut editors = Vec::default();
        make_property_editors(state, &mut editors, ctx);
        let content = StackPanelBuilder::new(
            WidgetBuilder::new().with_children(editors.iter().map(|v| v.1.lock().handle())),
        )
        .build(ctx);
        Self {
            handle: ExpanderBuilder::new(WidgetBuilder::new())
                .with_header(make_label("Properties", ctx))
                .with_content(content)
                .build(ctx),
            content,
            editors,
        }
    }
    fn iter(&self) -> impl Iterator<Item = &TileEditorRef> + '_ {
        self.editors.iter().map(|v| &v.1)
    }
    fn sync_to_model(&mut self, state: &TileEditorState, ui: &mut UserInterface) {
        if self.needs_rebuild(state) {
            for (_, editor) in self.editors.iter() {
                ui.send_message(WidgetMessage::remove(
                    editor.lock().handle(),
                    MessageDirection::ToWidget,
                ));
            }
            make_property_editors(state, &mut self.editors, &mut ui.build_ctx());
            for (_, editor) in self.editors.iter() {
                ui.send_message(WidgetMessage::link(
                    editor.lock().handle(),
                    MessageDirection::ToWidget,
                    self.content,
                ));
            }
        } else {
            for (_, editor) in self.editors.iter() {
                editor.lock().sync_to_model(state, ui);
            }
        }
    }
    fn needs_rebuild(&self, state: &TileEditorState) -> bool {
        !self
            .editors
            .iter()
            .map(|v| v.0)
            .eq(state.properties().map(|v| v.uuid))
    }
}

#[derive(Clone, Default, Visit, Reflect)]
struct ColliderEditors {
    handle: Handle<UiNode>,
    content: Handle<UiNode>,
    #[visit(skip)]
    #[reflect(hidden)]
    editors: Vec<(Uuid, TileEditorRef)>,
}

impl Debug for ColliderEditors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ColliderEditors")
            .field("handle", &self.handle)
            .field("content", &self.content)
            .finish()
    }
}

impl ColliderEditors {
    fn new(state: &TileEditorState, ctx: &mut BuildContext<'_>) -> Self {
        let mut editors = Vec::default();
        make_collider_editors(state, &mut editors, ctx);
        let content = StackPanelBuilder::new(
            WidgetBuilder::new().with_children(editors.iter().map(|v| v.1.lock().handle())),
        )
        .build(ctx);
        Self {
            handle: ExpanderBuilder::new(WidgetBuilder::new())
                .with_header(make_label("Colliders", ctx))
                .with_content(content)
                .build(ctx),
            content,
            editors,
        }
    }
    fn iter(&self) -> impl Iterator<Item = &TileEditorRef> + '_ {
        self.editors.iter().map(|v| &v.1)
    }
    fn sync_to_model(&mut self, state: &TileEditorState, ui: &mut UserInterface) {
        if self.needs_rebuild(state) {
            for (_, editor) in self.editors.iter() {
                ui.send_message(WidgetMessage::remove(
                    editor.lock().handle(),
                    MessageDirection::ToWidget,
                ));
            }
            make_collider_editors(state, &mut self.editors, &mut ui.build_ctx());
            for (_, editor) in self.editors.iter() {
                ui.send_message(WidgetMessage::link(
                    editor.lock().handle(),
                    MessageDirection::ToWidget,
                    self.content,
                ));
            }
        } else {
            for (_, editor) in self.editors.iter() {
                editor.lock().sync_to_model(state, ui);
            }
        }
    }
    fn needs_rebuild(&self, state: &TileEditorState) -> bool {
        !self
            .editors
            .iter()
            .map(|v| v.0)
            .eq(state.colliders().map(|v| v.uuid))
    }
}

#[derive(Visit, Reflect)]
pub struct TileInspector {
    handle: Handle<UiNode>,
    #[visit(skip)]
    #[reflect(hidden)]
    state: TileDrawStateRef,
    pages_palette: Handle<UiNode>,
    tiles_palette: Handle<UiNode>,
    tile_resource: TileResource,
    tile_set_page_creator: Handle<UiNode>,
    brush_page_creator: Handle<UiNode>,
    tile_size_inspector: InspectorField,
    create_tile: Handle<UiNode>,
    create_page: Handle<UiNode>,
    create_atlas: Handle<UiNode>,
    create_free: Handle<UiNode>,
    create_transform: Handle<UiNode>,
    #[visit(skip)]
    #[reflect(hidden)]
    tile_editors: Vec<TileEditorRef>,
    page_material_inspector: InspectorField,
    page_material_field: Handle<UiNode>,
    page_icon_field: Handle<UiNode>,
    property_editors: PropertyEditors,
    collider_editors: ColliderEditors,
}

impl Debug for TileInspector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TileInspector")
            .field("handle", &self.handle)
            .finish()
    }
}

impl TileInspector {
    pub fn new(
        state: TileDrawStateRef,
        pages_palette: Handle<UiNode>,
        tiles_palette: Handle<UiNode>,
        tile_resource: TileResource,
        _resource_manager: ResourceManager,
        sender: MessageSender,
        ctx: &mut BuildContext,
    ) -> Self {
        let create_page;
        let create_atlas;
        let create_free;
        let create_transform;

        let tile_editors: Vec<TileEditorRef> = vec![
            Arc::new(Mutex::new(TileMaterialEditor::new(ctx, sender.clone()))) as TileEditorRef,
            Arc::new(Mutex::new(TileColorEditor::new(ctx))) as TileEditorRef,
            Arc::new(Mutex::new(TileHandleEditor::new(None, ctx))) as TileEditorRef,
        ];

        let creator_label_0 = make_label("Create New Page", ctx);
        let creator_label_1 = make_label("Create New Page", ctx);

        let brush_page_creator = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_visibility(false)
                .on_row(1)
                .with_child(creator_label_0)
                .with_child({
                    create_page = make_button("Add Page", "Create a brush tile page.", 0, 0, ctx);
                    create_page
                }),
        )
        .build(ctx);
        let create_tile = make_button("Create Tile", "Add a tile to this page.", 0, 0, ctx);
        let tile_set_page_creator =
            GridBuilder::new(WidgetBuilder::new()
            .with_visibility(false)
            .with_child(creator_label_1)
            .with_child({
                create_atlas =
                    make_button("Tile Atlas", "Create a atlas texture tile page.", 1, 0, ctx);
                create_atlas
            })
            .with_child({
                create_free =
                    make_button("Free Tiles", "Create an arbitrary tile page, with no limits on material and uv coordinates.", 2, 0, ctx);
                create_free
            })
            .with_child({
                create_transform =
                    make_button("Transform", "Create a page that controls how tiles flip and rotate.", 3, 0, ctx);
                create_transform
            })
        ).add_column(Column::stretch())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .build(ctx);
        let page_material_field = MaterialFieldEditorBuilder::new(
            WidgetBuilder::new().on_column(1),
        )
        .build(ctx, sender.clone(), DEFAULT_TILE_MATERIAL.deep_copy());
        let page_material_inspector = InspectorField::new("Material", page_material_field, ctx);
        let tile_size_field =
            Vec2EditorBuilder::<u32>::new(WidgetBuilder::new().on_column(1)).build(ctx);
        let tile_size_inspector = InspectorField::new("Tile Size", tile_size_field, ctx);
        let page_icon_field = TileHandleFieldBuilder::new(WidgetBuilder::new())
            .with_label("Page Icon")
            .build(ctx);
        let tile_editor_state = TileEditorStateRef {
            page: None,
            state: state.clone(),
            pages_palette,
            tiles_palette,
            tile_resource: tile_resource.clone(),
        };
        let tile_editor_state_lock = tile_editor_state.lock();
        let property_editors = PropertyEditors::new(&tile_editor_state_lock, ctx);
        let collider_editors = ColliderEditors::new(&tile_editor_state_lock, ctx);
        let handle = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(tile_set_page_creator)
                .with_child(brush_page_creator)
                .with_child(page_icon_field)
                .with_child(page_material_inspector.handle)
                .with_child(tile_size_inspector.handle)
                .with_child(create_tile)
                .with_children(tile_editors.iter().map(|e| e.lock().handle()))
                .with_child(property_editors.handle)
                .with_child(collider_editors.handle),
        )
        .build(ctx);
        Self {
            handle,
            state,
            pages_palette,
            tiles_palette,
            tile_resource,
            tile_editors,
            brush_page_creator,
            tile_set_page_creator,
            page_material_inspector,
            page_material_field,
            tile_size_inspector,
            create_tile,
            create_page,
            create_atlas,
            create_free,
            create_transform,
            page_icon_field,
            property_editors,
            collider_editors,
        }
    }
    pub fn handle(&self) -> Handle<UiNode> {
        self.handle
    }
    pub fn set_tile_resource(&mut self, tile_resource: TileResource, ui: &mut UserInterface) {
        self.tile_resource = tile_resource;
        self.sync_to_model(ui);
    }
    fn tile_editor_state(&self, ui: &UserInterface) -> TileEditorStateRef {
        let page = if self.state.lock().selection_palette() != self.tiles_palette {
            None
        } else {
            ui.node(self.tiles_palette)
                .cast::<PaletteWidget>()
                .unwrap()
                .page
        };
        TileEditorStateRef {
            page,
            pages_palette: self.pages_palette,
            tiles_palette: self.tiles_palette,
            state: self.state.clone(),
            tile_resource: self.tile_resource.clone(),
        }
    }
    pub fn sync_to_model(&mut self, ui: &mut UserInterface) {
        let tile_editor_state = self.tile_editor_state(ui);
        let tile_editor_state = tile_editor_state.lock();
        self.property_editors.sync_to_model(&tile_editor_state, ui);
        self.collider_editors.sync_to_model(&tile_editor_state, ui);
        drop(tile_editor_state);
        self.sync_to_state(ui);
    }
    pub fn sync_to_state(&mut self, ui: &mut UserInterface) {
        let tile_editor_state = self.tile_editor_state(ui);
        let state = tile_editor_state.lock();
        let empty_tiles = state.empty_tiles().next().is_some();
        let empty_pages = state.empty_page_positions().next().is_some();
        let tile_set_empty_pages = state.tile_set().is_some() && empty_pages;
        let brush_empty_pages = state.brush().is_some() && empty_pages;
        let tile_data_selected = state.tile_data().next().is_some();
        let mat_page_selected = state.material_page().is_some();
        send_visibility(ui, self.tile_set_page_creator, tile_set_empty_pages);
        send_visibility(ui, self.brush_page_creator, brush_empty_pages);
        send_visibility(ui, self.create_tile, empty_tiles);
        send_visibility(ui, self.tile_set_page_creator, tile_set_empty_pages);
        send_visibility(ui, self.tile_size_inspector.handle, mat_page_selected);
        send_visibility(ui, self.page_material_inspector.handle, mat_page_selected);
        send_visibility(
            ui,
            self.page_icon_field,
            state.tile_set_pages().next().is_some() || state.brush_pages().next().is_some(),
        );
        send_visibility(ui, self.property_editors.handle, tile_data_selected);
        send_visibility(ui, self.collider_editors.handle, tile_data_selected);
        self.sync_to_page(&state, ui);
        let page_icon = self.find_page_icon(&state);
        send_sync_message(
            ui,
            TileHandleEditorMessage::value(
                self.page_icon_field,
                MessageDirection::ToWidget,
                page_icon,
            ),
        );
        let iter = self
            .tile_editors
            .iter()
            .chain(self.property_editors.iter())
            .chain(self.collider_editors.iter());
        for editor_ref in iter {
            let mut editor = editor_ref.lock();
            editor.sync_to_state(&state, ui);
            let draw_button = editor.draw_button();
            drop(editor);
            highlight_tool_button(
                draw_button,
                state.drawing_mode() == DrawingMode::Editor && state.is_active_editor(editor_ref),
                ui,
            );
        }
    }
    fn find_page_icon(&self, state: &TileEditorState) -> Option<TileDefinitionHandle> {
        if state.is_tile_set() {
            let mut iter = state.tile_set_pages().map(|(_, p)| p.icon);
            let icon = iter.next()?;
            if iter.all(|h| h == icon) {
                Some(icon)
            } else {
                None
            }
        } else if state.is_brush() {
            let mut iter = state.brush_pages().map(|(_, p)| p.icon);
            let icon = iter.next()?;
            if iter.all(|h| h == icon) {
                Some(icon)
            } else {
                None
            }
        } else {
            None
        }
    }
    fn sync_to_page(&mut self, state: &TileEditorState, ui: &mut UserInterface) {
        if let Some((_, mat)) = state.material_page() {
            send_sync_message(
                ui,
                Vec2EditorMessage::value(
                    self.tile_size_inspector.field,
                    MessageDirection::ToWidget,
                    mat.tile_size,
                ),
            );
            send_sync_message(
                ui,
                MaterialFieldMessage::material(
                    self.page_material_inspector.field,
                    MessageDirection::ToWidget,
                    mat.material.clone(),
                ),
            );
        }
    }
    pub fn handle_ui_message(
        &self,
        message: &UiMessage,
        ui: &mut UserInterface,
        sender: &MessageSender,
    ) {
        if message.flags == MSG_SYNC_FLAG || message.direction() == MessageDirection::ToWidget {
            return;
        }
        if !ui.is_node_child_of(message.destination(), self.handle()) {
            return;
        }
        let tile_editor_state = self.tile_editor_state(ui);
        let mut tile_editor_state = tile_editor_state.lock();
        let iter = self
            .tile_editors
            .iter()
            .chain(self.property_editors.iter())
            .chain(self.collider_editors.iter());
        for editor in iter {
            editor.lock().handle_ui_message(
                &mut tile_editor_state,
                message,
                ui,
                &self.tile_resource,
                sender,
            );
        }
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.create_atlas {
                self.create_tile_set_page(
                    TileSetPageSource::new_material(),
                    &tile_editor_state,
                    sender,
                );
            } else if message.destination() == self.create_free {
                self.create_tile_set_page(
                    TileSetPageSource::new_free(),
                    &tile_editor_state,
                    sender,
                );
            } else if message.destination() == self.create_transform {
                self.create_tile_set_page(
                    TileSetPageSource::new_transform(),
                    &tile_editor_state,
                    sender,
                );
            } else if message.destination() == self.create_page {
                self.create_brush_page(&tile_editor_state, sender);
            } else if message.destination() == self.create_tile {
                self.create_tile(&tile_editor_state, sender);
            } else {
                let iter = self
                    .tile_editors
                    .iter()
                    .chain(self.property_editors.iter())
                    .chain(self.collider_editors.iter());
                for editor_ref in iter {
                    let draw_button = editor_ref.lock().draw_button();
                    if message.destination() == draw_button {
                        if tile_editor_state.is_active_editor(editor_ref) {
                            tile_editor_state.set_active_editor(None);
                            tile_editor_state.set_drawing_mode(DrawingMode::Pick);
                        } else {
                            tile_editor_state.set_active_editor(Some(editor_ref.clone()));
                            tile_editor_state.set_drawing_mode(DrawingMode::Editor);
                        }
                    }
                }
            }
        } else if let Some(MaterialFieldMessage::Material(material)) = message.data() {
            if message.destination() == self.page_material_inspector.field {
                self.set_page_material(material.clone(), &tile_editor_state, sender);
            }
        } else if let Some(Vec2EditorMessage::<u32>::Value(size)) = message.data() {
            if message.destination() == self.tile_size_inspector.field {
                self.set_page_tile_size(*size, &tile_editor_state, sender);
            }
        } else if let Some(TileHandleEditorMessage::Value(Some(handle))) = message.data() {
            if message.destination() == self.page_icon_field {
                self.apply_page_icon(*handle, &tile_editor_state, sender);
            }
        }
    }
    fn apply_page_icon(
        &self,
        icon: TileDefinitionHandle,
        state: &TileEditorState,
        sender: &MessageSender,
    ) {
        let cmds = match &self.tile_resource {
            TileResource::Empty => return,
            TileResource::TileSet(tile_set) => state
                .page_positions()
                .map(|position| ModifyPageIconCommand {
                    tile_set: tile_set.clone(),
                    page: position,
                    icon,
                })
                .map(Command::new)
                .collect::<Vec<_>>(),
            TileResource::Brush(brush) => state
                .page_positions()
                .map(|position| ModifyBrushPageIconCommand {
                    brush: brush.clone(),
                    page: position,
                    icon,
                })
                .map(Command::new)
                .collect::<Vec<_>>(),
        };
        sender.do_command(CommandGroup::from(cmds).with_custom_name("Modify Tile Page Icon"));
    }
    fn create_tile(&self, state: &TileEditorState, sender: &MessageSender) {
        let TileResource::TileSet(tile_set) = &self.tile_resource else {
            return;
        };
        let mut update = TileSetUpdate::default();
        for handle in state.empty_tiles() {
            if state.is_material_page(handle.page()) {
                update.insert(handle, TileDataUpdate::MaterialTile(TileData::default()));
            } else if state.is_freeform_page(handle.page()) {
                update.insert(
                    handle,
                    TileDataUpdate::FreeformTile(TileDefinition::default()),
                );
            }
        }
        sender.do_command(SetTileSetTilesCommand {
            tile_set: tile_set.clone(),
            tiles: update,
        });
    }
    fn create_brush_page(&self, state: &TileEditorState, sender: &MessageSender) {
        let TileResource::Brush(brush) = &self.tile_resource else {
            return;
        };
        let cmds = state
            .empty_page_positions()
            .map(|position| SetBrushPageCommand {
                brush: brush.clone(),
                position,
                page: Some(TileMapBrushPage {
                    icon: TileDefinitionHandle::new(0, 0, 0, -1),
                    tiles: Tiles::default(),
                }),
            })
            .map(Command::new)
            .collect::<Vec<_>>();
        sender.do_command(CommandGroup::from(cmds).with_custom_name("Create Brush Page"));
    }
    fn create_tile_set_page(
        &self,
        source: TileSetPageSource,
        state: &TileEditorState,
        sender: &MessageSender,
    ) {
        let TileResource::TileSet(tile_set) = &self.tile_resource else {
            return;
        };
        let cmds = state
            .empty_page_positions()
            .filter_map(|position| {
                Some(SetTileSetPageCommand {
                    tile_set: tile_set.clone(),
                    position,
                    page: Some(TileSetPage {
                        icon: TileDefinitionHandle::try_new(position, Vector2::new(0, -1))?,
                        source: source.clone(),
                    }),
                })
            })
            .map(Command::new)
            .collect::<Vec<_>>();
        sender.do_command(CommandGroup::from(cmds).with_custom_name("Create Tile Set Page"));
    }
    fn set_page_material(
        &self,
        material: MaterialResource,
        state: &TileEditorState,
        sender: &MessageSender,
    ) {
        let TileResource::TileSet(tile_set) = self.tile_resource.clone() else {
            return;
        };
        if let Some((page, _)) = state.material_page() {
            sender.do_command(ModifyPageMaterialCommand {
                tile_set,
                page,
                material,
            });
        }
    }
    fn set_page_tile_size(
        &self,
        size: Vector2<u32>,
        state: &TileEditorState,
        sender: &MessageSender,
    ) {
        let TileResource::TileSet(tile_set) = self.tile_resource.clone() else {
            return;
        };
        if let Some((page, _)) = state.material_page() {
            sender.do_command(ModifyPageTileSizeCommand {
                tile_set,
                page,
                size,
            });
        }
    }
}
