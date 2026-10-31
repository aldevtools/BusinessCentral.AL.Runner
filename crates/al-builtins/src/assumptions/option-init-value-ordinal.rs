OptionInitValueOrdinal => {
    id: "option-init-value-ordinal",
    assumed: "An Option field whose InitValue is a number that names no member (`InitValue = 1`) starts at the member of that ordinal: on a fresh variable, after Init and after Insert. Measured on Windows BC of the newest major (corpus branch winprobe-wc6, tests/wc6-probe/ContosoProbe.al, cu 61320 CS_OptionInitValueOrdinal: Sunday..Saturday with InitValue = 1 gives Monday/1 three times). A number that is also the name of a member at another ordinal stays refused.",
    tests: "The Contoso company build of Microsoft's IntegrationTest and Uncategorized lanes for apps that pull in Power BI Report embeddings: its Accounting Period table extension reads PowerBI Reports Setup (\"First Day Of Week\", InitValue = 1) from Create Accounting Period, and the build runs in Microsoft's CI.",
    probe: "A corpus test of the measured case (fresh variable, Init, Insert and Get) on all three majors; the same with InitValue = 0 and with a number past the last member.",
}
