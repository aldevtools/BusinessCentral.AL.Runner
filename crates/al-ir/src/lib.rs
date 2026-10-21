//! Typed IR (ADR-011): every name resolved, every node typed, conversions
//! explicit. Plain data, serde-serialisable, no syntax nodes.

pub use al_builtins::builtin_id::BuiltinId;
use al_builtins::value::RtType;
use serde::{Deserialize, Serialize};

mod body;
mod facts;
mod objkind;
pub mod sym;
pub mod types;
pub mod walk;
pub use body::{Body, BodyFixup};
pub use facts::{FieldClass, FieldFacts, ObjectFacts, PageType, TableType};
pub use objkind::{ObjKind, UnknownKind};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Program {
    pub objects: Vec<Object>,
    /// The apps the objects belong to, in the order the compiler was given them (`Object::app`
    /// indexes this). Publisher and version come from the runner, which reads the manifests.
    #[serde(default)]
    pub apps: Vec<AppInfo>,
    /// Every `type(..; Alias)` of every `dotnet { }` block in the workspace (ADR-014). A
    /// `DotNet` variable's `RtType::DotNet(i)` and `Expr::DotNet::ty` index this list.
    #[serde(default)]
    pub dotnet_types: Vec<DotNetDecl>,
    /// The source files the positions refer to (`SrcPos::file` indexes this).
    #[serde(default)]
    pub sources: Vec<SourceFile>,
    /// Source positions of statements and routine declarations. A [`Stmt::Pos`] marker and
    /// [`Routine::pos`] hold an index into this table (`docs/architecture.md`, "Source positions").
    #[serde(default)]
    pub positions: Vec<SrcPos>,
    /// Event subscribers of dependency objects that were not compiled (ADR-013 decision 6), for
    /// events the runtime can raise. BC would run them; this program has no code for them, so
    /// raising such an event is refused instead of running without them.
    #[serde(default)]
    pub uncompiled_subscribers: Vec<UncompiledSubscriber>,
    /// `--tdd` (ADR-051): the signature of every stub the program holds (`Expr::TddStub::stub`), sorted, each once.
    /// Empty without `--tdd`.
    #[serde(default)]
    pub tdd_stubs: Vec<String>,
}

/// An event subscriber that is declared but not compiled (see [`Program::uncompiled_subscribers`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UncompiledSubscriber {
    /// The object whose event it subscribes to, as an index into `Program::objects`. `None` for the
    /// platform's own codeunits, which no app need declare; `platform` names which.
    pub publisher: Option<u32>,
    pub platform: PlatformPublisher,
    /// Lower-case event name.
    pub event: String,
    /// The `ElementName` argument; empty for an event without elements.
    pub element: String,
    /// Where the subscriber is declared: `Object "Name"`, for the refusal text.
    pub subscriber: String,
}

/// The platform codeunits whose events the runtime raises and no app has to declare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlatformPublisher {
    /// The subscriber names an object of the program (`UncompiledSubscriber::publisher`).
    None,
    /// `Global Triggers` (2000000002): database events around record writes.
    GlobalTriggers,
    /// `Caption Class` (42): `OnResolveCaptionClass`.
    CaptionClass,
    /// `Reporting Triggers`: `CustomDocumentMergerEx`, raised when a report renders with a custom layout.
    ReportingTriggers,
    /// `Filter Helper Triggers`: `MakeDateFilter`, raised when a Date filter text has a piece that does not read.
    FilterHelperTriggers,
}

/// The events of `Reporting Triggers` the runtime raises: `CustomDocumentMergerEx` for a custom layout, and
/// `SubstituteReport` and `OnDocumentReady` around an Xml `SaveAs` (events/lt-platform-events-v/PeVTests.al:432).
pub const RAISED_REPORTING_EVENTS: [&str; 3] = ["CustomDocumentMergerEx", "SubstituteReport", "OnDocumentReady"];

/// The events of `Filter Helper Triggers` the runtime raises (MakeDateFilter, MakeIntFilter and MakeTimeFilter:
/// events/lt-platform-triggers/PtObserve.al) or refuses to raise (MakeDateTimeFilter).
pub const RAISED_FILTER_EVENTS: [&str; 4] = [
    "MakeDateFilter",
    "MakeIntFilter",
    "MakeTimeFilter",
    "MakeDateTimeFilter",
];

/// The events of a page the test client raises (`al-vm/src/pages.rs`). A subscriber to any other
/// event of a page is refused when the page opens.
pub const RAISED_PAGE_EVENTS: [&str; 9] = [
    "onopenpageevent",
    "onqueryclosepageevent",
    "onclosepageevent",
    "onaftergetcurrrecordevent",
    "onnewrecordevent",
    "oninsertrecordevent",
    "onmodifyrecordevent",
    // Around the OnAction trigger of an action a TestPage invokes (corpus events/lt-global-triggers-statements/P114PageActionEvents.al).
    "onbeforeactionevent",
    "onafteractionevent",
];

/// A source file of the program: the app it belongs to and its path inside the app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFile {
    /// Index into `Program::apps`.
    pub app: u32,
    pub path: String,
    /// The text of the file, line by line, for an app whose manifest has
    /// `resourceExposurePolicy.includeSourceInSymbolFile` (the Code Coverage table shows it); else empty.
    #[serde(default)]
    pub lines: Vec<String>,
}

/// "No position": a routine the compiler made up (a page expression), which has no source.
pub const NO_POS: u32 = u32::MAX;

/// Where a statement or a routine declaration sits in the source. Lines and columns count from 1.
/// The column is a byte column. Page files that declare computed properties are numbered as
/// the author wrote them, not as the compiler's expanded text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SrcPos {
    /// Index into `Program::sources`.
    pub file: u32,
    pub line: u32,
    pub col: u32,
    /// The line the statement ends on (equal to `line` for a one-line statement).
    pub end_line: u32,
    /// The object (index into `Program::objects`) and routine (index into `Object::routines`)
    /// the position is in, so a position alone names the code it belongs to.
    pub object: u32,
    pub routine: u32,
}

impl Program {
    /// The position `id` names; `None` for [`NO_POS`] or an id outside the table.
    pub fn pos(&self, id: u32) -> Option<&SrcPos> {
        self.positions.get(id as usize)
    }
}

/// What `NavApp.GetCurrentModuleInfo` reports about an app.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AppInfo {
    pub id: String,
    pub name: String,
    pub publisher: String,
    /// `1.0.0.0`.
    pub version: String,
    /// Ids of the apps it depends on.
    pub dependencies: Vec<String>,
    /// The packaged resources (`resourceFolders` of the manifest): name relative to its folder, content.
    #[serde(default)]
    pub resources: Vec<(String, Vec<u8>)>,
    /// A dependency app, compiled from its package (ADR-013): its pages are precompiled ones as
    /// far as BC's behaviour goes. The app under test is not.
    #[serde(default)]
    pub dependency: bool,
    /// The manifest says `resourceExposurePolicy.includeSourceInSymbolFile = true`: the app's source
    /// lines are known (`SourceFile::lines`).
    #[serde(default)]
    pub include_source: bool,
    /// The manifest's `target` (`Cloud` when the manifest leaves it out, the compiler's default); `None` when no
    /// manifest was read.
    #[serde(default)]
    pub target: Option<String>,
    /// The files the app's reports name with `LayoutFile`: path relative to the app (`/` separators), content.
    #[serde(default)]
    pub layout_files: Vec<(String, Vec<u8>)>,
}

/// A `.NET` type declared by a `dotnet { }` block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DotNetDecl {
    pub alias: String,
    pub assembly: String,
    pub version: Option<String>,
    pub culture: Option<String>,
    pub public_key_token: Option<String>,
    /// The CLR type name as declared.
    pub type_name: String,
}

/// What an `Expr::DotNet` does to its receiver (ADR-014).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DotNetOp {
    /// `Alias.Alias(args)`: construct an instance of the declared type.
    Construct,
    /// `X.Member(args)`.
    Call(String),
    /// `X.Member` without parentheses: a property or field read.
    Get(String),
    /// `X.Member := value`; the value is the only argument.
    Set(String),
}

