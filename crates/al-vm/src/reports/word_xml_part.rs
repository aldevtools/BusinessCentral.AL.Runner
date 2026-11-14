//! `Report.WordXmlPart([Number,] [ExtendedFormat])`: the custom XML part a Word layout binds to, built from the report's data
//! items, columns and labels alone (nothing runs). Pinned by the corpus probe reports/WordXmlPartNames.al (al-runner issues 6993
//! and 6994, BC 27.5, 28.5 and 29.0) and reports/AqTests.al:244-264:
//! - `<?xml version="1.0" encoding="utf-16"?>`, then the root `NavWordReportXmlPart` whose namespace is
//!   `urn:microsoft-dynamics-nav/reports/<name>/<id>/`, or with the flag
//!   `urn:microsoft-dynamics-businesscentral/internal/extended/reports/<name>/<id>/`; lines end in CR LF, two spaces indent;
//! - `BCReportInformation`, then `Labels` when the report has a label or an IncludeCaption column, then with the flag
//!   `<Tooltips />`, then the data items as the report nests them;
//! - in an item its columns sorted by name, then its nested items; a column's text is its name, a Blob column is an empty
//!   element; with the flag a data item has `ElementType="DataItem"`, a column `ElementType="Column"` and its `DataType`, a
//!   label `ElementType="Parameter"` and `DataType="String"`, each an `ElementId` (`element_id`);
//! - the columns, data items and labels of the reportextensions that change the report join the report's (WordXmlPart_OneExtension_*,
//!   WordXmlPart_TwoExtensions_*).

use super::*;
use al_builtins::library_kind::LibraryKind;
use al_builtins::value::RtType;

const NS_PLAIN: &str = "urn:microsoft-dynamics-nav";
const NS_EXTENDED: &str = "urn:microsoft-dynamics-businesscentral/internal/extended";

/// The `ElementId` of an item, column or label named `name` (the AL name as declared, not its tag; a caption label has the id of
/// its column): 32-bit FNV-1a over the UTF-16LE bytes of the name, read as a signed number, made positive. Checked on every id
/// the probe shows (`Base` 440217728, `Sales Hdr. - No/1` 2125348297, `It's % 50 *` 1742776786, `FName` and `FNameCaption`
/// 1486599940). The one hash with no positive counterpart is refused.
pub(crate) fn element_id(name: &str) -> Result<u32, Signal> {
    // `xliff_hash` is the same FNV-1a, shifted by 2147483647.
    let signed = al_ir::xliff_hash(name) - 2_147_483_647;
    u32::try_from(signed.unsigned_abs())
        .ok()
        .filter(|_| signed != i64::from(i32::MIN))
        .map_or_else(
            || {
                unsupported(format!(
                    "Report.WordXmlPart: the ElementId of `{name}` (its hash is -2147483648)"
                ))
            },
            Ok,
        )
}

/// A separator: a run of them is one underscore (WordXmlPart_SpacesDashDot_*: `Sales Hdr. - No/1` is `Sales_Hdr_No_1`,
/// `Line-Item.2` is `Line_Item_2`; WordXmlPart_Punctuation_*: ` (`, `) ` and `:` are one underscore each).
fn separator(c: char) -> bool {
    matches!(c, ' ' | '.' | '-' | '/' | '(' | ')' | ':')
}

/// A character written as `_xHHHH_`, as XmlConvert encodes a name (WordXmlPart_Punctuation_*: `&`, `#`, `,`, `+`, `;`, `'`, `*`).
fn encoded(c: char) -> bool {
    matches!(c, '&' | '#' | ',' | '+' | ';' | '\'' | '*')
}

