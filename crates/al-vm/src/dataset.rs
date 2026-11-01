//! The dataset XML that `Report.SaveAs(.., ReportFormat::Xml, ..)` writes: a header, then one
//! `DataItem` element per row of every data item, nested the way the data items are.
//!
//! The shape is pinned by the probe in handlers/TestReportSaveAsDatasetContracts.al (written
//! against a real BC on Linux, en-US): the byte order mark and declaration, the order of the
//! header elements, the `DataItems` block to the character, and the escaping of text. What no
//! test pins is listed in docs/probes-needed.md ("Report.SaveAs dataset").
//!
//! Core code: no OS access (ADR-006). The writer builds a string; the caller writes the bytes.

use al_builtins::native::Signal;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

/// The name of a data item or column as the dataset spells it. A name made of letters, digits
/// and underscores is written as it is (handlers/TestReportSaveAsDatasetContracts.al: `P`,
/// `PNo`). The one other shape the corpus shows is the data item `"Change Log Setup (Table)"`,
/// which the dataset calls `Change_Log_Setup_Table` (handlers/TestReportNestedLinkImplicitParent.al:98);
/// a name with spaces and parentheses is written the way both readings of that one example agree
/// on, and any other name is refused.
pub(crate) fn dataset_name(name: &str) -> Result<String, Signal> {
    if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Ok(name.to_string());
    }
    // Dots, slashes and spaces: a run of them is one underscore; a single trailing dot is dropped; a single leading dot is an
    // underscore (ReportDatasetNames.al:SaveAsXml_DataItemNames_DotsAndSlashes `A. B`, `C/D`, `G..H`, `I./J`, `K  L`, `M.`, `.N`,
    // `O. . P`, `Q//R`). A name that also has a parenthesis is read by the example below.
    if !name.contains(['(', ')']) && name.contains(['.', '/', ' ']) {
        let separator = |c: char| matches!(c, '.' | '/' | ' ');
        let body = name
            .strip_suffix('.')
            .filter(|b| !b.ends_with(separator))
            .unwrap_or(name);
        let mut out = String::new();
        let mut in_run = false;
        for c in body.chars() {
            if separator(c) {
                if !in_run {
                    out.push('_');
                }
                in_run = true;
            } else if c.is_alphanumeric() || c == '_' {
                out.push(c);
                in_run = false;
            } else {
                return unsupported(format!(
                    "the dataset name of `{name}` (only letters, digits, underscores, dots, slashes, spaces and parentheses are pinned)"
                ));
            }
        }
        // A run at the end that is not one trailing dot, or at the start that is not one dot or a space run inside, is not pinned.
        if out.ends_with('_') && !name.ends_with('_') || name.starts_with([' ', '/']) {
            return unsupported(format!(
                "the dataset name of `{name}` (a trailing run other than one dot, or a leading space or slash, is not pinned)"
            ));
        }
        return Ok(out);
    }
    let simple = name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == ' ' || c == '(' || c == ')');
    if !simple || name.contains("__") || name.contains("  ") {
        return unsupported(format!(
            "the dataset name of `{name}` (only letters, digits, underscores, spaces and parentheses are pinned)"
        ));
    }
    // Reading one: every character that is not a letter or digit becomes an underscore, runs of
    // underscores collapse, the ends are trimmed. Reading two: a space becomes an underscore and a
    // parenthesis disappears. They agree on `Change Log Setup (Table)`.
    let mut one = String::new();
    for c in name.chars() {
        let c = if c.is_alphanumeric() { c } else { '_' };
        if !(c == '_' && one.ends_with('_')) {
            one.push(c);
        }
    }
    let one = one.trim_matches('_').to_string();
    let two: String = name
        .chars()
        .filter(|c| *c != '(' && *c != ')')
        .map(|c| if c == ' ' { '_' } else { c })
        .collect();
    if one != two || one.is_empty() {
        return unsupported(format!(
            "the dataset name of `{name}` (the pinned example does not say how this one is spelled)"
        ));
    }
    Ok(one)
}

/// Text as an XML element writes it: `&`, `<` and `>` become entities
/// (handlers/TestReportSaveAsDatasetContracts.al: `one <&>` is `one &lt;&amp;&gt;`). In an
/// attribute value a quote does too. A control character is refused: how the writer treats it is
/// not pinned.
pub(crate) fn escape(text: &str) -> Result<String, Signal> {
    escape_as(text, false)
}

fn escape_as(text: &str, attribute: bool) -> Result<String, Signal> {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attribute => out.push_str("&quot;"),
            c if c.is_control() => {
                return unsupported(format!(
                    "text with the control character U+{:04X} in a dataset (its escaping is not pinned)",
                    u32::from(c)
                ));
            }
            c => out.push(c),
        }
    }
    Ok(out)
}

