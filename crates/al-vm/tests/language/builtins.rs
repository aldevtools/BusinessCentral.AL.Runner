//! End to end: single builtins that need the compiled program (enum metadata) or session state.
//! Each test names the corpus test that pins the behaviour it asserts.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::AppSource;
use al_vm::{Engine, Session};

fn run(src: &str, routine: &str) -> Result<Value, Signal> {
    al_testkit::run(src, routine)
}

const STATUS: &str = "enum 50101 \"ALT Status\" { value(0; \" \") { Caption = ' '; } value(1; Draft) { Caption = 'Draft'; } value(5; Closed) { Caption = 'Closed'; } }\n";

/// Runs `procedure F(): Integer var <vars> begin <body> end;` next to the enum.
fn int_of(vars: &str, body: &str) -> Result<i32, Signal> {
    let src = format!("{STATUS}codeunit 50100 C {{ procedure F(): Integer var {vars} begin {body} end; }}");
    match run(&src, "F")? {
        Value::Integer(i) => Ok(i),
        other => panic!("{other:?}"),
    }
}

// ---------------------------------------------------------------- Enum

#[test]
fn enum_names_lists_every_declared_name() {
    // types/TestEnumNamesContracts.al:19-60: `"ALT Status".Names()` holds the names, blank ordinal included.
    let v = "N: List of [Text];";
    assert_eq!(int_of(v, "N := \"ALT Status\".Names(); exit(N.Count());").unwrap(), 3);
    assert_eq!(
        int_of(
            v,
            "N := \"ALT Status\".Names(); if N.Contains('Draft') and N.Contains('Closed') then exit(1); exit(0);"
        )
        .unwrap(),
        1
    );
}

#[test]
fn enum_names_works_on_a_variable() {
    // types/TestEnumNamesContracts.al:77-85 (`S.Names()`).
    let v = "S: Enum \"ALT Status\"; N: List of [Text];";
    assert_eq!(
        int_of(v, "S := \"ALT Status\"::Draft; N := S.Names(); exit(N.Count());").unwrap(),
        3
    );
}

#[test]
fn enum_ordinals_lists_every_declared_value() {
    // session/TestSystemExtended.al:381-390 and types/TestLanguageEdgeCases.al:91-105.
    let v = "O: List of [Integer];";
    assert_eq!(
        int_of(v, "O := \"ALT Status\".Ordinals(); exit(O.Count());").unwrap(),
        3
    );
    assert_eq!(int_of(v, "O := \"ALT Status\".Ordinals(); if O.Contains(0) and O.Contains(1) and O.Contains(5) then exit(1); exit(0);").unwrap(), 1);
}

#[test]
fn names_and_ordinals_line_up() {
    // types/TestEnumNamesContracts.al:62-70: equal counts; the Nth name belongs to the Nth ordinal.
    let v = "N: List of [Text]; O: List of [Integer];";
    assert_eq!(
        int_of(v, "N := \"ALT Status\".Names(); O := \"ALT Status\".Ordinals(); if (N.Get(2) = 'Draft') and (O.Get(2) = 1) then exit(1); exit(0);").unwrap(),
        1
    );
}

#[test]
fn enum_as_integer_returns_the_ordinal() {
    // types/TestEnum.al:27-40.
    let v = "S: Enum \"ALT Status\";";
    assert_eq!(
        int_of(v, "S := \"ALT Status\"::Closed; exit(S.AsInteger());").unwrap(),
        5
    );
    assert_eq!(int_of(v, "exit(S.AsInteger());").unwrap(), 0);
}

// types/lt-filter-enum-relation-duration-guid/B_EnumOptionVariant.al:EnumVariant_To_Integer pins an Enum in a Variant read as an
// Integer by assignment; FromInteger taking such a Variant is the same conversion (enum/lt-enum-fromint-variant/EnumFromIntegerVariant.al:
// FromInteger_VariantHolding `enum`, `decimal` 1 and `text` '1' all give Green; a fraction or a non-number text is not pinned: refused).
#[test]
fn enum_from_integer_takes_a_variant_that_holds_an_enum() {
    let v = "S: Enum \"ALT Status\"; T: Enum \"ALT Status\"; V: Variant;";
    assert_eq!(
        int_of(
            v,
            "T := \"ALT Status\"::Closed; V := T; S := \"ALT Status\".FromInteger(V); exit(S.AsInteger());"
        )
        .unwrap(),
        5
    );
}

#[test]
fn enum_from_integer_takes_a_variant_that_holds_a_whole_decimal_or_a_number_text() {
    let v = "S: Enum \"ALT Status\"; V: Variant; T: Text;";
    assert_eq!(
        int_of(v, "V := 5.0; S := \"ALT Status\".FromInteger(V); exit(S.AsInteger());").unwrap(),
        5
    );
    assert_eq!(
        int_of(
            v,
            "T := '5'; V := T; S := \"ALT Status\".FromInteger(V); exit(S.AsInteger());"
        )
        .unwrap(),
        5
    );
    assert!(int_of(v, "V := 1.5; S := \"ALT Status\".FromInteger(V); exit(S.AsInteger());").is_err());
    assert!(
        int_of(
            v,
            "T := 'abc'; V := T; S := \"ALT Status\".FromInteger(V); exit(S.AsInteger());"
        )
        .is_err()
    );
}

// foreach over Ordinals into an Enum variable (enum/lt-enum-foreach-integer/EnumForeachInteger.al:15): the variable holds the enum value.
#[test]
fn foreach_over_ordinals_into_an_enum_variable_gives_enum_values() {
    // Draft (1) and Closed (5) are the ordinals after the blank one: the loop sees the enum value of each.
    let r = int_of(
        "S: Enum \"ALT Status\"; N: Integer;",
        "foreach S in S.Ordinals() do if S = \"ALT Status\"::Closed then N += 10 else if S = \"ALT Status\"::Draft then N += 1; exit(N);",
    );
    assert_eq!(r.unwrap(), 11);
}

// enum/lt-enum-foreach-integer/EnumForeachInteger.al:32: foreach over a List of Enum into an Integer variable gives the ordinals.
#[test]
fn foreach_over_enums_into_an_integer_variable_gives_ordinals() {
    // Closed is ordinal 5 and Draft 1: 5 * 100 + 1.
    let r = int_of(
        "L: List of [Enum \"ALT Status\"]; V: Integer; N: Integer;",
        "L.Add(\"ALT Status\"::Closed); L.Add(\"ALT Status\"::Draft); foreach V in L do N := N * 100 + V; exit(N);",
    );
    assert_eq!(r.unwrap(), 501);
}

// errors/lt-ms-refusals-128/P128EvalBasic.al:P128_Evaluate_IntoText, _IntoCode, _IntoShortText_ResultUnused: a Text keeps the text as written,
// a Code is trimmed and upper case, and a text longer than the variable fails (No with the result used).
#[test]
fn evaluate_into_text_and_code() {
    let r = text_of(
        "T: Text; C: Code[5]; S: Text[5]; B1: Boolean; B2: Boolean; B3: Boolean;",
        "T := 'KEEP'; B1 := Evaluate(T, ' hello '); C := 'KEEP'; B2 := Evaluate(C, ' ab '); S := 'KEEP'; B3 := Evaluate(S, 'abcdef'); \
         exit('[' + T + ']' + Format(B1) + '[' + C + ']' + Format(B2) + '[' + S + ']' + Format(B3));",
    );
    assert_eq!(r.unwrap(), "[ hello ]Yes[AB]Yes[KEEP]No");
}

#[test]
fn enum_from_integer_carries_any_ordinal() {
    // types/TestEnum.al:15-22 (a declared ordinal), :83-91 and types/TestEnumDefaultImplTests.Codeunit.al:70
    // (an undeclared one keeps its number); types/TestEnumShippedOrdinalTests.Codeunit.al:91 spells the
    // receiver `Enum::"Name"`.
    let v = "S: Enum \"ALT Status\";";
    assert_eq!(
        int_of(
            v,
            "S := \"ALT Status\".FromInteger(5); if S = \"ALT Status\"::Closed then exit(1); exit(0);"
        )
        .unwrap(),
        1
    );
    assert_eq!(
        int_of(v, "S := \"ALT Status\".FromInteger(99); exit(S.AsInteger());").unwrap(),
        99
    );
    assert_eq!(
        int_of(v, "S := Enum::\"ALT Status\".FromInteger(1); exit(S.AsInteger());").unwrap(),
        1
    );
}

// ---------------------------------------------------------------- NumberSequence

fn text_of(vars: &str, body: &str) -> Result<String, Signal> {
    let src = format!("codeunit 50100 C {{ procedure F(): Text var {vars} begin {body} end; }}");
    match run(&src, "F")? {
        Value::Text(s) | Value::Code(s) => Ok(s.to_string()),
        other => panic!("{other:?}"),
    }
}

// platform/lt-platform-members/PlTNumberSequence.al:372-394 (UpperLimit_NoCycle): a Next past the BigInteger range raises the
// SQL error and the sequence is gone afterwards.
#[test]
fn number_sequence_next_past_the_range_raises_and_drops_the_sequence() {
    let src = "codeunit 50100 C {
        procedure F(): Text var a: Text; B: BigInteger; begin
            Evaluate(B, '9223372036854775807');
            NumberSequence.Insert('S', B, 1);
            a := Format(NumberSequence.Next('S'));
            if not TryNext() then a += '|' + GetLastErrorText() + '|' + GetLastErrorCode();
            exit(a + '|' + Format(NumberSequence.Exists('S')));
        end;
        [TryFunction] local procedure TryNext() begin NumberSequence.Next('S'); end;
    }";
    let r = run(src, "F");
    assert!(
        matches!(&r, Ok(Value::Text(t)) if &**t == "9223372036854775807|An unexpected error occurred after a database command was cancelled.|Sql:Sql|No"),
        "{r:?}"
    );
}

#[test]
fn number_sequence_honours_seed_and_increment_per_scope() {
    // session/TestNumberSequence.al:15-60: the default scope is company specific, a global one is
    // independent, and Next starts at the seed and advances by the increment.
    let r = text_of(
        "a: Text;",
        "NumberSequence.Insert('S', 5, 2);
         a := Format(NumberSequence.Exists('S', true)) + Format(NumberSequence.Exists('S', false));
         a += '|' + Format(NumberSequence.Next('S')) + ',' + Format(NumberSequence.Next('S'));
         NumberSequence.Insert('S', 100, 10, false);
         a += '|' + Format(NumberSequence.Next('S', false)) + ',' + Format(NumberSequence.Next('S', true));
         exit(a);",
    );
    assert_eq!(r.unwrap(), "YesNo|5,7|100,9");
}

