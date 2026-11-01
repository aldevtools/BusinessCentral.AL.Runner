//! Feature Key reads that differ from an ordinary listing: Get matches the ID in any case.

use super::mat::Mat;
use al_builtins::value::Value;
use al_data::{Row, StoreError, TableDef};

/// Get of a Feature Key row: the ID matches the listed row's in any case, and the row shows the listed casing; SetRange and
/// SetFilter stay case-sensitive (record/lt-y48-feature-key-writes/Y48FeatureKeyWrites.al:104-122 FK_Get_OtherCasing, the
/// shape of Microsoft's Web Service Management tests, which Get 'DisableSOAPwebservicesonMicrosoftUIpages').
pub(super) fn get_ignoring_case(m: &Mat, t: &TableDef, pk: &[Value]) -> Result<Option<Row>, StoreError> {
    if let Some(row) = m.get(t, pk)? {
        return Ok(Some(row));
    }
    let [Value::Text(want) | Value::Code(want)] = pk else {
        return Ok(None);
    };
    let listed = (0..)
        .map_while(|i| m.row_at(i))
        .map(|r| t.primary_key(r))
        .find(|k| matches!(k.as_slice(), [Value::Text(id) | Value::Code(id)] if id.as_str().eq_ignore_ascii_case(want.as_str())));
    match listed {
        Some(key) => m.get(t, &key),
        None => Ok(None),
    }
}
