//! What the session image makes of the session field table (`session_fields.rs`, al-runner issue 8211): the image's
//! copy of the `plain` fields ([`PlainFields`]), and [`Session::field_lines`]. A `plain` field added to the table is
//! imaged here without a line of its own, and the type written next to it there is the type it has here.
//!
//! The rule for what the thaw writes: a field it writes from the image or keeps from its target is an input of the login
//! key by content and is compared by the check. [`Session::field_lines`] renders every `plain` and `runner` field into
//! [`Session::state_lines`], which the key digests before the login and the check compares after it; the `linked` fields
//! are outputs of the login the image carries by hand, and the state lines render those AL code can read
//! (state_stamp.rs, state_facts.rs).

use crate::Session;
use crate::state_render::content;
use std::collections::{BTreeSet, HashMap, HashSet};

/// Makes [`PlainFields`] with `take` and `give`, and [`Session::field_lines`], from the table.
macro_rules! plain_fields {
    (
        plain { $($p:ident: $pt:ty,)* }
        runner { $($n:ident $([$nk:ident])?,)* }
        counter { $($c:ident,)* }
        linked { $($l:ident,)* }
        fresh { $($f:ident,)* }
        refuse { $($r:ident $([$tag:ident $(=> $ip:expr)?])? => $rp:expr,)* }
        rest { $($s:ident => $sp:expr,)* }
        own { $($o:ident $(=> $op:expr)?,)* }
    ) => {
        /// The `plain` fields of a session, by value.
        #[derive(Clone)]
        pub(crate) struct PlainFields {
            $(pub(crate) $p: $pt,)*
        }

        impl PlainFields {
            /// The plain fields of `s`.
            pub(crate) fn take(s: &Session) -> PlainFields {
                PlainFields { $($p: s.$p.clone(),)* }
            }

            /// Writes the plain fields into `s`.
            pub(crate) fn give(&self, s: &mut Session) {
                $(s.$p = self.$p.clone();)*
            }
        }

        impl Session {
            /// Every `plain` and `runner` field, one line each (`field.<name> <value>`, a map or set in sorted order):
            /// the fields the thaw writes from the image or keeps from its target. [`Session::state_lines`] holds them,
            /// so the login key holds each before the login and the check compares each after it (a field the thaw
            /// writes is an input of the key by content, or kept from the target).
            pub(crate) fn field_lines(&self) -> Vec<String> {
                let mut out = vec![$(format!("field.{} {}", stringify!($p), Canon::canon(&self.$p)),)*];
                $(runner_line!(out, self, $n $([$nk])?);)*
                out
            }
        }
    };
}

/// A field's value as one text that is the same for equal values: a map or a set in sorted order, everything else in its
/// debug form, through the one place the state lines use it (`state_render::content`) ([`Session::field_lines`]).
pub(crate) trait Canon {
    fn canon(&self) -> String;
}

impl<K: std::fmt::Debug, V: std::fmt::Debug, S> Canon for HashMap<K, V, S> {
    fn canon(&self) -> String {
        let mut v: Vec<String> = self.iter().map(|kv| content(&kv)).collect();
        v.sort();
        format!("{{{}}}", v.join(", "))
    }
}

impl<T: std::fmt::Debug, S> Canon for HashSet<T, S> {
    fn canon(&self) -> String {
        let mut v: Vec<String> = self.iter().map(|x| content(x)).collect();
        v.sort();
        format!("{{{}}}", v.join(", "))
    }
}

/// [`Canon`] in the debug form, for types whose debug form has one order.
macro_rules! canon_by_debug {
    ($($t:ty),* $(,)?) => { $(impl Canon for $t { fn canon(&self) -> String { content(self) } })* };
}

canon_by_debug!(
    bool,
    i32,
    i64,
    u64,
    String,
    al_builtins::native::AlError,
    al_host::random::NetRandom,
    crate::blob_transfer::Sources,
    crate::EncryptionKey,
    crate::HeldLogins,
);

impl<T: std::fmt::Debug> Canon for Option<T> {
    fn canon(&self) -> String {
        content(self)
    }
}

impl<T: std::fmt::Debug> Canon for Vec<T> {
    fn canon(&self) -> String {
        content(self)
    }
}

impl<T: std::fmt::Debug> Canon for BTreeSet<T> {
    fn canon(&self) -> String {
        content(self)
    }
}

/// The line of one `runner` field in [`Session::field_lines`]: none for one marked `[own_key_field]`.
macro_rules! runner_line {
    ($out:expr, $s:expr, $n:ident [own_key_field]) => {};
    ($out:expr, $s:expr, $n:ident) => {
        $out.push(format!("field.{} {}", stringify!($n), Canon::canon(&$s.$n)));
    };
}

crate::session_fields::session_fields!(plain_fields);

// `LoginImage` holds the plain fields and is `Send + Sync`: a field that holds an `Rc` cannot be put in this class.
const _: fn() = || {
    fn both<T: Send + Sync>() {}
    both::<PlainFields>();
};
