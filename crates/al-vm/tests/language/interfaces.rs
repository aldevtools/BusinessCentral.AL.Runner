//! Interface variables, dispatch and enum implementations, end to end.
//!
//! Each test cites the corpus test that pins the behaviour.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::AppSource;
use al_testkit::{int, text};
use al_vm::{Engine, Session};

/// corpus _fixtures/interfaces/IALTCompute.al
const COMPUTE: &str = r#"
interface IALTCompute
{
    procedure Compute(X: Integer): Integer;
}

codeunit 60011 ALTDouble implements IALTCompute
{
    procedure Compute(X: Integer): Integer
    begin
        exit(2 * X);
    end;
}

codeunit 60012 ALTSquare implements IALTCompute
{
    procedure Compute(X: Integer): Integer
    begin
        exit(X * X);
    end;
}
"#;

/// corpus types/TestEnumDefaultImplGreeter.Interface.al, ...Impls.Codeunit.al, ...Kind.Enum.al
const EDI: &str = r#"
interface "EDI Greeter"
{
    procedure Greet(): Text;
}

codeunit 60310 "EDI Default Impl" implements "EDI Greeter"
{
    procedure Greet(): Text begin exit('DEFAULT'); end;
}

codeunit 60311 "EDI Own Impl" implements "EDI Greeter"
{
    procedure Greet(): Text begin exit('OWN'); end;
}

codeunit 60312 "EDI Unknown Impl" implements "EDI Greeter"
{
    procedure Greet(): Text begin exit('UNKNOWN'); end;
}

enum 60309 "EDI Kind" implements "EDI Greeter"
{
    Extensible = true;
    DefaultImplementation = "EDI Greeter" = "EDI Default Impl";
    UnknownValueImplementation = "EDI Greeter" = "EDI Unknown Impl";

    value(0; "Falls Back") { Caption = 'Falls Back'; }
    value(1; "Has Own") { Caption = 'Has Own'; Implementation = "EDI Greeter" = "EDI Own Impl"; }
}
"#;

/// corpus types/TestEnumExtImpl*.al
const EEI: &str = r#"
interface "EEI Marker"
{
    procedure Marker(): Text;
}

codeunit 60890 "EEI Base Impl" implements "EEI Marker"
{
    procedure Marker(): Text begin exit('BASE'); end;
}

codeunit 60891 "EEI Ext Impl" implements "EEI Marker"
{
    procedure Marker(): Text begin exit('EXTENSION'); end;
}

codeunit 60892 "EEI Default Impl" implements "EEI Marker"
{
    procedure Marker(): Text begin exit('DEFAULT'); end;
}

enum 60893 "EEI Kind" implements "EEI Marker"
{
    Extensible = true;
    DefaultImplementation = "EEI Marker" = "EEI Default Impl";

    value(0; Base) { Implementation = "EEI Marker" = "EEI Base Impl"; }
}

enumextension 60890 "EEI Kind Ext" extends "EEI Kind"
{
    value(10; Extended) { Implementation = "EEI Marker" = "EEI Ext Impl"; }
    value(20; "Extended No Impl") { }
}
"#;

fn compile(src: &str) -> (al_sema::workspace::Workspace, al_sema::lower::Lowered) {
    al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src.into())],
        }],
        &Default::default(),
    )
}

fn run(fixtures: &str, body: &str, routine: &str) -> Result<Value, Signal> {
    run_with(fixtures, body, routine, &[])
}

/// Like `run`, passing enum values `(enum name, ordinal)` as the routine's arguments. The
/// corpus builds them with `Enum.FromInteger`, which is a separate builtin.
fn run_with(fixtures: &str, body: &str, routine: &str, enums: &[(&str, i32)]) -> Result<Value, Signal> {
    let src = format!("{fixtures}\ncodeunit 50100 T {{ {body} }}");
    let (ws, l) = compile(&src);
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let oi = l.program.objects.iter().position(|o| o.name == "T").expect("object");
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == routine)
        .expect("routine");
    assert!(blocked[oi][ri].is_none(), "refused: {:?}", blocked[oi][ri]);
    let args = enums
        .iter()
        .map(|(n, ord)| {
            let e = l.program.objects.iter().position(|o| o.name == *n).expect("enum");
            Value::Enum(e as u32, *ord)
        })
        .collect();
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, args).map(|(v, _)| v)
}

// corpus codeunit/TestCodeunitInterface.al:14-26 (Interface_Assign_DoubleImpl_ComputesDouble)
#[test]
fn a_codeunit_variable_assigns_to_an_interface_and_dispatches() {
    let r = run(
        COMPUTE,
        "procedure P(): Integer var C: Interface IALTCompute; D: Codeunit ALTDouble; begin C := D; exit(C.Compute(10)); end;",
        "P",
    );
    assert_eq!(int(r), 20);
}

