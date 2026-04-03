use super::*;

impl RBackend {
    pub(super) fn begin_branch_snapshot(&mut self) -> BranchSnapshot {
        self.value_tracker.branch_snapshot_depth += 1;
        BranchSnapshot {
            value_binding_log_len: self.value_tracker.value_binding_log.len(),
            var_version_log_len: self.value_tracker.var_version_log.len(),
            var_value_binding_log_len: self.value_tracker.var_value_binding_log.len(),
            last_assigned_value_log_len: self.value_tracker.last_assigned_value_log.len(),
        }
    }

    pub(super) fn rollback_branch_snapshot(&mut self, snapshot: BranchSnapshot) {
        while self.value_tracker.value_binding_log.len() > snapshot.value_binding_log_len {
            let Some(undo) = self.value_tracker.value_binding_log.pop() else {
                break;
            };
            if let Some(prev) = undo.prev {
                self.value_tracker.value_bindings.insert(undo.val_id, prev);
            } else {
                self.value_tracker.value_bindings.remove(&undo.val_id);
            }
        }
        while self.value_tracker.var_version_log.len() > snapshot.var_version_log_len {
            let Some(undo) = self.value_tracker.var_version_log.pop() else {
                break;
            };
            if let Some(prev) = undo.prev {
                self.value_tracker.var_versions.insert(undo.var, prev);
            } else {
                self.value_tracker.var_versions.remove(&undo.var);
            }
        }
        while self.value_tracker.var_value_binding_log.len() > snapshot.var_value_binding_log_len {
            let Some(undo) = self.value_tracker.var_value_binding_log.pop() else {
                break;
            };
            if let Some(prev) = undo.prev {
                self.value_tracker.var_value_bindings.insert(undo.var, prev);
            } else {
                self.value_tracker.var_value_bindings.remove(&undo.var);
            }
        }
        while self.value_tracker.last_assigned_value_log.len()
            > snapshot.last_assigned_value_log_len
        {
            let Some(undo) = self.value_tracker.last_assigned_value_log.pop() else {
                break;
            };
            if let Some(prev) = undo.prev {
                self.value_tracker
                    .last_assigned_value_ids
                    .insert(undo.var, prev);
            } else {
                self.value_tracker.last_assigned_value_ids.remove(&undo.var);
            }
        }
    }

    pub(super) fn end_branch_snapshot(&mut self) {
        if self.value_tracker.branch_snapshot_depth > 0 {
            self.value_tracker.branch_snapshot_depth -= 1;
        }
    }

    pub(super) fn join_branch_var_value_bindings(
        &mut self,
        then_var_versions: &FxHashMap<String, u64>,
        then_var_value_bindings: &FxHashMap<String, (usize, u64)>,
        else_var_versions: &FxHashMap<String, u64>,
        else_var_value_bindings: &FxHashMap<String, (usize, u64)>,
    ) {
        let mut vars = FxHashSet::default();
        vars.extend(then_var_versions.keys().cloned());
        vars.extend(else_var_versions.keys().cloned());
        vars.extend(then_var_value_bindings.keys().cloned());
        vars.extend(else_var_value_bindings.keys().cloned());

        for var in vars {
            let pre_version = self
                .value_tracker
                .var_versions
                .get(&var)
                .copied()
                .unwrap_or(0);
            let then_version = then_var_versions.get(&var).copied().unwrap_or(pre_version);
            let else_version = else_var_versions.get(&var).copied().unwrap_or(pre_version);
            let joined_version = then_version.max(else_version);

            let then_binding = then_var_value_bindings.get(&var).copied();
            let else_binding = else_var_value_bindings.get(&var).copied();

            if let (Some((then_val_id, _)), Some((else_val_id, _))) = (then_binding, else_binding)
                && then_val_id == else_val_id
            {
                self.log_var_version_change(&var);
                self.value_tracker
                    .var_versions
                    .insert(var.clone(), joined_version);
                self.log_var_value_binding_change(&var);
                self.value_tracker
                    .var_value_bindings
                    .insert(var.clone(), (then_val_id, joined_version));
                continue;
            }

            if joined_version != pre_version || then_binding != else_binding {
                self.log_var_version_change(&var);
                self.value_tracker
                    .var_versions
                    .insert(var.clone(), joined_version);
                self.log_var_value_binding_change(&var);
                self.value_tracker.var_value_bindings.remove(&var);
            }
        }
    }

