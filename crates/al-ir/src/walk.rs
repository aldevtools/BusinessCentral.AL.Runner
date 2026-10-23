//! Every reference to another object or routine that an [`Object`] holds (ADR-033).
//!
//! The IR names other objects by position (`Expr::Call::object`, `RtType::Record`, `Field::table`
//! and the like). Lowering numbers them in one way, the link step in another; [`walk_object`]
//! visits every such site so that both can renumber them. The walk destructures every IR struct
//! it enters without `..`, so a new IR field stops the build here until its class is decided.
//!
//! Site classes:
//!
//! - **object**: an index into `Program::objects`.
//! - **routine**: an index into the routine list of *another* object (or of an extension that
//!   owns a control). A routine index into the object's own list is a position in the object
//!   itself and stays as it is.
//!
//! Source positions (`Stmt::Pos`, `Routine::pos`) are not references; link shifts them
//! (`Routine::shift_positions`).

use crate::*;

/// What a walk does at each reference site. The order inside a pair (`object`, `routine`) is:
/// `routine` first with the object's value as it is before `object` is visited, then `object`.
pub trait RefVisitor {
    /// An object reference.
    fn object(&mut self, r: &mut usize);
    /// A routine reference into object `object`. `object` is the value of the object reference
    /// before it is visited.
    fn routine(&mut self, r: &mut usize, object: usize);
    /// A field reference. `table` says which table the field is a field of.
    fn field(&mut self, r: &mut usize, table: TableOf);
    /// A site that stands for a set only the program knows (`sym::Expansion`); `index` is the entry in the object's
    /// `Refs::expansions`. `site` gives access to what link has to write.
    fn expansion(&mut self, index: usize, site: ExpansionSite<'_>);
    /// A control reference of an expression: an index into the object's control references, which
    /// lowering wrote there (`Expr::Control`).
    fn control(&mut self, r: &mut usize);
    /// A routine body that is not decoded yet ([`Body`]). Return true when the visitor has deferred its work on the
    /// body to the decode ([`Body::fix`]); false (the default) decodes the body now and walks it.
    fn lazy_body(&mut self, _body: &mut Body) -> bool {
        false
    }
}

/// The marker lowering writes in place of the set a site stands for: an object or routine position that no
/// object has (`NO_OBJECT` is the position of "no table").
pub const EXPAND: usize = usize::MAX - 1;
/// The same marker in the `u32` list of `Expr::InterfaceTest` and as the ordinal in `Expr::EnumImpl`.
pub const EXPAND_U32: u32 = u32::MAX - 1;
pub const EXPAND_ORDINAL: i32 = i32::MIN;

/// What a site that stands for a set holds.
pub enum ExpansionSite<'a> {
    InterfaceCall {
        targets: &'a mut Vec<(usize, usize)>,
    },
    InterfaceTest {
        objects: &'a mut Vec<u32>,
    },
    EnumImpl {
        by_ordinal: &'a mut Vec<(i32, usize)>,
        unknown: &'a mut Option<usize>,
    },
}

/// Which table a field reference is a field of. Object references inside it have the value they
/// had before the walk visited them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableOf {
    /// The table object that is being walked (its keys, relations and calculation formulas).
    Own,
    /// The table of a field trigger: the table of the object being walked, or the one it extends.
    Trigger,
    /// The table that is this object reference.
    Object(usize),
    /// The value is already an index into the object's field references: lowering wrote it so,
    /// where the expression it sits in knows the table (`Place::Field`, `Const::FieldPos`).
    Embedded,
    /// The source table of the page being walked.
    PageSource,
    /// The source table of the page that is this object reference.
    SourceOf(usize),
    /// The table of data item `item` of the report that is object reference `report`.
    BaseItem { report: usize, item: usize },
    /// The structure does not say which table (the lowering did not resolve it).
    Unknown,
}

