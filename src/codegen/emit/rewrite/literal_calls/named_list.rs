pub(super) fn rewrite_literal_named_list_calls(output: &mut String) {
    if !output.contains("rr_named_list(") {
        return;
    }
    let mut out = Vec::with_capacity(output.lines().count());
    for line in output.lines() {
        if line.contains("rr_named_list <- function") {
            out.push(line.to_string());
            continue;
        }
        let mut rewritten = line.to_string();
        let mut search_start = 0usize;
        loop {
            let Some(start) = find_next_call(&rewritten, search_start, "rr_named_list") else {
                break;
            };
            let call_start = start + "rr_named_list".len();
            let Some(call_end) = find_matching_call_close(&rewritten, call_start) else {
                break;
            };
            let args_inner = &rewritten[call_start + 1..call_end];
            let Some(args) = split_top_level_args_local(args_inner) else {
                search_start = call_end + 1;
                continue;
            };
            if args.len() % 2 != 0 {
                search_start = call_end + 1;
                continue;
            }
            let mut fields = Vec::new();
            let mut ok = true;
            for pair in args.chunks(2) {
                let Some(name) = literal_record_field_name(pair[0].trim()) else {
                    ok = false;
                    break;
                };
                fields.push(format!("{name} = {}", pair[1].trim()));
            }
            if !ok {
                search_start = call_end + 1;
                continue;
            }
            let replacement = if fields.is_empty() {
                "list()".to_string()
            } else {
                format!("list({})", fields.join(", "))
            };
            let replacement_end = start + replacement.len();
            rewritten.replace_range(start..=call_end, &replacement);
            search_start = replacement_end;
        }
        out.push(rewritten);
    }
    let mut rewritten = out.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
