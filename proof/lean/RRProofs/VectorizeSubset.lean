namespace RRProofs

inductive LoopInstr where
  | pureAssign
  | eval
  | store
deriving Repr, DecidableEq

def isEffectful : LoopInstr -> Bool
  | .pureAssign => false
  | .eval => true
  | .store => true

def loopHasEffect : List LoopInstr -> Bool
  | [] => false
  | instr :: rest => isEffectful instr || loopHasEffect rest

def certifyExprMap (body : List LoopInstr) : Bool :=
  not (loopHasEffect body)

def certifyCondStoreBranch (branch : List LoopInstr) : Bool :=
  match branch with
  | [] => false
  | [.store] => true
  | _ => false

def certifyCondMap (thenBranch elseBranch : List LoopInstr) : Bool :=
  certifyCondStoreBranch thenBranch && certifyCondStoreBranch elseBranch

theorem certifyExprMap_rejects_eval :
    certifyExprMap [.pureAssign, .eval] = false := by
  simp [certifyExprMap, loopHasEffect, isEffectful]

theorem certifyExprMap_rejects_store :
    certifyExprMap [.pureAssign, .store] = false := by
  simp [certifyExprMap, loopHasEffect, isEffectful]

theorem certifyExprMap_accepts_pure_assigns :
    certifyExprMap [.pureAssign, .pureAssign] = true := by
  simp [certifyExprMap, loopHasEffect, isEffectful]

theorem certifyCondMap_rejects_branch_eval :
    certifyCondMap [.store, .eval] [.store] = false := by
  simp [certifyCondMap, certifyCondStoreBranch]

theorem certifyCondMap_rejects_branch_assign :
    certifyCondMap [.store, .pureAssign] [.store] = false := by
  simp [certifyCondMap, certifyCondStoreBranch]

theorem certifyCondMap_accepts_store_only :
    certifyCondMap [.store] [.store] = true := by
  simp [certifyCondMap, certifyCondStoreBranch]

end RRProofs
