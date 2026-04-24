pub(super) fn rewrite_literal_field_get_calls(output: &mut String) {
    if !output.contains("rr_field_get(") {
        return;
    }
    let mut out = Vec::with_capacity(output.lines().count());
    for line in output.lines() {
        if line.contains("<- function") {
            out.push(line.to_string());
            continue;
        }
        let mut rewritten = line.to_string();
        loop {
            let Some(start) = rewritten.find("rr_field_get(") else {
                break;
            };
            let call_start = start + "rr_field_get".len();
            let Some(call_end) = find_matching_call_close(&rewritten, call_start) else {
                break;
            };
            let args_inner = &rewritten[call_start + 1..call_end];
            let Some(args) = split_top_level_args_local(args_inner) else {
                break;
            };
            if args.len() != 2 {
                break;
            }
            let base = args[0].trim();
            let Some(name) = literal_record_field_name(args[1].trim()) else {
                break;
            };
            let replacement = format!(r#"{base}[["{name}"]]"#);
            rewritten.replace_range(start..=call_end, &replacement);
        }
        out.push(rewritten);
    }
    let mut rewritten = out.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
