import RRProofs.VerifyIrUseTraversalSubset

namespace RRProofs

inductive VkExpr where
  | const : VkExpr
  | load : Var -> VkExpr
  | len : VkExpr -> VkExpr
  | indices : VkExpr -> VkExpr
  | fieldGet : VkExpr -> VkExpr
  | range : VkExpr -> VkExpr -> VkExpr
  | unary : VkExpr -> VkExpr
  | binary : VkExpr -> VkExpr -> VkExpr
  | intrinsic : VkExpr -> VkExpr -> VkExpr
  | call : VkExpr -> VkExpr -> VkExpr
  | recordLit : VkExpr -> VkExpr -> VkExpr
  | fieldSet : VkExpr -> VkExpr -> VkExpr
  | index1d : VkExpr -> VkExpr -> VkExpr
  | index2d : VkExpr -> VkExpr -> VkExpr -> VkExpr
  | index3d : VkExpr -> VkExpr -> VkExpr -> VkExpr -> VkExpr
deriving Repr

def firstUndefinedVk (defined : DefSet) : VkExpr -> Option Var
  | .const => none
  | .load v => if v ∈ defined then none else some v
  | .len base => firstUndefinedVk defined base
  | .indices base => firstUndefinedVk defined base
  | .fieldGet base => firstUndefinedVk defined base
  | .unary base => firstUndefinedVk defined base
  | .range start finish =>
      match firstUndefinedVk defined start with
      | some v => some v
      | none => firstUndefinedVk defined finish
  | .binary start finish =>
      match firstUndefinedVk defined start with
      | some v => some v
      | none => firstUndefinedVk defined finish
  | .intrinsic start finish =>
      match firstUndefinedVk defined start with
      | some v => some v
      | none => firstUndefinedVk defined finish
  | .call start finish =>
      match firstUndefinedVk defined start with
      | some v => some v
      | none => firstUndefinedVk defined finish
  | .recordLit start finish =>
      match firstUndefinedVk defined start with
      | some v => some v
      | none => firstUndefinedVk defined finish
  | .fieldSet start finish =>
      match firstUndefinedVk defined start with
      | some v => some v
      | none => firstUndefinedVk defined finish
  | .index1d start finish =>
      match firstUndefinedVk defined start with
      | some v => some v
      | none => firstUndefinedVk defined finish
  | .index2d base r c =>
      match firstUndefinedVk defined base with
      | some v => some v
      | none =>
          match firstUndefinedVk defined r with
          | some v => some v
          | none => firstUndefinedVk defined c
  | .index3d base i j k =>
      match firstUndefinedVk defined base with
      | some v => some v
      | none =>
          match firstUndefinedVk defined i with
          | some v => some v
          | none =>
              match firstUndefinedVk defined j with
              | some v => some v
              | none => firstUndefinedVk defined k

def loadsDefinedVk (defined : DefSet) : VkExpr -> Prop
  | .const => True
  | .load v => v ∈ defined
  | .len base => loadsDefinedVk defined base
  | .indices base => loadsDefinedVk defined base
  | .fieldGet base => loadsDefinedVk defined base
  | .unary base => loadsDefinedVk defined base
  | .range start finish => loadsDefinedVk defined start ∧ loadsDefinedVk defined finish
  | .binary start finish => loadsDefinedVk defined start ∧ loadsDefinedVk defined finish
  | .intrinsic start finish => loadsDefinedVk defined start ∧ loadsDefinedVk defined finish
  | .call start finish => loadsDefinedVk defined start ∧ loadsDefinedVk defined finish
  | .recordLit start finish => loadsDefinedVk defined start ∧ loadsDefinedVk defined finish
  | .fieldSet start finish => loadsDefinedVk defined start ∧ loadsDefinedVk defined finish
  | .index1d start finish =>
      loadsDefinedVk defined start ∧ loadsDefinedVk defined finish
  | .index2d base r c =>
      loadsDefinedVk defined base ∧ loadsDefinedVk defined r ∧ loadsDefinedVk defined c
  | .index3d base i j k =>
      loadsDefinedVk defined base ∧ loadsDefinedVk defined i ∧
      loadsDefinedVk defined j ∧ loadsDefinedVk defined k

