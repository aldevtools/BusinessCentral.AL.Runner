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

// corpus codeunits/TestCodeunitInterface.al:14-26 (Interface_Assign_DoubleImpl_ComputesDouble)
#[test]
fn a_codeunit_variable_assigns_to_an_interface_and_dispatches() {
    let r = run(
        COMPUTE,
        "procedure P(): Integer var C: Interface IALTCompute; D: Codeunit ALTDouble; begin C := D; exit(C.Compute(10)); end;",
        "P",
    );
    assert_eq!(int(r), 20);
}

// corpus codeunits/TestCodeunitInterface.al:40-60 (Interface_Reassign_ChangesImpl)
#[test]
fn reassigning_an_interface_switches_the_implementation() {
    let r = run(
        COMPUTE,
        "procedure P(): Integer var C: Interface IALTCompute; D: Codeunit ALTDouble; S: Codeunit ALTSquare; A: Integer; begin C := D; A := C.Compute(4); C := S; exit(A * 100 + C.Compute(4)); end;",
        "P",
    );
    assert_eq!(int(r), 8 * 100 + 16);
}

// corpus codeunits/TestCodeunitInterface.al:85-96 (negative input)
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

// codeunits/TestInterfaceRuntimeContracts.al:76
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

// codeunits/TestInterfaceIsAsContracts.al:61-102: `is` is false, not an error, for an interface the
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
// codeunits/TestInterfaceRuntimeContracts.al (Interface_ExtendedInterface_IsAndAs).
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

// An interface that no codeunit in the workspace implements still lowers its calls (al-runner
// issue 6299: alc accepts the code, so the runner must not refuse the procedure). With no
// implementer the variable can only be unassigned when the call runs, so the call raises
// "Interface not initialized" (codeunits/TestInterfaceRuntimeContracts.al:76), and an `is` guard
// on an instance that does not implement it is false (codeunits/TestInterfaceIsAsContracts.al:61-102).
#[test]
fn a_call_on_an_interface_without_implementer_lowers_and_runs_behind_an_is_guard() {
    let fixtures = "interface IA { procedure A(): Text; } interface IV extends IA { procedure V(): Boolean; } \
        codeunit 60394 ImplA implements IA { procedure A(): Text begin exit('a'); end; }";
    let r = run(
        fixtures,
        "procedure P(): Text var S: Interface IA; V: Interface IV; A: Codeunit ImplA; begin S := A; \
         if S is IV then begin V := S as IV; V.V(); end; exit(S.A()); end;",
        "P",
    );
    assert_eq!(text(r), "a");
    let r = run(
        fixtures,
        "procedure P(): Boolean var V: Interface IV; begin exit(V.V()); end;",
        "P",
    );
    let Err(Signal::Error(e)) = r else { panic!("{r:?}") };
    assert_eq!(
        (e.message.as_str(), e.code.as_str()),
        ("Interface not initialized", "InvalidOperation")
    );
}

// An extensible enum with no values in the workspace that implements an interface nothing
// implements (al-runner issue 6299): a foreach over its Ordinals() never runs the body.
#[test]
fn a_loop_over_an_empty_extensible_enum_with_an_interface_call_runs() {
    let fixtures = "interface IS { procedure D(): Text; } enum 60395 ES implements IS { Extensible = true; }";
    let r = run(
        fixtures,
        "procedure P(): Integer var E: Enum ES; S: Interface IS; L: List of [Text]; begin \
         foreach E in Enum::ES.Ordinals() do begin S := E; L.Add(S.D()); end; exit(L.Count()); end;",
        "P",
    );
    assert_eq!(int(r), 0);
}

/// corpus codeunits/CwDefaultInterfaceMethods.al:20-63 (fixtures, BC 29, runtime 18.0).
const DEFAULTS: &str = r#"
interface "LT CW I68"
{
    procedure Req(): Text;

    procedure Opt(): Text
    begin
        exit('default');
    end;

    procedure Chain(): Text
    begin
        exit('default chain of ' + Req());
    end;
}

codeunit 85430 "LT CW Impl68 Inherit" implements "LT CW I68"
{
    procedure Req(): Text
    begin
        exit('inherit req');
    end;
}

codeunit 85431 "LT CW Impl68 Override" implements "LT CW I68"
{
    procedure Req(): Text
    begin
        exit('override req');
    end;

    procedure Opt(): Text
    begin
        exit('override opt');
    end;

    procedure Chain(): Text
    begin
        exit('override chain');
    end;
}

