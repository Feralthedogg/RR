use super::*;

impl<'a> MirLowerer<'a> {
    pub(super) fn lower_for(
        &mut self,
        iter: hir::HirForIter,
        body: hir::HirBlock,
        span: Span,
    ) -> RR<()> {
        let (var, start_id, end_id) = match iter {
            hir::HirForIter::Range {
                var, start, end, ..
            } => (var, self.lower_expr(start)?, self.lower_expr(end)?),
            hir::HirForIter::SeqAlong { var, xs } => {
                if let hir::HirExpr::Range { start, end } = xs {
                    (var, self.lower_expr(*start)?, self.lower_expr(*end)?)
                } else {
                    let xs_id = self.lower_expr(xs)?;
                    let start = self.add_value(ValueKind::Const(Lit::Int(1)), span);
                    let end = self.add_value(ValueKind::Len { base: xs_id }, span);
                    (var, start, end)
                }
            }
            hir::HirForIter::SeqLen { var, len } => {
                let start = self.add_value(ValueKind::Const(Lit::Int(1)), span);
                let end = self.lower_expr(len)?;
                (var, start, end)
            }
        };
        let pre_bb = self.curr_block;

        let header_bb = self.fn_ir.add_block();
        let body_bb = self.fn_ir.add_block();
        let exit_bb = self.fn_ir.add_block();

        self.write_var(var, start_id);
        self.add_pred(header_bb, pre_bb);
        self.terminate(Terminator::Goto(header_bb));

        self.curr_block = header_bb;
        let iv = self.read_var(var, header_bb)?;
        let cond = self.fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Le,
                lhs: iv,
                rhs: end_id,
            },
            span,
            Facts::empty(),
            None,
        );
        self.terminate(Terminator::If {
            cond,
            then_bb: body_bb,
            else_bb: exit_bb,
        });

        self.add_pred(body_bb, header_bb);
        self.seal_block(body_bb)?;
        self.curr_block = body_bb;
        self.loop_stack.push(LoopTargets {
            break_bb: exit_bb,
            continue_bb: header_bb,
            continue_step: Some((var, iv)),
        });
        self.lower_block_effects(body)?;
        self.loop_stack.pop();

        let curr_reachable = self
            .preds
            .get(&self.curr_block)
            .map(|ps| !ps.is_empty())
            .unwrap_or(false);
        if !self.is_terminated(self.curr_block) && curr_reachable {
            let one =
                self.fn_ir
                    .add_value(ValueKind::Const(Lit::Int(1)), span, Facts::empty(), None);
            let next_iv = self.fn_ir.add_value(
                ValueKind::Binary {
                    op: BinOp::Add,
                    lhs: iv,
                    rhs: one,
                },
                span,
                Facts::empty(),
                None,
            );
            self.write_var(var, next_iv);
            self.add_pred(header_bb, self.curr_block);
            self.terminate(Terminator::Goto(header_bb));
        }

        self.seal_block(header_bb)?;
        self.add_pred(exit_bb, header_bb);
        self.curr_block = exit_bb;
        self.seal_block(exit_bb)?;

        Ok(())
    }
}
