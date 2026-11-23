XmlportLinkedViewConst => {
    id: "xmlport-linked-view-const",
    assumed: "XmlPort import (Format = Xml) of a table element linked (LinkTable) to the table element right above it, which is right below the root text element, AutoSave without AutoUpdate, AutoReplace or UseTemporary, whose SourceTableView is one WHERE clause with one const filter, on a field that no node below the element carries and that is not a link field, with only field attributes (fieldattribute) without triggers below it, and whose only trigger is an OnAfterInitRecord that neither names its record nor calls a procedure of the port: when the row is not skipped and the field holds the filter's value before and after the save, the row is inserted as without the view. A skip or another value at either point refuses.",
    tests: "No Microsoft test. An outside report (al-runner issue 8877) names this element shape: the suite-level Evaluator element of the AI Test Toolkit's XmlPort 149031.",
    probe: "Corpus PR 1550, tests/al-language/xmlports/shapes-properties/XpAutoUpdateAssign.al Suite_Fresh_SuiteEvaluator (element SuiteEval, `where(LineNo = const(0))`, LineNo not carried, initial value 0: row `EA/0/e0`); measured identical on BC 27.5, 28.5 and 29.0.",
    awaiting: Awaiting {
        issues: &[8877],
        files: &["tests/al-language/xmlports/shapes-properties/XpAutoUpdateAssign.al"],
        since: "2026-11-23",
    },
}
