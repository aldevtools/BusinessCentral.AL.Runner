use super::{declared_separator, split_variable};

// xmlport/TestXmlPortTextFormatsContracts.al:VariableText_Import_QuotedUnquotedAndBlankValues
#[test]
fn a_variable_text_record_splits_into_its_values() {
    let split = |l: &str| split_variable(l, ",", "\"");
    assert_eq!(
        split("\"1\",\"a,b\",\"1,234.50\",\"03/05/24\""),
        ["1", "a,b", "1,234.50", "03/05/24"]
    );
    assert_eq!(split("\"2\",\"plain\",\"0\",\"\""), ["2", "plain", "0", ""]);
    assert_eq!(split("3,unquoted,5.5,01/02/24"), ["3", "unquoted", "5.5", "01/02/24"]);
    // A doubled delimiter is no escape: the text after the closing one starts the next value.
    assert_eq!(
        split("\"2\",\"say \"\"hi\"\"\",\"0\",\"\""),
        ["2", "say ", "hi", "", "0", ""]
    );
}

// platform/lt-xmlport-session-navapp-cookie/P94XpDelim.al:Getters_Declared, Getters_DeclaredTwoTables
#[test]
fn a_declared_separator_reads_back_as_backslash_text() {
    assert_eq!(declared_separator("<CR/LF>").unwrap(), "\\r\\n");
    assert_eq!(declared_separator("<LF>").unwrap(), "\\n");
    assert_eq!(declared_separator("<None>").unwrap(), "");
    assert_eq!(declared_separator(";").unwrap(), ";");
    assert!(declared_separator("<Space>").is_err());
}