// corpus codeunit/TestCodeunitInterface.al:40-60 (Interface_Reassign_ChangesImpl)
#[test]
fn reassigning_an_interface_switches_the_implementation() {
    let r = run(
        COMPUTE,
        "procedure P(): Integer var C: Interface IALTCompute; D: Codeunit ALTDouble; S: Codeunit ALTSquare; A: Integer; begin C := D; A := C.Compute(4); C := S; exit(A * 100 + C.Compute(4)); end;",
        "P",
    );
    assert_eq!(int(r), 8 * 100 + 16);
}

// corpus codeunit/TestCodeunitInterface.al:85-96 (negative input)
#[test]
fn dispatch_passes_arguments_through() {
    let r = run(
        COMPUTE,
        "procedure P(): Integer var C: Interface IALTCompute; D: Codeunit ALTDouble; begin C := D; exit(C.Compute(-3)); end;",
        "P",
    );
    assert_eq!(int(r), -6);
}

// corpus types/TestEnumDefaultImplTests.Codeunit.al:30-42
#[test]
fn an_enum_value_without_its_own_implementation_uses_the_default() {
    let r = run(
        EDI,
        "procedure P(): Text var G: Interface \"EDI Greeter\"; begin G := Enum::\"EDI Kind\"::\"Falls Back\"; exit(G.Greet()); end;",
        "P",
    );
    assert_eq!(text(r), "DEFAULT");
}

// corpus types/TestEnumDefaultImplTests.Codeunit.al:44-56
#[test]
fn an_enum_value_with_its_own_implementation_prefers_it() {
    let r = run(
        EDI,
        "procedure P(): Text var G: Interface \"EDI Greeter\"; begin G := Enum::\"EDI Kind\"::\"Has Own\"; exit(G.Greet()); end;",
        "P",
    );
    assert_eq!(text(r), "OWN");
}

// corpus types/TestEnumDefaultImplTests.Codeunit.al:58-78 (the enum value built with FromInteger)
#[test]
fn an_undeclared_ordinal_uses_the_unknown_value_implementation() {
    let r = run_with(
        EDI,
        "procedure P(K: Enum \"EDI Kind\"): Text var G: Interface \"EDI Greeter\"; begin G := K; exit(G.Greet()); end;",
        "P",
        &[("EDI Kind", 99)],
    );
    assert_eq!(text(r), "UNKNOWN");
}

// corpus types/TestEnumDefaultImplTests.Codeunit.al:80-100
#[test]
fn declared_ordinals_through_from_integer_keep_the_declared_chain() {
    let r = run_with(
        EDI,
        "procedure P(K: Enum \"EDI Kind\"): Text var G: Interface \"EDI Greeter\"; begin G := K; exit(G.Greet()); end;",
        "P",
        &[("EDI Kind", 0)],
    );
    assert_eq!(text(r), "DEFAULT");
    let r = run_with(
        EDI,
        "procedure P(K: Enum \"EDI Kind\"): Text var G: Interface \"EDI Greeter\"; begin G := K; exit(G.Greet()); end;",
        "P",
        &[("EDI Kind", 1)],
    );
    assert_eq!(text(r), "OWN");
}

// corpus types/TestEnumExtImplTests.Codeunit.al:33-76
#[test]
fn enum_extension_values_carry_their_own_and_the_base_default_implementation() {
    let r = run(
        EEI,
        "procedure P(): Text var M: Interface \"EEI Marker\"; R: Text; begin \
         M := Enum::\"EEI Kind\"::Extended; R := M.Marker(); \
         M := Enum::\"EEI Kind\"::\"Extended No Impl\"; R += '/' + M.Marker(); \
         M := Enum::\"EEI Kind\"::Base; R += '/' + M.Marker(); exit(R); end;",
        "P",
    );
    assert_eq!(text(r), "EXTENSION/DEFAULT/BASE");
}

// corpus types/TestInterfaceCodeunitStateField.al:11-27: the implementation keeps its own
// global state across the cast.
#[test]
fn the_implementation_keeps_its_instance_state() {
    let fixtures = r#"
interface IState { procedure Bump(): Integer; }
codeunit 60371 StateImpl implements IState
{
    var N: Integer;
    procedure Bump(): Integer begin N += 1; exit(N); end;
}
enum 60370 StateKind implements IState { value(0; V) { Implementation = IState = StateImpl; } }
"#;
    let r = run(
        fixtures,
        "procedure P(): Integer var S: Interface IState; K: Enum StateKind; begin K := K::V; S := K; S.Bump(); exit(S.Bump()); end;",
        "P",
    );
    assert_eq!(int(r), 2);
}