/// The XML tag of a report, data item, column or label name. Pinned (WordXmlPartNames.al): letters, also non-ASCII ones, digits
/// and underscores stay (`Digit_Under__x`, `Ümlaut_Bericht_é_ß`); a run of separators is one underscore and one trailing dot is
/// dropped (`Order Conf.` is `Order_Conf`); `%` is `Percent`; the characters of `encoded` and a leading digit are `_xHHHH_`
/// (`1LT` is `_x0031_LT`, `2nd_Item` is `_x0032_nd_Item`). Any other character, a leading or trailing separator other than one
/// trailing dot, a separator next to an underscore, `%` at an end, a leading underscore and a text XmlConvert would read as an
/// escape are refused.
pub(crate) fn xml_tag(name: &str) -> Result<String, Signal> {
    let refuse = |why: &str| unsupported(format!("Report.WordXmlPart of the name `{name}`: {why} is not pinned"));
    let body = match name.strip_suffix('.') {
        Some(b) if !b.ends_with(separator) => b,
        _ => name,
    };
    if body.is_empty() || body.starts_with(separator) || body.ends_with(separator) {
        return refuse("an empty name, or a leading or trailing separator other than one dot");
    }
    if body.starts_with(['_', '%']) || body.ends_with('%') {
        return refuse("a leading underscore, or a percent sign at an end");
    }
    if has_escape_shape(body) {
        return refuse("a text of the shape _xHHHH_");
    }
    let mut out = String::new();
    let mut in_run = false;
    let mut prev = None;
    for (i, c) in body.chars().enumerate() {
        let after_underscore = prev == Some('_');
        prev = Some(c);
        if separator(c) {
            if after_underscore {
                return refuse("a separator after an underscore");
            }
            if !in_run {
                out.push('_');
            }
            in_run = true;
            continue;
        }
        if in_run && c == '_' {
            return refuse("an underscore after a separator");
        }
        in_run = false;
        match c {
            '0'..='9' if i == 0 => out.push_str(&crate::dataset::leading_digit(c, body)?),
            '0'..='9' | '_' => out.push(c),
            '%' => out.push_str("Percent"),
            c if encoded(c) => out.push_str(&format!("_x{:04X}_", c as u32)),
            c if c.is_alphabetic() && (c as u32) < 0x1_0000 => out.push(c),
            _ => return refuse(&format!("the character `{c}`")),
        }
    }
    Ok(out)
}

/// `_x` followed by four hex digits and `_`: XmlConvert would encode its underscore.
fn has_escape_shape(s: &str) -> bool {
    let b = s.as_bytes();
    (0..b.len().saturating_sub(6)).any(|i| {
        b[i] == b'_' && b[i + 1] == b'x' && b[i + 2..i + 6].iter().all(u8::is_ascii_hexdigit) && b[i + 6] == b'_'
    })
}

/// The `DataType` of a column on a table field of type `ty` (WordXmlPart_TypesA_Extended, WordXmlPart_TypesB_Extended: an Enum
/// field is `Option`, a Guid `GUID`, a Blob `BLOB`; WordXmlPart_TypesC_Extended: a RecordId `RecordID`). Other types give `None`.
fn field_data_type(ty: &RtType) -> Option<&'static str> {
    Some(match ty {
        RtType::Code(_) => "Code",
        RtType::Text(_) => "Text",
        RtType::Decimal => "Decimal",
        RtType::Integer => "Integer",
        RtType::BigInteger => "BigInteger",
        RtType::Boolean => "Boolean",
        RtType::Date => "Date",
        RtType::Time => "Time",
        RtType::DateTime => "DateTime",
        RtType::Duration => "Duration",
        RtType::DateFormula => "DateFormula",
        RtType::Guid => "GUID",
        RtType::Option(_) | RtType::Enum(_) => "Option",
        RtType::Library(LibraryKind::Media) => "Media",
        RtType::Library(LibraryKind::Blob) => "BLOB",
        RtType::Library(LibraryKind::RecordId) => "RecordID",
        _ => return None,
    })
}

/// The `DataType` of a column bound to an element of a global array of type `ty`: the name of the type as for a field
/// (corpus PR 1336, reports/layouts-saveas/WordXmlPartEmptyItemsOrderArrays.al WordXmlPart_Shape_ArraysA_Extended and
/// _ArraysB_Extended, al-runner issue 7381: Text, Text[30], Code, Integer, Decimal, Boolean, Date, Time, DateTime, BigInteger,
/// Duration, Guid `GUID`, Option and Enum `Option`, a two-dimensional Text array and an index held in a variable `Text`).
/// Arrays of other types give `None`.
fn array_element_data_type(ty: &RtType) -> Option<&'static str> {
    match ty {
        RtType::Text(_)
        | RtType::Code(_)
        | RtType::Integer
        | RtType::Decimal
        | RtType::Boolean
        | RtType::Date
        | RtType::Time
        | RtType::DateTime
        | RtType::BigInteger
        | RtType::Duration
        | RtType::Guid
        | RtType::Option(_)
        | RtType::Enum(_) => field_data_type(ty),
        _ => None,
    }
}