impl Object {
    /// A compiled object of this kind (a shell of the same kind is not).
    pub fn is_kind(&self, kind: ObjKind) -> bool {
        self.kind == kind && !self.uncompiled
    }

    /// The kind as the text the compiler used to write: `codeunit`, or `uncompiled codeunit` for a shell.
    pub fn kind_text(&self) -> String {
        if self.uncompiled {
            format!("uncompiled {}", self.kind)
        } else {
            self.kind.to_string()
        }
    }

    /// A shell (an object of a dependency app that was not compiled) of this kind.
    pub fn is_shell_of(&self, kind: ObjKind) -> bool {
        self.kind == kind && self.uncompiled
    }
}

/// The name of an object property the lowering adds to a table whose caption is not pinned (a tableextension sets one over
/// another): its value is the reason a read of the table's caption is refused.
pub const CAPTION_REFUSED: &str = "$CaptionRefused";

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Object {
    pub kind: ObjKind,
    /// A shell: the object belongs to a dependency app that was not compiled (it keeps its number
    /// stable; nothing looks it up by kind and id as a runnable object).
    pub uncompiled: bool,
    pub id: Option<u32>,
    pub name: String,
    pub globals: Vec<Slot>,
    pub routines: Vec<Routine>,
    /// `Subtype = Test` and similar, as written.
    pub properties: Vec<(String, String)>,
    /// For enums: (declared value, name), extensions included.
    pub enum_values: Vec<(i32, String)>,
    /// For enums: the `Caption` of values that declare one, by ordinal.
    #[serde(default)]
    pub enum_captions: Vec<(i32, String)>,
    /// For tables: fields (extensions and system fields included), keys as
    /// field positions (`keys[0]` primary), `DataPerCompany`.
    pub table: Option<Table>,
    /// Global slots of the implicit `Rec` / `xRec` (tables, table extensions).
    pub rec_slot: Option<usize>,
    pub xrec_slot: Option<usize>,
    /// For pages: the control tree and page triggers (ADR-012).
    pub page: Option<Page>,
    /// Index of the app that owns the object, in the order the apps were given to the compiler.
    /// Builtins scoped to a module (`IsolatedStorage`) key on it.
    #[serde(default)]
    pub app: u32,
    /// For queries: dataitems, columns and the query-level properties.
    #[serde(default)]
    pub query: Option<Query>,
    /// For xmlports: the schema. Triggers and procedures are ordinary routines of the object.
    #[serde(default)]
    pub xmlport: Option<XmlPort>,
    /// For reports: the data items and report-level triggers. The triggers and procedures are
    /// ordinary routines of the object.
    #[serde(default)]
    pub report: Option<Report>,
    /// For reportextensions that change what a run does: the triggers and data items they add.
    #[serde(default)]
    pub report_ext: Option<ReportExt>,
    /// The namespace the object is declared in.
    #[serde(default)]
    pub namespace: Option<String>,
    /// For an extension object: the id of the object it extends, when that object is in the
    /// workspace (the metadata tables report it).
    #[serde(default)]
    pub extends_id: Option<u32>,
    /// For a report: the layouts its `rendering` section declares.
    #[serde(default)]
    pub layouts: Vec<LayoutDecl>,
}

/// `layout(Name) { Type = Word; ... }` of a report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutDecl {
    pub name: String,
    /// The properties as written (`Type`, `LayoutFile`, `ObsoleteState`, ...).
    pub properties: Vec<(String, String)>,
}

/// A report as the runtime sees it: processing only, with no layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    /// `ProcessingOnly = true`.
    pub processing_only: bool,
    /// `UseRequestPage` when the report declares it (it defaults to true).
    pub use_request_page: Option<bool>,
    /// The report declares a `requestpage` section.
    pub has_request_page: bool,
    /// `TransactionType`, lower-case, when the report declares it.
    pub transaction_type: Option<String>,
    /// Data items in source order; `parent` makes the tree. Top-level items have no parent.
    pub items: Vec<ReportItem>,
    /// `OnInitReport`, `OnPreReport` and `OnPostReport` as routine indices.
    pub on_init: Option<usize>,
    pub on_pre: Option<usize>,
    pub on_post: Option<usize>,
    /// `OnPreRendering(var RenderingPayload: JsonObject)` as a routine index.
    #[serde(default)]
    pub on_pre_rendering: Option<usize>,
    /// The reportextensions (object indices) that change what a run does, in the order they apply:
    /// by object id (ADR-025). Empty: none does.
    #[serde(default)]
    pub extensions: Vec<usize>,
    /// Global slot of the hidden flag `Report.UseRequestPage(x)` sets: 0 = not set, 1 = true, 2 = false.
    pub request_flag_slot: usize,
    /// Why running the report would render output (it is not `ProcessingOnly`, or it declares a
    /// layout): the runtime renders nothing, so a run that gets that far is refused. Its request
    /// page still opens first.
    #[serde(default)]
    pub output: Vec<String>,
    /// Why the runtime cannot run this report yet; empty = it can.
    pub unsupported: Vec<String>,
    /// The `Caption` property when it is a plain text literal.
    #[serde(default)]
    pub caption: Option<String>,
    /// The `FormatRegion` property: the culture of a SaveAs Xml dataset and of `Format` while it is written.
    #[serde(default)]
    pub format_region: Option<String>,
    /// The request page's `AboutTitle` and `AboutText` when they are plain text literals.
    #[serde(default)]
    pub about_title: Option<String>,
    #[serde(default)]
    pub about_text: Option<String>,
    /// The kind of the layout the report renders with by default.
    #[serde(default)]
    pub layout: ReportLayout,
    /// The `labels` section: (name, text) in declaration order. An empty text reads as the name.
    #[serde(default)]
    pub labels: Vec<(String, String)>,
    /// Why `Report.SaveAs(.., Xml, ..)` cannot write this report's dataset (a section or property
    /// whose effect on the dataset no test pins); empty = it can.
    #[serde(default)]
    pub dataset_unsupported: Vec<String>,
    /// `DefaultLayout` names a kind but the report declares no layout file (no `RDLCLayout`, `WordLayout` or
    /// `ExcelLayout` property, no `rendering` section): a rendered `Report.SaveAs` finds no valid layout.
    #[serde(default)]
    pub layout_file_missing: bool,
    /// The layouts of the `rendering` section, in declaration order.
    #[serde(default)]
    pub rendering_layouts: Vec<RenderingLayout>,
    /// `DefaultRenderingLayout`: the name of the layout the report renders with when none is named.
    #[serde(default)]
    pub default_rendering_layout: Option<String>,
}

/// One `layout(Name) { ... }` of a report's `rendering` section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderingLayout {
    pub name: String,
    /// The `Type` property, lower-case (`rdlc`, `word`, `excel`, `custom`); empty when it is not declared.
    pub kind: String,
    /// The `LayoutFile` as written, with the quotes taken off.
    pub file: String,
    /// `LayoutFile` relative to the app folder (`/` separators, `.` and `..` resolved).
    pub path: String,
    pub mime_type: String,
}

/// The layout a report renders with by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ReportLayout {
    /// The report declares no layout.
    #[default]
    None,
    /// `DefaultLayout = RDLC`, or `DefaultRenderingLayout` naming a layout of `Type = RDLC`.
    Rdlc,
    /// Any other layout, or one the runtime cannot tell the kind of.
    Other,
}

/// A `column(Name; Source)` of a data item. The source is an expression of the item's record, or
/// of the report: `routine` evaluates it, as a routine of the object that declares the column.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportColumn {
    pub name: String,
    pub routine: Option<usize>,
    /// Why the dataset cannot carry this column; set when it states a property no test pins.
    pub unsupported: Option<String>,
    /// The position of the table field the source names, when the source is just that field.
    #[serde(default)]
    pub field: Option<usize>,
    /// The position of the table field a source `Item.Field` names, where `Item` is the column's own data item. Only a Blob column
    /// reads it (a field the report did not calculate shows what the row stores); `field` stays None for such a source.
    #[serde(default)]
    pub item_field: Option<usize>,
    /// `IncludeCaption = true`: the dataset's Labels block carries `<Name>Caption`.
    #[serde(default)]
    pub include_caption: bool,
    /// The `DecimalPlaces` property as written (`1 : 1`); it decides the `decimalformatter`.
    #[serde(default)]
    pub decimal_places: Option<String>,
    /// The `OptionCaptionML` property as written (`ENU = '..', DEU = '..'`): an Option value shows the caption of the session
    /// language (record/lt-x-table-props/XTablePropsTests.al:741-785).
    #[serde(default)]
    pub option_caption_ml: Option<String>,
}

