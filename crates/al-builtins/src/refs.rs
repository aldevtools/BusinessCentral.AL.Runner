//! `RecordRef`, `FieldRef` and `KeyRef` methods. They need the table schema
//! and the store, so every implementation forwards its overload id to the VM
//! through [`Ctx::ref_op`]; the VM resolves the id to the method through the
//! catalog. Overloads not listed here are not implemented: the lowerer
//! refuses a call to them (hard rule 4).

use crate::native::{Ctx, NativeFn};
use crate::record_op::RecordOp;

macro_rules! forward {
    ($($id:literal),* $(,)?) => {
        &[$(($id, (|c: &mut dyn Ctx, r, a| c.ref_op($id, r, a)) as NativeFn)),*]
    };
}

pub const REGISTRY: &[(&str, NativeFn)] = forward![
    // RecordRef: the id numbers follow the catalog order, see `tests` below.
    "RecordRef.Open#2",
    "RecordRef.Close#1",
    "RecordRef.Number#1",
    "RecordRef.Name#1",
    "RecordRef.IsEmpty#1",
    "RecordRef.Count#1",
    "RecordRef.Field#1",
    "RecordRef.Field#2",
    "RecordRef.FieldCount#1",
    "RecordRef.FieldExist#1",
    "RecordRef.FieldExist#2",
    "RecordRef.FieldIndex#1",
    "RecordRef.Init#1",
    "RecordRef.Insert#1",
    "RecordRef.Insert#2",
    "RecordRef.Insert#3",
    "RecordRef.Modify#1",
    "RecordRef.Delete#1",
    "RecordRef.DeleteAll#1",
    "RecordRef.Find#1",
    "RecordRef.FindFirst#1",
    "RecordRef.FindLast#1",
    "RecordRef.FindSet#1",
    "RecordRef.FindSet#2",
    "RecordRef.Next#1",
    "RecordRef.Reset#1",
    "RecordRef.Rename#1",
    "RecordRef.GetTable#1",
    "RecordRef.SetTable#1",
    "RecordRef.SetTable#2",
    "RecordRef.KeyCount#1",
    "RecordRef.KeyIndex#1",
    "RecordRef.Caption#1",
    "RecordRef.CurrentCompany#1",
    "RecordRef.CurrentKey#1",
    "RecordRef.CurrentKeyIndex#1",
    "RecordRef.ChangeCompany#1",
    "RecordRef.IsTemporary#1",
    "RecordRef.HasFilter#1",
    "RecordRef.IsDirty#1",
    "RecordRef.GetFilters#1",
    "RecordRef.GetView#1",
    "RecordRef.FilterGroup#1",
    "RecordRef.SetView#1",
    "RecordRef.Duplicate#1",
    "RecordRef.RecordId#1",
    "RecordRef.GetPosition#1",
    "RecordRef.SetPosition#1",
    "RecordRef.LockTable#1",
    "RecordRef.ReadConsistency#1",
    "RecordRef.RecordLevelLocking#1",
    "RecordRef.ReadPermission#1",
    "RecordRef.WritePermission#1",
    "RecordRef.CountApprox#1",
    "RecordRef.ReadIsolation#1",
    "RecordRef.Truncate#1",
    "RecordRef.SetLoadFields#1",
    "RecordRef.AddLoadFields#1",
    "RecordRef.AreFieldsLoaded#1",
    "RecordRef.SetAutoCalcFields#1",
    "RecordRef.SecurityFiltering#1",
    "RecordRef.Ascending#1",
    "RecordRef.SetRecFilter#1",
    "RecordRef.Get#1",
    "RecordRef.GetBySystemId#1",
    "RecordRef.AddLink#1",
    "RecordRef.DeleteLink#1",
    "RecordRef.DeleteLinks#1",
    "RecordRef.HasLinks#1",
    "RecordRef.Copy#1",
    "RecordRef.Copy#2",
    "RecordRef.CopyLinks#1",
    "RecordRef.CopyLinks#2",
    "RecordRef.CopyLinks#3",
    "RecordRef.Mark#1",
    "RecordRef.MarkedOnly#1",
    "RecordRef.ClearMarks#1",
    "RecordId.TableNo#1",
    "RecordId.GetRecord#1",
    "RecordRef.SystemIdNo#1",
    "RecordRef.SystemCreatedAtNo#1",
    "RecordRef.SystemCreatedByNo#1",
    "RecordRef.SystemModifiedAtNo#1",
    "RecordRef.SystemModifiedByNo#1",
    // KeyRef
    "KeyRef.Active#1",
    "KeyRef.FieldCount#1",
    "KeyRef.FieldIndex#1",
    // FieldRef
    "FieldRef.Active#1",
    "FieldRef.Length#1",
    "FieldRef.Name#1",
    "FieldRef.Number#1",
    "FieldRef.SetRange#1",
    "FieldRef.SetFilter#1",
    "FieldRef.Value#1",
    "FieldRef.Caption#1",
    "FieldRef.Relation#1",
    "FieldRef.Type#1",
    "FieldRef.Class#1",
    "FieldRef.GetFilter#1",
    "FieldRef.GetRangeMin#1",
    "FieldRef.GetRangeMax#1",
    "FieldRef.Record#1",
    "FieldRef.Validate#1",
    "FieldRef.CalcField#1",
    "FieldRef.CalcSum#1",
    "FieldRef.IsOptimizedForTextSearch#1",
    "FieldRef.IsEnum#1",
    "FieldRef.EnumValueCount#1",
    "FieldRef.GetEnumValueName#1",
    "FieldRef.GetEnumValueCaption#1",
    "FieldRef.GetEnumValueOrdinal#1",
    "FieldRef.GetEnumValueNameFromOrdinalValue#1",
    "FieldRef.GetEnumValueCaptionFromOrdinalValue#1",
    "FieldRef.OptionCaption#1",
    "FieldRef.OptionMembers#1",
    "FieldRef.OptionString#1",
    // FieldRef.TestField: the untyped, `Any` and typed Value overloads and the ErrorInfo ones (the even numbers; the record's TestField
    // takes the ErrorInfo as its last argument).
    "FieldRef.TestField#1",
    "FieldRef.TestField#3",
    "FieldRef.TestField#5",
    "FieldRef.TestField#7",
    "FieldRef.TestField#9",
    "FieldRef.TestField#11",
    "FieldRef.TestField#13",
    "FieldRef.TestField#15",
    "FieldRef.TestField#17",
    "FieldRef.TestField#19",
    "FieldRef.TestField#21",
    "FieldRef.TestField#23",
    "FieldRef.TestField#25",
    "FieldRef.TestField#27",
    "FieldRef.TestField#29",
    "FieldRef.TestField#31",
    "FieldRef.TestField#33",
    "FieldRef.TestField#35",
    "FieldRef.TestField#2",
    "FieldRef.TestField#4",
    "FieldRef.TestField#6",
    "FieldRef.TestField#8",
    "FieldRef.TestField#10",
    "FieldRef.TestField#12",
    "FieldRef.TestField#14",
    "FieldRef.TestField#16",
    "FieldRef.TestField#18",
    "FieldRef.TestField#20",
    "FieldRef.TestField#22",
    "FieldRef.TestField#24",
    "FieldRef.TestField#26",
    "FieldRef.TestField#28",
    "FieldRef.TestField#30",
    "FieldRef.TestField#32",
    "FieldRef.TestField#34",
    "FieldRef.TestField#36",
    "FieldRef.FieldError#1",
    "FieldRef.FieldError#2",
    "FieldRef.FieldError#3",
    "KeyRef.Record#1",
];

