//! The declared id of an object, where the runtime prints it into an error text or hands it to AL code. Every runnable table,
//! codeunit, page, report, query and XmlPort declares one; an object without one (a duplicate id the compile renamed, a kind
//! that has none) refuses instead of showing object 0 (al-runner issue 7644).
//!
//! the declared id of an object where an error text or a value names it; an object without one
//! refuses instead of showing 0 (al-runner issue 7644). Entry points: `declared_id`,
//! `Session::object_id_of`.

use al_builtins::native::{Signal, unsupported};

/// Why a text or value that would name an object without a declared id refuses.
pub(crate) const NO_DECLARED_ID: &str = "an error text or value that names an object without a declared id";

/// The declared id `id` of an object, or a refusal when it has none.
pub(crate) fn declared_id(id: Option<u32>) -> Result<u32, Signal> {
    match id {
        Some(id) => Ok(id),
        None => unsupported(NO_DECLARED_ID),
    }
}

/// `<kind> <id>`, the ObjectId text of an object without its name ([`declared_id`]).
pub(crate) fn object_id_text(kind: &str, id: Option<u32>) -> Result<al_builtins::value::Value, Signal> {
    Ok(al_builtins::value::Value::Text(
        format!("{kind} {}", declared_id(id)?).into(),
    ))
}

impl crate::Session {
    /// The declared id of the object at index `object` ([`declared_id`]).
    pub(crate) fn object_id_of(&self, object: u32) -> Result<u32, Signal> {
        declared_id(self.engine.program.objects.get(object as usize).and_then(|o| o.id))
    }
}

#[cfg(test)]
mod tests {
    use super::declared_id;
    use al_builtins::native::Signal;

    #[test]
    fn an_object_without_an_id_refuses_and_one_with_an_id_answers_it() {
        assert!(matches!(declared_id(None), Err(Signal::Unsupported(_))));
        assert_eq!(declared_id(Some(50100)).unwrap(), 50100);
    }
}
