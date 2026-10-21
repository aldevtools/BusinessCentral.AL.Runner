//! The executable form of a routine: register bytecode (ADR-034, `docs/reviews/2026-10-bytecode-design.md`).
//!
//! A routine's registers are its locals followed by temporaries. Instructions call the shared helpers of the VM. Every statement and expression of the IR has instructions: the
//! compiler returns `None` only when a routine needs more registers than an instruction can name. A value capture runs
//! the captured form (`compile(.., true)`), which adds the statement and loop instructions of ADR-030.

mod compile;
mod run;
mod types;

use al_ir as ir;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

pub(crate) use compile::compile;

/// An instruction. Registers are `u16`, side-table indexes `u32`. `NONE` marks "no target".
#[derive(Debug, Clone, Copy)]
pub(crate) enum Op {
    /// `d := a` (a clone).
    Move {
        d: u16,
        a: u16,
    },
    /// `d := constant k`.
    Const {
        d: u16,
        k: u32,
    },
    /// `d := Global i`.
    LoadGlobal {
        d: u16,
        i: u32,
    },
    /// `d := read(place p)` (any place: fields, array elements, base globals).
    ReadPlace {
        d: u16,
        p: u32,
    },
    /// Local `i := a`; a Record or a Library value goes through `write` (copy semantics).
    StoreLocal {
        i: u16,
        a: u16,
        p: u32,
    },
    /// `write(place p, a)`.
    StorePlace {
        p: u32,
        a: u16,
    },
    Bin {
        op: ir::BinOp,
        d: u16,
        a: u16,
        b: u16,
    },
    /// `Bin` for an operator with an Integer fast path: two Integers are done here, anything else goes the general way.
    BinI {
        op: ir::BinOp,
        d: u16,
        a: u16,
        b: u16,
    },
    /// `d := Integer v`.
    ConstI {
        d: u16,
        v: i32,
    },
    // ---- typed instructions: the compiler proved both operands Integer (or Boolean) from `types::ExprTypes` ----
    // Each does the work on the unboxed value with no run-time dispatch on the operator. An operand of another kind
    // (an unsound type) takes the generic path and is counted (`typed fallbacks` in `AL_EXEC_REPORT`), so a wrong type
    // never changes a result.
    /// `d := a + b` etc. on Integers; the errors are those of `ops::int_arith`.
    AddInt {
        d: u16,
        a: u16,
        b: u16,
    },
    SubInt {
        d: u16,
        a: u16,
        b: u16,
    },
    MulInt {
        d: u16,
        a: u16,
        b: u16,
    },
    DivInt {
        d: u16,
        a: u16,
        b: u16,
    },
    ModInt {
        d: u16,
        a: u16,
        b: u16,
    },
    /// `d := a + k` (`a - k` is `a + (-k)` for any `k` but `i32::MIN`, which stays `SubInt`).
    AddIntK {
        d: u16,
        a: u16,
        k: i32,
    },
    /// `d := a = b` etc. on Integers, a Boolean.
    EqInt {
        d: u16,
        a: u16,
        b: u16,
    },
    NeInt {
        d: u16,
        a: u16,
        b: u16,
    },
    LtInt {
        d: u16,
        a: u16,
        b: u16,
    },
    LeInt {
        d: u16,
        a: u16,
        b: u16,
    },
    GtInt {
        d: u16,
        a: u16,
        b: u16,
    },
    GeInt {
        d: u16,
        a: u16,
        b: u16,
    },
    /// Jumps to `t` unless `a = b` etc. on Integers (a compare and `JumpIfNot` in one, no Boolean register).
    JumpUnlessEqInt {
        a: u16,
        b: u16,
        t: u32,
    },
    JumpUnlessNeInt {
        a: u16,
        b: u16,
        t: u32,
    },
    JumpUnlessLtInt {
        a: u16,
        b: u16,
        t: u32,
    },
    JumpUnlessLeInt {
        a: u16,
        b: u16,
        t: u32,
    },
    JumpUnlessGtInt {
        a: u16,
        b: u16,
        t: u32,
    },
    JumpUnlessGeInt {
        a: u16,
        b: u16,
        t: u32,
    },
    /// `JumpIfNot` on a register proven Boolean.
    JumpIfNotBool {
        a: u16,
        t: u32,
    },
    NotBool {
        d: u16,
        a: u16,
    },
    AndBool {
        d: u16,
        a: u16,
        b: u16,
    },
    OrBool {
        d: u16,
        a: u16,
        b: u16,
    },
    XorBool {
        d: u16,
        a: u16,
        b: u16,
    },
    Un {
        op: ir::UnOp,
        d: u16,
        a: u16,
    },
    Convert {
        d: u16,
        a: u16,
        t: u32,
    },
    Jump {
        t: u32,
    },
    JumpIfNot {
        a: u16,
        t: u32,
    },
    /// The stop flag is checked at each round of a loop.
    CheckStop,
    /// `d := the position of the call stack's top` (a loop remembers its line).
    SavePos {
        d: u16,
    },
    RestorePos {
        a: u16,
    },
    Pos {
        p: u32,
    },
    ForInit {
        s: u32,
    },
    ForTest {
        s: u32,
        t: u32,
    },
    ForSet {
        s: u32,
    },
    ForNext {
        s: u32,
    },
    Call {
        s: u32,
        d: u16,
    },
    Builtin {
        s: u32,
        d: u16,
    },
    /// `d := a[b]` for a text `a` (the UTF-16 code unit at position `b`).
    CharAt {
        d: u16,
        a: u16,
        b: u16,
    },
    /// `S += x`: `old` holds the value `S` had before `x` was evaluated, `a` holds `x`.
    AppendText {
        s: u32,
        old: u16,
        a: u16,
    },
    /// Jumps to `t` when `ops::compare(a, b)` is Equal / Less / Greater (the tests of a `case` label and of `in [...]`).
    JumpEq {
        a: u16,
        b: u16,
        t: u32,
    },
    JumpLt {
        a: u16,
        b: u16,
        t: u32,
    },
    JumpGt {
        a: u16,
        b: u16,
        t: u32,
    },
    DotNet {
        s: u32,
        d: u16,
    },
    /// `d := ThisInstance` (the running object).
    ThisInstance {
        d: u16,
    },
    /// `d := BaseInstance` (the base object of an extension).
    BaseInstance {
        d: u16,
    },
    /// An operation on one value whose other parameters are in the side table `aux`.
    Aux {
        s: u32,
        d: u16,
        a: u16,
    },
    /// The arguments of a TryFunction call whose result is used are evaluated inside the try: the code up to `end`
    /// (the `Call`) evaluates them; an error there is trapped as the call's error would be, `d` gets false and the
    /// code goes on after the call (misc/lt-ct-odd-leads/CtErrorLeads.al:76-93, :550-570 Item27_TryFunction_ArgumentRaises).
    TryArgs {
        end: u32,
        d: u16,
    },
    /// `asserterror`: the body is the code up to `end`; a caught error rolls back, anything else leaves as it would.
    Assert {
        end: u32,
        from: u16,
    },
    /// `foreach`: evaluates the collection in `a` once into the items held in the site's register.
    ForeachInit {
        s: u32,
    },
    /// The next item into the loop variable, or a jump to `t` when the items are used up.
    ForeachNext {
        s: u32,
        t: u32,
    },
    /// Resolves the target of the interface call whose site comes next, in the receiver's register, before the arguments run.
    IfaceResolve {
        r: u16,
        s: u32,
    },
    IfaceCall {
        s: u32,
        d: u16,
    },
    /// Lets go of a temporary whose value nobody reads.
    Clear {
        a: u16,
    },
    /// The statement marker `p` ran: the debugger may stop before the statement (ADR-048). Only in the debug form.
    DbgHit {
        p: u32,
    },
    // ---- value capture and iteration tracking (ADR-030): only in the captured form of a routine ----
    /// A statement marker ran: the statement is one of the open loops' pass.
    CapHit {
        p: u32,
    },
    /// The statement after the marker `p` begins.
    CapBegin {
        p: u32,
    },
    /// The statement ended without an error: the locals it wrote or changed are recorded (site `s`).
    CapEnd {
        s: u32,
    },
    /// A `break` or `continue` leaves a statement: it is no longer open, and nothing is recorded.
    CapPop,
    /// A loop statement starts; `d` gets its id (an Integer) or Null when nothing is tracked.
    CapLoopEnter {
        d: u16,
    },
    /// A pass of the loop in `a` starts.
    CapLoopPass {
        a: u16,
    },
    /// The loop in `a` ends.
    CapLoopExit {
        a: u16,
    },
    /// A builtin the VM runs on operands it holds itself (subscriptions, `CurrFieldNo`, `ClearAll`, `ApplicationArea`).
    Special {
        s: u32,
        d: u16,
    },
    /// `d := ` an error the code was lowered to raise (`Refused`, `Fail`, a builtin nothing implements).
    Fault {
        k: u32,
    },
    /// Forgets the null string a .NET call left (before the operand of a `DotNetCoerce` or `DotNetOperand` is evaluated).
    DotNetClear,
    /// `d := the .NET value a coerced to type t`.
    DotNetCoerce {
        d: u16,
        a: u16,
        t: u32,
    },
    /// `d := the operand a settled for its use h`.
    DotNetOperand {
        d: u16,
        a: u16,
        h: ir::OperandUse,
    },
    /// An assumption of an unprobed behavior ran (`Stmt::Assume`).
    Assume {
        a: al_builtins::assumptions::Assumption,
    },
    /// `Exit()`.
    Ret,
    /// `Exit(a)`.
    RetVal {
        a: u16,
    },
    /// A `break` or `continue` outside every loop of this routine: the signal goes to the caller.
    RaiseBreak,
    RaiseContinue,
}