impl TableOf {
    /// Which table, in words, for the message of a defect.
    pub fn describe(&self) -> &'static str {
        match self {
            TableOf::Own => "the table itself",
            TableOf::Trigger => "the table of a field trigger",
            TableOf::Object(_) => "a table the structure names",
            TableOf::Embedded => "a field the expression names",
            TableOf::PageSource => "the source table of the page",
            TableOf::SourceOf(_) => "the source table of a page the structure names",
            TableOf::BaseItem { .. } => "the table of a data item of the base report",
            TableOf::Unknown => "no table",
        }
    }
    fn object(table: Option<usize>) -> TableOf {
        match table {
            Some(t) if t != NO_OBJECT => TableOf::Object(t),
            _ => TableOf::Unknown,
        }
    }

    fn source_of(page: Option<usize>) -> TableOf {
        match page {
            Some(p) if p != NO_OBJECT => TableOf::SourceOf(p),
            _ => TableOf::Unknown,
        }
    }
}

fn fld<V: RefVisitor>(v: &mut V, r: &mut usize, table: TableOf) {
    v.field(r, table);
}

fn field_opt<V: RefVisitor>(v: &mut V, r: &mut Option<usize>, table: TableOf) {
    if let Some(x) = r {
        v.field(x, table);
    }
}

fn field32<V: RefVisitor>(v: &mut V, r: &mut u32, table: TableOf) {
    let mut x = *r as usize;
    v.field(&mut x, table);
    *r = x as u32;
}

/// What a table-less query item holds as its table (`QueryItem::table`): not an object.
pub const NO_OBJECT: usize = usize::MAX;

fn obj<V: RefVisitor>(v: &mut V, r: &mut usize) {
    if *r != NO_OBJECT {
        v.object(r);
    }
}

fn object32<V: RefVisitor>(v: &mut V, r: &mut u32) {
    let mut x = *r as usize;
    obj(v, &mut x);
    *r = x as u32;
}

fn object_opt<V: RefVisitor>(v: &mut V, r: &mut Option<usize>) {
    if let Some(x) = r {
        obj(v, x);
    }
}

fn pair<V: RefVisitor>(v: &mut V, p: &mut (usize, usize)) {
    v.routine(&mut p.1, p.0);
    obj(v, &mut p.0);
}

/// Visits every reference of `o`.
pub fn walk_object<V: RefVisitor>(o: &mut Object, v: &mut V) {
    let Object {
        kind: _,
        uncompiled: _,
        id: _,
        name: _,
        globals,
        routines,
        properties: _,
        enum_values: _,
        enum_captions: _,
        table,
        rec_slot: _,
        xrec_slot: _,
        page,
        app: _,
        query,
        xmlport,
        report,
        report_ext,
        namespace: _,
        // The id of the extended object, not a position.
        extends_id: _,
        layouts: _,
    } = o;
    for s in globals {
        walk_rt(&mut s.ty, v);
    }
    for r in routines {
        walk_routine(r, v);
    }
    if let Some(t) = table {
        walk_table(t, v);
    }
    if let Some(p) = page {
        walk_page(p, v);
    }
    if let Some(q) = query {
        walk_query(q, v);
    }
    if let Some(x) = xmlport {
        walk_xmlport(x, v);
    }
    if let Some(r) = report {
        walk_report(r, v);
    }
    if let Some(e) = report_ext {
        walk_report_ext(e, v);
    }
}

/// Visits the object references of a program-level list: the publisher of each uncompiled subscriber.
pub fn walk_subscribers<V: RefVisitor>(subs: &mut [UncompiledSubscriber], v: &mut V) {
    for s in subs {
        let UncompiledSubscriber {
            publisher,
            platform: _,
            event: _,
            element: _,
            subscriber: _,
        } = s;
        if let Some(p) = publisher {
            object32(v, p);
        }
    }
}

fn walk_rt<V: RefVisitor>(t: &mut RtType, v: &mut V) {
    match t {
        RtType::Enum(o)
        | RtType::Record(o, _)
        | RtType::Codeunit(o)
        | RtType::TestPage(o)
        | RtType::Page(o)
        | RtType::Query(o)
        | RtType::Interface(o) => object32(v, o),
        RtType::List(e) => walk_rt(e, v),
        RtType::Dictionary(k, e) => {
            walk_rt(k, v);
            walk_rt(e, v);
        }
        RtType::Array(_, e) => walk_rt(e, v),
        RtType::Boolean
        | RtType::Integer
        | RtType::BigInteger
        | RtType::Decimal
        | RtType::Byte
        | RtType::Char
        | RtType::Text(_)
        | RtType::Code(_)
        | RtType::Date
        | RtType::Time
        | RtType::DateTime
        | RtType::Duration
        | RtType::Guid
        | RtType::DateFormula
        | RtType::Option(_)
        | RtType::Variant
        | RtType::Library(_)
        // An index into `Program::dotnet_types`, which the program shares as a whole.
        | RtType::DotNet(_) => {}
    }
}

