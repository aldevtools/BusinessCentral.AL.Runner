use super::text::{split_variable_marked, text_separator};

// xmlports/TestXmlPortTextFormatsContracts.al:VariableText_Import_QuotedUnquotedAndBlankValues
#[test]
fn a_variable_text_record_splits_into_its_values() {
    let split = |l: &str| -> Vec<String> {
        split_variable_marked(l, ",", "\"")
            .into_iter()
            .map(|(v, _)| v)
            .collect()
    };
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

// metadata/session-system/P94XpDelim.al:Getters_* (escaped alike); xmlports/shapes-properties/Y88XmlPortProbes.al:Y88_VariableText_DeclaredSeparatorGetters (real characters);
// xmlports/shapes-properties/XpSeparatorGetterTokens.al:105-118 (lower-case tokens and <> are text, tokens inside a longer text are read)
#[test]
fn a_declared_separator_reads_back_as_its_characters() {
    let read = |v: &str| text_separator(Some(v), ",");
    assert_eq!(read("<CR/LF>").unwrap(), "\r\n");
    assert_eq!(read("<LF>").unwrap(), "\n");
    assert_eq!(read("<None>").unwrap(), "");
    assert_eq!(read(";").unwrap(), ";");
    assert_eq!(read("<tab>").unwrap(), "<tab>");
    assert_eq!(read("<>").unwrap(), "<>");
    assert_eq!(read(";<TAB>").unwrap(), ";\t");
    assert!(read("<Space>").is_err());
}
