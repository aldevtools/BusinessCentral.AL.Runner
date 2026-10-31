JsonOtherTextAsBooleanConversionError => {
    id: "json-other-text-as-boolean-conversion-error",
    assumed: "JsonValue.AsBoolean (and the other Boolean getters) of a JSON string that is not a spelling of true or false and is made of other characters than letters only or digits only (blanks inside, punctuation, a mix, as 'not a boolean') raises the same conversion error as a text of letters. Letters, digits and the empty text are pinned (json/JsonValueGetters.al:172 `text_asboolean`, UN_BooleanTextGetters_2 to _4).",
    tests: "No Microsoft test: an outside ISV app test that feeds a JSON property of the wrong type to a TryFunction around AsBoolean; it passes on the reporter's real-BC run.",
    probe: "JsonValue.AsBoolean of the strings 'not a boolean', 'a1', '-', 'true!': the result or the error text and code.",
}
