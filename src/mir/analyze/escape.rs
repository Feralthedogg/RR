use crate::mir::def::EscapeStatus;
use crate::mir::*;
use std::collections::VecDeque;

pub struct EscapeAnalysis;

impl EscapeAnalysis {
    pub fn analyze_function(fn_ir: &mut FnIR) {
        // 1. Initialize all to Local
        for val in &mut fn_ir.values {
            val.escape = EscapeStatus::Local;
        }

        let mut worklist = VecDeque::new();

        // 2. Identify Sources of Escape (Roots)
        // A. Return values
        for block in &fn_ir.blocks {
            if let Terminator::Return(Some(val_id)) = &block.term {
                // If return value is not already recognized as escaped, add it
                // Logic: Just push to worklist, we'll mark later
                worklist.push_back(*val_id);
            }
        }

        // B. Call Arguments
        // Scan all values for Call kind (and potentially other escaping kinds)
        for val in &fn_ir.values {
            if let ValueKind::Call { args, .. } = &val.kind {
                for arg in args {
                    worklist.push_back(*arg);
                }
            }

            // C. Function Parameters (Incoming values escape by definition/conservative)
            if let Some(var_name) = &val.origin_var
                && fn_ir.params.contains(var_name) {
                    worklist.push_back(val.id);
                }
        }

        // 3. Build Dependency Graph (Adjacency List)
        // graph[v] contains list of u such that "If v escapes, u escapes"
        let mut graph: Vec<Vec<ValueId>> = vec![vec![]; fn_ir.values.len()];

        // A. Phi Nodes: v = Phi(args). If v escapes, args escape.
        for val in &fn_ir.values {
            if let ValueKind::Phi { args } = &val.kind {
                for (arg, _) in args {
                    graph[val.id].push(*arg);
                }
            }
        }

        // B. Stores: Store(base, val). If base escapes, val escapes.
        for block in &fn_ir.blocks {
            for instr in &block.instrs {
                if let Instr::StoreIndex1D { base, val, .. } = instr {
                    graph[*base].push(*val);
                }
            }
        }

        // 4. Mark Roots and Propagate
        // Mark initial worklist items
        for &vid in &worklist {
            fn_ir.values[vid].escape = EscapeStatus::Escaped;
        }

        // BFS Propagation
        while let Some(vid) = worklist.pop_front() {
            // vid is known Escaped. Propagate to downstream dependencies.
            for &dep in &graph[vid] {
                if fn_ir.values[dep].escape != EscapeStatus::Escaped {
                    fn_ir.values[dep].escape = EscapeStatus::Escaped;
                    worklist.push_back(dep);
                }
            }
        }
    }
}