#[test]
fn number_sequence_current_restart_range_and_delete() {
    // session/TestNumberSequence.al:63-170.
    let r = text_of(
        "a: Text; i: BigInteger; first: BigInteger;",
        "NumberSequence.Insert('S', 10, 3, false);
         a := Format(NumberSequence.Current('S', false));
         first := NumberSequence.Range('S', 4, false);
         a += '|' + Format(first) + ',' + Format(NumberSequence.Current('S', false)) + ',' + Format(NumberSequence.Next('S', false));
         NumberSequence.Restart('S', 50, false);
         a += '|' + Format(NumberSequence.Next('S', false)) + ',' + Format(NumberSequence.Next('S', false));
         first := NumberSequence.Range('S', 2, i, false);
         a += '|' + Format(first) + ',' + Format(i);
         NumberSequence.Delete('S', false);
         exit(a + '|' + Format(NumberSequence.Exists('S', false)));",
    );
    assert_eq!(r.unwrap(), "10|10,19,22|50,53|56,3|No");
}

#[test]
fn number_sequence_errors_are_catchable_and_a_missing_delete_is_not_one() {
    // session/TestNumberSequence.al:173-205 and error-handling/TestNumberSequenceTrappableErrors.al.
    let r = text_of(
        "a: Text; v: BigInteger;",
        "NumberSequence.Insert('S', 1, 1, false);
         asserterror NumberSequence.Insert('S', 100, 10, false);
         a := GetLastErrorText();
         NumberSequence.Delete('S', false);
         NumberSequence.Delete('S', false);
         asserterror v := NumberSequence.Next('S', false);
         a += '|' + GetLastErrorText();
         asserterror v := NumberSequence.Current('S', false);
         a += '|' + GetLastErrorText();
         asserterror NumberSequence.Restart('S', 1, false);
         a += '|' + GetLastErrorText();
         asserterror v := NumberSequence.Range('S', 3, false);
         a += '|' + GetLastErrorText();
         exit(a + '|' + Format(NumberSequence.Exists('S', false)));",
    );
    let r = r.unwrap();
    let parts: Vec<&str> = r.split('|').collect();
    assert!(parts[0].contains("already exists"), "{r}");
    assert!(parts[1..=4].iter().all(|p| p.contains("does not exist")), "{r}");
    assert_eq!(parts[5], "No");
}

#[test]
fn number_sequence_survives_an_asserterror_rollback() {
    // session/TestNumberSequence.al:173-186 and :188-201: a failed call leaves the sequence as it
    // was; error-handling/TestNumberSequenceTrappableErrors.al:134 ("Number sequences are
    // database objects, not rows"): a sequence outlives the rollback an asserterror triggers.
    let r = text_of(
        "a: Text;",
        "NumberSequence.Insert('S', 1, 1, false);
         asserterror Error('x');
         exit(Format(NumberSequence.Exists('S', false)));",
    );
    assert_eq!(r.unwrap(), "Yes");
}

#[test]
fn number_sequence_defaults_texts_and_codes_the_probe_pins() {
    // session/TestNumberSequenceRuntimeContracts.al: Insert without arguments starts at 0 and steps by 1;
    // Restart without a seed makes the sequence 0 again.
    let r = text_of(
        "",
        "NumberSequence.Insert('S'); exit(Format(NumberSequence.Next('S')) + Format(NumberSequence.Next('S')));",
    );
    assert_eq!(r.unwrap(), "01");
    let r = text_of(
        "",
        "NumberSequence.Insert('S', 10, 5); NumberSequence.Restart('S'); exit(Format(NumberSequence.Current('S')));",
    );
    assert_eq!(r.unwrap(), "0");
    // Texts quote the name, the code is AL; an Insert of an existing name leaves the sequence gone;
    // Range with a count of 0 is the SQL error.
    let r = text_of(
        "",
        "asserterror NumberSequence.Next('M'); exit(GetLastErrorText() + '|' + GetLastErrorCode());",
    );
    assert_eq!(r.unwrap(), "The number sequence 'M' does not exist.|AL");
    let r = text_of(
        "",
        "NumberSequence.Insert('S', 1, 1); asserterror NumberSequence.Insert('S', 1, 1);
         exit(GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(NumberSequence.Exists('S')));",
    );
    assert_eq!(r.unwrap(), "The number sequence 'S' already exists.|AL|No");
    let r = text_of(
        "a: BigInteger;",
        "NumberSequence.Insert('S', 1, 1); asserterror a := NumberSequence.Range('S', 0, a);
         exit(GetLastErrorText() + '|' + GetLastErrorCode());",
    );
    assert_eq!(
        r.unwrap(),
        "An unexpected error occurred after a database command was cancelled.|Sql:Sql"
    );
}