pub(crate) struct CallSite {
    pub object: usize,
    pub routine: usize,
    pub instance: Option<u16>,
    pub arg_regs: Vec<u16>,
    /// The IR arguments with the value expressions blanked: only the by-reference places matter after the call.
    pub args: Vec<ir::Arg>,
    pub try_call: bool,
}

pub(crate) struct BuiltinSite {
    pub id: al_builtins::builtin_id::BuiltinId,
    /// A builtin the VM runs itself (`Session::vm_class_call`) instead of a registered native.
    pub vm_class: bool,
    pub native: Option<al_builtins::native::NativeFn>,
    pub class: al_builtins::builtin_id::Class,
    pub receiver: Option<u16>,
    pub receiver_arg: Option<ir::Arg>,
    pub arg_regs: Vec<u16>,
    pub args: Vec<ir::Arg>,
    pub ret_used: bool,
    /// A registered native whose receiver and arguments are all values (nothing is written back) and that has no
    /// special handling in `run_builtin`: it runs on the short path (`Session::run_plain_builtin`).
    pub plain: bool,
}

pub(crate) enum AuxSite {
    InterfaceTest {
        objects: Vec<u32>,
        cast: bool,
    },
    EnumImpl {
        by_ordinal: Vec<(i32, usize)>,
        unknown: Option<usize>,
    },
    Control {
        control: usize,
        runtime: bool,
    },
    FieldOf {
        pos: usize,
    },
    QueryColumn {
        column: usize,
    },
    /// `--tdd` (ADR-051 decision 4): the initial value of `ty` (`Null` for a statement), noting that the stub ran.
    TddStub {
        stub: String,
        ty: Option<al_builtins::value::RtType>,
    },
}

