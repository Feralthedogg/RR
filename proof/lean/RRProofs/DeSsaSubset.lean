namespace RRProofs

inductive CopyValue where
  | constInt : Int -> CopyValue
  | intrinsic1 : String -> CopyValue -> CopyValue
  | record1 : String -> CopyValue -> CopyValue
  | fieldGet : CopyValue -> String -> CopyValue
  | fieldSet : CopyValue -> String -> CopyValue -> CopyValue
deriving Repr

mutual
  def evalCopy : CopyValue -> Option Int
    | .constInt i => some i
    | .intrinsic1 op arg =>
        match evalCopy arg with
        | some i =>
            if op = "neg" then some (-i) else some i
        | none => none
    | .record1 _field value => evalCopy value
    | .fieldGet base field =>
        match base with
        | .record1 name value =>
            if name = field then evalCopy value else none
        | .fieldSet prior name value =>
            if name = field then evalCopy value else evalCopy (.fieldGet prior field)
        | _ => none
    | .fieldSet _base _field value => evalCopy value
end

mutual
  def copyFingerprint : CopyValue -> Int
    | .constInt i => i
    | .intrinsic1 op arg => 2000 + Int.ofNat op.length + 31 * copyFingerprint arg
    | .record1 field value => 3000 + Int.ofNat field.length + 37 * copyFingerprint value
    | .fieldGet base field => 4000 + Int.ofNat field.length + 41 * copyFingerprint base
    | .fieldSet base field value =>
        5000 + Int.ofNat field.length + 43 * copyFingerprint base + 47 * copyFingerprint value
end

def sameCanonicalValue (lhs rhs : CopyValue) : Bool :=
  copyFingerprint lhs == copyFingerprint rhs

theorem sameCanonicalValue_refl (v : CopyValue) : sameCanonicalValue v v := by
  simp [sameCanonicalValue]

def noMoveNeeded (existing incoming : CopyValue) : Bool :=
  sameCanonicalValue existing incoming

theorem noMoveNeeded_true_of_sameCanonicalValue
    {existing incoming : CopyValue}
    (h : sameCanonicalValue existing incoming) :
    noMoveNeeded existing incoming = true := by
  simp [noMoveNeeded, h]

theorem noMoveNeeded_self_field_get :
    noMoveNeeded
      (.fieldGet (.record1 "x" (.constInt 3)) "x")
      (.fieldGet (.record1 "x" (.constInt 3)) "x") = true := by
  exact noMoveNeeded_true_of_sameCanonicalValue (sameCanonicalValue_refl _)

theorem noMoveNeeded_self_intrinsic :
    noMoveNeeded
      (.intrinsic1 "neg" (.constInt 3))
      (.intrinsic1 "neg" (.constInt 3)) = true := by
  exact noMoveNeeded_true_of_sameCanonicalValue (sameCanonicalValue_refl _)

theorem noMoveNeeded_self_fieldset :
    noMoveNeeded
      (.fieldSet (.record1 "x" (.constInt 1)) "x" (.constInt 7))
      (.fieldSet (.record1 "x" (.constInt 1)) "x" (.constInt 7)) = true := by
  exact noMoveNeeded_true_of_sameCanonicalValue (sameCanonicalValue_refl _)

theorem self_sameCanonicalValue_preserves_eval (v : CopyValue) :
    sameCanonicalValue v v ∧ evalCopy v = evalCopy v := by
  exact ⟨sameCanonicalValue_refl v, rfl⟩

end RRProofs
