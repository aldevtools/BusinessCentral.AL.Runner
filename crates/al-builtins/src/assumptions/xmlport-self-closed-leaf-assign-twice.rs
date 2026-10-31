XmlportSelfClosedLeafAssignTwice => {
    id: "xmlport-self-closed-leaf-assign-twice",
    assumed: "XmlPort import of a leaf text element (no attributes or elements in the schema) written `<x />` runs its OnAfterAssignVariable twice, the variable set to the empty text before each run; `<x></x>`, `<x> </x>` and `<x>v</x>` run it once, and white space alone (`<x> </x>`) gives the empty text. Pinned below the root, below a table element and inside a text element below a table element (xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TR_EmptyLeaf_Import); assumed also for leaf text elements inside deeper text elements read the same way. Whether the variable is set again before the second run is not pinned (the corpus triggers only read it).",
    tests: "No Microsoft test: a partner app's SOAP XmlPorts import replies with empty leaf elements whose triggers log or count.",
    probe: "TR_EmptyLeaf_Import with the leaf two text elements deep, and a trigger that changes its own variable on the first run.",
}