/// One column as the part lists it.
struct PartColumn {
    tag: String,
    id_name: String,
    /// `None`: the DataType of its source is not pinned (refused only with the flag).
    data_type: Option<&'static str>,
    blob: bool,
}

/// One label of the `Labels` element: a label of the report or an extension, or the caption of an IncludeCaption column.
struct PartLabel {
    tag: String,
    id_name: String,
}

/// Sorts `v` by tag, the order BC lists columns and labels in: case-insensitive, an underscore before a digit, a digit before
/// a letter, a name before the longer names it starts (WordXmlPart_Order_Plain: `A_B`, `AB`, `Alpha`, `beta`, `Zed`, `zed2`;
/// labels `A_Label`, `bLow`, `Bz`, `zed2Caption`; corpus PR 1336, WordXmlPartEmptyItemsOrderArrays.al
/// WordXmlPart_Shape_NameOrder_Plain, al-runner issue 7367: `A_ A1 AA aB`, `ShipTo_Address ShipTo8Address ShipToAddress
/// ShipToAddress_ ShipToAddress_Lbl ShipToAddress1 ShipToAddress10 ShipToAddress2 SHIPTOADDRESS3`, labels `Lbl_Text Lbl8Text
/// lbl9Text LblAText`). Two names that first differ in a character that is not ASCII are refused: their order is not pinned.
fn sort_by_tag<T>(v: &mut [T], tag: impl Fn(&T) -> &str) -> Result<(), Signal> {
    // A tag holds letters, digits and underscores (`xml_tag`); the underscore sorts first.
    let key = |t: &str| -> String {
        t.to_lowercase()
            .chars()
            .map(|c| if c == '_' { '\u{1}' } else { c })
            .collect()
    };
    v.sort_by_cached_key(|x| key(tag(x)));
    for w in v.windows(2) {
        let (a, b) = (tag(&w[0]).to_lowercase(), tag(&w[1]).to_lowercase());
        let Some((x, y)) = a.chars().zip(b.chars()).find(|(x, y)| x != y) else {
            continue;
        };
        if !x.is_ascii() || !y.is_ascii() {
            return unsupported(format!(
                "Report.WordXmlPart: the order of `{}` and `{}` (not pinned)",
                tag(&w[0]),
                tag(&w[1])
            ));
        }
    }
    Ok(())
}

impl Session {
    pub(super) fn report_word_xml_part(
        &mut self,
        id: al_ir::BuiltinId,
        receiver: Option<Value>,
        args: &[Value],
    ) -> Result<Value, Signal> {
        let static_form = id == ids::REPORT_WORDXMLPART_2;
        let (flag_at, schema) = if static_form {
            if let Some(refused) = self.word_xml_part_missing(args.first())? {
                return Err(refused);
            }
            (1, self.report_by_number(id, args.first())?.1)
        } else {
            let inst = self
                .report_instance(receiver, super::quit_in_init::InitQuit::Refuses)?
                .0;
            (0, self.report_schema(&inst)?)
        };
        let extended = match args.get(flag_at) {
            None => false,
            Some(Value::Boolean(b)) => *b,
            Some(other) => return unsupported(format!("{id} with the flag {}", other.describe())),
        };
        let text = self.word_xml_part_text(&schema, extended)?;
        Ok(Value::Text(text.into()))
    }

    /// The error of the static form for an id no report has: AL:MetadataNotFound from a try function
    /// (reports/request-page/P91Static.al:167-168, ids 0 and 71999); elsewhere which object the text names is not pinned.
    fn word_xml_part_missing(&mut self, arg: Option<&Value>) -> Result<Option<Signal>, Signal> {
        let known = match arg {
            Some(Value::Integer(n)) => self.engine.program.objects.heads().iter().any(|o| {
                (o.is_kind(al_ir::ObjKind::Report) || o.is_shell_of(al_ir::ObjKind::Report))
                    && o.id == u32::try_from(*n).ok()
            }),
            _ => true,
        };
        if known {
            return Ok(None);
        }
        let n = match arg {
            Some(Value::Integer(n)) => i64::from(*n),
            _ => 0,
        };
        if let Some(why) = self.missing_may_be_skipped("Report", n) {
            return unsupported(why);
        }
        if self.tx.try_depth == 0 {
            return unsupported(
                "Report.WordXmlPart of an id no report has, called outside a try function (only from a try function is it pinned)",
            );
        }
        Ok(Some(self.missing_object("Report", n)))
    }

