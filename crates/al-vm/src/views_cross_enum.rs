//! A member of another enum in a page's view property, `const("Purchase Document Type"::Order)` on a field of enum "Purchase
//! Comment Document Type" (al-runner issue 7265): see `al_builtins::enum_qualifier::cross_enum_member` for the pin and what
//! stays refused.

use std::borrow::Cow;

use crate::Session;

impl Session {
    /// `value` as the member of the field's enum `field_enum` when the other enum names a member with the same name and ordinal
    /// (record/qualified-enum-filters/TestQualifiedEnumFilters.al:162); otherwise `value` as it is, which the filter parser then refuses.
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
            Some(member) => Cow::Owned(member),
            None => Cow::Borrowed(value),
        }
    }
}

impl Session {
    /// The filter value of a view property whose operands are qualified constants, with each operand resolved: a member of the
    /// field's own enum (quoted, as the filter parser reads a member name) or, on an Integer field, `Database::"Table"` and
    /// `Page::"Page"` as the object's number. The shapes and their pins: `al_builtins::enum_qualifier::resolve_qualified_operands`
    /// (pages/link-filter/LinkEnumQualifiedAndObjectConstants.al, al-runner issues 7405, 7411). `None` keeps the caller's
    /// behavior for every other value.
    pub(crate) fn view_qualified_operands(&self, value: &str, ty: &al_builtins::value::RtType) -> Option<String> {
        use al_builtins::value::RtType;
        let objects = &self.engine.program.objects;
        let short = |name: &str| name.rsplit(['.', '\u{1f}']).next().unwrap_or(name).to_string();
        let object_id = |kind: al_ir::ObjKind, name: &str| {
            objects
                .iter()
                .find(|o| o.kind == kind && short(&o.name).eq_ignore_ascii_case(name))
                .and_then(|o| o.id)
                .map(|id| id.to_string())
        };
        al_builtins::enum_qualifier::resolve_qualified_operands(value, |qualifier, name| match ty {
            RtType::Enum(id) => {
                let field_enum = &objects[*id as usize];
                (short(&field_enum.name).eq_ignore_ascii_case(qualifier) && !name.contains('\''))
                    .then(|| {
                        field_enum
                            .enum_values
                            .iter()
                            .find(|(_, n)| n.eq_ignore_ascii_case(name))
                    })
                    .flatten()
                    .map(|(_, n)| format!("'{n}'"))
            }
            RtType::Integer if qualifier.eq_ignore_ascii_case("Database") => object_id(al_ir::ObjKind::Table, name),
            RtType::Integer if qualifier.eq_ignore_ascii_case("Page") => object_id(al_ir::ObjKind::Page, name),
            _ => None,
        })
    }
}
