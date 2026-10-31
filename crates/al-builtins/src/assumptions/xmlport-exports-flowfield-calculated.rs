XmlportExportsFlowfieldCalculated => {
    id: "xmlport-exports-flowfield-calculated",
    assumed: "An XmlPort field element or attribute bound to a FlowField exports the calculated value, as if CalcFields had run on the row. (An Xml import skips a Lookup, Sum or Count FlowField element: pinned by xmlport/lt-ay-xmlport/AyXmlPortLookupFlowField.al and XmlPortFlowFieldSumCount.al.)",
    tests: "Import / Export Workflow (\"Workflow Step Argument\".\"Response Option Group\", a Lookup FlowField); 76 Tests-Workflow tests.",
    probe: "An export-only XmlPort with a fieldelement over a Sum FlowField and over a Lookup FlowField of a row with children: the exported text with and without a CalcFields on the row before; an import of the same element. Asked in corpus #650.",
}