fn walk_const<V: RefVisitor>(c: &mut Const, v: &mut V) {
    match c {
        Const::Enum(o, _) => object32(v, o),
        Const::FieldPos(f) => field32(v, f, TableOf::Embedded),
        Const::Boolean(_)
        | Const::Integer(_)
        | Const::BigInteger(_)
        | Const::Decimal(_)
        | Const::Text(_)
        | Const::Date(_)
        | Const::Time(_)
        | Const::DateTime(_)
        | Const::Option(..)
        | Const::DateFormula(_)
        | Const::Guid(_) => {}
    }
}

fn walk_routine<V: RefVisitor>(r: &mut Routine, v: &mut V) {
    let Routine {
        name: _,
        field,
        is_trigger: _,
        attributes,
        params: _,
        by_var: _,
        locals,
        ret,
        ret_local: _,
        body,
        pos: _,
        end_pos: _,
        begin_pos: _,
        container: _,
        expr_tys: _,
        reads_var_max_len: _,
    } = r;
    field_opt(v, field, TableOf::Trigger);
    for a in attributes {
        let Attribute { name: _, args, text: _ } = a;
        for c in args {
            walk_const(c, v);
        }
    }
    for s in locals {
        walk_rt(&mut s.ty, v);
    }
    if let Some(t) = ret {
        walk_rt(t, v);
    }
    if body.is_loaded() || !v.lazy_body(body) {
        walk_stmts(body, v);
    }
}

/// Every reference of a list of statements (the body of a routine that [`RefVisitor::lazy_body`] deferred).
pub fn walk_body<V: RefVisitor>(body: &mut [Stmt], v: &mut V) {
    walk_stmts(body, v);
}

fn walk_stmts<V: RefVisitor>(body: &mut [Stmt], v: &mut V) {
    for s in body {
        walk_stmt(s, v);
    }
}

fn walk_stmt<V: RefVisitor>(s: &mut Stmt, v: &mut V) {
    match s {
        Stmt::Assign(p, e) => {
            walk_place(p, v);
            walk_expr(e, v);
        }
        Stmt::Expr(e) => walk_expr(e, v),
        Stmt::If(c, a, b) => {
            walk_expr(c, v);
            walk_stmts(a, v);
            walk_stmts(b, v);
        }
        Stmt::While(c, b) => {
            walk_expr(c, v);
            walk_stmts(b, v);
        }
        Stmt::Repeat(b, c) => {
            walk_stmts(b, v);
            walk_expr(c, v);
        }
        Stmt::For {
            var,
            from,
            to,
            down: _,
            body,
        } => {
            walk_place(var, v);
            walk_expr(from, v);
            walk_expr(to, v);
            walk_stmts(body, v);
        }
        Stmt::Foreach { var, collection, body } => {
            walk_place(var, v);
            walk_expr(collection, v);
            walk_stmts(body, v);
        }
        Stmt::Case {
            scrutinee,
            branches,
            otherwise,
        } => {
            walk_expr(scrutinee, v);
            for (elems, body) in branches {
                for e in elems {
                    walk_set_elem(e, v);
                }
                walk_stmts(body, v);
            }
            walk_stmts(otherwise, v);
        }
        Stmt::Exit(e) => {
            if let Some(e) = e {
                walk_expr(e, v);
            }
        }
        Stmt::AssertError(b) => walk_stmts(b, v),
        Stmt::Break | Stmt::Pos(_) | Stmt::Assume(_) | Stmt::TddMark(_) | Stmt::Continue => {}
    }
}

