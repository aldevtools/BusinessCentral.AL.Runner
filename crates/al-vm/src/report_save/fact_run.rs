//! The facts known after the run: whether the run wrote a dataset row, whether the dataset holds `rgba(`, and the selection
//! rows read again.

use super::Schema;
use super::facts::{Facts, Rows};
use crate::{Session, Signal};

impl Session {
    /// `facts` with the run facts of a run that wrote `rows` dataset rows and the Xml `dataset`.
    pub(super) fn after_run_facts(
        &self,
        facts: &Facts,
        schema: &Schema,
        dataset: &[u8],
        rows: usize,
    ) -> Result<Facts, Signal> {
        const RGBA: &[u8] = b"rgba(";
        Ok(Facts {
            rows: Some(if rows == 0 { Rows::Zero } else { Rows::Some }),
            // The Xml (UTF-8) holds every column value and name and the labels (`dataset::Dataset::finish`); `rgba(` has no
            // character Xml escapes.
            dataset_rgba: Some(dataset.windows(RGBA.len()).any(|w| w == RGBA)),
            named: Some(self.named_fact(schema)?),
            ..*facts
        })
    }
}