/// What the header carries. The values of `Extension*`, `Tenant*`, `Environment*`, the company,
/// the user and the clock depend on the app, the company, the user and the time; the probe does
/// not compare them (only `ReportName`, `ReportId`, `CompanyName`, `AboutThis*` and
/// `ReportHelpLink` are read by tests).
pub(crate) struct Header {
    pub extension_id: String,
    pub extension_name: String,
    pub extension_publisher: String,
    pub extension_version: String,
    pub report_id: String,
    pub report_name: String,
    pub report_caption: String,
    pub about_title: String,
    pub about_text: String,
    pub help_link: String,
    pub company_name: String,
    pub company_id: String,
    pub user_name: String,
    /// (year, month, day, hour, minute) of the clock.
    pub now: (i32, u32, u32, u32, u32),
    /// BC 29 adds `ReportCaption` and the company information block
    /// (`al_version::Gate::ReportSaveAsXmlHeaderHasCompanyBlock`).
    pub company_block: bool,
    /// The company address lines, at most eight; the elements after them are empty.
    pub company_lines: Vec<String>,
    /// The values of the other company elements (`COMPANY_ELEMENTS`) that are not empty, by element name.
    pub company_values: Vec<(&'static str, String)>,
}

/// The company elements after the address lines, in order, with the caption element each one has (en-US), and the
/// field of the Company Information record that holds the value. The captions and the values of the CRONUS company:
/// misc/lt-probes-needed-2/PnBReportSaveAs.al:568-592 (`0666-666-6666` is its Phone No., `World Wide Bank` its Bank Name).
/// `CompanyLogo` (the Picture field, base64) has no caption.
pub(crate) const COMPANY_ELEMENTS: [(&str, Option<&str>); 13] = [
    ("CompanyPhone", Some("Phone No.")),
    ("CompanyFaxNo", Some("Fax No.")),
    ("CompanyEmail", Some("Email")),
    ("CompanyHomePage", Some("Home Page")),
    ("CompanyLogo", None),
    ("CompanyVATRegistrationNo", Some("VAT Registration No.")),
    ("CompanyRegistrationNo", Some("Registration No.")),
    ("CompanyBankName", Some("Bank Name")),
    ("CompanyBankAccountNo", Some("Bank Account No.")),
    ("CompanyBankBranchNo", Some("Bank Branch No.")),
    ("CompanyIBAN", Some("IBAN")),
    ("CompanyBankSWIFT", Some("SWIFT Code")),
    ("CompanyGiroNo", Some("Giro No.")),
];

/// The field of the Company Information record whose value an element of `COMPANY_ELEMENTS` writes.
pub(crate) const COMPANY_FIELDS: [(&str, &str); 13] = [
    ("CompanyPhone", "Phone No."),
    ("CompanyFaxNo", "Fax No."),
    ("CompanyEmail", "E-Mail"),
    ("CompanyHomePage", "Home Page"),
    ("CompanyLogo", "Picture"),
    ("CompanyVATRegistrationNo", "VAT Registration No."),
    ("CompanyRegistrationNo", "Registration No."),
    ("CompanyBankName", "Bank Name"),
    ("CompanyBankAccountNo", "Bank Account No."),
    ("CompanyBankBranchNo", "Bank Branch No."),
    ("CompanyIBAN", "IBAN"),
    ("CompanyBankSWIFT", "SWIFT Code"),
    ("CompanyGiroNo", "Giro No."),
];

/// The language and format region of the session. Only en-US is pinned.
pub(crate) const LANGUAGE: &str = "en-US";

fn element(out: &mut String, indent: usize, name: &str, value: &str) -> Result<(), Signal> {
    // An empty value is `<Name></Name>` on one line: the probe lists `ReportHelpLink`, `TenantEntraId`
    // and others among the elements with a value although a test run states none.
    out.push_str(&" ".repeat(indent));
    out.push_str(&format!("<{name}>{}</{name}>\r\n", escape(value)?));
    Ok(())
}

impl Header {
    /// The declaration, the root element and the information block, up to the data items.
    /// Elements: handlers/TestReportSaveAsDatasetContracts.al:SaveAs_Xml_WritesAByteOrderMarkDeclarationAndHeader;
    /// `BCReportInformation/ReportMetadata/...` for the report's own: handlers/TestReportDatasetHelpLink.al.
    /// `ReportRequest` and, from BC 29, `CompanyMetadata` follow `ReportMetadata` under `BCReportInformation`.
    fn write(&self, out: &mut String, language: &str, region: &str) -> Result<(), Signal> {
        out.push_str("<?xml version=\"1.0\" standalone=\"yes\"?>\r\n");
        out.push_str(&format!(
            "<ReportDataSet name=\"{}\" id=\"{}\" language=\"{language}\" formatRegion=\"{region}\">\r\n",
            escape_as(&self.report_name, true)?,
            escape_as(&self.report_id, true)?
        ));
        out.push_str("  <BCReportInformation>\r\n");
        out.push_str("    <ReportMetadata>\r\n");
        element(out, 6, "ExtensionId", &self.extension_id)?;
        element(out, 6, "ExtensionName", &self.extension_name)?;
        element(out, 6, "ExtensionPublisher", &self.extension_publisher)?;
        element(out, 6, "ExtensionVersion", &self.extension_version)?;
        element(out, 6, "ReportId", &self.report_id)?;
        element(out, 6, "ReportName", &self.report_name)?;
        if self.company_block {
            element(out, 6, "ReportCaption", &self.report_caption)?;
        }
        element(out, 6, "AboutThisReportTitle", &self.about_title)?;
        element(out, 6, "AboutThisReportText", &self.about_text)?;
        element(out, 6, "ReportHelpLink", &self.help_link)?;
        out.push_str("    </ReportMetadata>\r\n");
        // The request information is one element, its children two spaces further in (report/lt-report-dataset-header/
        // ReportDatasetHeaderRaw.al:Lines_Window2, `4|<ReportRequest>` and `6|<TenantEntraId>`). Tenant and environment: nothing in the
        // corpus says what a test run reports.
        out.push_str("    <ReportRequest>\r\n");
        element(out, 6, "TenantEntraId", "")?;
        element(out, 6, "EnvironmentName", "")?;
        element(out, 6, "EnvironmentType", "")?;
        element(out, 6, "CompanyName", &self.company_name)?;
        element(out, 6, "CompanyDisplayName", &self.company_name)?;
        element(out, 6, "CompanyId", &self.company_id)?;
        element(out, 6, "UserName", &self.user_name)?;
        let (y, mo, d, h, mi) = self.now;
        element(
            out,
            6,
            "DateAndTime",
            &format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:00"),
        )?;
        element(out, 6, "Language", language)?;
        element(out, 6, "FormatRegion", region)?;
        // The clock parts sit in `DateTimeValues`, two spaces further in, and `ReportRequest` closes after it
        // (misc/lt-probes-needed-2/PnBReportSaveAs.al:542-549, report/W3ReportDataset.al:524-534 on BC 29; on BC 27 and
        // 28 report/W3ReportDataset.al:256 counts 46 line ends, the two lines of this element more than a flat list).
        out.push_str("      <DateTimeValues>\r\n");
        element(out, 8, "Year", &y.to_string())?;
        element(out, 8, "MonthNumber", &mo.to_string())?;
        element(out, 8, "DayNumber", &d.to_string())?;
        element(out, 8, "Hour", &h.to_string())?;
        element(out, 8, "Minute", &mi.to_string())?;
        out.push_str("      </DateTimeValues>\r\n");
        out.push_str("    </ReportRequest>\r\n");
        if self.company_block {
            // BC 29: the company information is a sibling of `ReportRequest`, the address lines one element of their
            // own (misc/lt-probes-needed-2/PnBReportSaveAs.al:558-596, report/W3ReportDataset.al:532-590). The captions
            // are the texts those probes read (en-US); the values are the fields of the Company Information record.
            out.push_str("    <CompanyMetadata>\r\n");
            element(out, 6, "CompanyName", &self.company_name)?;
            element(out, 6, "CompanyDisplayName", &self.company_name)?;
            out.push_str("      <CompanyAddressLines>\r\n");
            for n in 1..=8 {
                element(
                    out,
                    8,
                    &format!("CompanyAddressLine{n}"),
                    self.company_lines.get(n - 1).map_or("", String::as_str),
                )?;
            }
            out.push_str("      </CompanyAddressLines>\r\n");
            for (name, caption) in COMPANY_ELEMENTS {
                let value = self
                    .company_values
                    .iter()
                    .find(|(n, _)| *n == name)
                    .map_or("", |(_, v)| v.as_str());
                element(out, 6, name, value)?;
                if let Some(caption) = caption {
                    element(out, 6, &format!("{name}Caption"), caption)?;
                }
            }
            out.push_str("    </CompanyMetadata>\r\n");
        }
        out.push_str("  </BCReportInformation>\r\n");
        Ok(())
    }
}

/// The attribute `decimalformatter` of a Decimal column. A plain Decimal field is pinned
/// (handlers/TestReportSaveAsDatasetContracts.al: `#,##0.00`); a field or column that states
/// `DecimalPlaces` or `AutoFormatType` may change it, which no test pins
/// (docs/probes-needed.md).
pub(crate) const DECIMAL_FORMATTER: &str = "#,##0.00";

/// The `decimalformatter` of a Decimal column. `places` is the `DecimalPlaces` of the column, else
/// of the field, as written: `min : max`, or one number for both. The format is `#,##0`, then the
/// minimum number of zeros after the point and the further optional places as `#`
/// (handlers/TestReportSaveAsColumnDetailsContracts.al:UndefinedTimesGuidsHoursAndDecimalPlaces and
/// handlers/PWReportSaveAsColumns.al:DecimalPlaces_SingleNumberRangeAndZero): `2 : 4` gives
/// `#,##0.00##`, `0 : 0` gives `#,##0`, `1` gives `#,##0.0`, `0 : 5` gives `#,##0.#####`, `0 : 2`
/// gives `#,##0.##`, `1 : 1` gives `#,##0.0`, `3 : 3` gives `#,##0.000`
/// (handlers/TestReportSaveAsColumnTypesContracts.al:230-244). No property is the default.
pub(crate) fn decimal_formatter(places: Option<&str>) -> Result<String, Signal> {
    let Some(places) = places else {
        return Ok(DECIMAL_FORMATTER.to_string());
    };
    let bound = |t: &str| t.trim().parse::<usize>().ok();
    let (min, max) = match places.split_once(':') {
        Some((a, b)) => (bound(a), bound(b)),
        None => (bound(places), bound(places)),
    };
    match (min, max) {
        (Some(min), Some(max)) if min <= max => {
            let mut f = String::from("#,##0");
            if max > 0 {
                f.push('.');
                f.push_str(&"0".repeat(min));
                f.push_str(&"#".repeat(max - min));
            }
            Ok(f)
        }
        _ => unsupported(format!(
            "DecimalPlaces = {places} in a dataset (not a number or a range of two)"
        )),
    }
}

/// A Time as the dataset writes it: `h:mm:ss`, a plain space, `AM` or `PM`
/// (handlers/TestReportSaveAsColumnTypesContracts.al:12-14, 12:34:56 PM). The hour is not padded
/// (the culture's pattern; no single-digit hour is measured). Milliseconds are cut off, not rounded: a stored field shows the
/// value the SQL column rounded (`sqltime`, 13:30:00.999 is stored as 13:30:01), a column bound to a variable its own value
/// (errors/lt-ms-refusals-132/P132ReportTimeMs.al:181 P132_Dataset_Time_Milliseconds, Y38_P132_Dataset_Time_NearNextSecond,
/// Y38_P132_Dataset_Time_OtherMilliseconds, and the global-variable column of the Y38 tests: 23:59:59.999 is 11:59:59 PM).
pub(crate) fn time_text(ms_of_day: i64) -> Result<String, Signal> {
    let (h, m, s) = (ms_of_day / 3_600_000, ms_of_day / 60_000 % 60, ms_of_day / 1000 % 60);
    let h12 = if h % 12 == 0 { 12 } else { h % 12 };
    Ok(format!("{h12}:{m:02}:{s:02} {}", if h < 12 { "AM" } else { "PM" }))
}

/// A Date as the dataset writes it: `m/d/yyyy` (the same file, `3/5/2024`).
pub(crate) fn date_text(day: i32) -> String {
    let (y, m, d) = al_builtins::format::civil_from_day(day);
    format!("{m}/{d}/{y}")
}

/// One column of a row: its name, an attribute written after the name, and its text.
pub(crate) struct Column {
    pub name: String,
    pub attribute: Option<(&'static str, String)>,
    pub text: String,
    /// The XML Schema type of the column in a .NET DataSet (`TestRequestPage.SaveAsXml`), for the
    /// column types that probe pins: Integer, Text/Code, Decimal, Date, Boolean and Option.
    pub schema_type: Option<&'static str>,
    /// What the .NET DataSet writes for the value.
    pub data: DataFileText,
}

/// The value of a column in a SaveAsXml data file (al-language-onprem/report/lt-aq/AqSaveAsXml.al:247).
#[derive(Clone, Default)]
pub(crate) enum DataFileText {
    /// The text the dataset writes.
    #[default]
    Same,
    /// Another text (`true` for a Boolean, `2024-12-25T00:00:00+00:00` for a Date).
    Text(String),
    /// No element (a blank Date).
    Omit,
}

/// A dataset being written: the `DataItems` block, and who is open.
#[derive(Default)]
pub(crate) struct Dataset {
    body: String,
    /// For every open `DataItem`: whether its own `DataItems` element is open.
    open: Vec<bool>,
    /// For every open `DataItem`: whether its data item declares nested items (an empty `DataItems` is written).
    declares: Vec<bool>,
    root: bool,
    /// The report's `FormatRegion` property: the `formatRegion` of the top data items and the culture of the columns
    /// (report/lt-y-report-props/YReportPropsTests.al:34-148). `None`: the session's en-US.
    pub region: Option<(String, al_builtins::format::Region)>,
    /// The culture name of the run's language when it is not the session's en-US (`de-DE` for 1031: report/
    /// W3ReportObjectIdPreviewLanguage.al:870, :893), written in the header and on the top data items.
    pub language: Option<String>,
    /// An element written, whatever its kind.
    pub rows: usize,
    /// The `Labels` block: (column name + `Caption`, text), in the order the columns come.
    pub labels: Vec<(String, String)>,
    /// Why the Labels block cannot be written (its order is not pinned). Refused when the dataset is written, not when the run
    /// starts: a run that errors before it writes the dataset raises its own error.
    pub labels_unpinned: Option<String>,
    /// Why the dataset cannot be written in the language a trigger set (`reports.rs` `saveas_language_set`).
    pub culture_unpinned: Option<String>,
    /// Every row, in the order written, for a .NET DataSet (`finish_data_set`).
    flat: Vec<FlatRow>,
    /// For every open `DataItem`: the state to go back to when `PrintOnlyIfDetail` drops its row, and how many rows its nested items wrote.
    marks: Vec<Mark>,
}

/// A row of a data item for a .NET DataSet (`Dataset::finish_data_set`).
struct FlatRow {
    /// 0 for a top-level data item, one more for each level of nesting.
    depth: usize,
    item: String,
    /// Whether nested items run under its rows (the base report's and those an extension adds).
    has_children: bool,
    cols: Vec<FlatCol>,
    kind: FlatKind,
}

/// What an entry of `Dataset::flat` stands for.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FlatKind {
    /// A row the item wrote.
    Row,
    /// The item ran under the row above (its `OnPreDataItem` did not break) and read no row.
    NoRows,
    /// The item read rows and wrote none (Skip, Break in OnAfterGetRecord, PrintOnlyIfDetail).
    RowsLeftOut,
}

/// A data item as the report declares it, depth first in run order, with its columns and the .NET type of each where it is
/// known without a row (a table field of a pinned type): the schema of a SaveAsXml data file.
pub(crate) struct DeclaredItem {
    pub name: String,
    pub columns: Vec<(String, Option<&'static str>)>,
}

struct FlatCol {
    name: String,
    /// The XML Schema type, for the column types that are pinned.
    ty: Option<&'static str>,
    text: String,
    /// A Decimal column's `decimalformatter`.
    formatter: Option<String>,
    data: DataFileText,
}

/// The dataset as it was before a row began.
struct Mark {
    body: usize,
    flat: usize,
    rows: usize,
    root: bool,
    /// Whether the parent row's `DataItems` element was open.
    parent_open: Option<bool>,
    /// Rows written by the nested items so far.
    kids: usize,
}

impl Dataset {
    /// `DataItem` elements sit at 4 spaces, and four more for each level of nesting. Its
    /// `Columns` and `DataItems` elements sit two deeper, a `Column` four deeper.
    fn pad(&mut self, spaces: usize) {
        self.body.push_str(&" ".repeat(spaces));
    }

