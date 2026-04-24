import RRProofs.VerifyIrArgEnvSubset

set_option linter.unnecessarySimpa false

namespace RRProofs

def firstMissingArgEnvExpr (env : ArgValueEnv) : ArgEnvExpr -> Option ArgValueId
  | .const _ => none
  | .use vid => if (env vid).isSome then none else some vid
  | .add lhs rhs =>
      match firstMissingArgEnvExpr env lhs with
      | some v => some v
      | none => firstMissingArgEnvExpr env rhs
  | .field base _ => firstMissingArgEnvExpr env base

def firstMissingArgEnvExprList (env : ArgValueEnv) : List ArgEnvExpr -> Option ArgValueId
  | [] => none
  | e :: rest =>
      match firstMissingArgEnvExpr env e with
      | some v => some v
      | none => firstMissingArgEnvExprList env rest

def firstMissingArgEnvFields (env : ArgValueEnv) : List ArgEnvField -> Option ArgValueId
  | [] => none
  | (_, e) :: rest =>
      match firstMissingArgEnvExpr env e with
      | some v => some v
      | none => firstMissingArgEnvFields env rest

theorem firstMissingArgEnvExpr_clean_rewriteArgPhiUse
    (env : ArgValueEnv) (phi arg : ArgValueId) :
    ∀ e,
      firstMissingArgEnvExpr (mergedArgEnv env phi arg) e = none ↔
        firstMissingArgEnvExpr env (rewriteArgPhiUse phi arg e) = none
  | .const _ => by simp [firstMissingArgEnvExpr, rewriteArgPhiUse]
  | .use vid => by
      by_cases h : vid = phi
      · subst h
        cases hArg : env arg <;> simp [firstMissingArgEnvExpr, rewriteArgPhiUse, mergedArgEnv, hArg]
      · cases hVid : env vid <;> simp [firstMissingArgEnvExpr, rewriteArgPhiUse, mergedArgEnv, h, hVid]
  | .add lhs rhs => by
      constructor
      · intro h
        cases hL : firstMissingArgEnvExpr (mergedArgEnv env phi arg) lhs with
        | some miss =>
            simp [firstMissingArgEnvExpr, hL] at h
        | none =>
            have hL' := (firstMissingArgEnvExpr_clean_rewriteArgPhiUse env phi arg lhs).1 hL
            have hR : firstMissingArgEnvExpr (mergedArgEnv env phi arg) rhs = none := by
              simpa [firstMissingArgEnvExpr, hL] using h
            have hR' := (firstMissingArgEnvExpr_clean_rewriteArgPhiUse env phi arg rhs).1 hR
            simpa [firstMissingArgEnvExpr, rewriteArgPhiUse, hL', hR']
      · intro h
        cases hL : firstMissingArgEnvExpr env (rewriteArgPhiUse phi arg lhs) with
        | some miss =>
            simp [firstMissingArgEnvExpr, rewriteArgPhiUse, hL] at h
        | none =>
            have hL' := (firstMissingArgEnvExpr_clean_rewriteArgPhiUse env phi arg lhs).2 hL
            have hR : firstMissingArgEnvExpr env (rewriteArgPhiUse phi arg rhs) = none := by
              simpa [firstMissingArgEnvExpr, rewriteArgPhiUse, hL] using h
            have hR' := (firstMissingArgEnvExpr_clean_rewriteArgPhiUse env phi arg rhs).2 hR
            simpa [firstMissingArgEnvExpr, hL', hR']
  | .field base name => by
      simpa [firstMissingArgEnvExpr, rewriteArgPhiUse] using
        firstMissingArgEnvExpr_clean_rewriteArgPhiUse env phi arg base

theorem firstMissingArgEnvExprList_clean_rewriteArgPhiUse
    (env : ArgValueEnv) (phi arg : ArgValueId) :
    ∀ es,
      firstMissingArgEnvExprList (mergedArgEnv env phi arg) es = none ↔
        firstMissingArgEnvExprList env (rewriteArgPhiUseList phi arg es) = none
  | [] => by simp [firstMissingArgEnvExprList, rewriteArgPhiUseList]
  | e :: rest => by
      constructor
      · intro h
        cases hE : firstMissingArgEnvExpr (mergedArgEnv env phi arg) e with
        | some miss =>
            simp [firstMissingArgEnvExprList, hE] at h
        | none =>
            have hE' := (firstMissingArgEnvExpr_clean_rewriteArgPhiUse env phi arg e).1 hE
            have hRest : firstMissingArgEnvExprList (mergedArgEnv env phi arg) rest = none := by
              simpa [firstMissingArgEnvExprList, hE] using h
            have hRest' := (firstMissingArgEnvExprList_clean_rewriteArgPhiUse env phi arg rest).1 hRest
            simpa [firstMissingArgEnvExprList, rewriteArgPhiUseList, hE', hRest']
      · intro h
        cases hE : firstMissingArgEnvExpr env (rewriteArgPhiUse phi arg e) with
        | some miss =>
            simp [firstMissingArgEnvExprList, rewriteArgPhiUseList, hE] at h
        | none =>
            have hE' := (firstMissingArgEnvExpr_clean_rewriteArgPhiUse env phi arg e).2 hE
            have hRest : firstMissingArgEnvExprList env (rewriteArgPhiUseList phi arg rest) = none := by
              simpa [firstMissingArgEnvExprList, rewriteArgPhiUseList, hE] using h
            have hRest' := (firstMissingArgEnvExprList_clean_rewriteArgPhiUse env phi arg rest).2 hRest
            simpa [firstMissingArgEnvExprList, hE', hRest']

