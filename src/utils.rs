use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Span {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}-{}:{}",
            self.start_line, self.start_col, self.end_line, self.end_col
        )
    }
}

impl Span {
    pub fn new(
        start_byte: usize,
        end_byte: usize,
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
    ) -> Self {
        Self {
            start_byte,
            end_byte,
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }

    pub fn merge(&self, other: Span) -> Span {
        if self.start_byte == 0 && self.end_byte == 0 && self.start_line == 0 {
            return other;
        }
        if other.start_byte == 0 && other.end_byte == 0 && other.start_line == 0 {
            return *self;
        }

        Span {
            start_byte: self.start_byte.min(other.start_byte),
            end_byte: self.end_byte.max(other.end_byte),
            start_line: self.start_line.min(other.start_line),
            start_col: if self.start_line < other.start_line {
                self.start_col
            } else {
                self.start_col.min(other.start_col)
            }, // Approximate
            end_line: self.end_line.max(other.end_line),
            end_col: if self.end_line > other.end_line {
                self.end_col
            } else {
                self.end_col.max(other.end_col)
            },
        }
    }
}

pub fn suggest(needle: &str, candidates: impl IntoIterator<Item = String>) -> Vec<String> {
    let needle = needle.trim();
    if needle.is_empty() {
        return Vec::new();
    }

    let mut scored: Vec<(usize, String)> = candidates
        .into_iter()
        .filter_map(|candidate| {
            let candidate = candidate.trim();
            if candidate.is_empty() || candidate == needle {
                return None;
            }
            Some((
                levenshtein_distance(needle, candidate),
                candidate.to_string(),
            ))
        })
        .collect();

    let max_dist = max_suggestion_distance(needle.chars().count());
    scored.retain(|(dist, _)| *dist <= max_dist);
    scored.sort_by(|(lhs_dist, lhs), (rhs_dist, rhs)| {
        lhs_dist
            .cmp(rhs_dist)
            .then(lhs.len().cmp(&rhs.len()))
            .then(lhs.cmp(rhs))
    });
    scored.dedup_by(|(_, lhs), (_, rhs)| lhs == rhs);

    scored.into_iter().take(3).map(|(_, name)| name).collect()
}

pub fn did_you_mean(needle: &str, candidates: impl IntoIterator<Item = String>) -> Option<String> {
    let suggestions = suggest(needle, candidates);
    match suggestions.len() {
        0 => None,
        1 => Some(format!("did you mean `{}`?", suggestions[0])),
        _ => Some(format!(
            "did you mean one of: {}?",
            suggestions
                .iter()
                .map(|name| format!("`{}`", name))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn max_suggestion_distance(len: usize) -> usize {
    match len {
        0..=3 => 1,
        4..=6 => 2,
        7..=10 => 3,
        _ => 4,
    }
}

fn levenshtein_distance(lhs: &str, rhs: &str) -> usize {
    let lhs_chars: Vec<char> = lhs.chars().collect();
    let rhs_chars: Vec<char> = rhs.chars().collect();
    let mut prev: Vec<usize> = (0..=rhs_chars.len()).collect();
    let mut curr = vec![0; rhs_chars.len() + 1];

    for (i, lhs_ch) in lhs_chars.iter().enumerate() {
        curr[0] = i + 1;
        for (j, rhs_ch) in rhs_chars.iter().enumerate() {
            let cost = usize::from(lhs_ch != rhs_ch);
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        prev.clone_from(&curr);
    }

    prev[rhs_chars.len()]
}