    /// Opens a row of a data item: `<DataItem ..>` and its `Columns`.
    pub(crate) fn begin(
        &mut self,
        name: &str,
        identity: Option<(u32, &str)>,
        columns: &[Column],
        declares_children: bool,
        has_children: bool,
    ) {
        let depth = self.open.len();
        self.marks.push(Mark {
            body: self.body.len(),
            flat: self.flat.len(),
            rows: self.rows,
            root: self.root,
            parent_open: self.open.last().copied(),
            kids: 0,
        });
        if depth > 0 {
            if let Some(parent) = self.marks.len().checked_sub(2).and_then(|i| self.marks.get_mut(i)) {
                parent.kids += 1;
            }
        }
        self.flat.push(FlatRow {
            depth,
            item: name.to_string(),
            has_children,
            kind: FlatKind::Row,
            cols: columns
                .iter()
                .map(|c| FlatCol {
                    name: c.name.clone(),
                    ty: c.schema_type,
                    text: c.text.clone(),
                    formatter: c
                        .attribute
                        .as_ref()
                        .filter(|(k, _)| *k == "decimalformatter")
                        .map(|(_, v)| v.clone()),
                    data: c.data.clone(),
                })
                .collect(),
        });
        if depth == 0 {
            if !self.root {
                self.root = true;
                self.body.push_str("  <DataItems>\r\n");
            }
        } else if !*self.open.last().expect("depth > 0") {
            *self.open.last_mut().expect("depth > 0") = true;
            self.pad(2 + 4 * depth);
            self.body.push_str("<DataItems>\r\n");
        }
        let at = 4 + 4 * depth;
        self.pad(at);
        self.body.push_str("<DataItem");
        if depth == 0 {
            let region = self.region.as_ref().map_or(LANGUAGE, |(r, _)| r.as_str());
            let language = self.language.as_deref().unwrap_or(LANGUAGE);
            self.body
                .push_str(&format!(" language=\"{language}\" formatRegion=\"{region}\""));
        }
        self.body.push_str(&format!(" name=\"{name}\""));
        // From BC 28 (`Gate::ReportSaveAsDataItemIdentity`): the data item's table id and a GUID.
        if let Some((table, guid)) = identity {
            self.body.push_str(&format!(" tableId=\"{table}\" systemId=\"{guid}\""));
        }
        self.body.push_str(">\r\n");
        self.pad(at + 2);
        self.body.push_str("<Columns>\r\n");
        for c in columns {
            self.pad(at + 4);
            self.body.push_str(&format!("<Column name=\"{}\"", c.name));
            if let Some((k, v)) = &c.attribute {
                self.body.push_str(&format!(" {k}=\"{v}\""));
            }
            self.body.push_str(&format!(">{}</Column>\r\n", c.text));
        }
        self.pad(at + 2);
        self.body.push_str("</Columns>\r\n");
        self.open.push(false);
        self.declares.push(declares_children);
        self.rows += 1;
    }

