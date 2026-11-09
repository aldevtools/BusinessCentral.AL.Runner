EnumOverloadInteger => {
    id: "enum-overload-integer",
    assumed: "Of two procedure overloads that differ only where one declares an Integer parameter and the other an enum, an Integer argument goes to the Integer overload and an argument of that enum to the enum overload, although each also converts to the other (PnE13EnumConv.al:70-73; compile matrix). Pinned for an Option parameter against an enum one (codeunits/CwInterfaceLeads.al:143-167 CW_Overload_OptionVsEnum); other shapes where two overloads take an enum argument are refused.",
    tests: "System Application, codeunit Email Account Impl. (8889): IsValidConnector(Connector: Enum \"Email Connector\") and IsValidConnector(Connector: Integer), reached by the Email tests and by Company-Initialize through Global Triggers OnDatabaseInsert.",
    probe: "Extend codeunits/CwInterfaceLeads.al CW_Overload_OptionVsEnum with Integer-against-enum overloads (Integer variable, literal, Option, BigInteger and enum arguments), enum A against enum B with AssignmentCompatibility (argument of A, of B, of a third enum), var parameters, and must-not-compile cases where BC reports an ambiguous call.",
}
