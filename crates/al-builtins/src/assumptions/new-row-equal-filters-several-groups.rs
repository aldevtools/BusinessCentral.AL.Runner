NewRowEqualFiltersSeveralGroups => {
    id: "new-row-equal-filters-several-groups",
    assumed: "A new page row takes the value of a field that has equality filters in several filter groups when they all name the same value; different values are not stamped.",
    tests: "Payment Journal and Incom. Doc. Attach. FactBox tests of Tests-General Journal (Cash Receipt Journal).",
    probe: "Open a page on a record with SetRange(F, 'A') in group 0 and group 2, insert a row, read F. Asked in corpus #641.",
}