/// What a reportextension adds to a run of the report it extends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportExt {
    /// Object index of the extended report.
    pub base: usize,
    /// `OnPreReport` and `OnPostReport` of the extension, as routine indices.
    pub on_pre: Option<usize>,
    pub on_post: Option<usize>,
    /// (base item index, global slot of the extension that names that item's record).
    pub aliases: Vec<(usize, usize)>,
    /// `modify(Item)` blocks.
    pub modifies: Vec<ReportExtModify>,
    /// Data items the extension adds with `addlast`.
    pub added: Vec<AddedItem>,
    /// Columns the extension adds to base items with `add(Item)`.
    #[serde(default)]
    pub adds: Vec<ReportExtAdd>,
    /// The `labels` section of the extension: (name, text) in declaration order.
    #[serde(default)]
    pub labels: Vec<(String, String)>,
}

/// The columns an `add(Item)` block of a reportextension gives a base item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportExtAdd {
    /// Index of the base item.
    pub item: usize,
    /// Routines of the extension.
    pub columns: Vec<ReportColumn>,
}

/// The triggers of a `modify(Item)` block of a reportextension.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportExtModify {
    /// Index of the base item.
    pub item: usize,
    /// `RequestFilterFields` as written, from the block's property (report/lt-y-report-props/YReportExt.al:231).
    #[serde(default)]
    pub request_filter_fields: Option<String>,
    /// `OnBeforeAfterGetRecord` and `OnAfterAfterGetRecord` as routine indices.
    pub before: Option<usize>,
    pub after: Option<usize>,
    /// `OnBeforePreDataItem` and `OnAfterPreDataItem` around the base item's `OnPreDataItem`.
    #[serde(default)]
    pub before_pre: Option<usize>,
    #[serde(default)]
    pub after_pre: Option<usize>,
    /// `OnBeforePostDataItem` and `OnAfterPostDataItem` around the base item's `OnPostDataItem`.
    #[serde(default)]
    pub before_post: Option<usize>,
    #[serde(default)]
    pub after_post: Option<usize>,
}

/// A data item added by `addlast`: under a base item, or (`item.parent`) under another added item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddedItem {
    /// `item.parent` indexes `ReportExt::added`.
    pub item: ReportItem,
    /// The base item it is added under (top-level added items).
    pub base_parent: Option<usize>,
    /// Where it sits among its siblings (`addlast` unless the extension says otherwise).
    #[serde(default)]
    pub place: AddedPlace,
}

/// A data item of a report or of the extension, as a sibling anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemAnchor {
    /// Index into `Report::items`.
    Base(usize),
    /// Index into `ReportExt::added`.
    Added(usize),
}

/// `addfirst`, `addlast`, `addbefore` and `addafter`
/// (reportextensiontrigger/TestReportExtensionDataItemPlacementContracts.al).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddedPlace {
    #[default]
    Last,
    First,
    Before(ItemAnchor),
    After(ItemAnchor),
}

/// A `dataitem` of a report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportItem {
    pub name: String,
    pub parent: Option<usize>,
    /// Global slot of the item's record, and the object index of its table.
    pub slot: Option<usize>,
    pub table: Option<usize>,
    /// `DataItemTableView` as written (`sorting(..) where(..)`).
    pub view: Option<String>,
    /// `DataItemLink` pairs (own field position, parent item field position).
    pub link: Vec<(usize, usize)>,
    /// `DataItemLinkReference`: the enclosing item (an index into `Report::items`) whose current row
    /// the link fields read, when that is not the parent. The pairs then hold field positions of
    /// that item's table.
    #[serde(default)]
    pub link_item: Option<usize>,
    /// `OnPreDataItem`, `OnAfterGetRecord` and `OnPostDataItem` as routine indices.
    pub on_pre: Option<usize>,
    pub on_after_get: Option<usize>,
    pub on_post: Option<usize>,
    /// `RequestFilterFields` as written (`"No.", Status`).
    #[serde(default)]
    pub request_filter_fields: Option<String>,
    /// The columns the dataset writes for each row, in source order.
    #[serde(default)]
    pub columns: Vec<ReportColumn>,
    /// `MaxIteration` of an `Integer` item: the rows after which the item stops (its `OnPostDataItem` still runs).
    #[serde(default)]
    pub max_iteration: Option<u32>,
    /// `PrintOnlyIfDetail = true`: the dataset leaves out a row whose nested items wrote no row.
    #[serde(default)]
    pub print_only_if_detail: bool,
    /// `CalcFields`: the field positions of the FlowFields calculated for each row, before the row's triggers.
    #[serde(default)]
    pub calc_fields: Vec<usize>,
}

/// The direction an xmlport declares (`Direction`, default `Both`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum XmlDirection {
    Import,
    Export,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum XmlNodeKind {
    /// `textelement`: a container when it has children, else it carries a variable's text.
    Text,
    /// `textattribute`: an attribute carrying a variable's text.
    TextAttr,
    /// `tableelement`: one element per record of a table.
    Table,
    /// `fieldelement`: an element carrying a field of the enclosing table element's record.
    Field,
    /// `fieldattribute`: an attribute carrying such a field.
    FieldAttr,
}

/// The `Format` of an xmlport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum XmlFormat {
    #[default]
    Xml,
    VariableText,
    FixedText,
}

