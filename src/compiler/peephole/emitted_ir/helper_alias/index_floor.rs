use super::*;
pub(crate) fn has_nested_index_vec_floor_candidates_ir(lines: &[String]) -> bool {
    let Some(re) = nested_index_vec_floor_re() else {
        return false;
    };
    lines.iter().any(|line| re.is_match(line))
}

pub(crate) fn apply_simplify_nested_index_vec_floor_calls_ir(program: &mut EmittedProgram) {
    let Some(re) = nested_index_vec_floor_re() else {
        return;
    };
    for item in &mut program.items {
        match item {
            EmittedItem::Raw(line) => {
                let mut rewritten = line.clone();
                loop {
                    let next = re
                        .replace_all(&rewritten, |caps: &Captures<'_>| {
                            format!(
                                "rr_index_vec_floor({})",
                                caps.name("inner").map(|m| m.as_str()).unwrap_or("")
                            )
                        })
                        .to_string();
                    if next == rewritten {
                        break;
                    }
                    rewritten = next;
                }
                *line = rewritten;
            }
            EmittedItem::Function(function) => {
                for stmt in &mut function.body {
                    let mut rewritten = stmt.text.clone();
                    loop {
                        let next = re
                            .replace_all(&rewritten, |caps: &Captures<'_>| {
                                format!(
                                    "rr_index_vec_floor({})",
                                    caps.name("inner").map(|m| m.as_str()).unwrap_or("")
                                )
                            })
                            .to_string();
                        if next == rewritten {
                            break;
                        }
                        rewritten = next;
                    }
                    if rewritten != stmt.text {
                        stmt.replace_text(rewritten);
                    }
                }
            }
        }
    }
}

pub(crate) fn run_post_passthrough_wrapper_cleanup_bundle_ir(lines: Vec<String>) -> Vec<String> {
    let needs_floor = has_nested_index_vec_floor_candidates_ir(&lines);
    let needs_copy = has_inlined_copy_vec_sequence_candidates_ir(&lines);
    if !needs_floor && !needs_copy {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    if needs_floor {
        apply_simplify_nested_index_vec_floor_calls_ir(&mut program);
    }
    if needs_copy {
        apply_collapse_inlined_copy_vec_sequences_ir(&mut program);
    }
    program.into_lines()
}