    /// Where the rows of a data item about to run will start (`item_done`).
    pub(crate) fn mark(&self) -> usize {
        self.flat.len()
    }

    /// A data item that started at `since` (`mark`) under the open row, its `OnPreDataItem` not broken, is done: when it wrote no
    /// row the data file needs to know whether it read none (`read_any` false) or left the rows it read out.
    pub(crate) fn item_done(&mut self, since: usize, read_any: bool) {
        let depth = self.open.len();
        if self.flat[since..]
            .iter()
            .any(|r| r.depth == depth && r.kind == FlatKind::Row)
        {
            return;
        }
        self.flat.push(FlatRow {
            depth,
            item: String::new(),
            has_children: false,
            cols: Vec::new(),
            kind: if read_any {
                FlatKind::RowsLeftOut
            } else {
                FlatKind::NoRows
            },
        });
    }

    /// Closes the row opened last; with `only_if_detail` (the item's `PrintOnlyIfDetail`) a row whose nested items wrote no row is
    /// taken back (corpus report/lt-report-requestpage/RptFilters.al:PrintOnlyIfDetail_Dataset, `1,3,`).
    pub(crate) fn end_row(&mut self, only_if_detail: bool) {
        let mark = self.marks.pop().expect("a row is open");
        if only_if_detail && mark.kids == 0 {
            self.body.truncate(mark.body);
            self.flat.truncate(mark.flat);
            self.rows = mark.rows;
            self.root = mark.root;
            self.open.pop();
            self.declares.pop();
            if let (Some(was), Some(last)) = (mark.parent_open, self.open.last_mut()) {
                *last = was;
            }
            if let Some(parent) = self.marks.last_mut() {
                parent.kids -= 1;
            }
            return;
        }
        let items_open = self.open.pop().expect("a row is open");
        let declares = self.declares.pop().unwrap_or(false);
        let at = 4 + 4 * self.open.len();
        if items_open {
            self.pad(at + 2);
            self.body.push_str("</DataItems>\r\n");
        } else if declares {
            self.pad(at + 2);
            self.body.push_str("<DataItems />\r\n");
        }
        self.pad(at);
        self.body.push_str("</DataItem>\r\n");
    }

