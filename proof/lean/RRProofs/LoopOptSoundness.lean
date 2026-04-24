import RRProofs.MirInvariantBundle
import RRProofs.DataflowOptSoundness
import RRProofs.CfgOptSoundness
import RRProofs.CfgHoist
import RRProofs.ReducedFnIR
import RRProofs.CfgSmallStep
import RRProofs.GraphLicmSound

namespace RRProofs.LoopOptSoundness

open RRProofs.MirInvariantBundle

theorem licm_zero_trip_preserves_semantics
    (c : LicmGraphCase)
    (entry locals : RRProofs.State)
    (hSafe : c.safeCandidate) :
    (RRProofs.runOriginalMachine c.fnir false entry locals).result? =
      (RRProofs.runHoistedMachine c.fnir false entry locals).result? := by
  exact graph_level_zero_trip_sound c entry locals hSafe

theorem licm_one_trip_preserves_semantics
    (c : LicmGraphCase)
    (entry locals : RRProofs.State)
    (hGraph : c.graphWf)
    (hSafe : c.safeCandidate) :
    (RRProofs.runOriginalMachine c.fnir true entry locals).result? =
      (RRProofs.runHoistedMachine c.fnir true entry locals).result? := by
  exact graph_level_one_trip_sound c entry locals hGraph hSafe

theorem licm_loop_carried_state_not_sound
    (entry locals : RRProofs.State)
    (h : locals "time" + 1 ≠ entry "time0") :
    (RRProofs.runOriginalMachine exampleLicmGraphCase.fnir true entry locals).result? ≠
      (RRProofs.runHoistedMachine exampleLicmGraphCase.fnir true entry locals).result? := by
  exact exampleLicmGraphCase_unsound_machine entry locals h

def bceOriginalRead (xs : List Nat) (idx : Nat) : Option Nat :=
  if idx < xs.length then RRProofs.MirSemanticsLite.getAt? xs idx else none

def bceOptimizedRead (xs : List Nat) (idx : Nat) : Option Nat :=
  RRProofs.MirSemanticsLite.getAt? xs idx

theorem getAt?_none_of_ge {xs : List α} {idx : Nat} (h : xs.length ≤ idx) :
    RRProofs.MirSemanticsLite.getAt? xs idx = none := by
  induction xs generalizing idx with
  | nil =>
      simp [RRProofs.MirSemanticsLite.getAt?]
  | cons head rest ih =>
      cases idx with
      | zero =>
          simp at h
      | succ idx =>
          simp [RRProofs.MirSemanticsLite.getAt?]
          exact ih (Nat.le_of_succ_le_succ h)

theorem bce_reduced_preserves_semantics
    (xs : List Nat) (idx : Nat) :
    bceOriginalRead xs idx = bceOptimizedRead xs idx := by
  unfold bceOriginalRead bceOptimizedRead
  by_cases h : idx < xs.length
  · simp [h]
  · simp [h]
    symm
    exact getAt?_none_of_ge (Nat.le_of_not_gt h)

def tcoOriginal : Nat -> Nat -> Nat
  | 0, acc => acc
  | n + 1, acc => tcoOriginal n (acc + 1)

def tcoOptimized (n acc : Nat) : Nat :=
  acc + n

theorem tco_reduced_preserves_semantics (n acc : Nat) :
    tcoOriginal n acc = tcoOptimized n acc := by
  induction n generalizing acc with
  | zero =>
      simp [tcoOriginal, tcoOptimized]
  | succ n ih =>
      simp [tcoOriginal, tcoOptimized, ih, Nat.add_left_comm, Nat.add_comm]

theorem loop_opt_identity_preserves_verify_ir_bundle
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (identityPass fn) := by
  exact identity_pass_preserves_verify_ir_bundle h

end RRProofs.LoopOptSoundness
