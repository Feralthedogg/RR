namespace RRProofs.MirSemanticsLite

inductive MirValue where
  | int : Int -> MirValue
  | bool : Bool -> MirValue
  | null : MirValue
  | array : List MirValue -> MirValue
  | record : List (String × MirValue) -> MirValue
deriving Repr

abbrev Env := List (String × MirValue)

def lookupEnv (env : Env) (name : String) : Option MirValue :=
  match env with
  | [] => none
  | (field, value) :: rest =>
      if field == name then some value else lookupEnv rest name

def updateEnv (env : Env) (name : String) (value : MirValue) : Env :=
  match env with
  | [] => [(name, value)]
  | (field, current) :: rest =>
      if field == name then
        (field, value) :: rest
      else
        (field, current) :: updateEnv rest name value

def updateAt? (xs : List α) (idx : Nat) (value : α) : Option (List α) :=
  match xs, idx with
  | [], _ => none
  | _ :: rest, 0 => some (value :: rest)
  | head :: rest, idx + 1 => do
      let tail <- updateAt? rest idx value
      pure (head :: tail)

def getAt? (xs : List α) (idx : Nat) : Option α :=
  match xs, idx with
  | [], _ => none
  | head :: _, 0 => some head
  | _ :: rest, idx + 1 => getAt? rest idx

def writeIndex1DValue (base : MirValue) (idx : Nat) (value : MirValue) : Option MirValue :=
  match base with
  | .array items => do
      let updated <- updateAt? items idx value
      pure (.array updated)
  | _ => none

def writeIndex2DValue
    (base : MirValue) (row : Nat) (col : Nat) (value : MirValue) : Option MirValue := do
  match base with
  | .array rows =>
      match getAt? rows row with
      | some rowValue =>
          let updatedRow <- writeIndex1DValue rowValue col value
          let updatedRows <- updateAt? rows row updatedRow
          pure (.array updatedRows)
      | none => none
  | _ => none

def writeIndex3DValue
    (base : MirValue) (i : Nat) (j : Nat) (k : Nat) (value : MirValue) : Option MirValue := do
  match base with
  | .array planes =>
      match getAt? planes i with
      | some plane =>
          let updatedPlane <- writeIndex2DValue plane j k value
          let updatedPlanes <- updateAt? planes i updatedPlane
          pure (.array updatedPlanes)
      | none => none
  | _ => none

inductive MirExpr where
  | const : MirValue -> MirExpr
  | load : String -> MirExpr
  | add : MirExpr -> MirExpr -> MirExpr
  | mul : MirExpr -> MirExpr -> MirExpr
  | neg : MirExpr -> MirExpr
  | lt : MirExpr -> MirExpr -> MirExpr
deriving Repr

def evalExpr (env : Env) : MirExpr -> Option MirValue
  | .const value => some value
  | .load name => lookupEnv env name
  | .add lhs rhs => do
      let lv <- evalExpr env lhs
      let rv <- evalExpr env rhs
      match lv, rv with
      | .int l, .int r => some (.int (l + r))
      | _, _ => none
  | .mul lhs rhs => do
      let lv <- evalExpr env lhs
      let rv <- evalExpr env rhs
      match lv, rv with
      | .int l, .int r => some (.int (l * r))
      | _, _ => none
  | .neg arg => do
      let v <- evalExpr env arg
      match v with
      | .int i => some (.int (-i))
      | _ => none
  | .lt lhs rhs => do
      let lv <- evalExpr env lhs
      let rv <- evalExpr env rhs
      match lv, rv with
      | .int l, .int r => some (.bool (l < r))
      | _, _ => none

inductive MirInstr where
  | assign : String -> MirExpr -> MirInstr
  | eval : MirExpr -> MirInstr
  | storeIndex1D : String -> MirExpr -> MirExpr -> MirInstr
  | storeIndex2D : String -> MirExpr -> MirExpr -> MirExpr -> MirInstr
  | storeIndex3D : String -> MirExpr -> MirExpr -> MirExpr -> MirExpr -> MirInstr
deriving Repr

inductive MirTerm where
  | goto : Nat -> MirTerm
  | ite : MirExpr -> Nat -> Nat -> MirTerm
  | ret : Option MirExpr -> MirTerm
  | unreachable : MirTerm
deriving Repr

structure MirPhiArm where
  pred : Nat
  expr : MirExpr
deriving Repr

structure MirPhi where
  dst : String
  arms : List MirPhiArm
deriving Repr

structure MirBlock where
  id : Nat
  phis : List MirPhi
  instrs : List MirInstr
  term : MirTerm
deriving Repr