#[test]
fn number_sequence_forms_the_corpus_does_not_pin_are_refused() {
    let r = text_of(
        "a: BigInteger;",
        "NumberSequence.Insert('S', 1, 1); a := NumberSequence.Range('S', -1, a); exit('');",
    );
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

// ---------------------------------------------------------------- Database.IsInWriteTransaction

const ROWS: &str = "table 50900 \"ALT Row\" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }\n";

fn text_with_table(body: &str) -> Result<String, Signal> {
    let src = format!("{ROWS}codeunit 50100 C {{ procedure F(): Text var R: Record \"ALT Row\"; begin {body} end; }}");
    match run(&src, "F")? {
        Value::Text(s) => Ok(s.to_string()),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_write_opens_a_write_transaction_until_the_commit() {
    // codeunit/TestWriteTxTestBoundary.al:93,103,142: an uncommitted Insert leaves one open; a
    // Commit (and the test boundary) closes it.
    let r = text_with_table(
        "exit(Format(Database.IsInWriteTransaction()));
         ",
    );
    assert_eq!(r.unwrap(), "No");
    let r = text_with_table(
        "R.Id := 1; R.Insert();
         exit(Format(Database.IsInWriteTransaction()));",
    );
    assert_eq!(r.unwrap(), "Yes");
    let r = text_with_table(
        "R.Id := 1; R.Insert(); Commit();
         exit(Format(Database.IsInWriteTransaction()));",
    );
    assert_eq!(r.unwrap(), "No");
}

// ---------------------------------------------------------------- WebServiceActionContext

fn wsac(body: &str) -> Result<String, Signal> {
    text_of(
        "c: WebServiceActionContext; d: WebServiceActionContext; a: Text; b: Boolean; v: Variant;",
        body,
    )
}

#[test]
fn wsac_fresh_instance_and_round_trips() {
    // session/TestWebServiceActionContext.al:180-250 (defaults) and :256-345 (round trips).
    let r = wsac(
        "a := Format(c.GetObjectId()) + '|' + Format(c.GetResultCode());
         c.SetObjectId(50123); c.SetObjectType(ObjectType::Page); c.SetResultCode(WebServiceActionResultCode::Created);
         a += '|' + Format(c.GetObjectId()) + ',' + Format(c.GetResultCode());
         if c.GetObjectType() = ObjectType::Page then a += ',page';
         c.SetObjectId(-7);
         exit(a + '|' + Format(c.GetObjectId()));",
    );
    assert_eq!(r.unwrap(), "0|None|50123,Created,page|-7");
}

#[test]
fn wsac_a_fresh_object_type_is_none_of_the_seven_members() {
    // session/TestWebServiceActionContext.al:187-200 and :411-428 (Clear returns it to the pristine value).
    let r = wsac(
        "if (c.GetObjectType() <> ObjectType::Table) and (c.GetObjectType() <> ObjectType::Page) and (c.GetObjectType() <> ObjectType::Report)
            and (c.GetObjectType() <> ObjectType::Codeunit) and (c.GetObjectType() <> ObjectType::XmlPort)
            and (c.GetObjectType() <> ObjectType::Query) and (c.GetObjectType() <> ObjectType::MenuSuite) then a := 'none';
         c.SetObjectType(ObjectType::Page);
         Clear(c);
         if c.GetObjectType() = d.GetObjectType() then a += ',pristine';
         exit(a);",
    );
    assert_eq!(r.unwrap(), "none,pristine");
}

#[test]
fn wsac_updated_reads_back_as_get() {
    // session/TestWebServiceActionContext.al:352-395: Get and Updated share the value 200.
    let r = wsac(
        "c.SetResultCode(WebServiceActionResultCode::Updated); a := Format(c.GetResultCode());
         c.SetResultCode(WebServiceActionResultCode::Get); a += ',' + Format(c.GetResultCode());
         c.SetResultCode(WebServiceActionResultCode::Deleted); a += ',' + Format(c.GetResultCode());
         c.SetResultCode(WebServiceActionResultCode::None); a += ',' + Format(c.GetResultCode());
         exit(a);",
    );
    assert_eq!(r.unwrap(), "Get,Get,Deleted,None");
}

#[test]
fn wsac_entity_keys_are_a_map_by_field_id() {
    // session/TestWebServiceActionContext.al:437-520: a duplicate field id returns false when the
    // result is used and raises (naming the type) when it is not; Clear empties the map.
    let r = wsac(
        "a := Format(c.AddEntityKey(1, 'x')) + Format(c.AddEntityKey(2, 42)) + Format(c.AddEntityKey(1, 'y')) + Format(c.AddEntityKey(1, 'x'));
         asserterror c.AddEntityKey(1, 'z');
         a += '|' + Format(StrPos(GetLastErrorText(), 'WebServiceActionContext') > 0);
         Clear(c);
         exit(a + '|' + Format(c.AddEntityKey(1, 'again')));",
    );
    assert_eq!(r.unwrap(), "YesYesNoNo|Yes|Yes");
}

#[test]
fn wsac_assignment_shares_the_context_and_clear_replaces_it() {
    // session/TestWebServiceActionContext.al:552-590: `Copy := Original` shares; the keys too.
    let r = wsac(
        "c.SetObjectId(50123); d := c; d.SetObjectId(50124); a := Format(c.GetObjectId());
         b := c.AddEntityKey(1, 'k');
         a += ',' + Format(d.AddEntityKey(1, 'k'));
         Clear(d);
         exit(a + ',' + Format(c.GetObjectId()) + ',' + Format(d.GetObjectId()));",
    );
    assert_eq!(r.unwrap(), "50124,No,50124,0");
}

#[test]
fn wsac_in_a_variant_has_an_empty_text() {
    // session/TestWebServiceActionContext.al:615-650.
    let r = wsac(
        "c.SetObjectId(50123); v := c;
         exit(Format(v.IsInteger()) + Format(v.IsText()) + Format(v.IsRecord()) + '|' + Format(v));",
    );
    assert_eq!(r.unwrap(), "NoNoNo|");
}

// ---------------------------------------------------------------- SessionSettings

fn settings(body: &str) -> Result<String, Signal> {
    text_of(
        "s: SessionSettings; t: SessionSettings; p: SessionSettings; a: Text; g: Guid; v: Variant;",
        body,
    )
}

#[test]
fn session_settings_properties_round_trip_and_are_independent() {
    // session/TestSessionSettings.al:25-165.
    let r = settings(
        "s.LanguageId(1030); s.LocaleId(2057); s.TimeZone('Pacific Standard Time'); s.ProfileId('ALT PROFILE');
         s.Company('ALT Company'); g := CreateGuid(); s.ProfileAppId(g);
         s.TimeZone('UTC');
         exit(Format(s.LanguageId()) + '|' + Format(s.LocaleId()) + '|' + s.TimeZone() + '|' + s.ProfileId() + '|' + s.Company() + '|' + Format(s.ProfileAppId() = g));",
    );
    assert_eq!(r.unwrap(), "1030|2057|UTC|ALT PROFILE|ALT Company|Yes");
}

#[test]
fn session_settings_profile_system_scope_ignores_its_argument() {
    // session/TestSessionSettings.al:172-215.
    let r = settings(
        "a := Format(s.ProfileSystemScope()); s.ProfileSystemScope(true); a += Format(s.ProfileSystemScope());
         s.ProfileSystemScope(false); exit(a + Format(s.ProfileSystemScope()));",
    );
    assert_eq!(r.unwrap(), "NoNoNo");
}

#[test]
fn session_settings_assignment_copies_by_value() {
    // session/TestSessionSettings.al:222-262.
    let r = settings(
        "s.Company('Source'); s.LanguageId(1030); t := s; t.Company('Target'); t.LanguageId(2057);
         exit(s.Company() + Format(s.LanguageId()) + '|' + t.Company() + Format(t.LanguageId()));",
    );
    assert_eq!(r.unwrap(), "Source1030|Target2057");
}

#[test]
fn session_settings_compare_by_value_and_format_shows_the_contents() {
    // session/TestSessionSettings.al:478-560.
    let r = settings(
        "s.Company('C'); s.LanguageId(1030); t.Company('C'); t.LanguageId(1030);
         a := Format(s = t);
         t.LanguageId(2057); a += Format(s = t);
         s.TimeZone('Pacific Standard Time'); t.TimeZone('UTC');
         a += Format(StrPos(Format(s), 'Pacific Standard Time') > 0) + Format(Format(s) = Format(t));
         v := s; 
         exit(a);",
    );
    assert_eq!(r.unwrap(), "YesNoYesNo");
}

#[test]
fn session_settings_clear_returns_to_a_pristine_instance() {
    // session/TestSessionSettings.al:565-595.
    let r = settings("s.Company('C'); s.LanguageId(1030); Clear(s); a := Format(s = p) + s.Company(); exit(a);");
    assert_eq!(r.unwrap(), "Yes");
}

#[test]
fn session_settings_defaults_of_an_instance_nobody_set() {
    // session/TestProbeSessionMiscContracts.al:SessionSettings_PristineProperties. RequestSessionUpdate(false) goes to a
    // handler (SessionSettings_RequestUpdateFalse_Handler); without one it is refused.
    let r = settings(
        "exit(Format(s.LanguageId()) + '|' + Format(s.LocaleId()) + '|' + s.TimeZone() + '|' + s.ProfileId() + '|' + s.Company());",
    );
    assert_eq!(r.unwrap(), "0|0|||");
    let r = settings("s.RequestSessionUpdate(false); exit('');");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

// ---------------------------------------------------------------- Notification

fn notification(body: &str) -> Result<String, Signal> {
    text_of("n: Notification; a: Text; v: Variant;", body)
}

#[test]
fn notification_message_and_data() {
    // handlers/TestNotificationHandler.al:28-35, session/TestMiscTypes.al:155-184,
    // session/TestCalcDateNotificationContracts.al:130-180.
    let r = notification(
        "n.Message('Hello'); a := n.Message();
         a += '|' + Format(n.HasData('k1'));
         n.SetData('k1', 'v1'); n.SetData('k2', 'v2');
         a += Format(n.HasData('k1')) + n.GetData('k1') + n.GetData('k2');
         n.SetData('k1', 'again');
         exit(a + n.GetData('k1'));",
    );
    assert_eq!(r.unwrap(), "Hello|NoYesv1v2again");
}

#[test]
fn notification_scope_and_actions_are_accepted() {
    // session/TestCalcDateNotificationContracts.al:182-188, session/TestSystemExtended.al:553-575,
    // session/TestMiscComplete.al:106-119: they must not throw.
    let r = notification(
        "n.Message('Test'); n.Scope(NotificationScope::LocalScope);
         n.AddAction('Click me', 60019, 'Initialize'); n.AddAction('Click Me', 60145, 'Initialize', 'Action description');
         v := n;
         exit(Format(v.IsNotification()));",
    );
    assert_eq!(r.unwrap(), "Yes");
}

// handlers/TestNotificationSendRecallContracts.al:65-69: a notification nobody set reads empty.
#[test]
fn a_notification_nobody_set_reads_empty() {
    let r = notification(
        "exit(n.Message() + '|' + n.GetData('missing') + '|' + Format(n.Scope()) + '|' + Format(n.HasData('k')));",
    );
    assert_eq!(r.unwrap(), "||LocalScope|No");
}

// ---------------------------------------------------------------- ErrorInfo

const EI_ROW: &str = "table 50901 \"EI Row\" { fields { field(1; Id; Integer) { } field(2; Payload; Text[30]) { } } keys { key(PK; Id) { } } }\n";

fn error_info(body: &str) -> Result<String, Signal> {
    let src = format!(
        "{EI_ROW}codeunit 50100 C {{ procedure F(): Text var e: ErrorInfo; f: ErrorInfo; r1: Record \"EI Row\"; r2: Record \"EI Row\"; a: Text; d: Dictionary of [Text, Text]; back: Dictionary of [Text, Text]; begin {body} end; }}"
    );
    match run(&src, "F")? {
        Value::Text(s) => Ok(s.to_string()),
        other => panic!("{other:?}"),
    }
}

#[test]
fn error_info_create_defaults() {
    // errorinfo/TestErrorInfoType.al:21-60: the zero-argument Create is collectible, Create(Message)
    // is not; Verbosity::Error and DataClassification::CustomerContent are the defaults.
    let r = error_info(
        "e := ErrorInfo.Create();
         a := Format(e.Verbosity()) + '|' + Format(e.DataClassification()) + '|' + Format(e.Collectible()) + '|' + e.Message() + e.Title() + e.DetailedMessage();
         f := ErrorInfo.Create('the message');
         exit(a + '|' + f.Message() + '|' + Format(f.Collectible()) + '|' + Format(f.Verbosity()) + '|' + Format(e.TableId()) + Format(e.FieldNo()) + Format(e.PageNo()));",
    );
    assert_eq!(r.unwrap(), "Error|CustomerContent|Yes||the message|No|Error|000");
}

#[test]
fn error_info_accessors_round_trip_independently() {
    // errorinfo/TestErrorInfoType.al:64-160 and :330-440.
    let r = error_info(
        "e := ErrorInfo.Create();
         e.Message('MSG'); e.Title('TITLE'); e.DetailedMessage('DETAIL'); e.ControlName('CTRL');
         e.Verbosity(Verbosity::Warning); e.DataClassification(DataClassification::SystemMetadata);
         e.PageNo(42); e.FieldNo(7); e.Collectible(false);
         e.ErrorType(ErrorType::Internal);
         a := e.Message() + e.Title() + e.DetailedMessage() + e.ControlName() + Format(e.Verbosity()) + Format(e.DataClassification());
         exit(a + Format(e.PageNo()) + Format(e.FieldNo()) + Format(e.Collectible()) + Format(e.ErrorType()));",
    );
    assert_eq!(r.unwrap(), "MSGTITLEDETAILCTRLWarningSystemMetadata427NoInternal");
}

#[test]
fn error_info_create_with_a_record_sets_table_id_record_id_and_field() {
    // errorinfo/TestErrorInfoType.al:243-326.
    let r = error_info(
        "r1.Id := 1; r1.Insert(); r2.Id := 2; r2.Insert();
         e := ErrorInfo.Create('m', true, r1, r1.FieldNo(Payload));
         a := Format(e.TableId() = Database::\"EI Row\") + Format(Format(e.RecordId()) = Format(r1.RecordId())) + Format(Format(e.RecordId()) = Format(r2.RecordId()));
         exit(a + Format(e.FieldNo()) + Format(e.PageNo()));",
    );
    assert_eq!(r.unwrap(), "YesYesNo20");
}

#[test]
fn error_info_custom_dimensions_round_trip() {
    // errorinfo/TestErrorInfoType.al:313-345.
    let r = error_info(
        "e := ErrorInfo.Create(); a := Format(e.CustomDimensions().Count());
         d.Add('alpha', 'one'); d.Add('beta', 'two'); e.CustomDimensions(d);
         back := e.CustomDimensions();
         exit(a + Format(back.Count()) + back.Get('alpha') + back.Get('beta'));",
    );
    assert_eq!(r.unwrap(), "02onetwo");
}

// record/ProbePropertyAssignmentForms.al:302-373 (BC 27.5): the defaults of Create() are Error, CustomerContent,
// collectible and Client; an assigned dictionary is held by reference, so an entry added later shows;
// RecordId assigned reads back (the TableId stays 0) and is blank before.
#[test]
fn error_info_defaults_the_shared_dictionary_and_the_assigned_record_id() {
    let r = error_info(
        "e := ErrorInfo.Create();
         a := Format(e.Verbosity()) + Format(e.DataClassification()) + Format(e.Collectible()) + Format(e.ErrorType());
         d.Add('k', 'v'); e.CustomDimensions := d; a += Format(e.CustomDimensions().Count()); d.Add('k2', 'v2');
         a += Format(e.CustomDimensions().Count()) + '|' + Format(e.RecordId()) + '|' + Format(e.TableId());
         r1.Id := 2; r1.Insert(); e.RecordId := r1.RecordId();
         exit(a + '|' + Format(e.RecordId() = r1.RecordId()) + '|' + Format(e.TableId()));",
    );
    assert_eq!(r.unwrap(), "ErrorCustomerContentYesClient12||0|Yes|0");
}

#[test]
fn error_info_actions_do_not_disturb_the_state() {
    // errorinfo/TestErrorInfoType.al:523-557.
    let r = error_info(
        "e := ErrorInfo.Create('an error', true); e.Title('a title');
         e.AddAction('Fix it', 50100, 'F'); e.AddNavigationAction('Show me');
         exit(e.Message() + e.Title() + Format(e.Collectible()));",
    );
    assert_eq!(r.unwrap(), "an errora titleYes");
}

// ---------------------------------------------------------------- small builtins

#[test]
fn a_progress_dialog_is_a_headless_no_op() {
    // handlers/TestHandlersDialogProgressNoOp.al:24-110: Open, Update (all forms) and Close run and
    // change nothing.
    let src = "codeunit 50100 C { var Window: Dialog; procedure F(): Integer var i: Integer; total: Integer; begin
        Window.Open('Processing record #1######');
        for i := 1 to 5 do begin Window.Update(1, i); Window.Update(); total += i; end;
        Window.Close();
        exit(total);
    end; }";
    assert!(matches!(run(src, "F"), Ok(Value::Integer(15))));
}

#[test]
fn select_latest_version_does_nothing() {
    // session/TestBCDatabaseContracts.al:170-187, session/TestMiscComplete.al:36-55.
    let r = text_of("", "SelectLatestVersion(); SelectLatestVersion(60000); exit('ok');");
    assert_eq!(r.unwrap(), "ok");
}

#[test]
fn company_name_is_the_company_of_the_session() {
    // session/TestSessionFunctions.al:25 (not empty), session/TestDatabase.al:69-75 (same as CurrentCompany).
    let src = "table 50900 \"ALT Row\" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
        codeunit 50100 C { procedure F(): Text var R: Record \"ALT Row\"; begin exit(Format(CompanyName() = R.CurrentCompany()) + Format(CompanyName() <> '')); end; }";
    assert!(matches!(run(src, "F"), Ok(Value::Text(ref s)) if s == "YesYes"));
}

#[test]
fn change_company_to_the_session_company_succeeds_and_an_unknown_company_is_false() {
    // record/TestSecurityFilteringContracts.al:108-138, record/TestRecordChangeCompanyRows.al:115:
    // `Rec.ChangeCompany(CompanyName())` is true and the rows stay visible. A name nobody made is
    // false (record/TestSecurityFilteringContracts.al:149-160; tests/companies.rs has the rest).
    let src = "table 50900 \"ALT Row\" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
        codeunit 50100 C { procedure F(): Text var R: Record \"ALT Row\"; B: Record \"ALT Row\"; ok: Boolean; begin
            R.Id := 1; R.Insert();
            ok := B.ChangeCompany(CompanyName());
            exit(Format(ok) + Format(B.Count()) + B.CurrentCompany());
        end; procedure G(): Text var R: Record \"ALT Row\"; ok: Boolean; begin ok := R.ChangeCompany('Some Other Company'); exit(Format(ok)); end; }";
    assert!(matches!(run(src, "F"), Ok(Value::Text(ref s)) if s.starts_with("Yes1")));
    assert!(matches!(run(src, "G"), Ok(Value::Text(ref s)) if s == "No"));
}

#[test]
fn a_sequential_guid_is_not_null_and_not_repeated() {
    // session/TestSystemExtended.al:306-328.
    let r = text_of(
        "a: Guid; b: Guid;",
        "a := Guid.CreateSequentialGuid(); b := Guid.CreateSequentialGuid(); exit(Format(IsNullGuid(a)) + Format(a = b));",
    );
    assert_eq!(r.unwrap(), "NoNo");
}

#[test]
fn text_builder_max_capacity_is_large() {
    // session/TestSystemExtended.al:450-473: positive and at least a million.
    let r = text_of("tb: TextBuilder;", "exit(Format(tb.MaxCapacity() >= 1000000));");
    assert_eq!(r.unwrap(), "Yes");
}

#[test]
fn a_version_is_read_by_evaluate_and_split_in_parts() {
    // session/TestMiscTypes.al:44-80.
    let r = text_of(
        "v: Version; w: Version;",
        "Evaluate(v, '1.2.3.4'); Evaluate(w, '5.6.7.8');
         exit(Format(v.Major()) + Format(v.Minor()) + Format(v.Build()) + Format(v.Revision()) + '|' + Format(w.Revision()));",
    );
    assert_eq!(r.unwrap(), "1234|8");
}

// ---------------------------------------------------------------- DataTransfer, CodeCoverageLog

fn transfer(body: &str) -> Result<String, Signal> {
    let src = format!(
        "table 50900 \"DT Src\" {{ fields {{ field(1; Id; Integer) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
         table 50901 \"DT Dst\" {{ fields {{ field(1; Id; Integer) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
         codeunit 50100 C {{ procedure F(): Text var DT: DataTransfer; a: Text; begin {body} end; }}"
    );
    match run(&src, "F")? {
        Value::Text(s) => Ok(s.to_string()),
        other => panic!("{other:?}"),
    }
}

#[test]
fn data_transfer_outside_upgrade_says_which_of_the_two_things_is_wrong() {
    // install/TestDataTransferContracts.al:44-160: a call before SetTables asks for SetTables; after it,
    // every staging or copying call says the transfer is for upgrade and install code only.
    let r = transfer(
        "asserterror DT.AddFieldValue(1, 1); a := GetLastErrorText();
         asserterror DT.CopyRows(); a += '|' + GetLastErrorText();
         DT.SetTables(Database::\"DT Src\", Database::\"DT Dst\");
         asserterror DT.AddFieldValue(1, 1); a += '|' + GetLastErrorText();
         asserterror DT.AddConstantValue('x', 1); a += '|' + GetLastErrorText();
         asserterror DT.AddSourceFilter(1, '=%1', 5); a += '|' + GetLastErrorText();
         asserterror DT.AddDestinationFilter(1, '=%1', 5); a += '|' + GetLastErrorText();
         asserterror DT.AddJoin(1, 1); a += '|' + GetLastErrorText();
         asserterror DT.CopyFields(); a += '|' + GetLastErrorText();
         asserterror DT.CopyRows(); a += '|' + GetLastErrorText();
         exit(a);",
    );
    let first = "SetTables must first be called before calling other methods on DataTransfer.";
    let later = "DataTransfer is only usable during upgrade and installation code.";
    let want = format!("{first}|{first}|{later}|{later}|{later}|{later}|{later}|{later}|{later}");
    assert_eq!(r.unwrap(), want);
}

#[test]
fn data_transfer_with_a_table_outside_the_workspace_is_refused() {
    let r = transfer("DT.SetTables(2000000001, Database::\"DT Dst\"); exit('');");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

#[test]
fn code_coverage_log_reports_the_requested_state() {
    // codeunit/TestCodeCoverageStartStop.al:30-102.
    let r = text_of(
        "a: Text; b: Boolean;",
        "a := Format(CodeCoverageLog());
         b := CodeCoverageLog(false, false); a += Format(b) + Format(CodeCoverageLog());
         b := CodeCoverageLog(true, false); a += Format(b) + Format(CodeCoverageLog());
         b := CodeCoverageLog(false, false); a += Format(b) + Format(CodeCoverageLog());
         b := CodeCoverageLog(true); a += Format(b) + Format(CodeCoverageLog());
         CodeCoverageLog(false);
         exit(a + Format(CodeCoverageLog()));",
    );
    assert_eq!(r.unwrap(), "NoNoNoYesYesNoNoYesYesNo");
}

// ---------------------------------------------------------------- Clear, conversions

#[test]
fn clear_empties_lists_and_dictionaries_also_through_a_var_parameter() {
    // types/TestClearContracts.al:130-150, collections/TestListByRefContracts.al:155,
    // collections/TestCollectionsDictionary.al:108.
    let src = "codeunit 50100 C {
        procedure F(): Text var l: List of [Integer]; d: Dictionary of [Text, Integer]; a: Text; begin
            l.Add(1); l.Add(2); d.Add('a', 1);
            Clear(l); Clear(d);
            a := Format(l.Count()) + Format(d.Count());
            l.Add(5); d.Add('a', 7);
            ClearViaVar(l);
            exit(a + Format(l.Count()) + Format(d.Count()));
        end;
        local procedure ClearViaVar(var l: List of [Integer]) begin Clear(l); end; }";
    assert!(matches!(run(src, "F"), Ok(Value::Text(ref s)) if s == "0001"));
}

#[test]
fn a_text_assigns_into_a_guid() {
    // record/TestRecordGetBySystemId.al ZeroGuid, install and system-id fixtures.
    let r = text_of(
        "g: Guid;",
        "g := '00000000-0000-0000-0000-000000000000'; exit(Format(IsNullGuid(g)) + Format(g));",
    );
    assert_eq!(r.unwrap(), "Yes{00000000-0000-0000-0000-000000000000}");
}

#[test]
fn datetime_plus_and_minus_durations() {
    // types/TestDurationContracts.al:44-171.
    let r = text_of(
        "a: DateTime; b: DateTime; d: Duration;",
        "a := CreateDateTime(20240115D, 100000T); d := 7200000; b := a + d;
         exit(Format(DT2Time(b) = 120000T) + Format((b - a) = d) + Format(DT2Time(b - d) = 100000T));",
    );
    assert_eq!(r.unwrap(), "YesYesYes");
}

// ---------------------------------------------------------------- IsolatedStorage

fn storage(body: &str) -> Result<String, Signal> {
    text_of(
        "v: Text; w: Text; s: SecretText; a: Text; secret: Boolean; ok: Boolean;",
        body,
    )
}

// Assumption isolated-storage-overwrite-tx: session/lt-cn-session-leads/CnIsolatedStorageLeads.al
// TO_IsoOverwrite_OutsideWriteTransaction (corpus branch al-runtime-rs-staging-pub-to): an overwrite right after a Commit
// opens the write transaction, plain and encrypted alike.
#[test]
fn isolated_storage_overwrite_after_a_commit_opens_the_write_transaction() {
    let r = storage(
        "IsolatedStorage.Set('k', 'v1'); Commit();
         a := Format(Database.IsInWriteTransaction());
         a += Format(IsolatedStorage.Set('k', 'v2')) + Format(Database.IsInWriteTransaction()); Commit();
         a += Format(IsolatedStorage.SetEncrypted('k', 'e1')) + Format(Database.IsInWriteTransaction()); Commit();
         a += Format(IsolatedStorage.Set('k', 'p3')) + Format(Database.IsInWriteTransaction()); Commit();
         IsolatedStorage.Get('k', v);
         exit(a + v);",
    );
    assert_eq!(r.unwrap(), "NoYesYesYesYesYesYesp3");
}

#[test]
fn isolated_storage_round_trips_and_deletes() {
    // session/TestIsolatedStorage.al:30-65 and :95-125.
    let r = storage(
        "a := Format(IsolatedStorage.Set('k', 'value')) + Format(IsolatedStorage.Contains('k'));
         a += Format(IsolatedStorage.Get('k', v)) + v;
         a += '|' + Format(IsolatedStorage.Delete('k')) + Format(IsolatedStorage.Contains('k'));
         a += Format(IsolatedStorage.Get('absent', w)) + '[' + w + ']';
         exit(a);",
    );
    assert_eq!(r.unwrap(), "YesYesYesvalue|YesNoNo[]");
}

#[test]
fn isolated_storage_scopes_are_apart_and_encrypted_entries_read_back_as_text() {
    // session/TestIsolatedStorage.al:97-110, session/TestIsolatedStoragePerApp.al:100-110 (the Company scope).
    let r = storage(
        "IsolatedStorage.Set('k', 'module'); IsolatedStorage.Set('k', 'company', DataScope::Company);
         ok := IsolatedStorage.Get('k', v); a := v;
         ok := IsolatedStorage.Get('k', DataScope::Company, v); a += ',' + v;
         ok := IsolatedStorage.SetEncrypted('e', 'secret value');
         ok := IsolatedStorage.Contains('e', DataScope::Module, secret);
         ok := IsolatedStorage.Get('e', v);
         exit(a + ',' + v + Format(secret));",
    );
    assert_eq!(r.unwrap(), "module,company,secret valueYes");
}

#[test]
fn isolated_storage_takes_secret_text_and_the_user_scopes() {
    let r = storage("s := 'hidden'; IsolatedStorage.Set('k', s); ok := IsolatedStorage.Get('k', s); exit(Format(ok));");
    assert_eq!(r.unwrap(), "Yes");
    // The user scopes are stores of their own (session/TestProbeSessionMiscContracts.al:IsolatedStorage_Behaviour).
    let r = storage("IsolatedStorage.Set('k', 'x', DataScope::User); exit('');");
    assert!(r.is_ok(), "{r:?}");
}

#[test]
fn isolated_storage_set_and_delete_open_the_write_transaction_and_contains_does_not() {
    // session/lt-cn-session-leads/CnIsolatedStorageLeads.al:238-263 (IsoSet_OpensWriteTransaction).
    let r = storage(
        "a := Format(Database.IsInWriteTransaction());
         IsolatedStorage.Set('k', 'v', DataScope::User); a += Format(Database.IsInWriteTransaction()); Commit();
         IsolatedStorage.Set('k', 'v'); a += Format(Database.IsInWriteTransaction()); Commit();
         IsolatedStorage.Delete('k'); a += Format(Database.IsInWriteTransaction()); Commit();
         ok := IsolatedStorage.Contains('k', DataScope::User); a += Format(Database.IsInWriteTransaction());
         exit(a);",
    );
    assert_eq!(r.unwrap(), "NoYesYesYesNo");
}

#[test]
fn isolated_storage_set_over_an_existing_key_inside_a_write_transaction_runs() {
    // Outside one: isolated_storage_overwrite_after_a_commit_opens_the_write_transaction.
    // Inside a write transaction the phase cannot change: the overwrite runs.
    let r = storage(
        "IsolatedStorage.Set('k', 'v'); IsolatedStorage.Set('k', 'w'); ok := IsolatedStorage.Get('k', v); exit(v);",
    );
    assert_eq!(r.unwrap(), "w");
}

#[test]
fn isolated_storage_set_then_guarded_codeunit_run_fails() {
    // session/lt-cn-session-leads/CnIsolatedStorageLeads.al:265-277 (IsoSet_ThenGuardedCodeunitRun) and :279-290 after a Commit.
    let src = "codeunit 50101 Noop { trigger OnRun() begin end; }
        codeunit 50100 C {
            procedure F(): Boolean begin IsolatedStorage.Set('k', 'v', DataScope::User); exit(Codeunit.Run(50101)); end;
            procedure G(): Boolean begin IsolatedStorage.Set('k', 'v', DataScope::User); Commit(); exit(Codeunit.Run(50101)); end;
        }";
    let e = al_testkit::run(src, "F").unwrap_err();
    assert!(
        format!("{e:?}").contains("An error occurred and the transaction is stopped."),
        "{e:?}"
    );
    assert!(matches!(al_testkit::run(src, "G"), Ok(Value::Boolean(true))));
}

#[test]
fn set_encrypted_keeps_214_bytes_and_refuses_216() {
    // session/lt-cn-session-leads/CnIsolatedStorageLeads.al:368-424: 214 ASCII and 107 two-byte characters are kept,
    // 216 ASCII and 108 two-byte characters fail with EncryptionDataTooLong.
    let set = |value: &str| {
        storage(&format!(
            "ok := IsolatedStorage.SetEncrypted('e', PadStr('', {value})); ok := IsolatedStorage.Get('e', v); exit(Format(StrLen(v)));"
        ))
    };
    assert_eq!(set("214, 'a'").unwrap(), "214");
    assert_eq!(set("107, 'é'").unwrap(), "107");
    for long in ["216, 'a'", "108, 'é'", "300, 'a'"] {
        let e = set(long).unwrap_err();
        let s = format!("{e:?}");
        assert!(
            s.contains("Unable to encrypt data. The provided string is too long.")
                && s.contains("EncryptionDataTooLong"),
            "{s}"
        );
    }
    // 215 bytes is a Linux suspect (:380-388), and characters of three UTF-8 bytes are not pinned.
    for refused in ["215, 'a'", "10, '€'"] {
        assert!(matches!(set(refused), Err(Signal::Unsupported(_))), "{refused}");
    }
}

#[test]
fn isolated_storage_get_into_text_of_a_secret_value_fails() {
    // misc/lt-cu-odd-leads/CuTryTests.al:181-232: a value stored from a SecretText (SetEncrypted or Set) cannot be read into a Text.
    for set in ["SetEncrypted", "Set"] {
        let e = storage(&format!(
            "s := SecretStrSubstNo('x'); ok := IsolatedStorage.{set}('k', s, DataScope::Module); IsolatedStorage.Get('k', DataScope::Module, v); exit(v);"
        ))
        .unwrap_err();
        let s = format!("{e:?}");
        assert!(
            s.contains("The operation was aborted to prevent exposing a secret value.")
                && s.contains("ForbiddenSecretAccess"),
            "{s}"
        );
    }
    // Into a SecretText it still reads.
    let r = storage(
        "s := SecretStrSubstNo('x'); ok := IsolatedStorage.SetEncrypted('k', s); exit(Format(IsolatedStorage.Get('k', s)));",
    );
    assert_eq!(r.unwrap(), "Yes");
}

// ---------------------------------------------------------------- NavApp module info

const DEP_ID: &str = "f1e2d3c4-b5a6-7890-fedc-ba9876543210";
const MAIN_ID: &str = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";

/// Two apps: `dep` (a callee codeunit) and `main` (depends on it, holds the test code).
fn run_two_apps(dep_src: &str, main_src: &str, routine: &str) -> Result<Value, Signal> {
    let (ws, mut l) = al_sema::lower::compile(
        vec![
            AppSource {
                id: DEP_ID.into(),
                name: "Dependency".into(),
                dependencies: vec![],
                files: vec![("d.al".into(), dep_src.into())],
            },
            AppSource {
                id: MAIN_ID.into(),
                name: "Main".into(),
                dependencies: vec![DEP_ID.into(), "63ca2fa4-4f03-4f2b-a480-172fef340d3f".into()],
                files: vec![("m.al".into(), main_src.into())],
            },
        ],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    // The runner patches what only the manifests know.
    l.program_mut().apps[0].publisher = "Dep Publisher".into();
    l.program_mut().apps[0].version = "2.3.4.5".into();
    l.program_mut().apps[1].publisher = "Main Publisher".into();
    l.program_mut().apps[1].version = "1.0.0.0".into();
    l.program_mut().apps[1].resources = vec![
        ("greeting.txt".into(), b"Hello from packaged resource".to_vec()),
        ("sub/nested.txt".into(), b"nested-resource-content".to_vec()),
        ("data.json".into(), b"{\"a\": 1}".to_vec()),
        // BiDataDriven.al (corpus PR 861) res/cases.csv and res/bom.csv.
        (
            "cases.csv".into(),
            b"id,value\nfirst,one\nsecond,two\nthird,three \xC3\xA4\xC3\xB6\n".to_vec(),
        ),
        ("bom.csv".into(), b"\xEF\xBB\xBFid,value\r\nbom,1\r\n".to_vec()),
    ];
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.routines.iter().any(|r| r.name == routine))
        .expect("object");
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == routine)
        .unwrap();
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

const CALLEE: &str = "codeunit 60384 Callee {
    procedure OwnName(): Text var i: ModuleInfo; begin NavApp.GetCurrentModuleInfo(i); exit(i.Name()); end;
    procedure OwnId(): Guid var i: ModuleInfo; begin NavApp.GetCurrentModuleInfo(i); exit(i.Id()); end;
    procedure CallerName(): Text var i: ModuleInfo; begin NavApp.GetCallerModuleInfo(i); exit(i.Name()); end;
    procedure CallerInfos(): List of [ModuleInfo] begin exit(NavApp.GetCallerCallstackModuleInfos()); end;
    procedure CallerInfosThroughOwnFrame(): List of [ModuleInfo] begin exit(Inner()); end;
    local procedure Inner(): List of [ModuleInfo] begin exit(NavApp.GetCallerCallstackModuleInfos()); end;
}";

fn module_text(body: &str) -> Result<String, Signal> {
    let main = format!(
        "codeunit 50100 T {{ procedure F(): Text var c: Codeunit Callee; i: ModuleInfo; j: ModuleInfo; v: Version; a: Text; t2: Text; ids: List of [ModuleInfo]; g: Guid; ins: InStream; o: JsonObject; names: List of [Text]; b1: Byte; b2: Byte; b3: Byte; i1: Integer; i2: Integer; i3: Integer; begin {body} end; }}"
    );
    match run_two_apps(CALLEE, &main, "F")? {
        Value::Text(s) => Ok(s.to_string()),
        other => panic!("{other:?}"),
    }
}

#[test]
fn module_info_reports_the_manifest_of_the_running_app() {
    // session/TestBCPlatformContracts.al:100-130 (Id, Name, AppVersion match app.json),
    // session/TestMiscTypes.al:100-150 (AppVersion, PackageId, DataVersion), TestNavAppExtended.al.
    let r = module_text(
        "NavApp.GetCurrentModuleInfo(i);
         v := i.AppVersion();
         Evaluate(g, '{a1b2c3d4-e5f6-7890-abcd-ef1234567890}');
         exit(i.Name() + '|' + i.Publisher() + '|' + Format(i.Id() = g) + '|'
              + Format(v.Major()) + Format(v.Minor()) + Format(v) + '|' + Format(IsNullGuid(i.PackageId())) + Format(i.PackageId() = i.Id())
              + '|' + Format(i.Dependencies().Count()));",
    );
    // The dependencies include the platform's Application when the manifest names an application version
    // (session/TestProbeSessionMiscContracts.al:ModuleInfo_CurrentModule, `dependency_Application`).
    assert_eq!(r.unwrap(), "Main|Main Publisher|Yes|101.0.0.0|NoNo|3");
}

#[test]
fn get_module_info_finds_installed_apps_and_names_an_unknown_id() {
    // session/TestNavAppExtended.al:94-150.
    let r = module_text(
        "NavApp.GetCurrentModuleInfo(i);
         a := Format(NavApp.GetModuleInfo(i.Id(), j)) + Format(j.Name() = i.Name());
         a += Format(NavApp.GetModuleInfo('00000000-dead-beef-0000-000000000004', j));
         ClearLastError();
         asserterror NavApp.GetModuleInfo('00000000-dead-beef-0000-000000000002', j);
         exit(a + '|' + GetLastErrorText());",
    );
    let r = r.unwrap();
    assert!(r.starts_with("YesYesNo|"), "{r}");
    assert!(r.contains("No installed extension was found with ID"), "{r}");
    assert!(r.contains("00000000-dead-beef-0000-000000000002"), "{r}");
}

#[test]
fn a_manifest_dependency_outside_the_workspace_is_refused_not_called_missing() {
    let r = module_text("NavApp.GetModuleInfo('63ca2fa4-4f03-4f2b-a480-172fef340d3f', j); exit('');");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

#[test]
fn the_callee_in_another_app_sees_its_own_module_and_its_caller() {
    // session/TestNavAppModuleInfoCallerIdentity.al:35-100 and TestNavAppCallerCallstackModuleInfos.al:25-100.
    let r = module_text(
        "NavApp.GetCurrentModuleInfo(i);
         a := c.OwnName() + ',' + c.CallerName() + '|';
         ids := c.CallerInfos();
         j := ids.Get(1);
         a += Format(ids.Count() >= 1) + Format(j.Id() = i.Id()) + '|';
         ids := c.CallerInfosThroughOwnFrame();
         a += Format(ids.Count()) + ',' + Format(ids.Get(1).Id() = c.OwnId()) + Format(ids.Get(2).Id() = i.Id());
         exit(a);",
    );
    assert_eq!(r.unwrap(), "Dependency,Main|YesYes|2,YesYes");
}

#[test]
fn is_installing_is_false_in_test_code() {
    // session/TestNavAppExtended.al, session/TestBCPlatformContracts.al:IsInstalling.
    let r = module_text("exit(Format(NavApp.IsInstalling()));");
    assert_eq!(r.unwrap(), "No");
}

#[test]
fn packaged_resources_read_back_exactly() {
    // session/TestNavAppGetResource.al:25-60.
    let r = module_text(
        "NavApp.GetResource('greeting.txt', ins, TextEncoding::UTF8); ins.ReadText(a);
         NavApp.GetResource('sub/nested.txt', ins, TextEncoding::UTF8); ins.ReadText(t2);
         exit(a + '|' + t2 + '|' + NavApp.GetResourceAsText('greeting.txt', TextEncoding::UTF8) + '|' + NavApp.GetResourceAsText('greeting.txt'));",
    );
    assert_eq!(
        r.unwrap(),
        "Hello from packaged resource|nested-resource-content|Hello from packaged resource|Hello from packaged resource"
    );
}

#[test]
fn utf8_resource_text_keeps_the_byte_order_mark() {
    // BiDataDriven.al:480-486 (codeunit 81611 BiDd_Resource_TextEncodings): 45 characters for cases.csv; bom.csv is 18
    // characters and starts with U+FEFF.
    let r = module_text(
        "a := NavApp.GetResourceAsText('bom.csv', TextEncoding::UTF8); t2 := NavApp.GetResourceAsText('cases.csv', TextEncoding::UTF8);
         exit(Format(StrLen(a)) + '|' + Format(a[1] = 65279) + '|' + Format(StrLen(t2)));",
    );
    assert_eq!(r.unwrap(), "18|Yes|45");
}

#[test]
fn resource_bytes_without_an_encoding() {
    // BiDataDriven.al:491-516 (BiDd_Resource_RawStream): GetResource without an encoding hands the raw bytes over.
    let r = module_text(
        "NavApp.GetResource('cases.csv', ins); ins.Read(b1); ins.Read(b2); ins.Read(b3); i1 := b1; i2 := b2; i3 := b3;
         exit(Format(i1) + ',' + Format(i2) + ',' + Format(i3));",
    );
    assert_eq!(r.unwrap(), "105,100,44");
    // BiDataDriven.al:481-482: without an encoding GetResourceAsText reads one character per byte (47 for cases.csv, whose
    // two UTF-8 pairs count twice); which characters the bytes from 128 up become is not pinned, so that is refused.
    let r = module_text("exit(Format(StrLen(NavApp.GetResourceAsText('cases.csv'))));");
    assert!(
        matches!(&r, Err(Signal::Unsupported(w)) if w.contains("not pinned")),
        "{r:?}"
    );
    let r = module_text("exit(Format(StrLen(NavApp.GetResourceAsText('greeting.txt'))));");
    assert_eq!(r.unwrap(), "28");
}

#[test]
fn a_missing_resource_is_named_in_the_error() {
    // session/TestNavAppGetResource.al:62-85: "A resource matching 'missing.txt' could not be found in app".
    let r = module_text(
        "asserterror NavApp.GetResource('missing.txt', ins); a := GetLastErrorText();
         asserterror t2 := NavApp.GetResourceAsText('missing.txt'); a += '|' + GetLastErrorText();
         asserterror o := NavApp.GetResourceAsJson('nonexistent.json'); a += '|' + GetLastErrorText();
         exit(a);",
    );
    let r = r.unwrap();
    for part in r.split('|') {
        assert!(part.starts_with("A resource matching '"), "{r}");
        assert!(part.contains("' could not be found in app"), "{r}");
    }
}

#[test]
fn resources_are_listed_with_an_optional_wildcard_filter() {
    // session/TestMiscComplete.al:139-165 (they must succeed); the filter is the documented wildcard.
    let r = module_text(
        "names := NavApp.ListResources(); a := Format(names.Count());
         names := NavApp.ListResources('*.json'); a += Format(names.Count()) + names.Get(1);
         names := NavApp.ListResources('*.xml');
         exit(a + Format(names.Count()));",
    );
    assert_eq!(r.unwrap(), "51data.json0");
}

// ---------------------------------------------------------------- Error(ErrorInfo), ExecutionContext

#[test]
fn error_with_an_error_info_raises_its_message() {
    // errorinfo/TestErrorInfoErrorType.al:50-65 (a client error reports the message verbatim).
    let r = text_of(
        "e: ErrorInfo; a: Text;",
        "e := ErrorInfo.Create('the client visible text'); e.ErrorType(ErrorType::Client);
         asserterror Error(e); a := GetLastErrorText();
         e.ErrorType(ErrorType::Internal); asserterror Error(e);
         exit(a + '|' + CopyStr(GetLastErrorText(), 1, 87));",
    );
    // session/TestProbeSessionMiscContracts.al:ErrorInfo_ErrorClientAndInternal: an internal error says only that an error
    // occurred, with a correlation id.
    assert_eq!(
        r.unwrap(),
        "the client visible text|An error has occurred. If the error persists, please contact your system administrator."
    );
    let r = text_of(
        "e: ErrorInfo;",
        "e := ErrorInfo.Create('the client visible text'); e.ErrorType(ErrorType::Client);
         asserterror Error(e); exit(GetLastErrorText());",
    );
    assert_eq!(r.unwrap(), "the client visible text");
}

#[test]
fn the_execution_context_is_normal_in_a_test() {
    // install/TestInstallEnvVisible_Tests.al:120-127, session/TestBCPlatformContracts.al:46-60.
    let r = text_of(
        "ec: ExecutionContext;",
        "ec := Session.GetExecutionContext();
         exit(Format(ec) + Format(Session.GetExecutionContext() = ExecutionContext::Normal)
              + Format(Session.GetCurrentModuleExecutionContext() = ExecutionContext::Normal)
              + Format(Session.GetModuleExecutionContext() = ExecutionContext::Normal));",
    );
    assert_eq!(r.unwrap(), "NormalYesYesYes");
}

// ---------------------------------------------------------------- collection errors

#[test]
fn list_get_outside_the_list_raises_the_invalid_argument_error() {
    // collections/TestListTrappableError.al:21-45: both forms of Get raise it.
    let r = text_of(
        "l: List of [Integer]; v: Integer; a: Text;",
        "asserterror l.Get(5, v); a := GetLastErrorText(); asserterror v := l.Get(5); exit(a + '|' + GetLastErrorText());",
    );
    let want = "An invalid argument was passed to a 'List' data type method.";
    assert_eq!(r.unwrap(), format!("{want}|{want}"));
}

#[test]
fn list_get_outside_the_list_with_the_result_used_still_answers_false() {
    let r = text_of("l: List of [Integer]; v: Integer;", "exit(Format(l.Get(5, v)));");
    assert_eq!(r.unwrap(), "No");
}

#[test]
fn dictionary_get_of_a_missing_key_raises_the_key_not_found_error() {
    // collections/TestCollectionsDictionary.al:42-55 pins the statement form; the value form
    // (collections/TestCollectionContracts.al:188) is pinned as an error with a text (probes-needed).
    let r = text_of(
        "d: Dictionary of [Text, Integer]; v: Integer; a: Text;",
        "d.Add('alpha', 11); asserterror d.Get('missing', v); a := GetLastErrorText(); asserterror v := d.Get('missing'); exit(a + '|' + GetLastErrorText());",
    );
    let want = "The given key was not present in the dictionary.";
    assert_eq!(r.unwrap(), format!("{want}|{want}"));
}

#[test]
fn list_index_errors_raise_the_pinned_error() {
    // session/TestProbeSessionMiscContracts.al:Dictionary_List_ErrorTexts: Get, RemoveAt, Set and Insert outside the list.
    for body in ["l.RemoveAt(3);", "l.Set(3, 1);", "l.Insert(4, 1);", "l.Get(0);"] {
        let r = text_of("l: List of [Integer];", &format!("{body} exit('');"));
        let Err(Signal::Error(e)) = r else {
            panic!("{body}: {r:?}");
        };
        assert_eq!(
            (e.message.as_str(), e.code.as_str()),
            (
                "An invalid argument was passed to a 'List' data type method.",
                "ArgumentOutOfRange"
            ),
            "{body}"
        );
    }
}

// ---------------------------------------------------------------- platform context

#[test]
fn a_test_runs_with_a_gui_on_a_service_tier() {
    // session/TestBCPlatformContracts.al:26-43 (GuiAllowed and IsServiceTier are true in a test);
    // handlers/TestConfirmNoHandler.al:28 (GuiAllowed is Yes inside a page trigger).
    let r = text_of(
        "",
        "exit(Format(GuiAllowed()) + Format(System.GuiAllowed()) + Format(IsServiceTier()));",
    );
    assert_eq!(r.unwrap(), "YesYesYes");
}

#[test]
fn the_last_used_row_version_is_positive_and_does_not_fall_after_an_insert() {
    // session/TestSystemExtended.al:635-659, session/TestBCDatabaseContracts.al:100-117.
    let src = "table 50902 \"RV Row\" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
        codeunit 50100 C { procedure F(): Text var r: Record \"RV Row\"; a: BigInteger; b: BigInteger;
        begin a := Database.LastUsedRowVersion(); r.Id := 1; r.Insert(); b := Database.LastUsedRowVersion();
              exit(Format(a > 0) + Format(b >= a) + Format(b = r.\"SystemRowVersion\")); end; }";
    match run(src, "F") {
        Ok(Value::Text(t)) => assert_eq!(t, "YesYesYes"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_lock_timeout_can_be_read_and_set() {
    // session/TestSystemExtended.al:608-630: callable in every form. The default is the documented one.
    let r = text_of(
        "a: Text;",
        "a := Format(Database.LockTimeout()); Database.LockTimeout(false); a += Format(Database.LockTimeout());
         Database.LockTimeout(true); exit(a + Format(Database.LockTimeout()));",
    );
    assert_eq!(r.unwrap(), "YesNoYes");
}

// ---------------------------------------------------------------- enum InitValue

#[test]
fn an_enum_field_starts_at_its_init_value_blank_or_named() {
    // types/TestBlankEnumInitTests.Codeunit.al:52-86 (InTestInsertInitializesBlankEnumToOrdinalZero,
    // MixedEnumInitValue...): a blank member as InitValue is ordinal 0, a named one its true ordinal.
    let src = "enum 50910 \"IV Level\" { value(0; \" \") { } value(5; Verbose) { } value(9; Quiet) { } }
        table 50911 \"IV Row\" { fields {
            field(1; K; Code[10]) { }
            field(2; Blank; Enum \"IV Level\") { InitValue = \" \"; }
            field(3; Named; Enum \"IV Level\") { InitValue = Verbose; }
            field(4; Opt; Option) { OptionMembers = \" \",Alpha; InitValue = \" \"; }
        } keys { key(PK; K) { } } }
        codeunit 50100 C { procedure F(): Text var r: Record \"IV Row\"; a: Text; begin
            r.Init(); r.K := 'M1'; r.Insert(false); r.Get('M1');
            exit(Format(r.Blank.AsInteger()) + '|' + Format(r.Named.AsInteger()) + '|' + Format(r.Opt = r.Opt::\" \")); end; }";
    match run(src, "F") {
        Ok(Value::Text(t)) => assert_eq!(t, "0|5|Yes"),
        other => panic!("{other:?}"),
    }
}
#[test]
fn a_date_formula_field_starts_at_its_init_value() {
    // The InitValue goes through the evaluator of Evaluate (types/TestBlankEnumInitTests.Codeunit.al:88-95
    // says so), and a calculable formula is stored as written (types/TestDateFormulaContracts.al:21-139).
    // "Filed Service Contract Header"."Price Update Period" is `InitValue = '1Y'`.
    let src = "table 50912 \"DF Row\" { fields {
            field(1; K; Code[10]) { }
            field(2; Period; DateFormula) { InitValue = '1Y'; }
        } keys { key(PK; K) { } } }
        codeunit 50100 C { procedure F(): Text var r: Record \"DF Row\"; begin
            r.Init(); r.K := 'M1'; r.Insert(false); r.Get('M1'); exit(Format(r.Period) + '|' + Format(CalcDate(r.Period, DMY2Date(1, 1, 2024)))); end; }";
    match run(src, "F") {
        Ok(Value::Text(t)) => assert_eq!(t, "1Y|01/01/25"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn table_connections_are_registered_found_by_any_case_and_unregistered() {
    // session/TestDatabaseTableConnectionCrm.al:34-117.
    let r = text_of(
        "",
        "exit(Format(HasTableConnection(TableConnectionType::CRM, 'ALTTEST'))
            + '|' + GetDefaultTableConnection(TableConnectionType::CRM)
            + '|' + Reg(TableConnectionType::CRM, 'ALTTEST', '@@test@@')
            + '|' + Format(HasTableConnection(TableConnectionType::CRM, 'ALTTEST'))
            + Format(HasTableConnection(TableConnectionType::CRM, 'alttest'))
            + Format(HasTableConnection(TableConnectionType::CRM, 'ALTNEVER'))
            + '|' + SetDef('ALTTEST')
            + '|' + GetDefaultTableConnection(TableConnectionType::CRM)
            + '|' + Unreg()
            + '|' + Format(HasTableConnection(TableConnectionType::CRM, 'ALTTEST'))
            + '|' + GetDefaultTableConnection(TableConnectionType::CRM));
         end;
         local procedure Reg(T: TableConnectionType; N: Text; C: Text): Text
         begin RegisterTableConnection(T, N, C); exit('reg');
         end;
         local procedure SetDef(N: Text): Text
         begin SetDefaultTableConnection(TableConnectionType::CRM, N); exit('def');
         end;
         local procedure Unreg(): Text
         begin UnregisterTableConnection(TableConnectionType::CRM, 'ALTTEST'); exit('unreg');",
    );
    assert_eq!(r.unwrap(), "No||reg|YesYesNo|def|ALTTEST|unreg|No|");
}

#[test]
fn registering_a_table_connection_twice_names_it_in_the_error() {
    // session/TestDatabaseTableConnectionCrm.al:76-87 (the name is the pinned fragment).
    let r = text_of(
        "",
        "RegisterTableConnection(TableConnectionType::CRM, 'ALTTEST', '@@test@@');
         asserterror RegisterTableConnection(TableConnectionType::CRM, 'ALTTEST', '@@test@@');
         exit(GetLastErrorText());",
    );
    assert!(r.unwrap().contains("ALTTEST"));
}

#[test]
fn registering_an_external_sql_connection_is_a_permission_error() {
    // session/TestMiscComplete.al:185-194 (the word `permission` is the pinned fragment).
    let r = text_of(
        "",
        "asserterror Database.RegisterTableConnection(TableConnectionType::ExternalSQL, 'TestConn', '');
         exit(GetLastErrorText());",
    );
    assert!(r.unwrap().contains("permission"));
}

#[test]
fn a_connection_string_other_than_the_test_connection_is_refused() {
    let r = text_of(
        "",
        "RegisterTableConnection(TableConnectionType::CRM, 'ALTTEST', 'Url=https://example.invalid'); exit('x');",
    );
    match r {
        Err(Signal::Unsupported(why)) => assert!(why.contains("@@test@@"), "{why}"),
        other => panic!("{other:?}"),
    }
}

// Session.LogMessage writes to the server's telemetry, which AL cannot observe (ADR-009): it
// returns, whatever it is given. Base Application's `Upgrade Tag` calls it from Company-Initialize.
#[test]
fn log_message_is_a_telemetry_write_that_changes_nothing() {
    let src = r#"codeunit 50100 C
{
    procedure Log(): Integer
    var D: Dictionary of [Text, Text]; N: Integer;
    begin
        Session.LogMessage('0000AAA', 'one', Verbosity::Normal, DataClassification::SystemMetadata, TelemetryScope::ExtensionPublisher, D);
        Session.LogMessage('0000AAB', 'two', Verbosity::Warning, DataClassification::SystemMetadata, TelemetryScope::All, 'Key', 'Value');
        N := 7;
        exit(N);
    end;
}"#;
    assert!(matches!(run(src, "Log"), Ok(Value::Integer(7))));
}

// Session.LogSecurityAudit writes to the server's security audit log, which AL cannot observe
// (ADR-009): it returns. Base Application's Change Log Management calls it
// (session/TestEnvironmentUserTenantContracts.al:315-374).
#[test]
fn log_security_audit_is_an_audit_write_that_changes_nothing() {
    let src = r#"codeunit 50100 C
{
    procedure Log(): Integer
    var N: Integer;
    begin
        Session.LogSecurityAudit('Change log', SecurityOperationResult::Success, 'activated', AuditCategory::ApplicationManagement);
        N := 7;
        exit(N);
    end;
}"#;
    let r = run(src, "Log");
    assert!(matches!(r, Ok(Value::Integer(7))), "{r:?}");
}

// types/TestDateFormulaDurationFormatContracts.al:30-43: a cleared DateFormula prints empty and
// CalcDate with it returns the date; Evaluate of an empty text succeeds and does the same.
#[test]
fn a_cleared_date_formula_is_the_empty_formula() {
    let r = text_of(
        "F: DateFormula; D: Date;",
        "Evaluate(F, '<1D>'); Clear(F); D := DMY2Date(5, 3, 2024); \
         exit(Format(F) + '|' + Format(CalcDate(F, D), 0, 9) + '|' + Format(Evaluate(F, '')) + '|' + Format(CalcDate('', D), 0, 9));",
    );
    assert_eq!(r.unwrap(), "|2024-03-05|Yes|2024-03-05");
}

// misc/lt-protected-var-abs-query-views/P146FormatCalcDate.al:P146_CalcDate_FormatBuilt_Formula: a plus sign, a percent sign
// and a number before the unit read as the number (`<+%1D>` and `<+%1D` add one day). Only the plus sign is pinned.
#[test]
fn calcdate_reads_a_percent_sign_after_a_plus_as_the_number_that_follows() {
    let r = text_of(
        "D: Date;",
        "D := DMY2Date(15, 1, 2024); exit(Format(CalcDate('<+%1D>', D), 0, 9) + '|' + Format(CalcDate(Format('<+%1D>', 5), D), 0, 9));",
    );
    assert_eq!(r.unwrap(), "2024-01-16|2024-01-16");
    // A minus or no sign in front of the percent sign reads the same (types/lt-date-formula-edges/DateFormulaEdges.al:Placeholders_PercentN).
    let r = text_of(
        "D: Date;",
        "D := DMY2Date(15, 1, 2024); exit(Format(CalcDate('<-%1D>', D), 0, 9) + '|' + Format(CalcDate('<%1D>', D), 0, 9));",
    );
    assert_eq!(r.unwrap(), "2024-01-14|2024-01-16");
}

// session/TestEnvironmentUserTenantContracts.al:195-201 and session/TestNavAppExtended.al:116-117:
// GetModuleInfo of the null app id is true and answers a module whose data version is the installed
// application's, at least 27. Its name is W1 and its publisher Microsoft (session/TestProbeSessionMiscContracts.al:ModuleInfo_NullAppId).
#[test]
fn get_module_info_of_the_null_id_answers_the_application_version() {
    let r = text_of(
        "G: Guid; M: ModuleInfo;",
        "exit(Format(NavApp.GetModuleInfo(G, M)) + Format(M.DataVersion().Major() >= 27));",
    );
    assert_eq!(r.unwrap(), "YesYes");
    let r = text_of(
        "G: Guid; M: ModuleInfo;",
        "NavApp.GetModuleInfo(G, M); exit(M.Name() + '|' + M.Publisher());",
    );
    assert_eq!(r.unwrap(), "W1|Microsoft");
}

// A table with a DateFormula field inserts with the field blank. Base Application's setup tables
// (General Ledger Setup, Sales & Receivables Setup) have such fields and Company-Initialize
// inserts them as `Init(); Insert();`.
#[test]
fn a_blank_date_formula_field_inserts() {
    let src = r#"table 50301 Setup
{
    fields { field(1; "Primary Key"; Code[10]) { } field(2; Grace; DateFormula) { } }
    keys { key(PK; "Primary Key") { } }
}
codeunit 50100 C
{
    procedure Go(): Integer
    var S: Record Setup;
    begin
        S.Init();
        S.Insert();
        exit(S.Count());
    end;
}"#;
    assert!(matches!(run(src, "Go"), Ok(Value::Integer(1))));
}

/// Red first: CanCreateTask was refused as "not pinned". The corpus pins it false in a
/// test (session/TestEnvironmentUserTenantContracts.al:238): there is no scheduler
/// and a test does not wait for a commit.
#[test]
fn task_scheduler_can_create_task_is_false() {
    let src = "codeunit 50100 C { procedure F(): Boolean begin exit(TaskScheduler.CanCreateTask()); end; }";
    assert!(matches!(run(src, "F"), Ok(Value::Boolean(false))));
}

/// Red first: `Session.ApplicationIdentifier` was refused as not implemented, which stopped
/// Base App's `Environment Information Impl.` and so every `Environment Information` call.
/// Measured on real BC: 'NAV' (session/TestEnvironmentUserTenantContracts.al:241).
#[test]
fn session_application_identifier_is_nav() {
    let src = "codeunit 50100 C { procedure F(): Text begin exit(Session.ApplicationIdentifier()); end; }";
    assert!(matches!(run(src, "F"), Ok(Value::Text(t)) if t == "NAV"));
}

/// Red first: `ProductName.Full()`, `Short()` and `Marketing()` were refused as not implemented.
/// Measured on real BC (session/TestEnvironmentUserTenantContracts.al:237-239).
#[test]
fn product_name_answers_the_three_forms() {
    for (member, want) in [
        ("Full", "Dynamics 365 Business Central"),
        ("Short", "Business Central"),
        ("Marketing", "Microsoft Dynamics 365 Business Central"),
    ] {
        let src = format!("codeunit 50100 C {{ procedure F(): Text begin exit(ProductName.{member}()); end; }}");
        assert!(matches!(run(&src, "F"), Ok(Value::Text(t)) if t == want), "{member}");
    }
}

/// Red first: `Session.CurrentExecutionMode` was refused as not implemented. A test run is in
/// Standard mode (session/TestEnvironmentUserTenantContracts.al:247); the ordinals are not pinned.
#[test]
fn current_execution_mode_is_standard() {
    let src = "codeunit 50100 C { procedure F(): Text begin exit(Format(Session.CurrentExecutionMode())); end; }";
    assert!(matches!(run(src, "F"), Ok(Value::Text(t)) if t == "Standard"));
}

#[test]
fn database_sid_is_a_stable_windows_style_sid_per_user_name() {
    // Database.SID, as probed on BC 27.5: corpus branch al-runtime-rs-staging (ae3fe88),
    // tests/al-language-onprem/session/PXDatabaseSid.al:31-46 (eight dash-separated parts, all but
    // the first numeric, prefix S-1-5-21-; the same for the same name; not the text of
    // UserSecurityId(); an unknown name gets one of that shape too).
    let r = text_of(
        "a: Text; b: Text; c: Text;",
        "a := Database.SID(UserId()); b := Database.SID(UserId()); c := Database.SID('NoSuchUser');
         exit(a + '|' + Format(a = b) + Format(a = Format(UserSecurityId())) + '|' + c);",
    )
    .unwrap();
    let parts: Vec<&str> = r.split('|').collect();
    assert_eq!(parts[1], "YesNo");
    for sid in [parts[0], parts[2]] {
        let p: Vec<&str> = sid.split('-').collect();
        assert_eq!(p.len(), 8, "{sid}");
        assert!(sid.starts_with("S-1-5-21-"), "{sid}");
        assert!(p[1..].iter().all(|n| n.parse::<i32>().is_ok()), "{sid}");
    }
}

// handlers/PWRefusedBuiltins.al:113-168 (BC 27.5): FieldError(Field, ErrorInfo) raises `<field> <message> in <table> <key>.`
// with the code NCLCSRTS:TableErrorStr and leaves the ErrorInfo's message alone; FieldRef.FieldError(ErrorInfo) the same.
#[test]
fn field_error_with_an_error_info_raises_the_formatted_text() {
    let r = error_info(
        "r1.Id := 5; e := ErrorInfo.Create('my message');
         asserterror r1.FieldError(Payload, e);
         a := GetLastErrorText() + '|' + GetLastErrorCode() + '|' + e.Message();
         exit(a);",
    );
    assert_eq!(
        r.unwrap(),
        "Payload my message in EI Row Id='5'.|NCLCSRTS:TableErrorStr|my message"
    );
}

// types/ProbeTypeFormsAndFilters.al:Clear_JsonTypes_GiveFreshUnassigned (BC 27.5): a JsonObject cleared or never filled formats
// as `{}`, a cleared JsonValue is null, and a cleared JsonToken is a null value.
#[test]
fn format_of_an_empty_json_object_and_of_a_cleared_json_token() {
    let src = "codeunit 50100 C { procedure F(): Text var Obj: JsonObject; Arr: JsonArray; Val: JsonValue; Tok: JsonToken; S: Text; begin
        S := Format(Obj); Obj.Add('a', 1); Arr.Add(1); Val.SetValue(5); Tok := Obj.AsToken();
        Clear(Obj); Clear(Arr); Clear(Val); Clear(Tok);
        exit(S + Format(Obj) + '|' + Format(Arr.Count()) + '|' + Format(Val.IsNull()) + '|' + Format(Val.IsUndefined()) + '|' + Format(Tok.IsValue()) + '|' + Format(Tok)); end; }";
    let r = run(src, "F");
    assert!(
        matches!(&r, Ok(Value::Text(t)) if t == "{}{}|0|Yes|No|Yes|null"),
        "{r:?}"
    );
}

// types/ProbeTypeFormsAndFilters.al:361-395 (BC 27.5): CopyFilter from a Code field to a Text field raises the
// compatibility error and keeps the target's filter; the filter copied is the one of the source's current filter group, into
// the target's current group; a source with no filter clears the target's.
#[test]
fn copy_filter_between_filter_groups_and_from_code_to_text() {
    let src = "table 50310 T { fields { field(1; Id; Integer) { } field(2; CodeF; Code[10]) { } field(3; TextF; Text[20]) { } } keys { key(PK; Id) { } } }
        codeunit 50100 C { procedure F(): Text var Src: Record T; Dst: Record T; S: Text; begin
            Src.SetFilter(CodeF, 'A*|B1'); Src.FilterGroup(2); Src.SetFilter(CodeF, '<>Z'); Src.FilterGroup(0);
            Dst.SetFilter(TextF, 'old'); asserterror Src.CopyFilter(CodeF, Dst.TextF);
            S := GetLastErrorCode() + '|' + Dst.GetFilter(TextF) + '|' + GetLastErrorText();
            Src.FilterGroup(2); Dst.Reset(); Src.CopyFilter(CodeF, Dst.CodeF); S += '|' + Dst.GetFilter(CodeF);
            Src.FilterGroup(0); Src.Reset(); Dst.SetFilter(CodeF, 'X'); Src.CopyFilter(CodeF, Dst.CodeF); S += '|[' + Dst.GetFilter(CodeF) + ']'; exit(S); end; }";
    let Ok(Value::Text(got)) = run(src, "F") else {
        panic!("not a text")
    };
    assert_eq!(
        got,
        "NCLCSRTS:TableEqualValue|old|The type of the constant must be compatible with that of the TextF field, and the length of the constant must not exceed the length of the field.\\\\Type: Text <-- Code\\Field: TextF <-- CodeF\\Table: T|<>Z|[]"
    );
}

// A multi-dimensional array indexed one dimension at a time (`A[x] [y]`, Base Application's test library
// "Backup Storage" does it on `array[20, 1000] of RecordRef`) is the element `A[x, y]` (assumed: no corpus test
// writes the chained form, docs/probes-needed.md). Issue 40.
#[test]
fn a_chained_index_of_a_two_dimensional_array_is_the_element() {
    let r = text_of(
        "A: array[2, 3] of Integer;",
        "A[2] [3] := 7; A[1, 2] := 5; exit(Format(A[2, 3]) + Format(A[1] [2]) + Format(A[1, 3]));",
    );
    assert_eq!(r.unwrap(), "750");
}

// `A[x]` alone on a two-dimensional array is a row; Clear of a row clears its cells (commit 70719d7e, corpus
// collections/lt-array-rows/ArrayRows.al).
#[test]
fn clear_of_a_row_of_a_two_dimensional_array_clears_its_cells() {
    let r = text_of(
        "A: array[2, 3] of Integer;",
        "A[2, 1] := 4; Clear(A[2]); exit(Format(A[2, 1]));",
    );
    assert_eq!(r.unwrap(), "0");
}

// A `for` loop over an Integer whose bounds are Option values (Microsoft's "Library - Manufacturing" loops
// `for Day := FromDay to ToDay` with `FromDay: Option`): the bounds take the loop variable's type as an assignment
// does, an Option becoming its ordinal (`ops::convert`). Documented assumption (docs/probes-needed.md). Issue 42.
#[test]
fn a_for_loop_over_an_integer_takes_option_bounds_by_ordinal() {
    let r = text_of(
        "O1: Option A,B,C,D; O2: Option A,B,C,D; I: Integer; s: Text;",
        "O1 := O1::B; O2 := O2::D; for I := O1 to O2 do s += Format(I); for I := O2 downto O1 do s += Format(I); exit(s);",
    );
    assert_eq!(r.unwrap(), "123321");
}
