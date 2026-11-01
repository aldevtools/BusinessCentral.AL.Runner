//! A FieldRef handed as a value to `FieldRef.Validate`, the `FieldRef.Value` setter, `SetRange` or `SetFilter` (al-runner
//! issue 6591): the source field's value goes in, and the target converts it as it converts a value of that type
//! (recordref/lt-y47-validate-fieldref/Y47ValidateFieldRef.al:231-402).

use super::Lib;
use crate::Session;
use al_builtins::native::Signal;
use al_builtins::value::{LibraryValue, RtType, Value};

impl Session {
    /// Replaces each FieldRef argument of `method` on the FieldRef `target` by the value of the field it points to.
    pub(super) fn fieldref_arg_values(&mut self, method: &str, target: &Lib, args: &mut [Value]) -> Result<(), Signal> {
        if !matches!(method, "Validate" | "Value" | "SetRange" | "SetFilter") {
            return Ok(());
        }
        for arg in args.iter_mut() {
            let Value::Library(_, source) = &*arg else { continue };
            if !matches!(
                &*source.borrow(),
                LibraryValue::FieldRef(..) | LibraryValue::KeyFieldRef(..)
            ) {
                continue;
            }
            let source: Lib = source.clone();
            *arg = self.fieldref_op("Value", &source, &mut [])?;
            if !matches!(method, "Validate" | "Value") {
                continue;
            }
            let ty = self.fieldref_type(target)?;
            match (&*arg, &ty) {
                // A Boolean into an Integer field is 1 or 0 (Y47ValidateFieldRef.al Y47_ValidateFieldRef_Matrix `bool_to_int`).
                (Value::Boolean(b), Some(RtType::Integer)) => *arg = Value::Integer(i32::from(*b)),
                // A Date into a Text or Code field is its Format text (`date_to_text`, 01/15/24).
                (Value::Date(..), Some(RtType::Text(_) | RtType::Code(_))) => {
                    let text = al_builtins::format::try_format(arg, 0, 0, &*self).map_err(Signal::Unsupported)?;
                    *arg = Value::Text(text.into());
                }
                _ => {}
            }
            // Validate converts the value as the Value setter does (Y47_ValidateFieldRef_Matrix: `int_to_text`, `dec_to_text`,
            // `text_to_int` with the Evaluate error, `dec_to_int` with Overflow, the length errors), where a Variant holding
            // the value would raise TypeConversion (`ops::convert`). The setter runs on a copy of the buffer: the target keeps
            // its value until Validate writes it.
            if method == "Validate"
                && let LibraryValue::FieldRef(rec, pos) | LibraryValue::KeyFieldRef(rec, pos) = &*target.borrow()
            {
                let copy = std::rc::Rc::new(std::cell::RefCell::new(rec.borrow().clone()));
                let scratch: Lib =
                    std::rc::Rc::new(std::cell::RefCell::new(LibraryValue::FieldRef(copy.clone(), *pos)));
                self.fieldref_op("Value", &scratch, &mut [arg.clone()])?;
                *arg = copy.borrow().fields[*pos].clone();
            }
        }
        Ok(())
    }

    /// The type of the field `target` points to.
    fn fieldref_type(&mut self, target: &Lib) -> Result<Option<RtType>, Signal> {
        let (rec, pos) = match &*target.borrow() {
            LibraryValue::FieldRef(r, p) | LibraryValue::KeyFieldRef(r, p) => (r.clone(), *p),
            _ => return Ok(None),
        };
        Ok(Some(self.table(&rec)?.fields[pos].ty.clone()))
    }
}