// codeunit/TestInterfaceRuntimeContracts.al:76
// (Interface_CallOnUnassignedVariable_Throws_InterfaceNotInitialized)
#[test]
fn a_call_on_an_unassigned_interface_raises_interface_not_initialized() {
    let r = run(
        COMPUTE,
        "procedure P(): Integer var C: Interface IALTCompute; begin exit(C.Compute(1)); end;",
        "P",
    );
    let Err(Signal::Error(e)) = r else { panic!("{r:?}") };
    assert_eq!(
        (e.message.as_str(), e.code.as_str()),
        ("Interface not initialized", "InvalidOperation")
    );
}

// ...Interface_CastUndeclaredEnumOrdinalWithoutUnknownImplementation_Throws
#[test]
fn an_undeclared_ordinal_without_an_unknown_implementation_raises_argument_out_of_range() {
    let fixtures = r#"
interface IPlain { procedure A(): Integer; }
codeunit 60381 PlainImpl implements IPlain { procedure A(): Integer begin exit(1); end; }
enum 60380 PlainKind implements IPlain { value(0; V) { Implementation = IPlain = PlainImpl; } }
"#;
    let r = run_with(
        fixtures,
        "procedure P(K: Enum PlainKind): Integer var G: Interface IPlain; begin G := K; exit(G.A()); end;",
        "P",
        &[("PlainKind", 99)],
    );
    let Err(Signal::Error(e)) = r else { panic!("{r:?}") };
    assert_eq!(
        (e.message.as_str(), e.code.as_str()),
        ("Enum does not contain value 99.", "ArgumentOutOfRange")
    );
}

// ...Interface_InListDictionaryAndArray_ShareTheInstance
#[test]
fn interfaces_inside_collections_share_the_instance() {
    let fixtures = "interface IBump { procedure Bump(): Integer; } codeunit 60390 BumpImpl implements IBump { var N: Integer; procedure Bump(): Integer begin N += 1; exit(N); end; }";
    let r = run(
        fixtures,
        "procedure P(): Text var Impl: Codeunit BumpImpl; F: Interface IBump; L: List of [Interface IBump]; D: Dictionary of [Text, Interface IBump]; A: array[2] of Interface IBump; begin \
         F := Impl; L.Add(F); D.Add('k', F); A[1] := F; \
         exit(Format(L.Get(1).Bump()) + Format(D.Get('k').Bump()) + Format(A[1].Bump()) + Format(F.Bump())); end;",
        "P",
    );
    assert_eq!(text(r), "1234");
}

// ...Interface_VarParameterAndReturnValue_CarryTheReference
#[test]
fn var_parameters_and_return_values_carry_the_reference() {
    let fixtures = "interface IBump { procedure Bump(): Integer; } codeunit 60390 BumpImpl implements IBump { var N: Integer; procedure Bump(): Integer begin N += 1; exit(N); end; }";
    let r = run(
        fixtures,
        "procedure Fill(var F: Interface IBump) var Impl: Codeunit BumpImpl; begin F := Impl; F.Bump(); end; \
         procedure Make(): Interface IBump var Impl: Codeunit BumpImpl; begin exit(Impl); end; \
         procedure P(): Text var F: Interface IBump; O: Interface IBump; begin Fill(F); O := Make(); \
         exit(Format(F.Bump()) + Format(O.Bump()) + Format(O.Bump())); end;",
        "P",
    );
    assert_eq!(text(r), "212");
}

// ...Interface_ExtendedInterface_IsAndAs
#[test]
fn an_extended_interface_converts_to_its_base_and_answers_is_and_as() {
    let fixtures = "interface IBase { procedure A(): Integer; } interface IMore extends IBase { procedure B(): Text; } \
        codeunit 60391 MoreImpl implements IMore { var N: Integer; procedure A(): Integer begin N += 10; exit(N); end; procedure B(): Text begin exit('b'); end; }";
    let r = run(
        fixtures,
        "procedure P(): Text var Impl: Codeunit MoreImpl; M: Interface IMore; B: Interface IBase; begin \
         M := Impl; B := M; \
         exit(Format(B.A()) + Format(B is IMore) + Format(B is IBase) + Format(M.A()) + (B as IMore).B()); end;",
        "P",
    );
    assert_eq!(text(r), "10YesYes20b");
}