/// An xmlport as the runtime sees it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct XmlPort {
    pub direction: XmlDirection,
    #[serde(default)]
    pub format: XmlFormat,
    /// `FieldSeparator`, `FieldDelimiter` and `RecordSeparator` as written, for the text formats
    /// (`None`: the default). `TextEncoding = UTF8` sets `utf8`.
    #[serde(default)]
    pub field_separator: Option<String>,
    #[serde(default)]
    pub field_delimiter: Option<String>,
    #[serde(default)]
    pub record_separator: Option<String>,
    /// `TableSeparator` as written (`None`: the default).
    #[serde(default)]
    pub table_separator: Option<String>,
    #[serde(default)]
    pub utf8: bool,
    /// `Encoding = UTF8` on a Format = Xml port: the export declares UTF-8 and writes UTF-8 bytes with no byte order mark
    /// (corpus xmlport/lt-af-xmlport/AfXmlPortEncoding.al:AF_Encoding_Export). `Encoding = UTF16` is the default.
    #[serde(default)]
    pub xml_utf8: bool,
    /// `InlineSchema = true`: the export writes an XSD inside the root element.
    #[serde(default)]
    pub inline_schema: bool,
    /// `UseLax = true`: an import skips an element of another namespace at the end of a sequence.
    #[serde(default)]
    pub use_lax: bool,
    /// `XmlVersionNo = V11`: the export is the same as V10's; an import of a document that parses is not pinned.
    #[serde(default)]
    pub xml_v11: bool,
    /// `Namespaces`: (prefix, uri) pairs.
    #[serde(default)]
    pub namespaces: Vec<(String, String)>,
    /// A `TextEncoding` property other than UTF8 was declared (its value is not kept).
    #[serde(default)]
    pub text_encoding_declared: bool,
    /// `FormatEvaluate = Xml`: values export and import in their XML forms. The default and
    /// `Legacy` use the display text (xmlport/TestXmlPortFormatEvaluateContracts.al).
    #[serde(default)]
    pub format_evaluate_xml: bool,
    /// The port declares `FormatEvaluate`. Without it an import reads the display text and, when that fails, the XML form
    /// (xmlport/lt-xmlport-query-datatransfer/XpTImportBasic.al:Import_OneItem; an export writes the display text).
    #[serde(default)]
    pub format_evaluate_declared: bool,
    /// Schema nodes in source order; `parent` makes the tree. Top-level nodes have no parent.
    pub nodes: Vec<XmlNode>,
    /// Global slots of the source `InStream` and destination `OutStream` set by
    /// `SetSource` / `SetDestination`.
    pub source_slot: usize,
    pub destination_slot: usize,
    /// Why the runtime cannot run this xmlport yet; empty = it can.
    pub unsupported: Vec<String>,
    /// Why running it (Export or Import) is refused although code that only declares or holds it
    /// is not: a construct the corpus does not pin, which most callers never reach.
    #[serde(default)]
    pub run_unsupported: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct XmlNode {
    /// The name in the schema (the AL variable for text and table nodes).
    pub name: String,
    /// The element or attribute name in the document (`XmlName`, else `name`).
    pub xml_name: String,
    pub kind: XmlNodeKind,
    pub parent: Option<usize>,
    /// Global slot of the Text variable (text nodes) or the record buffer (table nodes).
    pub slot: Option<usize>,
    /// For table nodes: the object index of the table.
    pub table: Option<usize>,
    /// For field nodes: the table node holding the record, and the field's position in the table.
    pub table_node: Option<usize>,
    pub field: Option<usize>,
    /// For nested table nodes: the `LinkTable` node and (own field, link-table field) positions.
    pub link_node: Option<usize>,
    pub link_fields: Vec<(usize, usize)>,
    /// Table node properties (`AutoSave` defaults to true, the others to false, as in AL).
    pub auto_save: bool,
    pub auto_update: bool,
    pub auto_replace: bool,
    pub use_temporary: bool,
    /// `MinOccurs = Zero`: the element may be missing from an imported document.
    #[serde(default)]
    pub min_zero: bool,
    /// `Width` of a field element in a FixedText port.
    #[serde(default)]
    pub width: Option<usize>,
    /// `NamespacePrefix` of a field node: the prefix the port declares in `Namespaces`. A field element is written
    /// `prefix:Name`; a field attribute is written without it (corpus xmlport/lt-y-xmlport-props/YXmlPortProps.al:
    /// NodeProperties_Export_NamesPrefixesAutoCalc).
    #[serde(default)]
    pub ns_prefix: Option<String>,
    /// `AutoCalcField` of a field node as declared (`None`: not declared; a FlowField is calculated then, by an assumption).
    #[serde(default)]
    pub auto_calc: Option<bool>,
    /// `Unbound = true` on a field element: its import is as without it, its export is not pinned
    /// (YXmlPortProps.al:712, BC's export of such a port does not end).
    #[serde(default)]
    pub unbound: bool,
    /// Triggers by lower-case name (`onbeforepassvariable`, ...) to routine index.
    pub triggers: Vec<(String, usize)>,
}

impl XmlNode {
    pub fn trigger(&self, name: &str) -> Option<usize> {
        self.triggers.iter().find(|(n, _)| n == name).map(|(_, r)| *r)
    }
}

/// A query object (corpus query/): a chain of dataitems joined by `DataItemLink`, and the
/// columns they project. Execution is the VM's (`al-vm/src/queries.rs`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Query {
    /// Dataitems in source order; each nests in `parent`.
    pub items: Vec<QueryItem>,
    /// `column(...)` and `filter(...)` elements in source order. A query column is identified by
    /// its position here.
    pub columns: Vec<QueryColumn>,
    /// `OrderBy` as (column, ascending).
    pub order_by: Vec<(usize, bool)>,
    /// The `TopNumberOfRows` property; 0 = all rows.
    pub top: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryItem {
    pub name: String,
    /// Object index of the source table.
    pub table: usize,
    /// The enclosing dataitem (index into `Query::items`); `None` for the root.
    pub parent: Option<usize>,
    /// `SqlJoinType = LeftOuterJoin`, which is also the default of a nested dataitem.
    pub left_outer: bool,
    /// `SqlJoinType = RightOuterJoin` or `FullOuterJoin`: the rows of this dataitem that no row above links to are kept too.
    #[serde(default)]
    pub right_outer: bool,
    /// `DataItemLink`: (field position in this table, field position in the linked item's table).
    pub link: Vec<(usize, usize)>,
    /// For each `link` pair, the enclosing dataitem whose current row it reads, when any pair reads an item
    /// further out than the parent (a link to the grandparent); empty when every pair reads the parent.
    #[serde(default)]
    pub link_items: Vec<usize>,
    /// `DataItemTableFilter`: conditions on fields of this table.
    pub table_filter: Vec<CalcCond>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryMethod {
    Sum,
    Count,
    Average,
    Min,
    Max,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryColumn {
    pub name: String,
    /// The `Caption` property when it is a plain text literal.
    pub caption: Option<String>,
    /// Index into `Query::items`.
    pub item: usize,
    /// Position of the source field in the dataitem's table; `None` for a `Count` column.
    pub field: Option<usize>,
    /// A `filter(...)` element: it filters its dataitem and is not part of the dataset.
    pub filter_element: bool,
    pub method: Option<QueryMethod>,
    pub reverse_sign: bool,
    /// Type of the value the column reads.
    pub ty: RtType,
    /// `ColumnFilter`: a `const` or `filter` condition on this column.
    pub static_filter: Option<CalcValue>,
}

/// A page object as the test client sees it (ADR-012). The AL code of a page
/// (triggers, procedures, and one synthetic routine per expression-valued
/// property) is ordinary routines of the same object; this is the layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page {
    /// The unprobed assumptions the page's layout rests on; the VM marks them when the page opens.
    #[serde(default)]
    pub assumes: Vec<al_builtins::assumptions::Assumption>,
    /// Controls in source order; `parent` makes the tree. `area(...)` sections
    /// are not controls.
    pub controls: Vec<Control>,
    /// Page triggers by lower-case name (`onopenpage`) to routine index.
    pub triggers: Vec<(String, usize)>,
    /// Global slot of `CurrPage`.
    pub curr_page_slot: usize,
    /// Why the test client cannot drive this page yet; empty = it can.
    pub unsupported: Vec<String>,
    /// The page extensions merged into `controls`, in the order they apply: each runs on an
    /// instance of its own that shares the page's `Rec`, `xRec` and `CurrPage`.
    #[serde(default)]
    pub extensions: Vec<usize>,
    /// Page triggers the extensions declare, by lower-case name: (name, extension object, routine).
    /// They run after the page's own trigger of that name.
    #[serde(default)]
    pub ext_triggers: Vec<(String, usize, usize)>,
    /// The `area(...)` sections of `actions` (what the Page Action table lists as action containers).
    #[serde(default)]
    pub action_areas: Vec<ActionArea>,
    /// Why the page is refused on a BC version whose client fails to compile a BigInteger literal in a control expression
    /// (al_version::Gate::BigIntegerLiteralInControlExpressionFrom28); empty = no such expression.
    #[serde(default)]
    pub big_integer_expressions: Vec<String>,
}

/// An `area(Name)` of a page's `actions` section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionArea {
    pub name: String,
    /// The controls written directly in the area, as indices into [`Page::controls`].
    pub roots: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlKind {
    Field,
    Group,
    Repeater,
    Action,
    /// `systemaction(OK)` and friends.
    SystemAction,
    ActionGroup,
    /// A `cuegroup`: a group whose fields take `SetValue` without effect
    /// (corpus handlers/TestPageCueGroupFixedGridContracts.al).
    CueGroup,
    Part,
    /// A data item of a report's request page: its `binding` is the global slot of the item's
    /// record, and the test client reads and writes that record's filters.
    DataItem,
}

/// Where a field control reads and writes its value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Binding {
    None,
    /// A field of the page's source record (position in the table).
    Field(usize),
    /// A global variable of the control's owner: the page, or the extension that declares the control.
    Var(usize),
    /// A global variable of the BASE page or report, which a control of an extension names (a `protected var`).
    /// It lives on the base object's instance whoever owns the control.
    BaseVar(usize),
    /// A read-only expression: a routine returning the value.
    Expr(usize),
    /// A `label` control: its value is its caption and it is not editable
    /// (corpus handlers/TestPageLabelControlContracts.al).
    Caption,
}

