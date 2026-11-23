//! Opening, closing, tree repeaters, landing on a row and part links of a page (`Session` methods).
//!
//! Opening and closing a page, rows loaded on open. Submodules: `page_open`, `close`, `landing`,
//! `or_group`, `part_links`, `record_refusals`, `saved_row`, `temp_list_key`, `tree`, `values`. Entry
//! points: `page_open`, `open_page_as` (page_open.rs), `page_close`, `close_with` (close.rs).

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
mod saved_row;
pub(in crate::pages) use repeater_new_line::RepeaterLine;
pub(crate) use saved_row::SavedRow;
mod temp_list_key;
mod temp_navigate_new;
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
