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
/// (the culture's pattern; no single-digit hour is measured) and milliseconds are refused.
pub(crate) fn time_text(ms_of_day: i64) -> Result<String, Signal> {
    if ms_of_day % 1000 != 0 {
        return unsupported("a Time with milliseconds in a dataset (not pinned)");
    }
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
    /// column types that probe pins: Integer and Text/Code.
    pub schema_type: Option<&'static str>,
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
    /// The rows of the top-level data items for a .NET DataSet: (name, schema type) of the columns
    /// of the first row, and the texts of every row.
    flat: Vec<Vec<(String, Option<&'static str>, String)>>,
    /// Whether the rows fit one flat table: one data item, no nested row.
    flat_ok: bool,
    /// For every open `DataItem`: the state to go back to when `PrintOnlyIfDetail` drops its row, and how many rows its nested items wrote.
    marks: Vec<Mark>,
}

/// The dataset as it was before a row began.
struct Mark {
    body: usize,
    flat: usize,
    rows: usize,
    root: bool,
    flat_ok: bool,
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
    ) {
        let depth = self.open.len();
        self.marks.push(Mark {
            body: self.body.len(),
            flat: self.flat.len(),
            rows: self.rows,
            root: self.root,
            flat_ok: self.flat_ok,
            parent_open: self.open.last().copied(),
            kids: 0,
        });
        if depth > 0 {
            if let Some(parent) = self.marks.len().checked_sub(2).and_then(|i| self.marks.get_mut(i)) {
                parent.kids += 1;
            }
        }
        if depth == 0 {
            if self.rows == 0 {
                self.flat_ok = true;
            }
            self.flat.push(
                columns
                    .iter()
                    .map(|c| (c.name.clone(), c.schema_type, c.text.clone()))
                    .collect(),
            );
            // A second data item (its rows have other columns) is not one table.
            if self.flat.len() > 1
                && self.flat[0]
                    .iter()
                    .map(|c| &c.0)
                    .ne(self.flat[self.flat.len() - 1].iter().map(|c| &c.0))
            {
                self.flat_ok = false;
            }
        } else {
            self.flat_ok = false;
        }
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

    /// Closes the row opened last; with `only_if_detail` (the item's `PrintOnlyIfDetail`) a row whose nested items wrote no row is
    /// taken back (corpus report/lt-report-requestpage/RptFilters.al:PrintOnlyIfDetail_Dataset, `1,3,`).
    pub(crate) fn end_row(&mut self, only_if_detail: bool) {
        let mark = self.marks.pop().expect("a row is open");
        if only_if_detail && mark.kids == 0 {
            self.body.truncate(mark.body);
            self.flat.truncate(mark.flat);
            self.rows = mark.rows;
            self.root = mark.root;
            self.flat_ok = mark.flat_ok;
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
    /// SaveAs_ReportDataFile_IsADataSet). Pinned for one data item whose columns are Integer or Text;
    /// anything else is refused.
    pub(crate) fn finish_data_set(self) -> Result<Vec<u8>, Signal> {
        if !self.flat_ok || self.flat.is_empty() {
            return unsupported(
                "a SaveAsXml data file of a report that is not one flat data item with rows (not pinned)",
            );
        }
        let mut out = String::new();
        out.push_str("<?xml version=\"1.0\" standalone=\"yes\"?>\r\n<DataSet>\r\n");
        out.push_str("  <xs:schema id=\"NewDataSet\" xmlns=\"\" xmlns:xs=\"http://www.w3.org/2001/XMLSchema\" xmlns:msdata=\"urn:schemas-microsoft-com:xml-msdata\">\r\n");
        out.push_str("    <xs:element name=\"NewDataSet\" msdata:IsDataSet=\"true\" msdata:MainDataTable=\"Result\" msdata:Locale=\"en-US\">\r\n");
        out.push_str("      <xs:complexType>\r\n        <xs:choice minOccurs=\"0\" maxOccurs=\"unbounded\">\r\n");
        out.push_str("          <xs:element name=\"Result\" msdata:Locale=\"en-US\">\r\n            <xs:complexType>\r\n              <xs:sequence>\r\n");
        for (name, ty, _) in &self.flat[0] {
            let Some(ty) = ty else {
                return unsupported(format!(
                    "the SaveAsXml data file column {name}, a type with no pinned .NET type"
                ));
            };
            out.push_str(&format!(
                "                <xs:element name=\"{name}\" type=\"{ty}\" minOccurs=\"0\" />\r\n"
            ));
        }
        out.push_str("              </xs:sequence>\r\n            </xs:complexType>\r\n          </xs:element>\r\n");
        out.push_str("        </xs:choice>\r\n      </xs:complexType>\r\n    </xs:element>\r\n  </xs:schema>\r\n");
        for row in &self.flat {
            out.push_str("  <Result>\r\n");
            for (name, _, text) in row {
                // An empty value is no element in a DataSet; the probe has none, so it is refused.
                if text.is_empty() {
                    return unsupported(format!(
                        "an empty value of column {name} in a SaveAsXml data file (not pinned)"
                    ));
                }
                out.push_str(&format!("    <{name}>{text}</{name}>\r\n"));
            }
            out.push_str("  </Result>\r\n");
        }
        out.push_str("</DataSet>\r\n");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(out.as_bytes());
        Ok(bytes)
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
        assert!(time_text(1).is_err());
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