/// What a `Fault` raises.
pub(crate) enum FaultKind {
    Unsupported(String),
    Error { message: String, code: String },
}

pub(crate) struct SpecialSite {
    pub id: al_builtins::builtin_id::BuiltinId,
    /// The record a `CurrFieldNo` belongs to.
    pub place: Option<ir::Place>,
    pub arg: Option<u16>,
    pub ret_used: bool,
}

pub(crate) struct ForeachSite {
    pub var: ir::Place,
    pub coll: u16,
    /// Registers that hold the items (a `Value::Array`) and the next position (an `Integer`).
    pub items: u16,
    pub at: u16,
    /// Registers that hold the List being run over and its version at the start: a change to the list ends the loop with an error
    /// (collections/RefSemantics.al:Foreach_ListAddWhileIterating).
    pub watch: u16,
    pub ver: u16,
    /// The collection when it is a variable (the text of the error of a .NET variable that holds nothing).
    pub collection: Option<ir::Expr>,
    pub cap_loop: Option<u16>,
}

pub(crate) struct IfaceSite {
    pub targets: Vec<(usize, usize)>,
    pub recv: u16,
    pub arg_regs: Vec<u16>,
    pub args: Vec<ir::Arg>,
}

pub(crate) struct AppendSite {
    pub place: ir::Place,
    pub max: Option<u32>,
}

pub(crate) struct DotNetSite {
    pub ty: Option<usize>,
    pub op: ir::DotNetOp,
    pub recv: Option<u16>,
    pub arg_regs: Vec<u16>,
    pub args: Vec<ir::Arg>,
}

pub(crate) struct ForSite {
    pub var: ir::Place,
    pub down: bool,
    pub from: u16,
    pub to: u16,
    /// Registers that hold the counter and the last value, as `Value::BigInteger`.
    pub i: u16,
    pub end: u16,
    /// The counter is a local of type Integer: the round-trip through `write` and `read` is a plain store and load.
    pub int_local: Option<u16>,
    /// The register that holds the capture's id of this loop (the captured form of a routine).
    pub cap_loop: Option<u16>,
}

/// The compiled form of one routine.
/// A statement a capture watches: its marker and the locals it writes by its shape.
pub(crate) struct CapSite {
    pub pos: u32,
    pub writes: Vec<usize>,
}