fn walk_set_elem<V: RefVisitor>(e: &mut SetElem, v: &mut V) {
    match e {
        SetElem::One(x) => walk_expr(x, v),
        SetElem::Range(a, b) => {
            if let Some(a) = a {
                walk_expr(a, v);
            }
            if let Some(b) = b {
                walk_expr(b, v);
            }
        }
    }
}

fn walk_place<V: RefVisitor>(p: &mut Place, v: &mut V) {
    match p {
        Place::Local(_) | Place::Global(_) | Place::BaseGlobal(_) => {}
        Place::Index(base, idx) => {
            walk_place(base, v);
            for e in idx {
                walk_expr(e, v);
            }
        }
        Place::Field(base, f) => {
            fld(v, f, TableOf::Embedded);
            walk_place(base, v);
        }
    }
}

fn walk_arg<V: RefVisitor>(a: &mut Arg, v: &mut V) {
    match a {
        Arg::Value(e) => walk_expr(e, v),
        Arg::Ref(p) => walk_place(p, v),
    }
}

fn walk_args<V: RefVisitor>(args: &mut [Arg], v: &mut V) {
    for a in args {
        walk_arg(a, v);
    }
}

fn walk_expr<V: RefVisitor>(e: &mut Expr, v: &mut V) {
    match e {
        Expr::Const(c) => walk_const(c, v),
        Expr::Load(p) => walk_place(p, v),
        Expr::Unary(_, x) => walk_expr(x, v),
        Expr::Binary(_, a, b) => {
            walk_expr(a, v);
            walk_expr(b, v);
        }
        Expr::In(x, set) => {
            walk_expr(x, v);
            for s in set {
                walk_set_elem(s, v);
            }
        }
        Expr::Ternary(c, a, b) => {
            walk_expr(c, v);
            walk_expr(a, v);
            walk_expr(b, v);
        }
        Expr::Convert(t, x) | Expr::DotNetCoerce(t, x) => {
            walk_rt(t, v);
            walk_expr(x, v);
        }
        Expr::Call {
            object,
            routine,
            instance,
            args,
            try_call: _,
        } => {
            v.routine(routine, *object);
            obj(v, object);
            if let Some(i) = instance {
                walk_expr(i, v);
            }
            walk_args(args, v);
        }
        Expr::InterfaceCall {
            receiver,
            targets,
            args,
        } => {
            walk_expr(receiver, v);
            match targets.first() {
                Some(&(EXPAND, index)) => v.expansion(index, ExpansionSite::InterfaceCall { targets }),
                _ => {
                    for t in targets {
                        pair(v, t);
                    }
                }
            }
            walk_args(args, v);
        }
        Expr::InterfaceTest {
            value,
            objects,
            cast: _,
        } => {
            walk_expr(value, v);
            match objects.first() {
                Some(&EXPAND_U32) => {
                    let index = objects[1] as usize;
                    v.expansion(index, ExpansionSite::InterfaceTest { objects });
                }
                _ => {
                    for o in objects {
                        object32(v, o);
                    }
                }
            }
        }
        Expr::EnumImpl {
            value,
            by_ordinal,
            unknown,
        } => {
            walk_expr(value, v);
            match by_ordinal.first() {
                Some(&(EXPAND_ORDINAL, index)) => v.expansion(index, ExpansionSite::EnumImpl { by_ordinal, unknown }),
                _ => {
                    for (_, o) in by_ordinal {
                        obj(v, o);
                    }
                    object_opt(v, unknown);
                }
            }
        }
        Expr::Control {
            page,
            control,
            runtime: _,
        } => {
            v.control(control);
            walk_expr(page, v);
        }
        Expr::FieldOf(x, f) => {
            fld(v, f, TableOf::Embedded);
            walk_expr(x, v);
        }
        Expr::QueryColumn { query, column } => {
            // The column of a query is named like a field of a table: by its number, which is its
            // position in the query's column list.
            fld(v, column, TableOf::Embedded);
            walk_expr(query, v);
        }
        Expr::DotNet {
            ty: _,
            receiver,
            op: _,
            args,
        } => {
            if let Some(r) = receiver {
                walk_expr(r, v);
            }
            walk_args(args, v);
        }
        Expr::CharAt(a, b) => {
            walk_expr(a, v);
            walk_expr(b, v);
        }
        Expr::DotNetOperand(x, _) => walk_expr(x, v),
        Expr::Builtin {
            id: _,
            receiver,
            args,
            ret_used: _,
        } => {
            if let Some(r) = receiver {
                walk_arg(r, v);
            }
            walk_args(args, v);
        }
        Expr::TddFit { inner, .. } | Expr::FitOrRefuse { inner, .. } => walk_expr(inner, v),
        Expr::TddStub { stub: _, args, ty } => {
            if let Some(t) = ty {
                walk_rt(t, v);
            }
            for a in args {
                walk_expr(a, v);
            }
        }
        Expr::BaseInstance | Expr::ThisInstance | Expr::Refused(_) | Expr::Fail { .. } => {}
    }
}