impl Binding {
    /// The global slot a variable binding reads and writes and the owner whose instance holds it,
    /// for a control owned by `owner`.
    pub fn var_slot(&self, owner: Option<usize>) -> Option<(usize, Option<usize>)> {
        match self {
            Binding::Var(slot) => Some((*slot, owner)),
            Binding::BaseVar(slot) => Some((*slot, None)),
            _ => None,
        }
    }
}

/// A control property: written as a literal, or an expression compiled to a
/// routine of the page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropVal {
    Lit(String),
    /// A routine of the control's owner (the page, or the extension that declares the control).
    Routine(usize),
    /// A routine of an extension that modified the control: (extension object, routine).
    Ext(usize, usize),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Control {
    pub name: String,
    pub kind: ControlKind,
    pub parent: Option<usize>,
    pub binding: Binding,
    /// Properties by lower-case name (`visible`, `caption`, `decimalplaces`, ...).
    pub props: Vec<(String, PropVal)>,
    /// Control triggers by lower-case name (`onvalidate`, `onaction`, ...) to routine index.
    pub triggers: Vec<(String, usize)>,
    /// For parts: the page object index.
    pub part_page: Option<usize>,
    /// For parts: how the part's rows follow the host's current row.
    #[serde(default)]
    pub part_link: Option<PartLink>,
    /// The page extension that declares the control; `None` for the page's own controls. The
    /// routines and global slots in `binding`, `triggers` and `PropVal::Routine` belong to it.
    #[serde(default)]
    pub owner: Option<usize>,
    /// Triggers that `modify` blocks of page extensions add to the control, by lower-case name
    /// (`onbeforevalidate`, `onaftervalidate`, `ondrilldown`, ...): (name, extension object,
    /// routine), in the order the extensions apply.
    #[serde(default)]
    pub ext_triggers: Vec<(String, usize, usize)>,
    /// For actions: what `RunObject` runs (ADR-012 follow-up).
    #[serde(default)]
    pub run_object: Option<RunObject>,
    /// For `actionref(Name; Target)` controls: the action they stand for.
    #[serde(default)]
    pub action_ref: Option<ActionRef>,
}

/// The action an `actionref` delegates to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionRef {
    /// Index of the action control on the same page.
    Target(usize),
    /// Why the target is not an action of this page the client can follow.
    Unresolved(String),
}

/// The kind of object a `RunObject` names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunKind {
    Page,
    Codeunit,
    Report,
    XmlPort,
    Query,
}

/// `RunPageMode` of an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RunMode {
    #[default]
    Default,
    View,
    Edit,
    Create,
}

/// The `RunObject` property of an action, with the properties that go with it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunObject {
    pub kind: RunKind,
    /// The page or codeunit object; `None` for a report, xmlport or query (the test client
    /// refuses those before it looks at the target) and for a target that did not resolve.
    pub object: Option<usize>,
    /// `RunPageOnRec = true`.
    pub on_rec: bool,
    pub mode: RunMode,
    /// `RunPageLink` as entries (target field, host field or filter) and `RunPageView` as
    /// written; `provider` is unused.
    pub link: Option<PartLink>,
    /// Why the test client cannot run this action yet; `None` = it can.
    pub unsupported: Option<String>,
}

/// `SubPageLink`, `SubPageView` and `Provider` of a part control (ADR-012 follow-up).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PartLink {
    /// `SubPageLink` entries, in source order.
    pub entries: Vec<LinkEntry>,
    /// `SubPageView` as written.
    pub view: Option<String>,
    /// `Provider = <part>`: the control index (on the same page) whose current row the link reads.
    pub provider: Option<usize>,
}

/// One `PartField = field(HostField)` / `const(..)` / `filter(..)` entry of a `SubPageLink`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LinkEntry {
    /// Position of the part's field, position of the host (or provider) field.
    Field(usize, usize),
    /// Position of the part's field, the filter text (`const(x)` and `filter(x)` alike).
    Filter(usize, String),
}

impl Control {
    pub fn prop(&self, name: &str) -> Option<&PropVal> {
        self.props.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }
    pub fn trigger(&self, name: &str) -> Option<usize> {
        self.triggers.iter().find(|(n, _)| n == name).map(|(_, r)| *r)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Table {
    pub fields: Vec<Field>,
    pub keys: Vec<Vec<usize>>,
    /// `Unique = true` per key (parallel to `keys`).
    pub unique_keys: Vec<bool>,
    /// The properties of each key as written (`Clustered`, `Enabled`, `SumIndexFields`, ...),
    /// parallel to `keys`; empty for a key the compiler adds.
    #[serde(default)]
    pub key_props: Vec<Vec<(String, String)>>,
    pub per_company: bool,
    /// Object index of each table extension (their triggers are not run yet).
    pub extensions: Vec<usize>,
    /// Why a use of the table is refused; empty = it is not (a text property longer than its MaxLength: not pinned).
    #[serde(default)]
    pub unsupported: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub id: u32,
    pub name: String,
    pub ty: RtType,
    /// Normal fields; FlowFields and FlowFilters are not stored.
    pub stored: bool,
    /// `InitValue`, if declared and understood.
    pub init: Option<Const>,
    /// `InitValue` as written; set with `init: None` means not understood yet.
    pub init_text: Option<String>,
    /// Lower-case name of the table a `TableRelation` points to.
    pub relation_table: Option<String>,
    /// Lower-case names of every table the relation names (all arms of a conditional one).
    pub relation_tables: Vec<String>,
    pub auto_increment: bool,
    /// The field declares a `TableRelation`.
    pub relation: bool,
    /// (object, routine) of the field's OnValidate trigger, if any.
    pub on_validate: Option<(usize, usize)>,
    /// The `OnBeforeValidate` triggers table extensions attach with `modify(Field)`, as
    /// (object, routine), in extension order.
    #[serde(default)]
    pub ext_before_validate: Vec<(usize, usize)>,
    /// The `OnAfterValidate` triggers table extensions attach with `modify(Field)`.
    #[serde(default)]
    pub ext_after_validate: Vec<(usize, usize)>,
    /// (object, routine) of the field's table-side OnLookup trigger, if any.
    pub on_lookup: Option<(usize, usize)>,
    /// (object, routine) of the routine that evaluates the field's `CaptionClass` when it is an expression: in the table
    /// or table extension whose declaration or `modify` block writes it last (`pages::field_caption_class_routines`).
    #[serde(default)]
    pub caption_class: Option<(usize, usize)>,
    /// Why a read of the field's caption is refused: two or more of the table and its extensions set its
    /// `CaptionClass` and one of them is an expression (which one BC reads is not pinned).
    #[serde(default)]
    pub caption_class_refusal: Option<String>,
    /// (object, routine) of the routine that evaluates the field's `AutoFormatExpression` when it is an expression
    /// (`pages::field_auto_format_routines`), as `caption_class`.
    #[serde(default)]
    pub auto_format_expression: Option<(usize, usize)>,
    /// Why the field's `AutoFormatExpression` is refused: two or more of the table and its extensions set it and one
    /// of them is an expression (which one BC reads is not pinned).
    #[serde(default)]
    pub auto_format_refusal: Option<String>,
    /// Every property of the field as written (`Caption`, `OptionCaption`, ...).
    pub properties: Vec<(String, String)>,
    /// The `TableRelation` arms in written order; empty when the field has none or its shape is
    /// not understood (`relation` tells the two apart).
    pub relation_arms: Vec<RelationArm>,
    /// For a relation the runtime does not understand (`relation` set, `relation_arms` empty):
    /// the tables, fields and condition fields it names, for the Table Relations Metadata table
    /// only. Empty when the relation is understood or not even that is clear.
    #[serde(default)]
    pub relation_meta: Vec<MetaRelationArm>,
    /// For a relation with `Field = filter(x)` conditions (`relation_arms` empty): the arms the runtime applies
    /// when a value is validated. Nothing but validation reads them.
    #[serde(default)]
    pub relation_runtime: Vec<RelationArm>,
    /// The `TableRelation` texts of a field a tableextension's `modify(..)` extended: the base's first, then each
    /// extension's. Empty when the field has at most one. Only the Field platform table reads them.
    #[serde(default)]
    pub relation_texts: Vec<String>,
    /// `FieldClass = FlowFilter`: holds the caller's filter, never a value.
    pub flow_filter: bool,
    /// A FlowField's `CalcFormula`, when lowered. A FlowField with neither
    /// this nor `calc_refusal` declares no formula at all.
    pub calc: Option<CalcFormula>,
    /// Why a declared `CalcFormula` cannot be calculated yet.
    pub calc_refusal: Option<String>,
    /// The object index of the table or table extension that declares the field; `None` for a
    /// system field. Its app is the app that owns the field's column (test data, ADR-020).
    #[serde(default)]
    pub declared_in: Option<u32>,
}

/// One arm of a `TableRelation`: `if (Kind = const(A)) Table.Field where(...)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationArm {
    /// The arm applies when every condition holds (none: always, as the else arm does).
    pub conditions: Vec<RelationCond>,
    /// Object index of the related table; `None` when the workspace does not declare it.
    pub table: Option<usize>,
    /// The related table as written.
    pub table_name: String,
    /// Position of the related field in the related table; `None`: the first primary key field.
    pub field: Option<usize>,
    /// Which related rows are valid; `None` when a where() clause is there but not understood.
    pub wheres: Option<Vec<RelationWhere>>,
    /// The unprobed assumptions the arm rests on; the VM marks them when a value is validated against it.
    #[serde(default)]
    pub assumes: Vec<al_builtins::assumptions::Assumption>,
}

/// An arm of a relation as the Table Relations Metadata table lists it (see [`Field::relation_meta`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaRelationArm {
    /// Positions of the own fields the arm's conditions name.
    pub conditions: Vec<usize>,
    /// Parallel to `conditions`: the condition is `Field = filter(...)`, not `const(...)`.
    #[serde(default)]
    pub filters: Vec<bool>,
    /// Parallel to `conditions`: the Condition Value text the table lists, when it is known: the
    /// ordinal of a `const` option member, the ordinals of a `filter(A | B)` joined by `|`.
    #[serde(default)]
    pub values: Vec<Option<String>>,
    /// Object index of the related table.
    pub table: usize,
    /// Position of the related field; `None`: the first primary key field.
    pub field: Option<usize>,
}