def evalPhiAssignments (pred : Nat) (phis : List MirPhi) (env : Env) :
    Option (List (String × MirValue)) :=
  match phis with
  | [] => some []
  | phi :: rest => do
      let arm <- phi.arms.find? (fun arm => arm.pred = pred)
      let value <- evalExpr env arm.expr
      let tail <- evalPhiAssignments pred rest env
      pure ((phi.dst, value) :: tail)

def applyPhiNodes (pred : Nat) (phis : List MirPhi) (env : Env) : Option Env := do
  let assigns <- evalPhiAssignments pred phis env
  pure (assigns.foldl (fun acc (dst, value) => updateEnv acc dst value) env)

def asNatIndex (value : MirValue) : Option Nat :=
  match value with
  | .int i => some i.natAbs
  | _ => none

def execInstr (env : Env) : MirInstr -> Option Env
  | .assign dst rhs => do
      let value <- evalExpr env rhs
      pure (updateEnv env dst value)
  | .eval rhs => do
      let _ <- evalExpr env rhs
      pure env
  | .storeIndex1D base idx rhs => do
      let baseValue <- lookupEnv env base
      let idxValue <- evalExpr env idx
      let rhsValue <- evalExpr env rhs
      let idxNat <- asNatIndex idxValue
      let updated <- writeIndex1DValue baseValue idxNat rhsValue
      pure (updateEnv env base updated)
  | .storeIndex2D base row col rhs => do
      let baseValue <- lookupEnv env base
      let rowValue <- evalExpr env row
      let colValue <- evalExpr env col
      let rhsValue <- evalExpr env rhs
      let rowNat <- asNatIndex rowValue
      let colNat <- asNatIndex colValue
      let updated <- writeIndex2DValue baseValue rowNat colNat rhsValue
      pure (updateEnv env base updated)
  | .storeIndex3D base i j k rhs => do
      let baseValue <- lookupEnv env base
      let iValue <- evalExpr env i
      let jValue <- evalExpr env j
      let kValue <- evalExpr env k
      let rhsValue <- evalExpr env rhs
      let iNat <- asNatIndex iValue
      let jNat <- asNatIndex jValue
      let kNat <- asNatIndex kValue
      let updated <- writeIndex3DValue baseValue iNat jNat kNat rhsValue
      pure (updateEnv env base updated)

def execInstrs (env : Env) : List MirInstr -> Option Env
  | [] => some env
  | instr :: rest => do
      let env' <- execInstr env instr
      execInstrs env' rest

inductive BlockExit where
  | jump : Nat -> Env -> BlockExit
  | done : Option MirValue -> Env -> BlockExit
  | stuck : BlockExit
deriving Repr

def execTerm (env : Env) : MirTerm -> BlockExit
  | .goto target => .jump target env
  | .ite cond thenBlk elseBlk =>
      match evalExpr env cond with
      | some (.bool true) => .jump thenBlk env
      | some (.bool false) => .jump elseBlk env
      | _ => .stuck
  | .ret none => .done none env
  | .ret (some expr) =>
      match evalExpr env expr with
      | some value => .done (some value) env
      | none => .stuck
  | .unreachable => .stuck

def execBlockEntry (pred : Nat) (blk : MirBlock) (env : Env) : BlockExit :=
  match applyPhiNodes pred blk.phis env with
  | none => .stuck
  | some env' =>
      match execInstrs env' blk.instrs with
      | none => .stuck
      | some env'' => execTerm env'' blk.term

def observableOfExit : BlockExit -> Option (Option MirValue)
  | .jump _ _ => none
  | .done result _ => some result
  | .stuck => none

theorem applyPhiNodes_nil (pred : Nat) (env : Env) :
    applyPhiNodes pred [] env = some env := by
  simp [applyPhiNodes, evalPhiAssignments]

theorem execInstrs_append (env : Env) (instrPrefix suffix : List MirInstr) :
    execInstrs env (instrPrefix ++ suffix) =
      match execInstrs env instrPrefix with
      | some env' => execInstrs env' suffix
      | none => none := by
  induction instrPrefix generalizing env with
  | nil =>
      simp [execInstrs]
  | cons instr rest ih =>
      cases h : execInstr env instr <;> simp [execInstrs, h, ih]

theorem execBlockEntry_without_phis (pred : Nat) (blk : MirBlock) (env : Env) :
    execBlockEntry pred { blk with phis := [] } env =
      match execInstrs env blk.instrs with
      | some env' => execTerm env' blk.term
      | none => .stuck := by
  simp [execBlockEntry, applyPhiNodes_nil]
  cases execInstrs env blk.instrs <;> rfl

end RRProofs.MirSemanticsLite