theorem firstUndefinedVk_none_of_loadsDefined (defined : DefSet) :
    ∀ e, loadsDefinedVk defined e -> firstUndefinedVk defined e = none
  | .const, _ => rfl
  | .load v, h => by
      have hv : v ∈ defined := by simpa [loadsDefinedVk] using h
      simp [firstUndefinedVk, hv]
  | .len base, h => firstUndefinedVk_none_of_loadsDefined defined base h
  | .indices base, h => firstUndefinedVk_none_of_loadsDefined defined base h
  | .fieldGet base, h => firstUndefinedVk_none_of_loadsDefined defined base h
  | .unary base, h =>
      firstUndefinedVk_none_of_loadsDefined defined base h
  | .range start finish, h => by
      rcases h with ⟨hStart, hFinish⟩
      simp [firstUndefinedVk,
        firstUndefinedVk_none_of_loadsDefined defined start hStart,
        firstUndefinedVk_none_of_loadsDefined defined finish hFinish]
  | .binary start finish, h => by
      rcases h with ⟨hStart, hFinish⟩
      simp [firstUndefinedVk,
        firstUndefinedVk_none_of_loadsDefined defined start hStart,
        firstUndefinedVk_none_of_loadsDefined defined finish hFinish]
  | .intrinsic start finish, h => by
      rcases h with ⟨hStart, hFinish⟩
      simp [firstUndefinedVk,
        firstUndefinedVk_none_of_loadsDefined defined start hStart,
        firstUndefinedVk_none_of_loadsDefined defined finish hFinish]
  | .call start finish, h => by
      rcases h with ⟨hStart, hFinish⟩
      simp [firstUndefinedVk,
        firstUndefinedVk_none_of_loadsDefined defined start hStart,
        firstUndefinedVk_none_of_loadsDefined defined finish hFinish]
  | .recordLit start finish, h => by
      rcases h with ⟨hStart, hFinish⟩
      simp [firstUndefinedVk,
        firstUndefinedVk_none_of_loadsDefined defined start hStart,
        firstUndefinedVk_none_of_loadsDefined defined finish hFinish]
  | .fieldSet start finish, h => by
      rcases h with ⟨hStart, hFinish⟩
      simp [firstUndefinedVk,
        firstUndefinedVk_none_of_loadsDefined defined start hStart,
        firstUndefinedVk_none_of_loadsDefined defined finish hFinish]
  | .index1d start finish, h => by
      rcases h with ⟨hStart, hFinish⟩
      simp [firstUndefinedVk,
        firstUndefinedVk_none_of_loadsDefined defined start hStart,
        firstUndefinedVk_none_of_loadsDefined defined finish hFinish]
  | .index2d base r c, h => by
      rcases h with ⟨hBase, hR, hC⟩
      simp [firstUndefinedVk,
        firstUndefinedVk_none_of_loadsDefined defined base hBase,
        firstUndefinedVk_none_of_loadsDefined defined r hR,
        firstUndefinedVk_none_of_loadsDefined defined c hC]
  | .index3d base i j k, h => by
      rcases h with ⟨hBase, hI, hJ, hK⟩
      simp [firstUndefinedVk,
        firstUndefinedVk_none_of_loadsDefined defined base hBase,
        firstUndefinedVk_none_of_loadsDefined defined i hI,
        firstUndefinedVk_none_of_loadsDefined defined j hJ,
        firstUndefinedVk_none_of_loadsDefined defined k hK]

def exampleIntrinsicVk : VkExpr :=
  .intrinsic (.load "x") (.fieldGet (.load "tmp"))

def exampleRecordFieldSetVk : VkExpr :=
  .fieldSet (.recordLit (.load "x") .const) (.load "tmp")

def exampleIndex3DVk : VkExpr :=
  .index3d (.load "base") (.load "i") (.load "j") (.load "k")

def exampleRangeBinaryVk : VkExpr :=
  .binary (.range (.load "x") (.load "tmp")) (.unary (.load "x"))

theorem exampleIntrinsicVk_scan_clean :
    firstUndefinedVk
      (iterateOutMap 0 [] exampleStableReachable exampleStablePredMap exampleStableAssignMap
        5 exampleStableSeed 3)
      exampleIntrinsicVk = none := by
  rw [exampleStableSeed_iterate_five_block3]
  exact firstUndefinedVk_none_of_loadsDefined _ _ (by simp [exampleIntrinsicVk, loadsDefinedVk])

theorem exampleRecordFieldSetVk_scan_clean :
    firstUndefinedVk
      (iterateOutMap 0 [] exampleStableReachable exampleStablePredMap exampleStableAssignMap
        5 exampleStableSeed 3)
      exampleRecordFieldSetVk = none := by
  rw [exampleStableSeed_iterate_five_block3]
  exact firstUndefinedVk_none_of_loadsDefined _ _ (by simp [exampleRecordFieldSetVk, loadsDefinedVk])

theorem exampleIndex3DVk_scan_clean :
    firstUndefinedVk ["base", "i", "j", "k"] exampleIndex3DVk = none := by
  exact firstUndefinedVk_none_of_loadsDefined _ _ (by simp [exampleIndex3DVk, loadsDefinedVk])

theorem exampleRangeBinaryVk_scan_clean :
    firstUndefinedVk
      (iterateOutMap 0 [] exampleStableReachable exampleStablePredMap exampleStableAssignMap
        5 exampleStableSeed 3)
      exampleRangeBinaryVk = none := by
  rw [exampleStableSeed_iterate_five_block3]
  exact firstUndefinedVk_none_of_loadsDefined _ _ (by simp [exampleRangeBinaryVk, loadsDefinedVk])

end RRProofs
