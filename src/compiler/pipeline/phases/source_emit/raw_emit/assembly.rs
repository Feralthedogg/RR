use super::*;

pub(crate) fn assemble_emitted_fragments(
    fragments: &[EmittedFnFragment],
    use_optimized: bool,
) -> (String, Vec<MapEntry>) {
    let mut final_output = String::new();
    let mut final_source_map = Vec::new();
    let mut line_offset = 0u32;

    for fragment in fragments {
        let (code, map) = if use_optimized {
            (
                fragment.optimized_code.as_ref().unwrap_or(&fragment.code),
                fragment.optimized_map.as_ref().unwrap_or(&fragment.map),
            )
        } else {
            (&fragment.code, &fragment.map)
        };
        let mut shifted_map = map.clone();
        for entry in &mut shifted_map {
            entry.r_line = entry.r_line.saturating_add(line_offset);
        }
        line_offset = line_offset.saturating_add(emitted_segment_line_count(code));
        final_output.push_str(code);
        final_output.push('\n');
        final_source_map.extend(shifted_map);
    }

    (final_output, final_source_map)
}
