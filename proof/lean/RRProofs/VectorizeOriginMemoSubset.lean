namespace RRProofs

abbrev TinyValueId := Nat

inductive TinyKind where
  | load : String -> TinyKind
  | other : TinyKind
deriving Repr

structure TinyNode where
  id : TinyValueId
  originVar? : Option String
  kind : TinyKind
deriving Repr

def boundaryRewrite (target : String) (replacement : TinyValueId) (node : TinyNode) : TinyValueId :=
  match node.originVar? with
  | some origin =>
      if origin = target then
        match node.kind with
        | .load v => if v = target then node.id else replacement
        | .other => replacement
      else node.id
  | none => node.id

theorem boundaryRewrite_exactLoad_keeps_root
    (target : String) (replacement : TinyValueId) (root : TinyValueId) :
    boundaryRewrite target replacement
      { id := root, originVar? := some target, kind := .load target } = root := by
  simp [boundaryRewrite]

theorem boundaryRewrite_origin_nonload_uses_replacement
    (target : String) (replacement root : TinyValueId) :
    boundaryRewrite target replacement
      { id := root, originVar? := some target, kind := .other } = replacement := by
  simp [boundaryRewrite]

theorem boundaryRewrite_origin_mismatched_load_uses_replacement
    (target other : String) (h : other ≠ target) (replacement root : TinyValueId) :
    boundaryRewrite target replacement
      { id := root, originVar? := some target, kind := .load other } = replacement := by
  simp [boundaryRewrite, h]

theorem boundaryRewrite_unrelated_keeps_root
    (target other : String) (h : other ≠ target) (replacement root : TinyValueId) :
    boundaryRewrite target replacement
      { id := root, originVar? := some other, kind := .other } = root := by
  simp [boundaryRewrite, h]

abbrev TinyMemo := TinyValueId -> Option TinyValueId

def memoizedResult (memo : TinyMemo) (root computed : TinyValueId) : TinyValueId :=
  match memo root with
  | some mapped => mapped
  | none => computed

def recordMemo (memo : TinyMemo) (root mapped : TinyValueId) : TinyMemo :=
  fun q => if q = root then some mapped else memo q

theorem memoizedResult_hit_reuses (memo : TinyMemo) (root computed mapped : TinyValueId)
    (h : memo root = some mapped) :
    memoizedResult memo root computed = mapped := by
  simp [memoizedResult, h]

theorem memoizedResult_miss_uses_computed (memo : TinyMemo) (root computed : TinyValueId)
    (h : memo root = none) :
    memoizedResult memo root computed = computed := by
  simp [memoizedResult, h]

theorem recordMemo_reuses_recorded_root
    (memo : TinyMemo) (root mapped computed : TinyValueId) :
    memoizedResult (recordMemo memo root mapped) root computed = mapped := by
  simp [memoizedResult, recordMemo]

def allocateRewriteId (next root : TinyValueId) (changed : Bool) : TinyValueId :=
  if changed then next else root

theorem allocateRewriteId_unchanged_reuses_root (next root : TinyValueId) :
    allocateRewriteId next root false = root := by
  simp [allocateRewriteId]

theorem allocateRewriteId_changed_uses_fresh (next root : TinyValueId) :
    allocateRewriteId next root true = next := by
  simp [allocateRewriteId]

end RRProofs
