/// reports/dataset/ReportDatasetHeaderRaw.al:174 and text/PnBReportSaveAs.al:162: the
/// header's ExtensionName is the app's name, " by " and its publisher.
#[test]
fn the_extension_name_of_the_header_names_the_publisher() {
    assert_eq!(
        super::header_extension_name("AL Language Coverage Tests", "AL Language"),
        "AL Language Coverage Tests by AL Language"
    );
}