    fn word_xml_part_text(&self, schema: &Schema, extended: bool) -> Result<String, Signal> {
        let engine = self.engine.clone();
        let report = schema.report();
        let tree = ItemTree {
            report,
            exts: report
                .extensions
                .iter()
                .map(|&e| engine.program.objects[e].report_ext.as_ref())
                .collect(),
        };
        let number = self.object_id_of(schema.object as u32);
        let ns = if extended { NS_EXTENDED } else { NS_PLAIN };
        let mut out = String::from("<?xml version=\"1.0\" encoding=\"utf-16\"?>\r\n");
        out.push_str(&format!(
            "<NavWordReportXmlPart xmlns=\"{ns}/reports/{}/{number}/\">\r\n",
            xml_tag(schema.name())?
        ));
        out.push_str(&self.word_xml_part_report_information());
        let labels = self.word_xml_part_labels(schema.object, &tree)?;
        if !labels.is_empty() {
            out.push_str("  <Labels>\r\n");
            for l in &labels {
                let attrs = if extended {
                    format!(
                        " ElementType=\"Parameter\" ElementId=\"{}\" DataType=\"String\"",
                        element_id(&l.id_name)?
                    )
                } else {
                    String::new()
                };
                out.push_str(&format!("    <{t}{attrs}>{t}</{t}>\r\n", t = l.tag));
            }
            out.push_str("  </Labels>\r\n");
        }
        if extended {
            out.push_str("  <Tooltips />\r\n");
        }
        let roots: Vec<ItemRef> = (0..report.items.len())
            .filter(|&i| report.items[i].parent.is_none())
            .map(ItemRef::Base)
            .collect();
        self.word_xml_part_items(schema.object, &tree, &roots, 1, extended, &mut out)?;
        out.push_str("</NavWordReportXmlPart>");
        Ok(out)
    }

    /// The labels of the report and its extensions and the captions of the IncludeCaption columns of every item, sorted
    /// (WordXmlPart_Labels_*, WordXmlPart_TwoExtensions_*: `BFlagCaption` of an extension column, `ExtLabel` of an extension).
    fn word_xml_part_labels(&self, object: usize, tree: &ItemTree<'_>) -> Result<Vec<PartLabel>, Signal> {
        let mut out = Vec::new();
        let declared = tree
            .report
            .labels
            .iter()
            .chain(tree.exts.iter().flatten().flat_map(|x| x.labels.iter()));
        for (name, _) in declared {
            out.push(PartLabel {
                tag: xml_tag(name)?,
                id_name: name.clone(),
            });
        }
        let mut stack: Vec<ItemRef> = (0..tree.report.items.len())
            .filter(|&i| tree.report.items[i].parent.is_none())
            .map(ItemRef::Base)
            .collect();
        while let Some(r) = stack.pop() {
            for (_, c) in self.word_xml_part_item_columns(object, tree, r) {
                if c.include_caption {
                    out.push(PartLabel {
                        tag: xml_tag(&format!("{}Caption", c.name))?,
                        id_name: c.name.clone(),
                    });
                }
            }
            stack.extend(tree.children(r));
        }
        sort_by_tag(&mut out, |l| &l.tag)?;
        Ok(out)
    }