    /// The bytes of the .NET DataSet `TestRequestPage.SaveAsXml` writes: the schema of the rows (one
    /// element per column with its .NET type) and a `Result` element per row, behind a byte order
    /// mark (corpus al-language-onprem/handlers/TestRequestPageSaveAsXmlContracts.al:
    /// SaveAs_ReportDataFile_IsADataSet, one data item with Integer and Text columns).
    ///
    /// The file is one flat table `Result` (al-language-onprem/handlers/Y17SaveAsXmlTrees.al, BC 27, 28 and 29 alike):
    /// - The schema lists the columns of every declared data item, depth first in run order, also of an item that wrote no row
    ///   (:521 A3, :562 B2, :644 C1 `TTotal` after the nested items, :685 D1 no row at all, :726 D2, :767 B3).
    /// - A row of an item with no nested items gives one Result with the columns of the rows above it and its own (:544 A3).
    /// - Under a row, its nested items in run order: each row of a nested item gives its Results; a nested item that ran and read
    ///   no row gives one Result with the columns down to the row (:544 A3 C 1/20 and P 2; :585 B2 P 2 before its D row, P 3
    ///   after its C row); one that broke in OnPreDataItem gives none (:790 B3).
    /// - A Result with no column is not written (:626 B0, column-less items over an Integer loop).
    ///
    /// A Decimal column is `xs:decimal` with its value as it is and a `<Name>Format` column (`#,##0.00`) after it
    /// (AqSaveAsXml.al:257, `AmountCFormat`). An empty Text is `<Name />` (AqSaveAsXml.al:258 `<NameC />`). Refused: a row whose
    /// every nested item broke, an item that read rows and wrote none, a column of an unpinned type, a column name used twice.
    pub(crate) fn finish_data_set(self, declared: &[DeclaredItem]) -> Result<Vec<u8>, Signal> {
        let flat = &self.flat;
        // The columns of the schema: the declared ones; a written row gives the type where the declaration does not.
        let mut schema: Vec<(&str, &'static str)> = Vec::new();
        for item in declared {
            let rows: Vec<&FlatRow> = flat
                .iter()
                .filter(|r| r.kind == FlatKind::Row && r.item == item.name)
                .collect();
            for r in &rows {
                if r.cols.iter().map(|c| &c.name).ne(item.columns.iter().map(|(n, _)| n)) {
                    return unsupported(format!(
                        "a SaveAsXml data file whose data item {} wrote other columns than it declares (not pinned)",
                        item.name
                    ));
                }
            }
            for (k, (name, declared_ty)) in item.columns.iter().enumerate() {
                let written = rows.first().map(|r| r.cols[k].ty);
                let ty: &'static str = match (written, *declared_ty) {
                    (Some(Some(t)), _) | (None, Some(t)) => t,
                    _ => {
                        return unsupported(format!(
                            "the SaveAsXml data file column {name}, a type with no pinned .NET type"
                        ));
                    }
                };
                if schema.iter().any(|(n, _)| n == name) {
                    return unsupported(format!(
                        "a SaveAsXml data file with the column {name} in two data items (not pinned)"
                    ));
                }
                // Only the default formatter is pinned.
                if ty == "xs:decimal" && rows.iter().any(|r| r.cols[k].formatter.as_deref() != Some("#,##0.00")) {
                    return unsupported(format!(
                        "the SaveAsXml data file Decimal column {name} with a formatter other than #,##0.00 (not pinned)"
                    ));
                }
                schema.push((name, ty));
            }
        }
        if let Some(r) = flat
            .iter()
            .find(|r| r.kind == FlatKind::Row && !declared.iter().any(|d| d.name == r.item))
        {
            return unsupported(format!(
                "a SaveAsXml data file with a row of data item {} the report does not declare (not pinned)",
                r.item
            ));
        }
        let mut out = String::new();
        out.push_str("<?xml version=\"1.0\" standalone=\"yes\"?>\r\n<DataSet>\r\n");
        out.push_str("  <xs:schema id=\"NewDataSet\" xmlns=\"\" xmlns:xs=\"http://www.w3.org/2001/XMLSchema\" xmlns:msdata=\"urn:schemas-microsoft-com:xml-msdata\">\r\n");
        out.push_str("    <xs:element name=\"NewDataSet\" msdata:IsDataSet=\"true\" msdata:MainDataTable=\"Result\" msdata:Locale=\"en-US\">\r\n");
        out.push_str("      <xs:complexType>\r\n        <xs:choice minOccurs=\"0\" maxOccurs=\"unbounded\">\r\n");
        out.push_str("          <xs:element name=\"Result\" msdata:Locale=\"en-US\">\r\n            <xs:complexType>\r\n              <xs:sequence>\r\n");
        for (name, ty) in &schema {
            let attributes = crate::data_file_types::element_attributes(ty);
            out.push_str(&format!(
                "                <xs:element name=\"{name}\" {attributes} minOccurs=\"0\" />\r\n"
            ));
            if *ty == "xs:decimal" {
                out.push_str(&format!(
                    "                <xs:element name=\"{name}Format\" type=\"xs:string\" minOccurs=\"0\" />\r\n"
                ));
            }
        }
        out.push_str("              </xs:sequence>\r\n            </xs:complexType>\r\n          </xs:element>\r\n");
        out.push_str("        </xs:choice>\r\n      </xs:complexType>\r\n    </xs:element>\r\n  </xs:schema>\r\n");
        let mut results: Vec<Vec<&FlatCol>> = Vec::new();
        let mut i = 0;
        while i < flat.len() {
            // An item at the top that wrote no row adds nothing (Y17SaveAsXmlTrees.al:708 D1).
            if flat[i].kind == FlatKind::Row {
                Self::results_of(flat, i, &[], &mut results)?;
            }
            i = Self::subtree_end(flat, i);
        }
        for row in results.iter().filter(|r| !r.is_empty()) {
            out.push_str("  <Result>\r\n");
            for c in row {
                let text = match &c.data {
                    DataFileText::Omit => continue,
                    DataFileText::Text(t) => t,
                    DataFileText::Same => &c.text,
                };
                if text.is_empty() {
                    // An empty Text is an empty element; an empty value of another type is not pinned.
                    if c.ty != Some("xs:string") {
                        return unsupported(format!(
                            "an empty value of column {} in a SaveAsXml data file (not pinned)",
                            c.name
                        ));
                    }
                    out.push_str(&format!("    <{} />\r\n", c.name));
                } else {
                    out.push_str(&format!("    <{name}>{text}</{name}>\r\n", name = c.name));
                }
                if c.ty == Some("xs:decimal") {
                    let f = c.formatter.as_deref().unwrap_or_default();
                    out.push_str(&format!("    <{name}Format>{f}</{name}Format>\r\n", name = c.name));
                }
            }
            out.push_str("  </Result>\r\n");
        }
        out.push_str("</DataSet>\r\n");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(out.as_bytes());
        Ok(bytes)
    }

    /// The index after the entry at `i` and everything nested under it.
    fn subtree_end(flat: &[FlatRow], i: usize) -> usize {
        let depth = flat[i].depth;
        let mut j = i + 1;
        while j < flat.len() && flat[j].depth > depth {
            j += 1;
        }
        j
    }

    /// The Results of row `i` of `flat` under the columns `above` (`finish_data_set`).
    fn results_of<'a>(
        flat: &'a [FlatRow],
        i: usize,
        above: &[&'a FlatCol],
        out: &mut Vec<Vec<&'a FlatCol>>,
    ) -> Result<(), Signal> {
        let mut cols = above.to_vec();
        cols.extend(flat[i].cols.iter());
        let end = Self::subtree_end(flat, i);
        if end == i + 1 {
            if flat[i].has_children {
                return unsupported(
                    "a SaveAsXml data file with a row whose every nested data item broke in OnPreDataItem (not pinned)",
                );
            }
            out.push(cols);
            return Ok(());
        }
        let mut j = i + 1;
        while j < end {
            match flat[j].kind {
                FlatKind::Row => Self::results_of(flat, j, &cols, out)?,
                FlatKind::NoRows => out.push(cols.clone()),
                // Whether the row above gets a Result of its own is not pinned. Under no column at all there is nothing to
                // write either way (a Result with no column is not written).
                FlatKind::RowsLeftOut if cols.is_empty() => {}
                FlatKind::RowsLeftOut => {
                    return unsupported(
                        "a SaveAsXml data file of a report whose nested data item read rows and wrote none (Skip, Break or PrintOnlyIfDetail; not pinned)",
                    );
                }
            }
            j = Self::subtree_end(flat, j);
        }
        Ok(())
    }