/// Visits the references of what a table extension adds to its table. Its field references are
/// fields of the table it extends.
pub fn walk_table_contribution<V: RefVisitor>(c: &mut crate::sym::TableContribution, v: &mut V) {
    let crate::sym::TableContribution {
        fields,
        keys,
        modifies,
        properties: _,
    } = c;
    for f in fields {
        walk_field(f, v);
    }
    for k in keys {
        let crate::sym::KeyDecl {
            fields,
            unique: _,
            props: _,
        } = k;
        for f in fields {
            fld(v, f, TableOf::Own);
        }
    }
    for m in modifies {
        let crate::sym::FieldModify {
            field: _,
            properties: _,
            before_validate,
            after_validate,
        } = m;
        for p in before_validate.iter_mut().chain(after_validate.iter_mut()) {
            pair(v, p);
        }
    }
}

fn walk_table<V: RefVisitor>(t: &mut Table, v: &mut V) {
    let Table {
        fields,
        keys,
        unique_keys: _,
        key_props: _,
        per_company: _,
        extensions,
        unsupported: _,
    } = t;
    for k in keys {
        for f in k {
            fld(v, f, TableOf::Own);
        }
    }
    for f in fields {
        walk_field(f, v);
    }
    for e in extensions {
        obj(v, e);
    }
}

fn walk_field<V: RefVisitor>(f: &mut Field, v: &mut V) {
    let Field {
        id: _,
        name: _,
        ty,
        stored: _,
        init,
        init_text: _,
        relation_table: _,
        relation_tables: _,
        auto_increment: _,
        relation: _,
        on_validate,
        ext_before_validate,
        ext_after_validate,
        on_lookup,
        caption_class,
        caption_class_refusal: _,
        auto_format_expression,
        auto_format_refusal: _,
        properties: _,
        relation_arms,
        relation_meta,
        relation_runtime,
        relation_texts: _,
        flow_filter: _,
        tdd_length_unknown: _,
        calc,
        calc_refusal: _,
        declared_in,
    } = f;
    walk_rt(ty, v);
    if let Some(c) = init {
        walk_const(c, v);
    }
    if let Some(p) = on_validate {
        pair(v, p);
    }
    for p in ext_before_validate {
        pair(v, p);
    }
    for p in ext_after_validate {
        pair(v, p);
    }
    if let Some(p) = on_lookup {
        pair(v, p);
    }
    if let Some(p) = caption_class {
        pair(v, p);
    }
    if let Some(p) = auto_format_expression {
        pair(v, p);
    }
    for a in relation_arms.iter_mut().chain(relation_runtime.iter_mut()) {
        let RelationArm {
            conditions,
            table,
            table_name: _,
            field,
            wheres,
            assumes: _,
        } = a;
        let related = TableOf::object(*table);
        for c in conditions {
            let RelationCond {
                field,
                value,
                filter: _,
            } = c;
            fld(v, field, TableOf::Own);
            walk_const(value, v);
        }
        field_opt(v, field, related);
        if let Some(ws) = wheres {
            for w in ws {
                match w {
                    RelationWhere::Value { related: f, text: _ } => fld(v, f, related),
                    RelationWhere::Field { related: f, own } => {
                        fld(v, f, related);
                        fld(v, own, TableOf::Own);
                    }
                }
            }
        }
        object_opt(v, table);
    }
    for m in relation_meta {
        let MetaRelationArm {
            conditions,
            filters: _,
            values: _,
            table,
            field,
        } = m;
        for c in conditions {
            fld(v, c, TableOf::Own);
        }
        field_opt(v, field, TableOf::object(Some(*table)));
        obj(v, table);
    }
    if let Some(c) = calc {
        let CalcFormula {
            method: _,
            negate: _,
            table,
            field,
            conds,
        } = c;
        let source = TableOf::object(Some(*table));
        field_opt(v, field, source);
        walk_conds(conds, source, v);
        obj(v, table);
    }
    if let Some(d) = declared_in {
        object32(v, d);
    }
}