// codeunit/TestInterfaceIsAsContracts.al:61-102: `is` is false, not an error, for an interface the
// instance does not implement and for an unassigned variable; `as` to such an interface, or on an
// unassigned variable, raises "The expression cannot be the target interface." / UnableToCast.
#[test]
fn is_is_false_and_as_raises_unable_to_cast_for_a_missing_implementation() {
    let fixtures = "interface IA { procedure A(): Text; } interface IB { procedure B(): Text; } \
        codeunit 60392 ImplA implements IA { procedure A(): Text begin exit('a'); end; } \
        codeunit 60393 ImplAB implements IA, IB { procedure A(): Text begin exit('ab-a'); end; procedure B(): Text begin exit('ab-b'); end; }";
    let body = |stmt: &str| {
        format!(
            "procedure P(): Text var Un: Interface IA; IA: Interface IA; IB: Interface IB; \
             A: Codeunit ImplA; AB: Codeunit ImplAB; begin IA := A; {stmt} end;"
        )
    };
    let r = run(
        fixtures,
        &body("exit(Format(IA is IA) + Format(IA is IB) + Format(Un is IA) + Format(Un is IB));"),
        "P",
    );
    assert_eq!(text(r), "YesNoNoNo");
    for stmt in ["IB := IA as IB;", "IB := Un as IB;", "IA := Un as IA;"] {
        let r = run(fixtures, &body(&format!("{stmt} exit('');")), "P");
        let Err(Signal::Error(e)) = r else {
            panic!("{stmt}: {r:?}")
        };
        assert_eq!(
            (e.message.as_str(), e.code.as_str()),
            ("The expression cannot be the target interface.", "UnableToCast"),
            "{stmt}"
        );
    }
}

// An enum value made by FromInteger goes straight into an interface variable. The two halves are
// pinned apart: FromInteger carries any ordinal (types/TestEnumDefaultImplTests.Codeunit.al:70) and
// an enum value converts to the interface its enum implements (:75, :96-98); a real project writes
// `Workflow := Enum::"X".FromInteger(n)`, which was refused as "an unknown type into interface".
#[test]
fn an_enum_value_from_frominteger_converts_to_the_interface_directly() {
    let r = run(
        EDI,
        "procedure P(): Text var G: Interface \"EDI Greeter\"; n: Integer; s: Text; begin
            n := 1; G := Enum::\"EDI Kind\".FromInteger(n); s := G.Greet();
            n := 0; G := \"EDI Kind\".FromInteger(n); s += '|' + G.Greet();
            n := 99; G := Enum::\"EDI Kind\".FromInteger(n); s += '|' + G.Greet();
            exit(s);
        end;",
        "P",
    );
    let Ok(Value::Text(s)) = r else { panic!("{r:?}") };
    assert_eq!(s, "OWN|DEFAULT|UNKNOWN");
}

// al-runner issue 6249: an interface that extends another and declares a method of the same name with other parameters. A
// call of the base interface's overload through the derived interface resolves to the base overload, as alc compiles it (the
// reporter's run). That base procedures are callable through the derived interface is pinned by
// codeunit/TestInterfaceRuntimeContracts.al (Interface_ExtendedInterface_IsAndAs).
#[test]
fn a_base_overload_is_called_through_a_derived_interface_that_declares_the_name_again() {
    let fixtures = "interface IEBase { procedure SetValue(Value: Text; Count: Integer); procedure GetValue(): Text; } \
        interface IEDerived extends IEBase { procedure SetValue(Value: Text; Extra: Text; Count: Integer); } \
        interface IEFactory { procedure GetSetup(): Interface IEDerived; } \
        codeunit 60394 IEImpl implements IEDerived { SingleInstance = true; var Stored: Text; \
          procedure SetValue(Value: Text; Count: Integer) begin Stored := Value; end; \
          procedure SetValue(Value: Text; Extra: Text; Count: Integer) begin Stored := Value + Extra; end; \
          procedure GetValue(): Text begin exit(Stored); end; } \
        codeunit 60395 IEFactoryImpl implements IEFactory { procedure GetSetup(): Interface IEDerived var Impl: Codeunit IEImpl; begin exit(Impl); end; }";
    let r = run(
        fixtures,
        "procedure P(): Text var F: Codeunit IEFactoryImpl; I: Interface IEFactory; D: Interface IEDerived; s: Text; begin \
         I := F; D := I.GetSetup(); \
         D.SetValue('a', 1); s := D.GetValue(); \
         D.SetValue('b', 'c', 2); s += '|' + D.GetValue(); \
         I.GetSetup().SetValue('d', 3); s += '|' + D.GetValue(); \
         exit(s); end;",
        "P",
    );
    assert_eq!(text(r), "a|bc|d");
}
