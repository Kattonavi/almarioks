//! Search (T10, RF-59 to RF-63, RF-92): the query, the match and the order, on our own types.
//!
//! The vault side (`OpenedVault::search`, in `kdbx`) walks the groups outside the recycle bin and hands each entry
//! over with only the values a window may see; this module never sees a protected value (RF-61).

use crate::{
    text,
    vault::{EntrySummary, title_order_key},
};

/// A query that filters: its words, already folded (RF-59, RF-60).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    words: Vec<String>,
}

impl Query {
    /// Splits the query on spaces (Unicode `White_Space`) and folds each word. `None` if nothing is left after
    /// trimming: the caller shows the current group, unfiltered (RF-92).
    ///
    /// A word that folds to the empty text (a lone combining mark) is dropped; a query made only of those
    /// matches every entry outside the bin (decision pending).
    #[must_use]
    pub fn parse(query: &str) -> Option<Self> {
        let query = text::trim(query);
        if query.is_empty() {
            return None;
        }
        let words = query
            .split(char::is_whitespace)
            .map(text::fold)
            .filter(|word| !word.is_empty())
            .collect();
        Some(Self { words })
    }

    /// The folded words.
    #[must_use]
    pub fn words(&self) -> &[String] {
        &self.words
    }
}

/// One result: the entry as a list shows it and the path of its group, root first (plan "Precisiones" 7, 11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    /// The entry; protected standard fields are `None`.
    pub summary: EntrySummary,
    /// Names of the groups from the root (included) down to the entry's group.
    pub group_path: Vec<String>,
}

/// What a search answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchResult {
    /// The query is empty after trimming: no filter, show the current group (RF-92).
    NoFilter,
    /// The entries that match, in the order of RF-63 (empty: no results, RF-64).
    Matches(Vec<SearchHit>),
}

/// An entry offered to the search: what a window may see of it, protected values already `None`.
pub(crate) struct Candidate {
    pub(crate) summary: EntrySummary,
    pub(crate) notes: Option<String>,
    pub(crate) group_path: Vec<String>,
}

/// Keeps the candidates where every word is part of the folded title, user name, URL, a tag or the notes
/// (RF-59, RF-60), and orders them: first those whose folded title holds every word, then the rest, each block in
/// title order (RF-63, spec "Orden por título").
pub(crate) fn run(
    query: &Query,
    candidates: impl IntoIterator<Item = Candidate>,
) -> Vec<SearchHit> {
    let mut ranked: Vec<(bool, (String, crate::vault::EntryId), SearchHit)> = Vec::new();
    for candidate in candidates {
        let summary = &candidate.summary;
        let title = summary.title.as_deref().map(text::fold);
        let mut folded: Vec<String> = [
            summary.username.as_deref(),
            summary.url.as_deref(),
            candidate.notes.as_deref(),
        ]
        .into_iter()
        .flatten()
        .map(text::fold)
        .collect();
        folded.extend(summary.tags.iter().map(|tag| text::fold(tag)));
        let in_title = |word: &String| title.as_deref().is_some_and(|t| t.contains(word.as_str()));
        let all_in_title = query.words.iter().all(in_title);
        let matches = all_in_title
            || query.words.iter().all(|word| {
                in_title(word) || folded.iter().any(|field| field.contains(word.as_str()))
            });
        if matches {
            let key = title_order_key(summary.title.as_deref(), summary.id);
            ranked.push((
                !all_in_title,
                key,
                SearchHit {
                    summary: candidate.summary,
                    group_path: candidate.group_path,
                },
            ));
        }
    }
    ranked.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    ranked.into_iter().map(|(_, _, hit)| hit).collect()
}

#[cfg(test)]
mod tests {
    use super::Query;

    #[test]
    fn rf_92_an_empty_query_does_not_filter() {
        assert_eq!(Query::parse(""), None);
        assert_eq!(Query::parse(" \t\u{3000}\u{A0} "), None);
    }

    #[test]
    fn rf_59_rf_60_words_are_split_on_spaces_and_folded() {
        let query = Query::parse("  CANCIÓN\u{3000}ana\tGit  ").unwrap_or_else(|| unreachable!());
        assert_eq!(query.words(), ["cancion", "ana", "git"]);
    }
}