/// The conditions of a formula or a filter on the table `source`.
fn walk_conds<V: RefVisitor>(conds: &mut [CalcCond], source: TableOf, v: &mut V) {
    for c in conds {
        let CalcCond { field, value } = c;
        fld(v, field, source);
        walk_calc_value(value, v);
    }
}

fn walk_calc_value<V: RefVisitor>(value: &mut CalcValue, v: &mut V) {
    match value {
        CalcValue::Const(c) => walk_const(c, v),
        // Positions in the table being calculated.
        CalcValue::Field(f) | CalcValue::UpperLimit(f) | CalcValue::FieldFilter(f) => fld(v, f, TableOf::Own),
        CalcValue::Filter(_) => {}
    }
}

fn walk_page<V: RefVisitor>(p: &mut Page, v: &mut V) {
    let Page {
        assumes: _,
        controls,
        triggers: _,
        curr_page_slot: _,
        unsupported: _,
        extensions,
        ext_triggers,
        action_areas: _,
        big_integer_expressions: _,
    } = p;
    // The page each part shows, before the walk renumbers it (a `Provider` link reads the row of
    // another part).
    let part_pages: Vec<Option<usize>> = controls.iter().map(|c| c.part_page).collect();
    for c in controls {
        walk_control(c, &part_pages, None, v);
    }
    for (_, ext, r) in ext_triggers {
        v.routine(r, *ext);
        obj(v, ext);
    }
    for e in extensions {
        obj(v, e);
    }
}

/// Visits the references of what a page extension adds to its page. The routines of its controls are its own.
pub fn walk_page_contribution<V: RefVisitor>(c: &mut crate::sym::PageContribution, v: &mut V) {
    let crate::sym::PageContribution {
        controls,
        mods,
        triggers: _,
    } = c;
    for c in controls {
        let host = Some(match c.provider_page {
            Some(p) => TableOf::source_of(Some(p)),
            None => TableOf::PageSource,
        });
        walk_control(&mut c.control, &[], host, v);
    }
    for m in mods {
        let crate::sym::ControlMod {
            target: _,
            props,
            triggers: _,
        } = m;
        for (_, p) in props.iter_mut() {
            if let PropVal::Ext(o, r) = p {
                v.routine(r, *o);
                obj(v, o);
            }
        }
    }
}

fn walk_part_link<V: RefVisitor>(l: &mut PartLink, target: TableOf, host: TableOf, v: &mut V) {
    let PartLink {
        entries,
        view: _,
        provider: _,
        consts: _,
    } = l;
    for e in entries {
        match e {
            LinkEntry::Field(a, b) => {
                fld(v, a, target);
                fld(v, b, host);
            }
            LinkEntry::Filter(a, _) => fld(v, a, target),
        }
    }
}