    pub(super) fn join_branch_last_assigned_values(
        &mut self,
        then_last_assigned: &FxHashMap<String, usize>,
        else_last_assigned: &FxHashMap<String, usize>,
    ) {
        let mut vars = FxHashSet::default();
        vars.extend(self.value_tracker.last_assigned_value_ids.keys().cloned());
        vars.extend(then_last_assigned.keys().cloned());
        vars.extend(else_last_assigned.keys().cloned());
        for var in vars {
            let pre = self
                .value_tracker
                .last_assigned_value_ids
                .get(&var)
                .copied();
            let then = then_last_assigned.get(&var).copied().or(pre);
            let else_ = else_last_assigned.get(&var).copied().or(pre);
            self.log_last_assigned_value_change(&var);
            if then == else_ {
                if let Some(val_id) = then {
                    self.value_tracker
                        .last_assigned_value_ids
                        .insert(var, val_id);
                } else {
                    self.value_tracker.last_assigned_value_ids.remove(&var);
                }
            } else {
                self.value_tracker.last_assigned_value_ids.remove(&var);
            }
        }
    }

    pub(super) fn join_branch_known_full_end_exprs(
        &mut self,
        pre_known_full_end_exprs: &FxHashMap<String, String>,
        then_known_full_end_exprs: &FxHashMap<String, String>,
        else_known_full_end_exprs: &FxHashMap<String, String>,
    ) {
        let mut vars = FxHashSet::default();
        vars.extend(pre_known_full_end_exprs.keys().cloned());
        vars.extend(then_known_full_end_exprs.keys().cloned());
        vars.extend(else_known_full_end_exprs.keys().cloned());

        for var in vars {
            let pre = pre_known_full_end_exprs.get(&var);
            let then = then_known_full_end_exprs.get(&var).or(pre);
            let else_ = else_known_full_end_exprs.get(&var).or(pre);
            match (then, else_) {
                (Some(lhs), Some(rhs)) if lhs == rhs => {
                    self.loop_analysis
                        .known_full_end_exprs
                        .insert(var, lhs.clone());
                }
                _ => {
                    self.loop_analysis.known_full_end_exprs.remove(&var);
                }
            }
        }
    }

    pub(super) fn log_value_binding_change(&mut self, val_id: usize) {
        if self.value_tracker.branch_snapshot_depth == 0 {
            return;
        }
        self.value_tracker.value_binding_log.push(ValueBindingUndo {
            val_id,
            prev: self.value_tracker.value_bindings.get(&val_id).cloned(),
        });
    }

    pub(super) fn log_var_version_change(&mut self, var: &str) {
        if self.value_tracker.branch_snapshot_depth == 0 {
            return;
        }
        self.value_tracker.var_version_log.push(VarVersionUndo {
            var: var.to_string(),
            prev: self.value_tracker.var_versions.get(var).copied(),
        });
    }

    pub(super) fn log_var_value_binding_change(&mut self, var: &str) {
        if self.value_tracker.branch_snapshot_depth == 0 {
            return;
        }
        self.value_tracker
            .var_value_binding_log
            .push(VarValueBindingUndo {
                var: var.to_string(),
                prev: self.value_tracker.var_value_bindings.get(var).copied(),
            });
    }

    pub(super) fn log_last_assigned_value_change(&mut self, var: &str) {
        if self.value_tracker.branch_snapshot_depth == 0 {
            return;
        }
        self.value_tracker
            .last_assigned_value_log
            .push(LastAssignedValueUndo {
                var: var.to_string(),
                prev: self.value_tracker.last_assigned_value_ids.get(var).copied(),
            });
    }
}