macro_rules! record_methods {
    ($($id:literal => $m:ident),* $(,)?) => {
        pub const RECORD_REGISTRY: &[(&str, NativeFn)] =
            &[$(($id, (|c: &mut dyn Ctx, r, a| c.record(RecordOp::$m, r, a)) as NativeFn)),*];
        /// Each Record overload id with the record method it forwards to.
        pub const RECORD_METHODS: &[(&str, RecordOp)] = &[$(($id, RecordOp::$m)),*];
    };
}

// Record builtins that read table metadata or guard on session state.
record_methods! {
    "Record.TestField#1" => TestField,
    "Record.TestField#3" => TestField,
    "Record.TestField#5" => TestField,
    "Record.TestField#7" => TestField,
    "Record.TestField#9" => TestField,
    "Record.TestField#11" => TestField,
    "Record.TestField#13" => TestField,
    "Record.TestField#15" => TestField,
    "Record.TestField#17" => TestField,
    "Record.TestField#19" => TestField,
    "Record.TestField#21" => TestField,
    "Record.TestField#23" => TestField,
    // The ErrorInfo overloads: the check is the same; a failing one is refused (al-vm recmeta.rs).
    "Record.TestField#2" => TestField,
    "Record.TestField#4" => TestField,
    "Record.TestField#6" => TestField,
    "Record.TestField#8" => TestField,
    "Record.TestField#10" => TestField,
    "Record.TestField#12" => TestField,
    "Record.TestField#14" => TestField,
    "Record.TestField#16" => TestField,
    "Record.TestField#18" => TestField,
    "Record.TestField#20" => TestField,
    "Record.TestField#22" => TestField,
    "Record.TestField#24" => TestField,
    "Record.FieldCaption#1" => FieldCaption,
    "Record.FieldName#1" => FieldName,
    "Record.FieldNo#1" => FieldNo,
    "Record.FieldActive#1" => FieldActive,
    "Record.TableCaption#1" => TableCaption,
    "Record.TableName#1" => TableName,
    "Record.RecordId#1" => RecordId,
    "Record.Insert#3" => Insert,
    "Record.Mark#1" => Mark,
    "Record.MarkedOnly#1" => MarkedOnly,
    "Record.ClearMarks#1" => ClearMarks,
    "Record.GetRangeMin#1" => GetRangeMin,
    "Record.GetRangeMax#1" => GetRangeMax,
    "Record.GetBySystemId#1" => GetBySystemId,
    "Record.Relation#1" => Relation,
    "Record.GetPosition#1" => GetPosition,
    "Record.SetPosition#1" => SetPosition,
    "Record.TransferFields#1" => TransferFields,
    "Record.TransferFields#2" => TransferFields,
    "Record.LockTable#1" => LockTable,
    "Record.Consistent#1" => Consistent,
    "Record.ReadConsistency#1" => ReadConsistency,
    "Record.RecordLevelLocking#1" => RecordLevelLocking,
    "Record.ReadPermission#1" => ReadPermission,
    "Record.WritePermission#1" => WritePermission,
    "Record.CountApprox#1" => CountApprox,
    "Record.CurrentCompany#1" => CurrentCompany,
    "Record.ReadIsolation#1" => ReadIsolation,
    "Record.SecurityFiltering#1" => SecurityFiltering,
    "Record.ModifyAll#1" => ModifyAll,
    "Record.IsTemporary#1" => IsTemporary,
    "Record.IsDirty#1" => IsDirty,
    "Record.Ascending#1" => Ascending,
    "Record.ChangeCompany#1" => ChangeCompany,
    "Record.SetAscending#1" => SetAscending,
    "Record.GetAscending#1" => GetAscending,
    "Record.CurrentKey#1" => CurrentKey,
    "Record.HasFilter#1" => HasFilter,
    "Record.GetFilters#1" => GetFilters,
    "Record.GetFilter#1" => GetFilter,
    "Record.GetView#1" => GetView,
    "Record.SetView#1" => SetView,
    "Record.FilterGroup#1" => FilterGroup,
    "Record.SetRecFilter#1" => SetRecFilter,
    "Record.CopyFilters#1" => CopyFilters,
    "Record.CopyFilter#1" => CopyFilter,
    "Record.Copy#1" => Copy,
    "Record.FieldError#1" => FieldError,
    "Record.FieldError#2" => FieldError,
    "Record.FieldError#3" => FieldError,
    "Record.AddLink#1" => AddLink,
    "Record.DeleteLink#1" => DeleteLink,
    "Record.DeleteLinks#1" => DeleteLinks,
    "Record.HasLinks#1" => HasLinks,
    "Record.CopyLinks#1" => CopyLinks,
    "Record.CopyLinks#2" => CopyLinks,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;

    /// An id that drifts from the catalog (a corpus bump reorders overloads)
    /// must not silently bind to another method: each id's method name is
    /// the text in the id, and its first parameter is what this table says.
    #[test]
    fn ids_name_the_methods_and_overloads_they_claim() {
        let cat = Catalog::shared();
        let params = |id: &str| {
            let s = cat.by_id(id).unwrap_or_else(|| panic!("{id} is not in the catalog"));
            let (recv, name) = id.rsplit_once('#').unwrap().0.split_once('.').unwrap();
            assert!(
                s.receiver == recv && s.name == name,
                "{id} is {}.{}",
                s.receiver,
                s.name
            );
            s.params.iter().map(|p| p.ty.clone()).collect::<Vec<_>>()
        };
        assert_eq!(params("RecordRef.Open#2")[0], "Integer");
        assert_eq!(params("RecordRef.Field#2")[0], "Integer");
        assert_eq!(params("RecordRef.FieldExist#2")[0], "Integer");
        assert!(params("RecordRef.Insert#1").is_empty());
        assert_eq!(params("RecordRef.Insert#2"), ["Boolean"]);
        assert!(params("RecordRef.FindSet#1").len() == 1 && params("RecordRef.FindSet#2").len() == 2);
        assert_eq!(params("RecordRef.GetTable#1"), ["Record"]);
        assert_eq!(params("RecordRef.SetTable#1"), ["Record"]);
        for (id, _) in REGISTRY.iter().chain(RECORD_REGISTRY) {
            params(id);
        }
        // The Record overloads forward to a method name: it must be the id's own method.
        for (id, method) in RECORD_METHODS {
            assert_eq!(&cat.by_id(id).unwrap().name, method.name(), "{id}");
        }
        // The ErrorInfo overloads registered are those of FieldError (handlers/PWRefusedBuiltins.al:113-168)
        // and Record.TestField's, whose check that holds raises nothing and whose failure is refused
        // (issue 41, docs/probes-needed.md). FieldRef.TestField's are those of the Record's, pinned by record/lt-testfield-typed-values/
        // FieldRefTestFieldTypedNumeric.al:P189_FldRef_Byte_WithErrorInfo and its siblings.
        for (id, _) in REGISTRY.iter().chain(RECORD_REGISTRY) {
            let has = cat.by_id(id).unwrap().params.iter().any(|p| p.ty == "ErrorInfo");
            if has {
                assert!(
                    id.contains("FieldError#3")
                        || id.starts_with("Record.TestField#")
                        || id.starts_with("FieldRef.TestField#"),
                    "{id}"
                );
            }
        }
    }
}