pub(crate) struct Chunk {
    pub code: Vec<Op>,
    pub cap_sites: Vec<CapSite>,
    pub consts: Vec<ir::Const>,
    pub places: Vec<ir::Place>,
    pub types: Vec<al_builtins::value::RtType>,
    pub calls: Vec<CallSite>,
    pub builtins: Vec<BuiltinSite>,
    pub fors: Vec<ForSite>,
    pub appends: Vec<AppendSite>,
    pub dotnets: Vec<DotNetSite>,
    pub aux: Vec<AuxSite>,
    pub foreachs: Vec<ForeachSite>,
    pub specials: Vec<SpecialSite>,
    pub faults: Vec<FaultKind>,
    pub ifaces: Vec<IfaceSite>,
    /// Registers `0..nlocals` are the routine's locals.
    pub nlocals: u16,
    /// Temporaries after the locals.
    pub temps: u16,
}

/// The chunks of a program, compiled the first time a routine is called (every routine runs as bytecode).
pub(crate) struct ExecProgram {
    chunks: Vec<Vec<OnceLock<Option<Box<Chunk>>>>>,
    /// The captured form of a routine (value capture and iteration tracking), built on first use by a capture.
    captured: Vec<Vec<OnceLock<Option<Box<Chunk>>>>>,
    /// The debug form of a routine (ADR-048), built on first use by a session with a debugger.
    debug: Vec<Vec<OnceLock<Option<Box<Chunk>>>>>,
}

impl ExecProgram {
    pub(crate) fn new(program: &ir::Program) -> ExecProgram {
        ExecProgram {
            chunks: program
                .objects
                .iter()
                .map(|o| o.routines.iter().map(|_| OnceLock::new()).collect())
                .collect(),
            captured: program
                .objects
                .iter()
                .map(|o| o.routines.iter().map(|_| OnceLock::new()).collect())
                .collect(),
            debug: program
                .objects
                .iter()
                .map(|o| o.routines.iter().map(|_| OnceLock::new()).collect())
                .collect(),
        }
    }

    /// The captured form of routine `routine` of object `object` .
    pub(crate) fn captured_chunk(&self, program: &ir::Program, object: usize, routine: usize) -> Option<&Chunk> {
        self.captured[object][routine]
            .get_or_init(|| compile(&program.objects[object].routines[routine], true).map(Box::new))
            .as_deref()
    }

    /// The debug form of routine `routine` of object `object` (ADR-048).
    pub(crate) fn debug_chunk(&self, program: &ir::Program, object: usize, routine: usize) -> Option<&Chunk> {
        self.debug[object][routine]
            .get_or_init(|| compile::compile_debug(&program.objects[object].routines[routine]).map(Box::new))
            .as_deref()
    }

    /// The chunk of routine `routine` of object `object`, if it compiles.
    pub(crate) fn chunk(&self, program: &ir::Program, object: usize, routine: usize) -> Option<&Chunk> {
        let cell = &self.chunks[object][routine];
        cell.get_or_init(|| {
            let c = compile(&program.objects[object].routines[routine], false)?;
            STATS.routines.fetch_add(1, Ordering::Relaxed);
            Some(Box::new(c))
        })
        .as_deref()
    }
}

/// Whether the counters count (`AL_EXEC_REPORT=1`), read once.
pub(crate) fn counting() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("AL_EXEC_REPORT").is_some())
}

/// Process-wide counters for the fallback report (`AL_EXEC_REPORT=1`).
pub(crate) struct Stats {
    pub routines: AtomicU64,
    pub bytecode_calls: AtomicU64,
    /// Typed instructions that met an operand of another kind (a local that holds a value of another kind than its declared one, for example an Integer variable that a List Get filled with a BigInteger; the generic path gives the result).
    pub typed_fallbacks: AtomicU64,
}

pub(crate) static STATS: Stats = Stats {
    routines: AtomicU64::new(0),
    bytecode_calls: AtomicU64::new(0),
    typed_fallbacks: AtomicU64::new(0),
};

/// One line for the report: routines compiled, calls by form, typed instructions that fell back.
pub fn report() -> String {
    let g = |a: &AtomicU64| a.load(Ordering::Relaxed);
    format!(
        "exec: {} routines compiled; calls: {} bytecode; typed fallbacks: {}",
        g(&STATS.routines),
        g(&STATS.bytecode_calls),
        g(&STATS.typed_fallbacks),
    )
}

/// The instructions the bytecode compiler emits for a routine, one `Debug` text each (tests of instruction selection).
pub fn disassemble(program: &ir::Program, object: usize, routine: usize) -> Option<Vec<String>> {
    let c = compile(&program.objects[object].routines[routine], false)?;
    Some(c.code.iter().map(|op| format!("{op:?}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_instruction_is_small() {
        assert!(std::mem::size_of::<Op>() <= 16, "{}", std::mem::size_of::<Op>());
    }
}
