use super::*;
#[derive(Clone, Debug)]
pub(crate) struct LocalFunctionSpan {
    pub(crate) name: String,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

pub(crate) fn local_function_spans(lines: &[String]) -> Vec<LocalFunctionSpan> {
    let mut funcs = Vec::new();
    let scope_end = lines.len().saturating_sub(1);
    let mut idx = 0usize;
    while idx < lines.len() {
        let trimmed = lines[idx].trim();
        let Some((name, _)) = trimmed.split_once(" <- function(") else {
            idx += 1;
            continue;
        };
        let open_idx = idx + 1;
        if open_idx >= lines.len() || lines[open_idx].trim() != "{" {
            idx += 1;
            continue;
        }
        let Some(end) = RBackend::block_end_for_open_brace(lines, open_idx, scope_end) else {
            idx += 1;
            continue;
        };
        funcs.push(LocalFunctionSpan {
            name: name.trim().to_string(),
            start: idx,
            end,
        });
        idx = end + 1;
    }
    funcs
}