    /// The columns of item `r` with the object that declares each: its own, then for an item of the report those every
    /// extension adds to it with `add(Item)`.
    fn word_xml_part_item_columns<'t>(
        &self,
        object: usize,
        tree: &ItemTree<'t>,
        r: ItemRef,
    ) -> Vec<(usize, &'t ir::ReportColumn)> {
        let owner = match r {
            ItemRef::Base(_) => object,
            ItemRef::Added(k, _) => tree.report.extensions[k],
        };
        let mut out: Vec<(usize, &ir::ReportColumn)> = tree.item(r).columns.iter().map(|c| (owner, c)).collect();
        if let ItemRef::Base(i) = r {
            for (k, x) in tree.exts.iter().enumerate() {
                let Some(x) = x else { continue };
                for add in x.adds.iter().filter(|a| a.item == i) {
                    out.extend(add.columns.iter().map(|c| (tree.report.extensions[k], c)));
                }
            }
        }
        out
    }

    fn word_xml_part_items(
        &self,
        object: usize,
        tree: &ItemTree<'_>,
        items: &[ItemRef],
        depth: usize,
        extended: bool,
        out: &mut String,
    ) -> Result<(), Signal> {
        // Sibling data items keep the order the report runs them in (WordXmlPart_Order_Plain: `Zulu` before `Alpha2`, `Yank`
        // before `Bravo`).
        let pad = "  ".repeat(depth);
        for &r in items {
            let item = tree.item(r);
            let tag = xml_tag(&item.name)?;
            let columns = self.word_xml_part_columns(object, tree, r)?;
            let children = tree.children(r);
            let attrs = if extended {
                format!(" ElementType=\"DataItem\" ElementId=\"{}\"", element_id(&item.name)?)
            } else {
                String::new()
            };
            // A data item with no column and no nested item is an empty element where the report has it (corpus PR 1336,
            // WordXmlPartEmptyItemsOrderArrays.al WordXmlPart_Shape_EmptyAlone_*, _EmptyAfter_*, _EmptyMixed_*: alone, after
            // and before items with columns, nested, and the only child of an item; al-runner issue 7383).
            if columns.is_empty() && children.is_empty() {
                out.push_str(&format!("{pad}<{tag}{attrs} />\r\n"));
                continue;
            }
            out.push_str(&format!("{pad}<{tag}{attrs}>\r\n"));
            for c in &columns {
                let attrs = if extended {
                    let Some(ty) = c.data_type else {
                        return unsupported(format!(
                            "Report.WordXmlPart(.., true) of the column `{}`: the DataType of its source is not pinned",
                            c.id_name
                        ));
                    };
                    format!(
                        " ElementType=\"Column\" ElementId=\"{}\" DataType=\"{ty}\"",
                        element_id(&c.id_name)?
                    )
                } else {
                    String::new()
                };
                if c.blob {
                    out.push_str(&format!("{pad}  <{}{attrs} />\r\n", c.tag));
                } else {
                    out.push_str(&format!("{pad}  <{t}{attrs}>{t}</{t}>\r\n", t = c.tag));
                }
            }
            self.word_xml_part_items(object, tree, &children, depth + 1, extended, out)?;
            out.push_str(&format!("{pad}</{tag}>\r\n"));
        }
        Ok(())
    }

    /// The columns of item `r`, sorted (WordXmlPart_TypesA_*: `ZCode` .. `RPrice` are listed `RPrice` .. `ZCode`; extension
    /// columns sort in with the report's, WordXmlPart_TwoExtensions_*).
    fn word_xml_part_columns(&self, object: usize, tree: &ItemTree<'_>, r: ItemRef) -> Result<Vec<PartColumn>, Signal> {
        let table = tree
            .item(r)
            .table
            .and_then(|t| self.engine.tables.get(t).cloned().flatten());
        let mut out = Vec::new();
        for (owner, c) in self.word_xml_part_item_columns(object, tree, r) {
            let field = c.field.or(c.item_field).and_then(|p| table.as_deref()?.fields.get(p));
            let data_type = match field {
                Some(f) => field_data_type(&f.ty),
                None => self.word_xml_part_expression_type(owner, c),
            };
            out.push(PartColumn {
                tag: xml_tag(&c.name)?,
                id_name: c.name.clone(),
                data_type,
                blob: field.is_some_and(|f| f.ty == RtType::Library(LibraryKind::Blob)),
            });
        }
        sort_by_tag(&mut out, |c| &c.tag)?;
        Ok(out)
    }

    /// The `DataType` of a column whose source is not a field of its item's table (WordXmlPart_TypesB_Extended,
    /// WordXmlPart_TypesC_Extended): a text literal and a one-argument `Format` are `String`, an Integer literal `Integer`, a
    /// Label `TextConst`, a procedure of the object that answers a Text `Text`, a global of type Text, Code, Integer, Boolean,
    /// Date or Decimal the name of its type, a field read through a data item (`Rows.Amount`) the type of the field. Other
    /// sources give `None`.
    fn word_xml_part_expression_type(&self, object: usize, c: &ir::ReportColumn) -> Option<&'static str> {
        let o = &self.engine.program.objects[object];
        let routine = o.routines.get(c.routine?)?;
        let [ir::Stmt::Exit(Some(e))] = routine.body.stmts().as_slice() else {
            return None;
        };
        match e {
            ir::Expr::Const(ir::Const::Text(_)) => Some("String"),
            ir::Expr::Const(ir::Const::Integer(_)) => Some("Integer"),
            ir::Expr::Builtin {
                id: ids::SYSTEM_TRANSLATEDTEXT_1,
                ..
            } => Some("TextConst"),
            ir::Expr::Builtin {
                id: ids::SYSTEM_FORMAT_1,
                args,
                ..
            } if args.len() == 1 => Some("String"),
            ir::Expr::Load(place @ ir::Place::Global(_)) => {
                match crate::data_file_types::place_type(self, object, place)? {
                    ty @ (RtType::Text(_)
                    | RtType::Code(_)
                    | RtType::Integer
                    | RtType::Boolean
                    | RtType::Date
                    | RtType::Decimal) => field_data_type(&ty),
                    _ => None,
                }
            }
            ir::Expr::Load(place @ ir::Place::Field(..)) => {
                field_data_type(&crate::data_file_types::place_type(self, object, place)?)
            }
            ir::Expr::Load(ir::Place::Index(..)) => array_element_data_type(&self.global_array_element(object, e)?),
            // Over array elements (WordXmlPart_Shape_ArraysB_Extended): a Text element and a text literal joined by `+` are
            // `String`, two Integer elements added `Integer`.
            ir::Expr::Binary(ir::BinOp::Add, l, r) => {
                match (
                    self.global_array_element(object, l),
                    self.global_array_element(object, r),
                    r.as_ref(),
                ) {
                    (Some(RtType::Text(_)), None, ir::Expr::Const(ir::Const::Text(_))) => Some("String"),
                    (Some(RtType::Integer), Some(RtType::Integer), _) => Some("Integer"),
                    _ => None,
                }
            }
            ir::Expr::Call {
                object: callee,
                routine,
                instance: None,
                ..
            } if *callee == object => match o.routines.get(*routine)?.ret.as_ref()? {
                RtType::Text(_) => Some("Text"),
                _ => None,
            },
            _ => None,
        }
    }

    /// The type of `e` when it reads one element of a global array of object `object`.
    fn global_array_element(&self, object: usize, e: &ir::Expr) -> Option<RtType> {
        match e {
            ir::Expr::Load(place @ ir::Place::Index(base, _)) if matches!(base.as_ref(), ir::Place::Global(_)) => {
                crate::data_file_types::place_type(self, object, place)
            }
            _ => None,
        }
    }

    /// The `BCReportInformation` element: each field's text is its display name, the same with and without the flag
    /// (WordXmlPart_ReportInformation_Plain, _ReportInformationTail_Plain, _ReportInformationTail2_Plain,
    /// _ReportInformation_ExtendedSameAsPlain). BC 29 adds `ReportCaption` and the `CompanyMetadata` block, as in a dataset
    /// (`Gate::ReportSaveAsXmlHeaderHasCompanyBlock`).
    fn word_xml_part_report_information(&self) -> String {
        let company_block = self.has(al_version::Gate::ReportSaveAsXmlHeaderHasCompanyBlock);
        let mut out = String::from("  <BCReportInformation>\r\n");
        let mut add = |depth: usize, tag: &str, text: Option<&str>| {
            let pad = "  ".repeat(depth);
            match text {
                Some(t) => out.push_str(&format!("{pad}<{tag}>{t}</{tag}>\r\n")),
                None => out.push_str(&format!("{pad}<{tag}>\r\n")),
            }
        };
        add(2, "ReportMetadata", None);
        for (tag, text) in REPORT_METADATA {
            if *tag != "ReportCaption" || company_block {
                add(3, tag, Some(text));
            }
        }
        add(2, "/ReportMetadata", None);
        add(2, "ReportRequest", None);
        for (tag, text) in REPORT_REQUEST {
            add(3, tag, Some(text));
        }
        add(3, "DateTimeValues", None);
        for (tag, text) in DATE_TIME_VALUES {
            add(4, tag, Some(text));
        }
        add(3, "/DateTimeValues", None);
        add(2, "/ReportRequest", None);
        if company_block {
            add(2, "CompanyMetadata", None);
            add(3, "CompanyName", Some("Company Name"));
            add(3, "CompanyDisplayName", Some("Company Display Name"));
            add(3, "CompanyAddressLines", None);
            for n in 1..=8 {
                add(
                    4,
                    &format!("CompanyAddressLine{n}"),
                    Some(&format!("Company Address Line {n}")),
                );
            }
            add(3, "/CompanyAddressLines", None);
            for (tag, text) in COMPANY_METADATA {
                add(3, tag, Some(text));
            }
            add(2, "/CompanyMetadata", None);
        }
        out.push_str("  </BCReportInformation>\r\n");
        out
    }
}