fn walk_control<V: RefVisitor>(
    c: &mut Control,
    part_pages: &[Option<usize>],
    host_override: Option<TableOf>,
    v: &mut V,
) {
    let Control {
        name: _,
        kind: _,
        parent: _,
        binding,
        props,
        triggers,
        part_page,
        part_link,
        owner,
        ext_triggers,
        run_object,
        action_ref: _,
    } = c;
    // The routines of `binding`, `triggers` and `PropVal::Routine` belong to the extension that
    // owns the control; those of the page itself are positions in the page.
    let owner_ref = *owner;
    match binding {
        Binding::Field(f) => fld(v, f, TableOf::PageSource),
        Binding::Expr(r) => {
            if let Some(o) = owner_ref {
                v.routine(r, o);
            }
        }
        Binding::None | Binding::Var(_) | Binding::BaseVar(_) | Binding::Caption => {}
    }
    for (_, p) in props.iter_mut() {
        match p {
            PropVal::Lit(_) => {}
            PropVal::Routine(r) => {
                if let Some(o) = owner_ref {
                    v.routine(r, o);
                }
            }
            PropVal::Ext(o, r) => {
                v.routine(r, *o);
                obj(v, o);
            }
        }
    }
    if let Some(o) = owner_ref {
        for (_, r) in triggers.iter_mut() {
            v.routine(r, o);
        }
    }
    if let Some(l) = part_link {
        // The host field is a field of the page, or of the part named by `Provider`.
        let host = match (host_override, l.provider) {
            (Some(h), _) => h,
            (None, Some(p)) => TableOf::source_of(part_pages.get(p).copied().flatten()),
            (None, None) => TableOf::PageSource,
        };
        walk_part_link(l, TableOf::source_of(*part_page), host, v);
    }
    object_opt(v, part_page);
    for (_, ext, r) in ext_triggers {
        v.routine(r, *ext);
        obj(v, ext);
    }
    if let Some(r) = run_object {
        let RunObject {
            kind: _,
            object,
            on_rec: _,
            mode: _,
            link,
            unsupported: _,
        } = r;
        if let Some(l) = link {
            walk_part_link(l, TableOf::source_of(*object), TableOf::PageSource, v);
        }
        object_opt(v, object);
    }
    object_opt(v, owner);
}

fn walk_query<V: RefVisitor>(q: &mut Query, v: &mut V) {
    let Query {
        items,
        columns,
        order_by: _,
        top: _,
    } = q;
    // The table of each item, before the walk renumbers it.
    let tables: Vec<Option<usize>> = items.iter().map(|i| Some(i.table)).collect();
    let parents: Vec<Option<usize>> = items.iter().map(|i| i.parent).collect();
    for (n, i) in items.iter_mut().enumerate() {
        let QueryItem {
            name: _,
            table,
            parent: _,
            left_outer: _,
            right_outer: _,
            link,
            link_items,
            table_filter,
        } = i;
        let own = TableOf::object(Some(*table));
        for (k, (mine, theirs)) in link.iter_mut().enumerate() {
            // The item whose row the pair reads: one further out when `link_items` says so.
            let other = link_items.get(k).copied().or(parents[n]);
            fld(v, mine, own);
            fld(
                v,
                theirs,
                TableOf::object(other.and_then(|j| tables.get(j).copied().flatten())),
            );
        }
        walk_conds(table_filter, own, v);
        obj(v, table);
    }
    for c in columns {
        let QueryColumn {
            name: _,
            caption: _,
            item,
            field,
            filter_element: _,
            method: _,
            reverse_sign: _,
            ty,
            static_filter,
        } = c;
        field_opt(v, field, TableOf::object(tables.get(*item).copied().flatten()));
        walk_rt(ty, v);
        if let Some(f) = static_filter {
            walk_calc_value(f, v);
        }
    }
}

fn walk_xmlport<V: RefVisitor>(x: &mut XmlPort, v: &mut V) {
    let XmlPort {
        direction: _,
        format: _,
        field_separator: _,
        field_delimiter: _,
        record_separator: _,
        table_separator: _,
        text_encoding_declared: _,
        utf8: _,
        xml_utf8: _,
        namespaces: _,
        inline_schema: _,
        use_lax: _,
        xml_v11: _,
        format_evaluate_xml: _,
        format_evaluate_declared: _,
        no_default_fields_validation: _,
        nodes,
        source_slot: _,
        destination_slot: _,
        unsupported: _,
        run_unsupported: _,
        export_unsupported: _,
    } = x;
    let tables: Vec<Option<usize>> = nodes.iter().map(|n| n.table).collect();
    for n in nodes {
        let XmlNode {
            name: _,
            xml_name: _,
            kind: _,
            parent: _,
            slot: _,
            table,
            table_node,
            field,
            link_node,
            link_fields,
            auto_save: _,
            auto_update: _,
            auto_replace: _,
            use_temporary: _,
            min_zero: _,
            max_unbounded: _,
            width: _,
            ns_prefix: _,
            auto_calc: _,
            unbound: _,
            required: _,
            field_validate: _,
            source_view: _,
            triggers: _,
        } = n;
        let of_node = |i: &Option<usize>| TableOf::object(i.and_then(|i| tables.get(i).copied().flatten()));
        field_opt(v, field, of_node(table_node));
        for (mine, theirs) in link_fields {
            fld(v, mine, TableOf::object(*table));
            fld(v, theirs, of_node(link_node));
        }
        object_opt(v, table);
    }
}

