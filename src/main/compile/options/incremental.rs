use rr::compiler::IncrementalOptions;

pub(super) fn parse_incremental_phases(raw: &str) -> Option<IncrementalOptions> {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() || matches!(normalized.as_str(), "auto" | "on" | "true") {
        return Some(IncrementalOptions::auto());
    }
    if matches!(normalized.as_str(), "off" | "0" | "false" | "none") {
        return Some(IncrementalOptions::disabled());
    }
    if matches!(normalized.as_str(), "all" | "3") {
        return Some(IncrementalOptions::all_phases());
    }
    if matches!(normalized.as_str(), "1" | "phase1") {
        return Some(IncrementalOptions::phase1_only());
    }

    let mut options = IncrementalOptions {
        enabled: true,
        auto: false,
        phase1: false,
        phase2: false,
        phase3: false,
        strict_verify: false,
    };
    for token in normalized.split(',') {
        if !parse_incremental_phase_token(token.trim(), &mut options) {
            return None;
        }
    }
    if !options.phase1 && !options.phase2 && !options.phase3 {
        return None;
    }
    Some(options)
}

fn parse_incremental_phase_token(token: &str, options: &mut IncrementalOptions) -> bool {
    match token {
        "1" | "phase1" => options.phase1 = true,
        "2" | "phase2" => options.phase2 = true,
        "3" | "phase3" => options.phase3 = true,
        _ => return false,
    }
    true
}