/// The elements of `ReportMetadata` and their texts (`ReportCaption` only from BC 29).
const REPORT_METADATA: &[(&str, &str)] = &[
    ("ExtensionId", "Extension ID"),
    ("ExtensionName", "Extension Name"),
    ("ExtensionPublisher", "Extension Publisher"),
    ("ExtensionVersion", "Extension Version"),
    ("ReportId", "Report ID"),
    ("ReportName", "Report Name"),
    ("ReportCaption", "Report Caption"),
    ("AboutThisReportTitle", "About This Report Title"),
    ("AboutThisReportText", "About This Report Text"),
    ("ReportHelpLink", "Report Help Link"),
];

/// The elements of `ReportRequest` before `DateTimeValues`.
const REPORT_REQUEST: &[(&str, &str)] = &[
    ("TenantEntraId", "Tenant Entra Id"),
    ("EnvironmentName", "Environment Name"),
    ("EnvironmentType", "Environment Type"),
    ("CompanyName", "Company Name"),
    ("CompanyDisplayName", "Company Display Name"),
    ("CompanyId", "Company ID"),
    ("UserName", "User Name"),
    ("DateAndTime", "Date and Time"),
    ("Language", "Language"),
    ("FormatRegion", "Format Region"),
];

const DATE_TIME_VALUES: &[(&str, &str)] = &[
    ("Year", "Year"),
    ("MonthNumber", "Month Number"),
    ("DayNumber", "Day Number"),
    ("Hour", "Hour"),
    ("Minute", "Minute"),
];