theorem firstMissingArgEnvFields_clean_rewriteArgPhiUse
    (env : ArgValueEnv) (phi arg : ArgValueId) :
    ∀ fs,
      firstMissingArgEnvFields (mergedArgEnv env phi arg) fs = none ↔
        firstMissingArgEnvFields env (rewriteArgPhiUseFields phi arg fs) = none
  | [] => by simp [firstMissingArgEnvFields, rewriteArgPhiUseFields]
  | (name, e) :: rest => by
      constructor
      · intro h
        cases hE : firstMissingArgEnvExpr (mergedArgEnv env phi arg) e with
        | some miss =>
            simp [firstMissingArgEnvFields, hE] at h
        | none =>
            have hE' := (firstMissingArgEnvExpr_clean_rewriteArgPhiUse env phi arg e).1 hE
            have hRest : firstMissingArgEnvFields (mergedArgEnv env phi arg) rest = none := by
              simpa [firstMissingArgEnvFields, hE] using h
            have hRest' := (firstMissingArgEnvFields_clean_rewriteArgPhiUse env phi arg rest).1 hRest
            simpa [firstMissingArgEnvFields, rewriteArgPhiUseFields, hE', hRest']
      · intro h
        cases hE : firstMissingArgEnvExpr env (rewriteArgPhiUse phi arg e) with
        | some miss =>
            simp [firstMissingArgEnvFields, rewriteArgPhiUseFields, hE] at h
        | none =>
            have hE' := (firstMissingArgEnvExpr_clean_rewriteArgPhiUse env phi arg e).2 hE
            have hRest : firstMissingArgEnvFields env (rewriteArgPhiUseFields phi arg rest) = none := by
              simpa [firstMissingArgEnvFields, rewriteArgPhiUseFields, hE] using h
            have hRest' := (firstMissingArgEnvFields_clean_rewriteArgPhiUse env phi arg rest).2 hRest
            simpa [firstMissingArgEnvFields, hE', hRest']

theorem exampleCallArgEnvExprs_scan_preserved_on_selected_edge :
    firstMissingArgEnvExprList (mergedArgEnv exampleArgEnv 9 1) exampleCallArgEnvExprs = none ↔
      firstMissingArgEnvExprList exampleArgEnv (rewriteArgPhiUseList 9 1 exampleCallArgEnvExprs) = none := by
  simpa using firstMissingArgEnvExprList_clean_rewriteArgPhiUse exampleArgEnv 9 1 exampleCallArgEnvExprs

theorem exampleCallArgEnvExprs_scan_clean_from_selected_eval :
    firstMissingArgEnvExprList (mergedArgEnv exampleArgEnv 9 1) exampleCallArgEnvExprs = none := by
  simp [firstMissingArgEnvExprList, firstMissingArgEnvExpr, mergedArgEnv,
    exampleArgEnv, exampleCallArgEnvExprs]

theorem exampleRecordArgEnvFields_scan_preserved_on_selected_edge :
    firstMissingArgEnvFields (mergedArgEnv exampleArgEnv 12 7) exampleRecordArgEnvFields = none ↔
      firstMissingArgEnvFields exampleArgEnv (rewriteArgPhiUseFields 12 7 exampleRecordArgEnvFields) = none := by
  simpa using firstMissingArgEnvFields_clean_rewriteArgPhiUse exampleArgEnv 12 7 exampleRecordArgEnvFields

theorem exampleRecordArgEnvFields_scan_clean_from_selected_eval :
    firstMissingArgEnvFields (mergedArgEnv exampleArgEnv 12 7) exampleRecordArgEnvFields = none := by
  simp [firstMissingArgEnvFields, firstMissingArgEnvExpr, mergedArgEnv,
    exampleArgEnv, exampleRecordArgEnvFields]

end RRProofs
