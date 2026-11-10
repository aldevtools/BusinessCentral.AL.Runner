//! Opaque handles on the test client's state (ADR-072 decision 3): what `Value::TestPage` and `Value::TestControl`
//! hold. The state of a page (`PageState`) and of a control handle (`ControlRef`) belongs to the crate that runs the
//! page client (al-vm, `pages/state.rs`). This crate only stores the handle, clones it, drops it and prints it.
//!
//! A handle owns one value of a type this crate does not know. The VM puts its `Rc<RefCell<PageState>>` (or
//! `Rc<ControlRef>`) in and borrows it back with `get`, so a `Value` holds exactly the references it held before the
//! state moved: one `Rc` per value, cloned when the value is cloned. The page client counts those references
//! (al-vm `page_held_bindings.rs`, `pages/rows.rs`), so a handle never adds one of its own.

use std::any::Any;
use std::fmt;
use std::sync::OnceLock;

/// What a handle can do with the value it owns without knowing its type.
trait Held: Any {
    fn as_any(&self) -> &dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    fn clone_held(&self) -> Box<dyn Held>;
    fn fmt_held(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

impl<T: Any + Clone + fmt::Debug> Held for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
    fn clone_held(&self) -> Box<dyn Held> {
        Box::new(self.clone())
    }
    fn fmt_held(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

macro_rules! opaque_handle {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        pub struct $name(Box<dyn Held>);

        impl $name {
            /// A handle that owns `held`. Cloning the handle clones `held`; printing it prints `held`.
            pub fn new<T: Any + Clone + fmt::Debug>(held: T) -> $name {
                $name(Box::new(held))
            }

            /// The value the handle owns, when it is a `T`.
            pub fn get<T: Any>(&self) -> Option<&T> {
                (*self.0).as_any().downcast_ref()
            }

            /// Takes the value out of the handle, when it is a `T`.
            pub fn into_inner<T: Any>(self) -> Result<T, $name> {
                if (*self.0).as_any().is::<T>() {
                    match self.0.into_any().downcast::<T>() {
                        Ok(held) => Ok(*held),
                        Err(_) => unreachable!("the type was checked on the line above"),
                    }
                } else {
                    Err(self)
                }
            }
        }

        impl Clone for $name {
            fn clone(&self) -> $name {
                $name((*self.0).clone_held())
            }
        }

        // Prints what the value it owns prints, with no wrapper: `Value::identical` compares handles by this text.
        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                (*self.0).fmt_held(f)
            }
        }
    };
}

opaque_handle!(
    /// A `TestPage`, a `TestPart` or a `Page` variable: the test client's state of one page (ADR-012), owned by al-vm.
    PageHandle
);
opaque_handle!(
    /// A `TestField`, `TestAction` or `TestFilter`: a page and the control of it the handle stands for, owned by al-vm.
    ControlHandle
);

/// How the crate that owns the page state builds a closed page of an object (`set_closed_page`).
static CLOSED_PAGE: OnceLock<fn(u32) -> PageHandle> = OnceLock::new();

/// Registers the constructor of a closed page: `Value::default_for` of a `TestPage` or `Page` type has no `Ctx` to
/// ask, so the crate that owns the page state (al-vm) names its constructor here before it makes a value. The first
/// registration stays.
pub fn set_closed_page(make: fn(u32) -> PageHandle) {
    let _ = CLOSED_PAGE.set(make);
}

/// What a page variable holds where no page client is linked in (a unit test of this crate): the object only.
#[derive(Clone, Debug)]
pub struct NoPageClient(pub u32);

impl PageHandle {
    /// A page of object `object` that was never opened: what a declared `TestPage` or `Page` variable holds.
    pub fn closed(object: u32) -> PageHandle {
        match CLOSED_PAGE.get() {
            Some(make) => make(object),
            None => PageHandle::new(NoPageClient(object)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Debug)]
    struct State(u32);

    #[test]
    fn a_handle_holds_one_reference_and_a_clone_holds_one_more() {
        let state = Rc::new(RefCell::new(State(7)));
        let handle = PageHandle::new(state.clone());
        assert_eq!(Rc::strong_count(&state), 2);
        let copy = handle.clone();
        assert_eq!(Rc::strong_count(&state), 3);
        // Borrowing the state back adds no reference.
        let held: &Rc<RefCell<State>> = copy.get().unwrap();
        assert!(Rc::ptr_eq(held, &state));
        assert_eq!(Rc::strong_count(&state), 3);
        drop(copy);
        assert_eq!(Rc::strong_count(&state), 2);
        // Taking it out moves the handle's reference.
        let taken: Rc<RefCell<State>> = handle.into_inner().ok().unwrap();
        assert_eq!(Rc::strong_count(&state), 2);
        assert_eq!(taken.borrow().0, 7);
    }

    #[test]
    fn a_handle_prints_what_it_holds_with_no_wrapper() {
        let state = Rc::new(RefCell::new(State(7)));
        let handle = PageHandle::new(state.clone());
        assert_eq!(format!("{handle:?}"), format!("{state:?}"));
        assert_eq!(format!("{handle:#?}"), format!("{state:#?}"));
        let control = ControlHandle::new(Rc::new(State(3)));
        assert_eq!(format!("{control:?}"), "State(3)");
    }

    #[test]
    fn a_handle_of_another_type_gives_nothing() {
        let handle = PageHandle::new(Rc::new(State(1)));
        assert!(handle.get::<Rc<RefCell<State>>>().is_none());
        let handle = handle.into_inner::<Rc<RefCell<State>>>().unwrap_err();
        assert!(handle.get::<Rc<State>>().is_some());
    }

    #[test]
    fn a_closed_page_without_a_page_client_holds_the_object() {
        // No test of this crate registers a constructor (al-vm does, in `Engine::with_config`).
        let handle = PageHandle::closed(42);
        assert_eq!(handle.get::<NoPageClient>().map(|p| p.0), Some(42));
    }
}
