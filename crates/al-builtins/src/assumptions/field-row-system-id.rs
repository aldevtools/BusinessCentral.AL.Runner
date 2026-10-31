FieldRowSystemId => {
    id: "field-row-system-id",
    assumed: "The SystemId of the Field rows of SystemModifiedAt and SystemModifiedBy (fields 2000000003-4) follows the rule measured for the other rows on the majors of gate FieldRowSystemIdFrom28: the Guid of the bytes of 2000000041, the table number and the field number, each little-endian, then four zero bytes (platform/lt-field-row-systemid/FieldRowSystemId.al:49-170, platform/lt-wc/WcFieldRowCopy.al:48, :60-63). Before that gate every row has the null Guid (pinned).",
    tests: "No Microsoft test: a partner app's field picker page buffers Field rows of any table in a temporary Field record.",
    probe: "The SystemId of the Field rows of fields 2000000003 and 2000000004 of an own table, on every supported major.",
}
