//! Opening, closing, tree repeaters, landing on a row and part links of a page (`Session` methods).

use super::*;

mod caller_slots;
pub(crate) use caller_slots::written_by_routine;
mod close;
mod handed_moved;
mod landing;
mod modal_card_find;
mod or_group;
mod page_open;
mod part_links;
mod part_rec_row;
mod plain_first;
mod query_close_unhandled;
mod record_refusals;
mod repeater_new_line;
pub(in crate::pages) use repeater_new_line::RepeaterLine;
mod temp_list_key;
mod temp_ws_preloaded;
pub(in crate::pages) use temp_ws_preloaded::TempWsEdit;
mod tree;
mod tree_var;
mod values;

/// The filter group of the cut that hides the rows below indentation 0 of a tree repeater.
pub(super) const TREE_GROUP: i32 = 6;

/// A tree's cut taken off the page's Rec while the page's AL code runs (`hide_tree_cut`), with the view and the row Rec had then.
pub(super) struct TreeCut {
    cut: Vec<al_builtins::filter::FieldFilter>,
    view: super::actions::ListView,
    key: Option<Vec<Value>>,
}
