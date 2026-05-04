use super::*;
pub(crate) struct IndexedGatherHoistPlan {
    pub(crate) hoists: Vec<String>,
    pub(crate) replacements: Vec<(String, String)>,
}

pub(crate) struct IndexedGatherHoistBuilder<'a> {
    pub(crate) lines: &'a [String],
    pub(crate) loop_end: usize,
    pub(crate) loop_var: &'a str,
    pub(crate) assigned: &'a FxHashSet<String>,
    pub(crate) indent: String,
    pub(crate) used_names: &'a mut FxHashSet<String>,
    pub(crate) loop_temps: FxHashMap<(String, String), String>,
    pub(crate) hoists: Vec<String>,
    pub(crate) replacements: Vec<(String, String)>,
}

impl IndexedGatherHoistBuilder<'_> {
    pub(crate) fn collect_alias(&mut self, alias_info: &LoopIndexAlias) {
        for line in self
            .lines
            .iter()
            .take(self.loop_end)
            .skip(alias_info.line_idx + 1)
        {
            if line_reassigns_name(line, &alias_info.alias) {
                break;
            }
            self.collect_alias_line_calls(line, alias_info);
        }
    }

    pub(crate) fn collect_alias_line_calls(&mut self, line: &str, alias_info: &LoopIndexAlias) {
        for call in collect_index1_read_alias_calls(line, &alias_info.alias) {
            let Some((base, _idx_alias)) = parse_index1_read_base_alias(&call) else {
                continue;
            };
            if self.assigned.contains(&base) {
                continue;
            }
            let temp = self.temp_for_gather(&base, &alias_info.index_vec);
            self.replacements
                .push((call, format!("{temp}[{}]", self.loop_var)));
        }
    }

    pub(crate) fn temp_for_gather(&mut self, base: &str, index_vec: &str) -> String {
        self.loop_temps
            .entry((base.to_string(), index_vec.to_string()))
            .or_insert_with(|| {
                let raw_suffix = semantic_index_suffix(index_vec);
                let suffix = raw_suffix
                    .strip_prefix("idx_")
                    .unwrap_or(raw_suffix.as_str())
                    .to_string();
                let temp = unique_semantic_temp_name(&format!("{base}_{suffix}"), self.used_names);
                self.hoists.push(format!(
                    "{}{temp} <- rr_gather({base}, {index_vec})",
                    self.indent
                ));
                temp
            })
            .clone()
    }

    pub(crate) fn finish(self) -> Option<IndexedGatherHoistPlan> {
        (!self.hoists.is_empty()).then_some(IndexedGatherHoistPlan {
            hoists: self.hoists,
            replacements: self.replacements,
        })
    }
}

pub(crate) fn build_loop_invariant_indexed_gather_plan(
    lines: &[String],
    loop_start: usize,
    loop_end: usize,
    loop_var: &str,
    used_names: &mut FxHashSet<String>,
) -> Option<IndexedGatherHoistPlan> {
    let aliases =
        collect_loop_index_aliases(&lines[(loop_start + 1)..loop_end], loop_start + 1, loop_var);
    if aliases.is_empty() {
        return None;
    }

    let assigned = collect_loop_assigned_bases(&lines[(loop_start + 1)..loop_end]);
    let mut builder = IndexedGatherHoistBuilder {
        lines,
        loop_end,
        loop_var,
        assigned: &assigned,
        indent: line_indent(&lines[loop_start]),
        used_names,
        loop_temps: FxHashMap::default(),
        hoists: Vec::new(),
        replacements: Vec::new(),
    };

    for alias_info in aliases {
        if !assigned.contains(&alias_info.index_vec) {
            builder.collect_alias(&alias_info);
        }
    }
    builder.finish()
}

pub(crate) fn apply_indexed_gather_hoist_plan(
    out: &mut Vec<String>,
    loop_start: usize,
    loop_end: usize,
    mut plan: IndexedGatherHoistPlan,
) -> usize {
    plan.replacements
        .sort_by_key(|(lhs, _)| std::cmp::Reverse(lhs.len()));
    plan.replacements.dedup();
    for line in out.iter_mut().take(loop_end).skip(loop_start + 1) {
        for (from, to) in &plan.replacements {
            if line.contains(from) {
                *line = line.replace(from, to);
            }
        }
    }

    let inserted = plan.hoists.len();
    out.splice(loop_start..loop_start, plan.hoists);
    loop_end + inserted + 1
}
