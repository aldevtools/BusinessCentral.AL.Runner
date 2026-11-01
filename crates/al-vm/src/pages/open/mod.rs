//! Opening, closing, tree repeaters, landing on a row and part links of a page (`Session` methods).

use super::*;

mod close;
mod landing;
mod page_open;
mod part_links;
mod part_rec_row;
mod record_refusals;
mod tree;
mod values;

/// The filter group of the cut that hides the rows below indentation 0 of a tree repeater.
pub(super) const TREE_GROUP: i32 = 6;

/// A tree's cut taken off the page's Rec while the page's AL code runs (`hide_tree_cut`), with the view and the row Rec had then.
pub(super) struct TreeCut {
    cut: Vec<al_builtins::filter::FieldFilter>,
    view: super::actions::ListView,
    key: Option<Vec<Value>>,
}