codeunit 85432 "LT CW Impl68 Internal" implements "LT CW I68"
{
    internal procedure Req(): Text
    begin
        exit('internal req');
    end;

    internal procedure Opt(): Text
    begin
        exit('internal opt');
    end;
}
"#;

// A codeunit that does not declare an interface member with a body runs the default body, and a
// default body calls the implementor's own members; one that declares it runs its own; internal
// procedures still implement (corpus codeunits/CwDefaultInterfaceMethods.al:114-158,
// CW_Default_InheritAndOverride and CW_Default_NonPublicImplementations).
#[test]
fn a_codeunit_without_a_member_that_has_a_default_body_runs_the_default() {
    let r = run(
        DEFAULTS,
        "procedure P(): Text var I: Interface \"LT CW I68\"; Inh: Codeunit \"LT CW Impl68 Inherit\"; \
         Ovr: Codeunit \"LT CW Impl68 Override\"; Intl: Codeunit \"LT CW Impl68 Internal\"; \
         begin I := Inh; exit(I.Opt() + '|' + I.Chain() + '|' + Ovr.Opt() + '|' + Intl.Opt()); end;",
        "P",
    );
    assert_eq!(
        text(r),
        "default|default chain of inherit req|override opt|internal opt"
    );
    let r = run(
        DEFAULTS,
        "procedure P(): Text var I: Interface \"LT CW I68\"; Ovr: Codeunit \"LT CW Impl68 Override\"; \
         Intl: Codeunit \"LT CW Impl68 Internal\"; \
         begin I := Ovr; exit(I.Opt() + '|' + I.Chain()); end;",
        "P",
    );
    assert_eq!(text(r), "override opt|override chain");
    let r = run(
        DEFAULTS,
        "procedure P(): Text var I: Interface \"LT CW I68\"; Intl: Codeunit \"LT CW Impl68 Internal\"; \
         begin I := Intl; exit(I.Req() + '|' + I.Opt() + '|' + I.Chain()); end;",
        "P",
    );
    assert_eq!(text(r), "internal req|internal opt|default chain of internal req");
}

// A local procedure of the member's name next to a default body: calling through the interface is
// not pinned (codeunits/CwDefaultInterfaceMethods.al:6-9, not asserted), so the call on that
// instance refuses when it runs; the codeunit's own call runs its local procedure (:150).
#[test]
fn a_local_procedure_of_a_member_with_a_default_body_refuses_the_interface_call() {
    let fixtures = format!(
        "{DEFAULTS}\ncodeunit 85433 \"LT CW Impl68 Local\" implements \"LT CW I68\" {{ \
         procedure Req(): Text begin exit('local req'); end; \
         local procedure Opt(): Text begin exit('local opt'); end; \
         procedure CallOwnOpt(): Text begin exit(Opt()); end; }}"
    );
    let r = run(
        &fixtures,
        "procedure P(): Text var L: Codeunit \"LT CW Impl68 Local\"; begin exit(L.CallOwnOpt()); end;",
        "P",
    );
    assert_eq!(text(r), "local opt");
    let r = run(
        &fixtures,
        "procedure P(): Text var I: Interface \"LT CW I68\"; L: Codeunit \"LT CW Impl68 Local\"; \
         begin I := L; exit(I.Opt()); end;",
        "P",
    );
    let Err(Signal::Unsupported(why)) = r else {
        panic!("{r:?}")
    };
    assert!(why.contains("does not list"), "{why}");
}

// `this.Member(..)` in a default body calls the implementor's member, overloads chosen by arity, as
// Microsoft's PEPPOL Tax Info Provider (BC 29) does in GetTaxExemptionReason (codeunits/interfaces/
// R6924dThisInDefaultBody.al:93 R6924d_This_Overload). The pass is clean: no assumption is marked.
#[test]
fn this_in_a_default_body_calls_the_member_of_the_implementing_codeunit() {
    let fixtures = "interface IT { procedure Get(A: Text): Text; \
         procedure Get(A: Text; B: Text): Text begin exit(this.Get(A + B)); end; } \
         codeunit 60396 ImplT implements IT { procedure Get(A: Text): Text begin exit('impl ' + A); end; }";
    al_builtins::assumptions::clear_hits();
    let r = run(
        fixtures,
        "procedure P(): Text var I: Interface IT; C: Codeunit ImplT; begin I := C; exit(I.Get('a', 'b')); end;",
        "P",
    );
    assert_eq!(text(r), "impl ab");
    assert_eq!(al_builtins::assumptions::take_hits(), vec![]);
}
