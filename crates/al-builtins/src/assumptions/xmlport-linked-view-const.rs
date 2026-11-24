XmlportLinkedViewConst => {
    id: "xmlport-linked-view-const",
    assumed: "XmlPort import (Format = Xml) of a table element linked (LinkTable) with one LinkFields pair to the table element right above it, which is right below the root text element and has AutoSave without AutoUpdate, AutoReplace or UseTemporary; the element itself MinOccurs = Zero, AutoSave without AutoUpdate, AutoReplace or UseTemporary, the first of its rows under the parent row, whose SourceTableView is one WHERE clause with one const filter on an Integer field that no node below the element carries and that is not a link field, with only field attributes (fieldattribute) of its own without triggers below it, and whose only trigger is an OnAfterInitRecord that calls no procedure of the port and uses the port only to call Skip; no routine of the port names the element's record: when the row is not skipped and the field holds the filter's value before and after the save, the row is inserted as without the view. A skip or another value at either point refuses.",
    tests: "No Microsoft test. An outside report (al-runner issue 8877) names this element shape: the suite-level Evaluator element of the AI Test Toolkit's XmlPort 149031.",
    probe: "Corpus PR 1550, tests/al-language/xmlports/shapes-properties/XpAutoUpdateAssign.al Suite_Fresh_SuiteEvaluator (element SuiteEval, `where(LineNo = const(0))`, LineNo not carried, initial value 0: row `EA/0/e0`); measured identical on the three supported majors.",
    awaiting: Awaiting {
        issues: &[8877],
        files: &["tests/al-language/xmlports/shapes-properties/XpAutoUpdateAssign.al"],
        since: "2026-11-23",
    },
}
