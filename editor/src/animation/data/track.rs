use super::*;
use crate::fyrox::core::pool::{ErasedHandle, Handle};
use crate::fyrox::generic_animation::value::ValueBinding;
use crate::fyrox::gui::{grid::Column, grid::GridBuilder, widget::WidgetBuilder, BuildContext};

pub const KEY_SIZE: f32 = 50.0;
pub const EXPANDER_COLUMN: usize = 0;
pub const NAME_COLUMN: usize = 1;
pub const MODEL_COLUMN: usize = 2;
pub const VALUE_TO_MODEL: usize = 3;
pub const MODEL_TO_VALUE: usize = 4;
pub const VALUE_COLUMN: usize = 5;
pub const INTERP_TO_KEY: usize = 6;
pub const LEFT_TAN_COLUMN: usize = 7;
pub const RIGHT_TAN_COLUMN: usize = 8;
pub const PREV_COLUMN: usize = 9;
pub const NEXT_COLUMN: usize = 10;

#[derive(Debug)]
pub struct TrackDataView {
    pub curves: Vec<CurveDataView>,
    id: Uuid,
    binding: ValueBinding,
    track_enabled_switch: Handle<UiNode>,
    track_enabled: bool,
    thumb: f32,
    name_text: Handle<UiNode>,
    next_key: Handle<UiNode>,
    prev_key: Handle<UiNode>,
}

pub struct TargetDataView {
    pub tracks: Vec<TrackDataView>,
    pub target: ErasedHandle,
    name_text: Handle<UiNode>,
    model_to_key: Handle<UiNode>,
    value_to_model: Handle<UiNode>,
    remove_key: Handle<UiNode>,
    next_key: Handle<UiNode>,
    prev_key: Handle<UiNode>,
}

pub struct TrackDataList {
    pub content: Handle<UiNode>,
    pub targets: Vec<TargetDataView>,
}

impl TrackDataList {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let grid = GridBuilder::new(WidgetBuilder::new().on_row(1))
            .add_column(Column::strict(16.0)) // Track expander checkbox
            .add_column(Column::stretch()) // Name
            .add_column(Column::auto()) // Model
            .add_column(Column::auto()) // Model -> Key
            .add_column(Column::auto()) // Model <- Current
            .add_column(Column::auto()) // Current
            .add_column(Column::auto()) // Left Tangent
            .add_column(Column::auto()) // Right Tangent
            .add_column(Column::strict(KEY_SIZE)) // Previous key
            .add_column(Column::strict(KEY_SIZE)) // Next key
            .build(ctx);
        Self {
            content: grid,
            targets: Default::default(),
        }
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
        todo!();
    }
}
