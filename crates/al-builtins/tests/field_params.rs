//! Every catalog parameter that names a field (`Param::names_field`): a record field passed there is lowered to its
//! position, everywhere else to its value. A catalog change that adds or drops one shows here.

#[test]
fn the_parameters_that_name_a_field() {
    let c = al_builtins::catalog::Catalog::load();
    let mut got: Vec<String> = c
        .signatures
        .iter()
        .flat_map(|s| {
            s.params
                .iter()
                .filter(|p| p.names_field())
                .map(move |p| format!("{}.{}({}: {})", s.receiver, s.name, p.name, p.ty))
        })
        .collect();
    got.sort();
    got.dedup();
    let want = [
        "FilterPageBuilder.AddField(Field: Any)",
        "Page.Run(Field: Any)",
        "Page.RunModal(Field: Any)",
        "Query.ColumnCaption(Column: Any)",
        "Query.ColumnName(Column: Any)",
        "Query.ColumnNo(Column: Any)",
        "Query.GetFilter(Column: Any)",
        "Query.SetFilter(Column: Any)",
        "Query.SetRange(Column: Any)",
        "Record.AddLoadFields(Fields: Any)",
        "Record.AreFieldsLoaded(Fields: Any)",
        "Record.CalcFields(Field1: Any)",
        "Record.CalcFields(Field2: Any)",
        "Record.CalcSums(Field1: Any)",
        "Record.CalcSums(Field2: Any)",
        "Record.CopyFilter(FromField: Any)",
        "Record.FieldActive(Field: Any)",
        "Record.FieldCaption(Field: Any)",
        "Record.FieldError(Field: Any)",
        "Record.FieldName(Field: Any)",
        "Record.FieldNo(Field: Any)",
        "Record.GetAscending(Field: Any)",
        "Record.GetFilter(Field: Any)",
        "Record.GetRangeMax(Field: Any)",
        "Record.GetRangeMin(Field: Any)",
        "Record.LoadFields(Fields: Any)",
        "Record.ModifyAll(Field: Any)",
        "Record.Relation(Field: Any)",
        "Record.SetAscending(Field: Any)",
        "Record.SetAutoCalcFields(Field1: Any)",
        "Record.SetCurrentKey(Field1: Any)",
        "Record.SetCurrentKey(Field2: Any)",
        "Record.SetFilter(Field: Any)",
        "Record.SetLoadFields(Fields: Any)",
        "Record.SetRange(Field: Any)",
        "Record.TestField(Field: Any)",
        "Record.Validate(Field: Any)",
        "TestFilter.GetFilter(Field: TestFilterField)",
        "TestFilter.SetCurrentKey(Field1: TestFilterField)",
        "TestFilter.SetCurrentKey(Field2: TestFilterField)",
        "TestFilter.SetFilter(Field: TestFilterField)",
    ];
    assert_eq!(got, want, "\n{}", got.join("\n"));
}

/// The parameters whose name starts with `Field` but take a value (al-runner issue 7616).
#[test]
fn a_parameter_that_only_starts_with_field_takes_a_value() {
    let c = al_builtins::catalog::Catalog::load();
    for (recv, method, param) in [
        ("RecordRef", "Field", "FieldName"),
        ("RecordRef", "FieldExist", "FieldName"),
        ("WebServiceActionContext", "AddEntityKey", "FieldValue"),
    ] {
        let p = c
            .overloads(recv, method)
            .into_iter()
            .flat_map(|s| s.params.iter())
            .find(|p| p.name == param)
            .unwrap_or_else(|| panic!("{recv}.{method} has no parameter {param}"));
        assert!(!p.names_field(), "{recv}.{method}({param})");
    }
}