    /// The bytes of the whole dataset: the byte order mark, the header, the data items.
    pub(crate) fn finish(self, header: &Header) -> Result<Vec<u8>, Signal> {
        debug_assert!(self.open.is_empty());
        if let Some(why) = self.labels_unpinned.or(self.culture_unpinned) {
            return Err(Signal::Unsupported(why));
        }
        let mut out = String::new();
        // The header names the run's language and region as the data items do (report/lt-bd-format-region/BdRegionTests.al:809,
        // :928 for a trigger's setter; the property's region never gets here, `dataset_header`).
        header.write(
            &mut out,
            self.language.as_deref().unwrap_or(LANGUAGE),
            self.region.as_ref().map_or(LANGUAGE, |(r, _)| r.as_str()),
        )?;
        // handlers/TestReportSaveAsColumnTypesContracts.al:222-229: between the information block and
        // the data items.
        if !self.labels.is_empty() {
            out.push_str("  <Labels>\r\n");
            for (name, text) in &self.labels {
                out.push_str(&format!(
                    "    <Label name=\"{}\">{}</Label>\r\n",
                    escape_as(name, true)?,
                    escape(text)?
                ));
            }
            out.push_str("  </Labels>\r\n");
        }
        out.push_str(&self.body);
        if self.root {
            out.push_str("  </DataItems>\r\n");
        } else if self.body.is_empty() {
            // A report with no row at all still writes the element, in the XmlWriter's short form `  <DataItems />`
            // (report/PTSaveAsEmpty.al:no_rows; handlers/PWReportSaveAsColumns.al:Labels_NoRowAtAll).
            out.push_str("  <DataItems />\r\n");
        }
        out.push_str("</ReportDataSet>");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(out.as_bytes());
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col(name: &str, ty: &'static str, text: &str) -> Column {
        Column {
            name: name.into(),
            attribute: (ty == "xs:decimal").then(|| ("decimalformatter", "#,##0.00".to_string())),
            text: text.into(),
            schema_type: Some(ty),
            data: DataFileText::Same,
        }
    }

    fn data_file_rows(d: Dataset, items: &[DeclaredItem]) -> String {
        let bytes = d.finish_data_set(items).unwrap();
        let text = String::from_utf8(bytes[3..].to_vec()).unwrap().replace("\r\n", "|");
        text[text.find("</xs:schema>|").unwrap() + 13..].to_string()
    }

    // al-language-onprem/report/lt-aq/AqSaveAsXml.al:264 SaveAsXml_NestedDataItems_DataFile (and TO_SaveAsXml_Nested_Rows on
    // corpus branch al-runtime-rs-staging-pub-to): one Result per child row with the parent's columns first, a parent without
    // child rows only its own; a Decimal adds its Format column (:257), an empty Text is an empty element (:258).
    #[test]
    fn a_nested_data_item_gives_one_result_per_child_row() {
        let items = declared(&[
            ("Parent", &[("PId", "xs:int"), ("PName", "xs:string")]),
            ("Child", &[("CLine", "xs:int"), ("CQty", "xs:decimal")]),
        ]);
        let mut d = Dataset::default();
        d.begin(
            "Parent",
            None,
            &[col("PId", "xs:int", "1"), col("PName", "xs:string", "a")],
            true,
            true,
        );
        let since = d.mark();
        d.begin(
            "Child",
            None,
            &[col("CLine", "xs:int", "10"), col("CQty", "xs:decimal", "1.5")],
            false,
            false,
        );
        d.end_row(false);
        d.item_done(since, true);
        d.end_row(false);
        d.begin(
            "Parent",
            None,
            &[col("PId", "xs:int", "2"), col("PName", "xs:string", "")],
            true,
            true,
        );
        let since = d.mark();
        d.item_done(since, false);
        d.end_row(false);
        assert_eq!(
            data_file_rows(d, &items),
            "  <Result>|    <PId>1</PId>|    <PName>a</PName>|    <CLine>10</CLine>|    <CQty>1.5</CQty>|    <CQtyFormat>#,##0.00</CQtyFormat>|  </Result>|  <Result>|    <PId>2</PId>|    <PName />|  </Result>|</DataSet>|"
        );
    }

    // AqSaveAsXml.al:283 SaveAsXml_TwoTopLevelDataItems_DataFile: the rows of each item with only its own columns, one item after the other.
    #[test]
    fn data_items_side_by_side_give_their_rows_one_after_the_other() {
        let items = declared(&[("Parent", &[("PId", "xs:int")]), ("Child", &[("CLine", "xs:int")])]);
        let mut d = Dataset::default();
        d.begin("Parent", None, &[col("PId", "xs:int", "1")], false, false);
        d.end_row(false);
        d.begin("Child", None, &[col("CLine", "xs:int", "10")], false, false);
        d.end_row(false);
        assert_eq!(
            data_file_rows(d, &items),
            "  <Result>|    <PId>1</PId>|  </Result>|  <Result>|    <CLine>10</CLine>|  </Result>|</DataSet>|"
        );
    }

    fn declared(items: &[(&str, &[(&str, &'static str)])]) -> Vec<DeclaredItem> {
        items
            .iter()
            .map(|(name, cols)| DeclaredItem {
                name: name.to_string(),
                columns: cols.iter().map(|(c, t)| (c.to_string(), Some(*t))).collect(),
            })
            .collect()
    }

    fn data_file(d: Dataset, items: &[DeclaredItem]) -> Result<String, Signal> {
        let bytes = d.finish_data_set(items)?;
        Ok(String::from_utf8(bytes[3..].to_vec()).unwrap().replace("\r\n", "|"))
    }

    fn rows_of(text: &str) -> String {
        text[text.find("</xs:schema>|").unwrap() + 13..].replace("  ", "")
    }

    fn schema_columns(text: &str) -> String {
        text.split('|')
            .filter(|l| l.contains("minOccurs=\"0\" />"))
            .map(|l| l.trim().to_string() + "|")
            .collect()
    }

    // al-language-onprem/handlers/Y17SaveAsXmlTrees.al:544 Y17_SaveAsXml_A3_Rows: one Result per leaf with the columns of every
    // level above it; a middle row whose nested item ran with no row gives a Result with the columns down to it (C 1/20), a parent
    // whose child item has no row one with its own (P 2). :521 the schema lists the declared columns depth first.
    #[test]
    fn three_levels_give_one_result_per_leaf_and_one_per_item_without_rows() {
        let items = declared(&[
            ("P", &[("PId", "xs:int"), ("PName", "xs:string")]),
            ("C", &[("CLine", "xs:int"), ("CQty", "xs:decimal")]),
            ("G", &[("GSub", "xs:int"), ("GNote", "xs:string")]),
        ]);
        let mut d = Dataset::default();
        let p = |id: &str, name: &str| vec![col("PId", "xs:int", id), col("PName", "xs:string", name)];
        let c = |line: &str, qty: &str| vec![col("CLine", "xs:int", line), col("CQty", "xs:decimal", qty)];
        let g = |sub: &str, note: &str| vec![col("GSub", "xs:int", sub), col("GNote", "xs:string", note)];
        d.begin("P", None, &p("1", "a"), true, true);
        let cs = d.mark();
        d.begin("C", None, &c("10", "1.5"), true, true);
        let gs = d.mark();
        for (sub, note) in [("1", "x"), ("2", "y")] {
            d.begin("G", None, &g(sub, note), false, false);
            d.end_row(false);
        }
        d.item_done(gs, true);
        d.end_row(false);
        d.begin("C", None, &c("20", "2"), true, true);
        let gs = d.mark();
        d.item_done(gs, false);
        d.end_row(false);
        d.item_done(cs, true);
        d.end_row(false);
        d.begin("P", None, &p("2", ""), true, true);
        let cs = d.mark();
        d.item_done(cs, false);
        d.end_row(false);
        d.begin("P", None, &p("3", "c"), true, true);
        let cs = d.mark();
        d.begin("C", None, &c("10", "0"), true, true);
        let gs = d.mark();
        d.begin("G", None, &g("1", "z"), false, false);
        d.end_row(false);
        d.item_done(gs, true);
        d.end_row(false);
        d.item_done(cs, true);
        d.end_row(false);
        let text = data_file(d, &items).unwrap();
        assert_eq!(
            schema_columns(&text),
            "<xs:element name=\"PId\" type=\"xs:int\" minOccurs=\"0\" />|<xs:element name=\"PName\" type=\"xs:string\" minOccurs=\"0\" />|<xs:element name=\"CLine\" type=\"xs:int\" minOccurs=\"0\" />|<xs:element name=\"CQty\" type=\"xs:decimal\" minOccurs=\"0\" />|<xs:element name=\"CQtyFormat\" type=\"xs:string\" minOccurs=\"0\" />|<xs:element name=\"GSub\" type=\"xs:int\" minOccurs=\"0\" />|<xs:element name=\"GNote\" type=\"xs:string\" minOccurs=\"0\" />|"
        );
        assert_eq!(
            rows_of(&text),
            "<Result>|<PId>1</PId>|<PName>a</PName>|<CLine>10</CLine>|<CQty>1.5</CQty>|<CQtyFormat>#,##0.00</CQtyFormat>|<GSub>1</GSub>|<GNote>x</GNote>|</Result>|<Result>|<PId>1</PId>|<PName>a</PName>|<CLine>10</CLine>|<CQty>1.5</CQty>|<CQtyFormat>#,##0.00</CQtyFormat>|<GSub>2</GSub>|<GNote>y</GNote>|</Result>|<Result>|<PId>1</PId>|<PName>a</PName>|<CLine>20</CLine>|<CQty>2</CQty>|<CQtyFormat>#,##0.00</CQtyFormat>|</Result>|<Result>|<PId>2</PId>|<PName />|</Result>|<Result>|<PId>3</PId>|<PName>c</PName>|<CLine>10</CLine>|<CQty>0</CQty>|<CQtyFormat>#,##0.00</CQtyFormat>|<GSub>1</GSub>|<GNote>z</GNote>|</Result>|</DataSet>|"
        );
    }

    // Y17SaveAsXmlTrees.al:585 Y17_SaveAsXml_B2_Rows: children in declaration order per parent; a child item without rows gives a
    // Result with the parent's columns, also when the other child has rows (P 2, P 3). :790 B3_Rows: a child item that breaks in
    // OnPreDataItem gives none (P 3 has only its C row).
    #[test]
    fn a_child_item_without_rows_gives_the_parent_alone_and_a_broken_one_nothing() {
        let items = declared(&[
            ("P", &[("PId", "xs:int")]),
            ("C", &[("CLine", "xs:int")]),
            ("D", &[("DNo", "xs:int")]),
        ]);
        let run = |d_breaks: bool| {
            let mut d = Dataset::default();
            for (p, c_rows, d_rows) in [("1", 1, 1), ("2", 0, 1), ("3", 1, 0)] {
                d.begin("P", None, &[col("PId", "xs:int", p)], true, true);
                for (item, n, column, breaks) in [("C", c_rows, "CLine", false), ("D", d_rows, "DNo", d_breaks)] {
                    if breaks {
                        continue;
                    }
                    let since = d.mark();
                    for _ in 0..n {
                        d.begin(item, None, &[col(column, "xs:int", "7")], false, false);
                        d.end_row(false);
                    }
                    d.item_done(since, n > 0);
                }
                d.end_row(false);
            }
            d
        };
        assert_eq!(
            rows_of(&data_file(run(false), &items).unwrap()),
            "<Result>|<PId>1</PId>|<CLine>7</CLine>|</Result>|<Result>|<PId>1</PId>|<DNo>7</DNo>|</Result>|<Result>|<PId>2</PId>|</Result>|<Result>|<PId>2</PId>|<DNo>7</DNo>|</Result>|<Result>|<PId>3</PId>|<CLine>7</CLine>|</Result>|<Result>|<PId>3</PId>|</Result>|</DataSet>|"
        );
        // P 2 of the second run: C has no row, D breaks: one Result with the parent's columns.
        assert_eq!(
            rows_of(&data_file(run(true), &items).unwrap()),
            "<Result>|<PId>1</PId>|<CLine>7</CLine>|</Result>|<Result>|<PId>2</PId>|</Result>|<Result>|<PId>3</PId>|<CLine>7</CLine>|</Result>|</DataSet>|"
        );
    }

    // Y17SaveAsXmlTrees.al:626 B0_Rows: rows of items without columns give no Result; :708 D1_Rows: no row at all is the schema of
    // the declared columns and no Result.
    #[test]
    fn items_without_columns_give_no_result_and_no_row_gives_only_the_schema() {
        let items = declared(&[("Top", &[]), ("C", &[]), ("Loop", &[("LNum", "xs:int")])]);
        let mut d = Dataset::default();
        d.begin("Top", None, &[], true, true);
        let since = d.mark();
        for _ in 0..3 {
            d.begin("C", None, &[], false, false);
            d.end_row(false);
        }
        d.item_done(since, true);
        let since = d.mark();
        for n in ["1", "2"] {
            d.begin("Loop", None, &[col("LNum", "xs:int", n)], false, false);
            d.end_row(false);
        }
        d.item_done(since, true);
        d.end_row(false);
        let text = data_file(d, &items).unwrap();
        assert_eq!(
            schema_columns(&text),
            "<xs:element name=\"LNum\" type=\"xs:int\" minOccurs=\"0\" />|"
        );
        assert_eq!(
            rows_of(&text),
            "<Result>|<LNum>1</LNum>|</Result>|<Result>|<LNum>2</LNum>|</Result>|</DataSet>|"
        );
        let items = declared(&[("P", &[("PId", "xs:int"), ("PName", "xs:string")])]);
        let mut d = Dataset::default();
        let since = d.mark();
        d.item_done(since, false);
        let text = data_file(d, &items).unwrap();
        assert_eq!(rows_of(&text), "</DataSet>|");
        assert!(schema_columns(&text).contains("PName"));
    }

    // al-language-onprem/report/lt-aq/AqSaveAsXml.al:247 SaveAsXml_ColumnTypes_DataFile: a Date is `xs:dateTime` with
    // `msdata:DateTimeMode="Local"`, written `2024-12-25T00:00:00+00:00` and left out when blank; a Boolean `xs:boolean`, `true` or
    // `false`; an Option `xs:string`, the member.
    #[test]
    fn dates_booleans_and_options_have_their_data_file_types() {
        let items = declared(&[(
            "Row",
            &[("DayC", "xs:dateTime"), ("FlagC", "xs:boolean"), ("KindC", "xs:string")],
        )]);
        let row = |day: DataFileText, day_text: &str, flag: bool, kind: &str| {
            vec![
                Column {
                    data: day,
                    ..col("DayC", "xs:dateTime", day_text)
                },
                Column {
                    data: DataFileText::Text(flag.to_string()),
                    ..col("FlagC", "xs:boolean", if flag { "True" } else { "False" })
                },
                col("KindC", "xs:string", kind),
            ]
        };
        let mut d = Dataset::default();
        d.begin(
            "Row",
            None,
            &row(
                DataFileText::Text("2024-12-25T00:00:00+00:00".into()),
                "12/25/2024",
                true,
                "Gamma",
            ),
            false,
            false,
        );
        d.end_row(false);
        d.begin("Row", None, &row(DataFileText::Omit, "", false, "Alpha"), false, false);
        d.end_row(false);
        let text = data_file(d, &items).unwrap();
        assert_eq!(
            schema_columns(&text),
            "<xs:element name=\"DayC\" msdata:DateTimeMode=\"Local\" type=\"xs:dateTime\" minOccurs=\"0\" />|<xs:element name=\"FlagC\" type=\"xs:boolean\" minOccurs=\"0\" />|<xs:element name=\"KindC\" type=\"xs:string\" minOccurs=\"0\" />|"
        );
        assert_eq!(
            rows_of(&text),
            "<Result>|<DayC>2024-12-25T00:00:00+00:00</DayC>|<FlagC>true</FlagC>|<KindC>Gamma</KindC>|</Result>|<Result>|<FlagC>false</FlagC>|<KindC>Alpha</KindC>|</Result>|</DataSet>|"
        );
    }

    // al-language-onprem/report/lt-y51-saveasxml-time/Y51SaveAsXmlTime.al:276: a Guid names its .NET type in `msdata:DataType`.
    #[test]
    fn a_guid_column_names_its_dotnet_type() {
        let g = crate::data_file_types::of_value(&al_builtins::value::Value::Guid([0; 16]), 29)
            .unwrap()
            .0;
        let items = declared(&[("Row", &[("GC", g)])]);
        let mut d = Dataset::default();
        d.begin(
            "Row",
            None,
            &[col("GC", g, "00000000-0000-0000-0000-000000000000")],
            false,
            false,
        );
        d.end_row(false);
        let text = data_file(d, &items).unwrap();
        assert_eq!(
            schema_columns(&text),
            "<xs:element name=\"GC\" msdata:DataType=\"System.Guid, System.Private.CoreLib, Version=10.0.0.0, Culture=neutral, PublicKeyToken=7cec85d7bea7798e\" type=\"xs:string\" minOccurs=\"0\" />|"
        );
    }

    // Not pinned: a row whose every nested item broke, a nested item that read rows and wrote none (Skip, Break in
    // OnAfterGetRecord), a declared column whose type is unknown. A top item that left out every row writes nothing.
    #[test]
    fn data_file_shapes_no_test_pins_are_refused() {
        let items = declared(&[("P", &[("PId", "xs:int")]), ("C", &[("CLine", "xs:int")])]);
        let mut d = Dataset::default();
        d.begin("P", None, &[col("PId", "xs:int", "1")], true, true);
        d.end_row(false);
        assert!(matches!(d.finish_data_set(&items), Err(Signal::Unsupported(_))));
        let mut d = Dataset::default();
        d.begin("P", None, &[col("PId", "xs:int", "1")], true, true);
        let since = d.mark();
        d.item_done(since, true);
        d.end_row(false);
        assert!(matches!(d.finish_data_set(&items), Err(Signal::Unsupported(_))));
        let mut d = Dataset::default();
        let since = d.mark();
        d.item_done(since, true);
        assert_eq!(rows_of(&data_file(d, &items).unwrap()), "</DataSet>|");
        let unknown = vec![DeclaredItem {
            name: "P".into(),
            columns: vec![("PId".into(), None)],
        }];
        let d = Dataset::default();
        assert!(matches!(d.finish_data_set(&unknown), Err(Signal::Unsupported(_))));
    }

    // handlers/TestReportSaveAsColumnTypesContracts.al:222-244
    #[test]
    fn decimal_places_decide_the_formatter() {
        assert_eq!(decimal_formatter(None).unwrap(), "#,##0.00");
        assert_eq!(decimal_formatter(Some("1 : 1")).unwrap(), "#,##0.0");
        assert_eq!(decimal_formatter(Some("3:3")).unwrap(), "#,##0.000");
        // handlers/TestReportSaveAsColumnDetailsContracts.al and handlers/PWReportSaveAsColumns.al
        for (places, want) in [
            ("2 : 4", "#,##0.00##"),
            ("0 : 0", "#,##0"),
            ("1", "#,##0.0"),
            ("0 : 5", "#,##0.#####"),
            ("0 : 2", "#,##0.##"),
        ] {
            assert_eq!(decimal_formatter(Some(places)).unwrap(), want, "{places}");
        }
        assert!(decimal_formatter(Some("4 : 2")).is_err());
        assert!(decimal_formatter(Some("x")).is_err());
    }

    #[test]
    fn time_and_date_follow_the_en_us_pattern() {
        assert_eq!(time_text(45_296_000).unwrap(), "12:34:56 PM");
        assert_eq!(time_text(0).unwrap(), "12:00:00 AM");
        // Milliseconds are cut off (errors/lt-ms-refusals-132/P132ReportTimeMs.al P132_Dataset_Time_Milliseconds, Y38_*).
        assert_eq!(time_text(1).unwrap(), "12:00:00 AM");
        assert_eq!(time_text(48_600_999).unwrap(), "1:30:00 PM");
        assert_eq!(time_text(86_399_999).unwrap(), "11:59:59 PM");
    }

    #[test]
    fn plain_names_are_kept() {
        assert_eq!(dataset_name("PNo").unwrap(), "PNo");
        assert_eq!(
            dataset_name("Change_Log_Setup__Table___Table_No__").unwrap(),
            "Change_Log_Setup__Table___Table_No__"
        );
    }

    // handlers/TestReportNestedLinkImplicitParent.al:98 (`//DataItem[@name="Change_Log_Setup_Table"]`)
    #[test]
    fn a_name_with_spaces_and_parentheses_is_spelled_the_way_the_example_shows() {
        assert_eq!(
            dataset_name("Change Log Setup (Table)").unwrap(),
            "Change_Log_Setup_Table"
        );
        assert_eq!(
            dataset_name("Change Log Setup (Field)").unwrap(),
            "Change_Log_Setup_Field"
        );
    }

    // ReportDatasetNames.al:SaveAsXml_DataItemNames_DotsAndSlashes.
    #[test]
    fn a_run_of_dots_slashes_and_spaces_is_one_underscore() {
        for (name, want) in [
            ("A. B", "A_B"),
            ("C/D", "C_D"),
            ("G..H", "G_H"),
            ("I./J", "I_J"),
            ("K  L", "K_L"),
            ("M.", "M"),
            (".N", "_N"),
            ("O. . P", "O_P"),
            ("Q//R", "Q_R"),
        ] {
            assert_eq!(dataset_name(name).unwrap(), want, "{name}");
        }
        assert!(dataset_name("M..").is_err());
        assert!(dataset_name("/M").is_err());
    }

    #[test]
    fn a_name_the_example_does_not_cover_is_refused() {
        assert!(dataset_name("A(B)").is_err());
        assert!(dataset_name("").is_err());
    }

    // handlers/TestReportSaveAsDatasetContracts.al:SaveAs_Xml_DataItemsBlock_ExactText (`one <&>`)
    #[test]
    fn text_is_escaped() {
        assert_eq!(escape("one <&>").unwrap(), "one &lt;&amp;&gt;");
        assert!(escape("a\u{1}b").is_err());
    }
}
