//! A member of another enum in a page's view property, `const("Purchase Document Type"::Order)` on a field of enum "Purchase
//! Comment Document Type" (al-runner issue 7265): see `al_builtins::enum_qualifier::cross_enum_member` for the pin and what
//! stays refused.

use std::borrow::Cow;

use crate::Session;

impl Session {
    /// `value` as the member of the field's enum `field_enum` when the other enum names a member with the same name and ordinal
    /// (marked as the assumption cross-enum-view-member); otherwise `value` as it is, which the filter parser then refuses.
    pub(crate) fn cross_enum_member<'a>(&self, value: &'a str, field_enum: u32) -> Cow<'a, str> {
        let objects = &self.engine.program.objects;
        let members_of = |name: &str| {
            objects
                .iter()
                .find(|o| {
                    o.kind == al_ir::ObjKind::Enum
                        && o.name
                            .rsplit(['.', '\u{1f}'])
                            .next()
                            .unwrap_or(&o.name)
                            .eq_ignore_ascii_case(name)
                })
                .map(|o| o.enum_values.clone())
        };
        match al_builtins::enum_qualifier::cross_enum_member(
            value,
            &objects[field_enum as usize].enum_values,
            members_of,
        ) {
            Some(member) => {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::CrossEnumViewMember);
                Cow::Owned(member)
            }
            None => Cow::Borrowed(value),
        }
    }
}