fn walk_report_item<V: RefVisitor>(i: &mut ReportItem, parent: TableOf, v: &mut V) {
    let ReportItem {
        name: _,
        parent: _,
        slot: _,
        table,
        view: _,
        link,
        link_item: _,
        on_pre: _,
        on_after_get: _,
        on_post: _,
        request_filter_fields: _,
        columns,
        max_iteration: _,
        print_only_if_detail: _,
        calc_fields,
    } = i;
    let own = TableOf::object(*table);
    for f in calc_fields {
        fld(v, f, own);
    }
    for (mine, theirs) in link {
        fld(v, mine, own);
        fld(v, theirs, parent);
    }
    for c in columns {
        walk_report_column(c, own, v);
    }
    object_opt(v, table);
}

fn walk_report_column<V: RefVisitor>(c: &mut ReportColumn, table: TableOf, v: &mut V) {
    let ReportColumn {
        name: _,
        routine: _,
        unsupported: _,
        field,
        item_field,
        include_caption: _,
        caption_table,
        caption_field,
        value_unsupported: _,
        decimal_places: _,
        option_caption_ml: _,
    } = c;
    field_opt(v, field, table);
    field_opt(v, item_field, table);
    field_opt(v, caption_field, TableOf::object(*caption_table));
    object_opt(v, caption_table);
}

fn walk_report<V: RefVisitor>(r: &mut Report, v: &mut V) {
    let Report {
        processing_only: _,
        use_request_page: _,
        has_request_page: _,
        transaction_type: _,
        items,
        on_init: _,
        on_pre: _,
        on_post: _,
        on_pre_rendering: _,
        extensions,
        request_flag_slot: _,
        output: _,
        unsupported: _,
        caption: _,
        format_region: _,
        about_title: _,
        about_text: _,
        layout: _,
        labels: _,
        dataset_unsupported: _,
        layout_file_missing: _,
        rendering_layouts: _,
        default_rendering_layout: _,
    } = r;
    let tables: Vec<Option<usize>> = items.iter().map(|i| i.table).collect();
    for i in items {
        // The link reads the row of the enclosing item that `link_item` names, else the parent's.
        let enclosing = i.link_item.or(i.parent);
        let parent = TableOf::object(enclosing.and_then(|p| tables.get(p).copied().flatten()));
        walk_report_item(i, parent, v);
    }
    for e in extensions {
        obj(v, e);
    }
}

fn walk_report_ext<V: RefVisitor>(e: &mut ReportExt, v: &mut V) {
    let ReportExt {
        base,
        on_pre: _,
        on_post: _,
        aliases: _,
        modifies: _,
        added,
        adds,
        labels: _,
    } = e;
    let base_report = *base;
    let added_tables: Vec<Option<usize>> = added.iter().map(|a| a.item.table).collect();
    for a in added {
        let AddedItem {
            item,
            base_parent,
            place: _,
        } = a;
        // An added item is nested under another added item or under a base item.
        let parent = match (item.parent, base_parent) {
            (Some(p), _) => TableOf::object(added_tables.get(p).copied().flatten()),
            (None, Some(b)) => TableOf::BaseItem {
                report: base_report,
                item: *b,
            },
            (None, None) => TableOf::Unknown,
        };
        walk_report_item(item, parent, v);
    }
    for a in adds {
        let ReportExtAdd { item, columns } = a;
        for c in columns {
            walk_report_column(
                c,
                TableOf::BaseItem {
                    report: base_report,
                    item: *item,
                },
                v,
            );
        }
    }
    obj(v, base);
}
