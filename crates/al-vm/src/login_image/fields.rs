//! The image's copy of the `plain` fields of [`Session`] ([`PlainFields`]), made from the session field table
//! (`session_fields.rs`, al-runner issue 8211): a `plain` field added there is imaged here without a line of its own, and
//! the type written next to it there is the type it has here.

use crate::Session;
use std::collections::{BTreeSet, HashMap, HashSet};

/// Makes [`PlainFields`] with `take` and `give` from the table.
macro_rules! plain_fields {
    (
        plain { $($p:ident: $pt:ty,)* }
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
    };
}

crate::session_fields::session_fields!(plain_fields);

// `LoginImage` holds the plain fields and is `Send + Sync`: a field that holds an `Rc` cannot be put in this class.
const _: fn() = || {
    fn both<T: Send + Sync>() {}
    both::<PlainFields>();
};