/// `Field = const(x)` on the referencing table: `field` is a position in its own field list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationCond {
    pub field: usize,
    /// The constant of `Field = const(x)`; for a `filter` condition the filter text as written.
    pub value: Const,
    /// `Field = filter(text)`: the arm applies when the field's value passes the filter `value` holds.
    #[serde(default)]
    pub filter: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RelationWhere {
    /// `related = const(text)` / `filter(text)`: `text` is a filter on the related field.
    Value { related: usize, text: String },
    /// `related = field(own)`: the related field equals the referencing row's field.
    Field { related: usize, own: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CalcMethod {
    Sum,
    Count,
    Average,
    Min,
    Max,
    Exist,
    Lookup,
}

/// A FlowField's `CalcFormula` with every name resolved to positions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalcFormula {
    pub method: CalcMethod,
    /// A leading `-`: negates a sum, count, average, ... and `not`s an exist.
    pub negate: bool,
    /// Object index of the source table.
    pub table: usize,
    /// Position of the aggregated or looked-up field in the source table.
    pub field: Option<usize>,
    pub conds: Vec<CalcCond>,
}

/// One where-condition: a field of the source table against a value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalcCond {
    /// Position in the source table.
    pub field: usize,
    pub value: CalcValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CalcValue {
    /// `const(..)`, typed as the source field.
    Const(Const),
    /// `filter(..)`: a filter expression over the source field's type, with
    /// AL double-quoted names rewritten as single-quoted literals.
    Filter(String),
    /// `field(X)`: position of X in the table being calculated.
    Field(usize),
    /// `field(upperlimit(X))`.
    UpperLimit(usize),
    /// `field(filter(X))`: X's value read as a filter expression.
    FieldFilter(usize),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Slot {
    pub name: String,
    pub ty: RtType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Routine {
    pub name: String,
    /// For field triggers: the field position in the table.
    pub field: Option<usize>,
    pub is_trigger: bool,
    pub attributes: Vec<Attribute>,
    /// Parameters occupy the first locals; `by_var[i]` for parameter i.
    pub params: usize,
    pub by_var: Vec<bool>,
    /// All locals: parameters, then the named return value (if any), then declared locals.
    pub locals: Vec<Slot>,
    pub ret: Option<RtType>,
    /// Local index of a named return value.
    pub ret_local: Option<usize>,
    /// The statements; decoded on first read when the routine comes from a compact form (see [`Body`]).
    pub body: Body,
    /// Position of the `procedure` or `trigger` keyword ([`NO_POS`] for a routine without source).
    /// BC counts the line of a call stack frame from this line.
    #[serde(default = "no_pos")]
    pub pos: u32,
    /// Position of the `end` that closes the body: BC shows an event publisher's frame at this
    /// line ([`NO_POS`] without source).
    #[serde(default = "no_pos")]
    pub end_pos: u32,
    /// Position of the `begin` that opens the body (the Code Coverage table lists it as an
    /// `Empty` line); [`NO_POS`] without source.
    #[serde(default = "no_pos")]
    pub begin_pos: u32,
    /// The nearest enclosing section of a trigger that is not at the object's own level: its keyword
    /// in lower case and its first argument (`("action", "PageAction")`, `("dataitem", "Row")`).
    /// BC names such a trigger in a call stack as `"<name> - <trigger>"`.
    #[serde(default)]
    pub container: Option<(String, String)>,
    /// The static type of each expression of the body, in post-order (see [`types`]). Empty: every one is unknown.
    #[serde(default)]
    pub expr_tys: Vec<types::ExprTy>,
    /// The body reads MaxStrLen of a `var` Text or Code parameter (`Text.MaxStrLenOfVarParameter#1`), or hands one on to a
    /// `var` Text or Code parameter of a call: a call hands the routine the declared lengths of its arguments, which no
    /// other call pays for.
    #[serde(default)]
    pub reads_var_max_len: bool,
}

fn no_pos() -> u32 {
    NO_POS
}

impl Routine {
    /// Moves every source position id of the routine (its own and its `Stmt::Pos` markers) up by
    /// `by` (wrapping, so a `by` made from a negative distance moves them down): the object was lowered with positions numbered from 0 and now follows `by` positions
    /// of other objects in `Program::positions`. [`NO_POS`] stays.
    pub fn shift_positions(&mut self, by: u32) {
        if self.pos != NO_POS {
            self.pos = self.pos.wrapping_add(by);
        }
        if self.end_pos != NO_POS {
            self.end_pos = self.end_pos.wrapping_add(by);
        }
        if self.begin_pos != NO_POS {
            self.begin_pos = self.begin_pos.wrapping_add(by);
        }
        if self.body.is_loaded() {
            shift_stmts(&mut self.body, by);
        } else {
            self.body
                .fix(std::sync::Arc::new(ShiftPositions(by)))
                .expect("a shift cannot fail");
        }
    }
}

/// [`Routine::shift_positions`] of a body that is not decoded yet.
struct ShiftPositions(u32);

impl BodyFixup for ShiftPositions {
    fn apply(&self, body: &mut Vec<Stmt>) -> Result<(), String> {
        shift_stmts(body, self.0);
        Ok(())
    }
}

pub(crate) fn shift_stmts(stmts: &mut [Stmt], by: u32) {
    for s in stmts {
        match s {
            Stmt::Pos(id) => *id = id.wrapping_add(by),
            Stmt::If(_, then, otherwise) => {
                shift_stmts(then, by);
                shift_stmts(otherwise, by);
            }
            Stmt::While(_, body) | Stmt::Repeat(body, _) | Stmt::AssertError(body) => shift_stmts(body, by),
            Stmt::For { body, .. } | Stmt::Foreach { body, .. } => shift_stmts(body, by),
            Stmt::Case {
                branches, otherwise, ..
            } => {
                for (_, body) in branches {
                    shift_stmts(body, by);
                }
                shift_stmts(otherwise, by);
            }
            Stmt::Assign(..) | Stmt::Expr(_) | Stmt::Exit(_) | Stmt::Break | Stmt::Continue | Stmt::Assume(_) => {}
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub args: Vec<Const>,
    /// The attribute as written (`[EventSubscriber(ObjectType::Table, ...)]`).
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Const {
    Boolean(bool),
    Integer(i32),
    BigInteger(i64),
    /// Decimal as its exact text, to keep the IR free of float rounding.
    Decimal(String),
    Text(String),
    /// Day number (1 = 0001-01-01), 0 for `0D`.
    Date(i32),
    /// Milliseconds since midnight plus one, 0 for `0T`.
    Time(i64),
    DateTime(i64),
    Option(i32, Vec<String>),
    Enum(u32, i32),
    /// A date formula as written (`1Y`), already checked to be one the runtime can calculate with.
    DateFormula(String),
    /// A Guid (a `const('..')` of a CalcFormula on a Guid field).
    Guid([u8; 16]),
    /// A field reference passed to a record method (`SetRange(Field, ...)`): its position.
    FieldPos(u32),
}

/// Where an assignment writes or a `var` argument points.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Place {
    Local(usize),
    Global(usize),
    /// Global slot `n` of the *base object's instance*, in a routine of a page or report extension
    /// (ADR-023). The lowering emits it nowhere else. The slot is the variable's position in the
    /// base object's globals.
    BaseGlobal(usize),
    /// Element of an array place.
    Index(Box<Place>, Vec<Expr>),
    /// Field at position `.1` of the record in place `.0`.
    Field(Box<Place>, usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    IntDiv,
    Mod,
    And,
    Or,
    Xor,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    /// `=` and `<>` with a `DotNet` operand (ADR-024): a CLR string compares case sensitively
    /// (tests/al-language-onprem-dotnet/TestDotNetValueKindsContracts.al, codeunit 61400,
    /// `IntegerProperties_TakePartInIntegerExpressions_AndStringCompareIsCaseSensitive`);
    /// any other pair of kinds compares as `Eq` and `Ne` do.
    DotNetEq,
    DotNetNe,
    /// `<`, `<=`, `>`, `>=` with a `DotNet` operand: two .NET enum members by number (the same enum), two texts only when
    /// every reading of the order agrees (see `dotnet_order`), anything else as AL does.
    DotNetLt,
    DotNetLe,
    DotNetGt,
    DotNetGe,
}

/// How a settled `DotNet` operand may be used (ADR-024).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperandUse {
    /// Arithmetic and logic: a primitive, and a null `string` as the empty text.
    Value,
    /// `+`, `=`, `<>`: as `Value`, and a .NET enum is its number.
    Enum,
    /// A `Variant` parameter: a primitive, a .NET object or enum as the handle, a null `string`
    /// as the empty Variant.
    Variant,
    /// An argument for a scalar parameter of a builtin: a primitive only. A null is refused
    /// because `StrLen(null string)` raises an error `asserterror` cannot catch.
    Strict,
    /// An operand of `<`, `<=`, `>`, `>=`: a .NET enum stays an enum (the
    /// comparison orders members of one enum by number), a text stays a text.
    Order,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnOp {
    Neg,
    Plus,
    Not,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Arg {
    Value(Expr),
    /// Passed to a `var` parameter.
    Ref(Place),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    Const(Const),
    Load(Place),
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    /// `x in [a, b..c]`
    In(Box<Expr>, Vec<SetElem>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    /// Implicit or explicit conversion to a slot type.
    Convert(RtType, Box<Expr>),
    /// A procedure of object `object` (index into `Program::objects`) called
    /// on the current instance (`instance` = None) or on a codeunit value.
    /// `try_call`: the target is a `[TryFunction]` whose result is used, so an
    /// error is caught and the call yields `false`.
    Call {
        object: usize,
        routine: usize,
        instance: Option<Box<Expr>>,
        args: Vec<Arg>,
        try_call: bool,
    },
    /// A call through an interface: `receiver` evaluates to the codeunit instance the
    /// interface variable holds, and the call runs the target routine of that
    /// instance's object. `targets` are (object, routine) of every implementation.
    InterfaceCall {
        receiver: Box<Expr>,
        targets: Vec<(usize, usize)>,
        args: Vec<Arg>,
    },
    /// The instance of the page or report that the running extension extends (ADR-023). A call
    /// with `instance: Some(BaseInstance)` runs a procedure of the base object on it.
    BaseInstance,
    /// `this` in a codeunit: the running instance as a Codeunit value.
    ThisInstance,
    /// Code that lowers but refuses when it runs, with the reason (a call of a control add-in procedure:
    /// control add-ins are not modelled).
    Refused(String),
    /// Code that lowers and raises an AL error with this text and code when it runs (`this.<protected variable of the
    /// base report>` in a reportextension: BC compiles it and raises when it runs).
    Fail {
        message: String,
        code: String,
    },
    /// `value is I` (`cast` false: a Boolean) or `value as I` (`cast` true: the same interface
    /// value) on an interface value. `objects` are the codeunits that implement `I`.
    InterfaceTest {
        value: Box<Expr>,
        objects: Vec<u32>,
        cast: bool,
    },
    /// An enum value cast to an interface: a fresh instance of the codeunit that
    /// implements the interface for the value's ordinal. `by_ordinal` has every declared
    /// ordinal of the enum (extensions included), already resolved through the value's own
    /// `Implementation` and the enum's `DefaultImplementation`; `unknown` is the
    /// `UnknownValueImplementation` for ordinals the enum does not declare.
    EnumImpl {
        value: Box<Expr>,
        by_ordinal: Vec<(i32, usize)>,
        unknown: Option<usize>,
    },
    /// A control of the test page `page` evaluates to (`TP.Ctl`), by index
    /// into the page's control list (ADR-012).
    Control {
        page: Box<Expr>,
        control: usize,
        /// `CurrPage.<Control>` in the page's own code, not a test page variable: a part that is not visible
        /// is still there for the page's code.
        #[serde(default)]
        runtime: bool,
    },
    /// Field `pos` of the record `base` evaluates to (`GetLocation(Code)."Bin Mandatory"`: a field of a
    /// procedure's result, which is not a place).
    FieldOf(Box<Expr>, usize),
    /// The current row's value of column `column` of a query variable (`Q.Amount`).
    QueryColumn {
        query: Box<Expr>,
        column: usize,
    },
    /// A member of a `DotNet` value (ADR-014). `ty` is the declared alias of the receiver
    /// variable (`None` when the receiver is the result of another DotNet call); a receiver
    /// that holds no object is the static member access of the declared type. Plain variable
    /// arguments are `Arg::Ref` so a by-reference .NET parameter can write them back. The result
    /// has no static type: an assignment wraps it in `DotNetCoerce`.
    DotNet {
        ty: Option<usize>,
        receiver: Option<Box<Expr>>,
        op: DotNetOp,
        args: Vec<Arg>,
    },
    /// `Text[i]` read: the character at the 1-based position `.1` of the text `.0`, as a Char. A
    /// position outside the text is refused when it runs (the error text is not pinned).
    CharAt(Box<Expr>, Box<Expr>),
    /// A DotNet result stored into a slot of this type. The kinds must match exactly (Text and
    /// Code, Integer and BigInteger and Decimal widen); anything else is Unsupported, because
    /// BC's conversion of a .NET result to a differently typed AL variable is not pinned.
    DotNetCoerce(RtType, Box<Expr>),
    /// A `DotNet` value used as an operand (ADR-024): the run-time kind of the value decides what
    /// it is. The kinds the host returns by value pass; the rest depends on the use.
    DotNetOperand(Box<Expr>, OperandUse),
    /// A catalog builtin by overload id (`Record.Insert#2`), with its receiver.
    /// `ret_used`: the result is used, so methods with an optional Boolean
    /// result report failure as `false` instead of raising an error.
    Builtin {
        id: BuiltinId,
        receiver: Option<Box<Arg>>,
        args: Vec<Arg>,
        ret_used: bool,
    },
    /// `--tdd` (ADR-051 decision 4): a call of a procedure that does not exist yet, in a test app. The arguments are
    /// evaluated left to right, as for a call, and the result is the initial value of `ty` (`None`: the call is a
    /// statement and yields nothing). It writes nothing, raises nothing and runs no trigger or event. `stub` is the
    /// signature the report names (`Program::tdd_stubs` lists them all).
    TddStub {
        stub: String,
        args: Vec<Expr>,
        ty: Option<RtType>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SetElem {
    One(Expr),
    Range(Option<Expr>, Option<Expr>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Stmt {
    Assign(Place, Expr),
    Expr(Expr),
    If(Expr, Vec<Stmt>, Vec<Stmt>),
    While(Expr, Vec<Stmt>),
    Repeat(Vec<Stmt>, Expr),
    For {
        var: Place,
        from: Expr,
        to: Expr,
        down: bool,
        body: Vec<Stmt>,
    },
    Foreach {
        var: Place,
        collection: Expr,
        body: Vec<Stmt>,
    },
    Case {
        scrutinee: Expr,
        branches: Vec<(Vec<SetElem>, Vec<Stmt>)>,
        otherwise: Vec<Stmt>,
    },
    Exit(Option<Expr>),
    Break,
    AssertError(Vec<Stmt>),
    /// Marks the statement that follows it in the same list: the VM records the position as the
    /// routine's current one (`Program::positions[id]`). It does nothing else.
    Pos(u32),
    /// The statement that follows runs code that rests on an unprobed assumption of the lowering
    /// (`al_builtins::assumptions`): the VM marks the assumption when this runs and does nothing else.
    Assume(al_builtins::assumptions::Assumption),
    /// `continue`: skip the rest of the loop body and go on with the next round.
    Continue,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn routine_with(body: Vec<Stmt>, pos: u32) -> Routine {
        Routine {
            name: "R".into(),
            field: None,
            container: None,
            is_trigger: false,
            attributes: vec![],
            params: 0,
            by_var: vec![],
            locals: vec![],
            ret: None,
            ret_local: None,
            pos,
            end_pos: NO_POS,
            begin_pos: NO_POS,
            body: body.into(),
            expr_tys: vec![],
            reads_var_max_len: false,
        }
    }

    #[test]
    fn shifting_positions_reaches_every_nested_marker_and_leaves_no_position_alone() {
        let t = Expr::Const(Const::Boolean(true));
        let marked = |id| vec![Stmt::Pos(id), Stmt::Break];
        let mut r = routine_with(
            vec![
                Stmt::Pos(0),
                Stmt::If(t.clone(), marked(1), marked(2)),
                Stmt::While(t.clone(), marked(3)),
                Stmt::Repeat(marked(4), t.clone()),
                Stmt::For {
                    var: Place::Local(0),
                    from: t.clone(),
                    to: t.clone(),
                    down: false,
                    body: marked(5),
                },
                Stmt::Foreach {
                    var: Place::Local(0),
                    collection: t.clone(),
                    body: marked(6),
                },
                Stmt::Case {
                    scrutinee: t.clone(),
                    branches: vec![(vec![], marked(7)), (vec![], marked(8))],
                    otherwise: marked(9),
                },
                Stmt::AssertError(vec![Stmt::If(t.clone(), marked(10), vec![])]),
            ],
            11,
        );
        r.shift_positions(100);
        assert_eq!(r.pos, 111);
        let json = serde_json::to_string(&r.body).unwrap();
        let mut ids: Vec<u32> = json
            .split("{\"Pos\":")
            .skip(1)
            .map(|rest| rest.split('}').next().unwrap().parse().unwrap())
            .collect();
        ids.sort_unstable();
        assert_eq!(ids, (100..=110).collect::<Vec<u32>>());
        let mut none = routine_with(vec![Stmt::Break], NO_POS);
        none.shift_positions(5);
        assert_eq!(none.pos, NO_POS);
    }

    #[test]
    fn program_round_trips_through_serde() {
        let p = Program {
            dotnet_types: vec![DotNetDecl {
                alias: "Convert".into(),
                assembly: "netstandard".into(),
                version: None,
                culture: Some("neutral".into()),
                public_key_token: None,
                type_name: "System.Convert".into(),
            }],
            apps: vec![],
            tdd_stubs: vec!["C: procedure \"P\"()".into()],
            sources: vec![],
            positions: vec![],
            uncompiled_subscribers: vec![UncompiledSubscriber {
                publisher: Some(0),
                platform: PlatformPublisher::None,
                event: "onafterinsertevent".into(),
                element: String::new(),
                subscriber: "codeunit \"S\"".into(),
            }],
            objects: vec![Object {
                kind: ObjKind::Codeunit,
                uncompiled: false,
                id: Some(50100),
                name: "C".into(),
                globals: vec![Slot {
                    name: "G".into(),
                    ty: RtType::Integer,
                }],
                properties: vec![("Subtype".into(), "Test".into())],
                enum_values: vec![],
                enum_captions: vec![],
                table: None,
                rec_slot: None,
                xrec_slot: None,
                page: None,
                app: 0,
                query: None,
                xmlport: None,
                report: None,
                report_ext: None,
                namespace: None,
                extends_id: None,
                layouts: Vec::new(),
                routines: vec![Routine {
                    name: "P".into(),
                    field: None,
                    container: None,
                    is_trigger: false,
                    attributes: vec![Attribute {
                        name: "Test".into(),
                        args: vec![],
                        text: "[Test]".into(),
                    }],
                    params: 0,
                    by_var: vec![],
                    locals: vec![Slot {
                        name: "L".into(),
                        ty: RtType::Text(None),
                    }],
                    ret: None,
                    ret_local: None,
                    pos: NO_POS,
                    end_pos: NO_POS,
                    begin_pos: NO_POS,
                    expr_tys: vec![],
                    reads_var_max_len: false,
                    body: vec![
                        Stmt::Pos(0),
                        Stmt::Assign(
                            Place::Global(0),
                            Expr::Binary(
                                BinOp::Add,
                                Box::new(Expr::Const(Const::Integer(1))),
                                Box::new(Expr::Load(Place::Global(0))),
                            ),
                        ),
                        Stmt::Expr(Expr::DotNetCoerce(
                            RtType::Text(None),
                            Box::new(Expr::DotNet {
                                ty: Some(0),
                                receiver: Some(Box::new(Expr::Load(Place::Global(0)))),
                                op: DotNetOp::Call("ToBase64String".into()),
                                args: vec![Arg::Ref(Place::Local(0))],
                            }),
                        )),
                        Stmt::AssertError(vec![Stmt::Expr(Expr::Builtin {
                            id: al_builtins::builtin_id::ids::DIALOG_ERROR_2,
                            receiver: None,
                            args: vec![Arg::Value(Expr::Const(Const::Text("x".into())))],
                            ret_used: false,
                        })]),
                    ]
                    .into(),
                }],
            }],
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: Program = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }

    /// ADR-023: the two variants for base-object access serialize like the rest of the IR.
    #[test]
    fn base_access_variants_round_trip() {
        let e = Expr::Call {
            object: 1,
            routine: 2,
            instance: Some(Box::new(Expr::BaseInstance)),
            args: vec![Arg::Ref(Place::Index(
                Box::new(Place::BaseGlobal(3)),
                vec![Expr::Load(Place::BaseGlobal(4))],
            ))],
            try_call: false,
        };
        let json = serde_json::to_string(&e).unwrap();
        let back: Expr = serde_json::from_str(&json).unwrap();
        assert_eq!(back, e);
        let s = Stmt::Assign(Place::BaseGlobal(0), Expr::Const(Const::Integer(1)));
        let back: Stmt = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
    }
}
