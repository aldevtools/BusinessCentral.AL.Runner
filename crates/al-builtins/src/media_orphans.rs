//! `Media.FindOrphans()` and `MediaSet.FindOrphans()` (media/lt-y48-find-orphans/Y48FindOrphans.al:229-410).
//!
//! A media is an orphan when it has a Tenant Media row and no stored row holds it, neither in a Media field nor as an item of
//! a media set (`FO_DetachedTenantMediaSetRows`: the items of a set no field holds are no orphans; :305-341 a media imported
//! into a record that was never inserted, or not modified after the import, is one; :378-391 a media only a temporary record
//! holds is one). A media set is an orphan when no stored row holds it in a MediaSet field. The corpus observes only whether
//! the list contains an id, never its order: the list is in id order here. Which rows hold what is the session's to say
//! ([`Ctx::media_holders`]).

use crate::media::Id;
use crate::native::{Ctx, NativeFn, Signal};
use crate::value::Value;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

/// The media and media set ids the stored rows hold in Media and MediaSet fields.
#[derive(Debug, Default)]
pub struct MediaHolders {
    pub media: HashSet<Id>,
    pub sets: HashSet<Id>,
}

fn list(mut ids: Vec<Id>) -> Value {
    ids.sort();
    Value::List(Rc::new(RefCell::new(
        ids.into_iter().map(Value::Guid).collect::<Vec<_>>().into(),
    )))
}

fn store_ids(c: &mut dyn Ctx) -> Result<(Vec<Id>, Vec<(Id, Vec<Id>)>), Signal> {
    let Some(store) = c.media_store() else {
        return Err(Signal::Unsupported("FindOrphans: this context holds no media".into()));
    };
    if !store.outside_tenant.borrow().is_empty() {
        return Err(Signal::Unsupported(
            "FindOrphans with media of a Media Resources row in the session (whether BC counts them is not pinned)"
                .into(),
        ));
    }
    let objects: Vec<Id> = store.objects.borrow().keys().copied().collect();
    // A set is Tenant Media Set rows, one per item: a set none of whose items has its Tenant Media row any more (the Delete of
    // the row that held it removes them) is no set (streams/lt-stream-read-write-media/P102MediaOrphans.al:55-59
    // `deleted_in_orphans` No).
    let sets = store
        .sets
        .borrow()
        .iter()
        .map(|(k, v)| {
            (
                *k,
                v.iter().copied().filter(|i| objects.contains(i)).collect::<Vec<_>>(),
            )
        })
        .filter(|(_, items)| !items.is_empty())
        .collect();
    Ok((objects, sets))
}

fn media_find_orphans(c: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    let (objects, sets) = store_ids(c)?;
    let held = c.media_holders()?;
    let in_a_set: HashSet<Id> = sets.into_iter().flat_map(|(_, items)| items).collect();
    Ok(list(
        objects
            .into_iter()
            .filter(|id| !held.media.contains(id) && !in_a_set.contains(id))
            .collect(),
    ))
}

fn media_set_find_orphans(c: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    let (_, sets) = store_ids(c)?;
    let held = c.media_holders()?;
    Ok(list(
        sets.into_iter()
            .map(|(id, _)| id)
            .filter(|id| !held.sets.contains(id))
            .collect(),
    ))
}

pub const REGISTRY: &[(&str, NativeFn)] = &[
    ("Media.FindOrphans#1", media_find_orphans),
    ("MediaSet.FindOrphans#1", media_set_find_orphans),
];
