import RRProofs.DeSsaSubset
import RRProofs.MirInvariantBundle

namespace RRProofs.DeSsaBoundarySoundness

open RRProofs.MirInvariantBundle

def copyBoundaryOriginal (existing incoming : CopyValue) : Option Int :=
  if noMoveNeeded existing incoming then evalCopy existing else evalCopy incoming

def copyBoundaryOptimized (existing _incoming : CopyValue) : Option Int :=
  evalCopy existing

theorem de_ssa_redundant_move_elimination_preserves_eval
    {existing incoming : CopyValue}
    (h : noMoveNeeded existing incoming = true) :
    copyBoundaryOriginal existing incoming = copyBoundaryOptimized existing incoming := by
  simp [copyBoundaryOriginal, copyBoundaryOptimized, h]

theorem de_ssa_self_field_get_preserves_eval :
    copyBoundaryOriginal
      (.fieldGet (.record1 "x" (.constInt 3)) "x")
      (.fieldGet (.record1 "x" (.constInt 3)) "x")
      =
    copyBoundaryOptimized
      (.fieldGet (.record1 "x" (.constInt 3)) "x")
      (.fieldGet (.record1 "x" (.constInt 3)) "x") := by
  exact de_ssa_redundant_move_elimination_preserves_eval noMoveNeeded_self_field_get

theorem de_ssa_self_intrinsic_preserves_eval :
    copyBoundaryOriginal
      (.intrinsic1 "neg" (.constInt 3))
      (.intrinsic1 "neg" (.constInt 3))
      =
    copyBoundaryOptimized
      (.intrinsic1 "neg" (.constInt 3))
      (.intrinsic1 "neg" (.constInt 3)) := by
  exact de_ssa_redundant_move_elimination_preserves_eval noMoveNeeded_self_intrinsic

theorem de_ssa_self_fieldset_preserves_eval :
    copyBoundaryOriginal
      (.fieldSet (.record1 "x" (.constInt 1)) "x" (.constInt 7))
      (.fieldSet (.record1 "x" (.constInt 1)) "x" (.constInt 7))
      =
    copyBoundaryOptimized
      (.fieldSet (.record1 "x" (.constInt 1)) "x" (.constInt 7))
      (.fieldSet (.record1 "x" (.constInt 1)) "x" (.constInt 7)) := by
  exact de_ssa_redundant_move_elimination_preserves_eval noMoveNeeded_self_fieldset

theorem de_ssa_boundary_identity_preserves_verify_ir_bundle
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (identityPass fn) := by
  exact identity_pass_preserves_verify_ir_bundle h

theorem de_ssa_boundary_identity_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (identityPass fn) env = execEntry fn env := by
  exact identity_pass_preserves_semantics fn env

end RRProofs.DeSsaBoundarySoundness