/// The elements of `CompanyMetadata` after the address lines (BC 29).
const COMPANY_METADATA: &[(&str, &str)] = &[
    ("CompanyPhone", "Company Phone"),
    ("CompanyPhoneCaption", "Company Phone Caption"),
    ("CompanyFaxNo", "Company Fax No."),
    ("CompanyFaxNoCaption", "Company Fax No. Caption"),
    ("CompanyEmail", "Company Email"),
    ("CompanyEmailCaption", "Company Email Caption"),
    ("CompanyHomePage", "Company Home Page"),
    ("CompanyHomePageCaption", "Company Home Page Caption"),
    ("CompanyLogo", "Company Logo"),
    ("CompanyVATRegistrationNo", "Company VAT Registration No."),
    (
        "CompanyVATRegistrationNoCaption",
        "Company VAT Registration No. Caption",
    ),
    ("CompanyRegistrationNo", "Company Registration No."),
    ("CompanyRegistrationNoCaption", "Company Registration No. Caption"),
    ("CompanyBankName", "Company Bank Name"),
    ("CompanyBankNameCaption", "Company Bank Name Caption"),
    ("CompanyBankAccountNo", "Company Bank Account No."),
    ("CompanyBankAccountNoCaption", "Company Bank Account No. Caption"),
    ("CompanyBankBranchNo", "Company Bank Branch No."),
    ("CompanyBankBranchNoCaption", "Company Bank Branch No. Caption"),
    ("CompanyIBAN", "Company IBAN"),
    ("CompanyIBANCaption", "Company IBAN Caption"),
    ("CompanyBankSWIFT", "Company Bank SWIFT"),
    ("CompanyBankSWIFTCaption", "Company Bank SWIFT Caption"),
    ("CompanyGiroNo", "Company Giro No."),
    ("CompanyGiroNoCaption", "Company Giro No. Caption"),
];
