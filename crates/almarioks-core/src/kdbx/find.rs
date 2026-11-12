//! Search over an opened vault (T10, RF-59 to RF-63, RF-92): walks the groups from the root, skips the recycle bin
//! with everything below it (RF-62), and offers each entry to [`crate::search`] with only what a window may see
//! (protected fields are `None`, RF-61).

use keepass::db::{GroupId as KpGroupId, fields};

use super::{
    open::OpenedVault,
    read::{summary, visible},
};
use crate::search::{Candidate, Query, SearchResult, run};

impl OpenedVault {
    /// Searches every entry outside the recycle bin (RF-59 to RF-63). A query that is empty after trimming answers
    /// [`SearchResult::NoFilter`] (RF-92). Each hit carries the path of its group, root included.
    #[must_use]
    pub fn search(&self, query: &str) -> SearchResult {
        let Some(query) = Query::parse(query) else {
            return SearchResult::NoFilter;
        };
        let bin = self.db.recycle_bin().map(|bin| bin.id());
        let mut candidates = Vec::new();
        let mut pending: Vec<(KpGroupId, Vec<String>)> = vec![(self.db.root().id(), Vec::new())];
        while let Some((id, mut path)) = pending.pop() {
            let Some(group) = self.db.group(id) else {
                continue;
            };
            if Some(id) == bin {
                continue;
            }
            path.push(group.name.clone());
            for entry in group.entries() {
                candidates.push(Candidate {
                    summary: summary(&entry),
                    notes: visible(&entry, fields::NOTES),
                    group_path: path.clone(),
                });
            }
            for child in group.group_ids() {
                pending.push((child, path.clone()));
            }
        }
        SearchResult::Matches(run(&query, candidates))
    }
}
