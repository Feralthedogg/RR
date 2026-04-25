namespace RRProofs

abbrev ReducedTraitType := String

structure ReducedTraitImpl where
  traitName : String
  forType : ReducedTraitType
  methodName : String
  targetName : String
  negative : Bool
  deriving DecidableEq, Repr

def resolvesTraitMethod
    (traitName methodName : String)
    (receiverType : ReducedTraitType)
    (implEntry : ReducedTraitImpl) : Bool :=
  implEntry.traitName == traitName
    && implEntry.methodName == methodName
    && implEntry.forType == receiverType
    && implEntry.negative == false

def resolveTraitTarget
    (impls : List ReducedTraitImpl)
    (traitName methodName : String)
    (receiverType : ReducedTraitType) : Option String :=
  (impls.find? (resolvesTraitMethod traitName methodName receiverType)).map
    (fun implEntry => implEntry.targetName)

theorem trait_resolution_preserves_static_dispatch_target
    (implEntry : ReducedTraitImpl)
    (tail : List ReducedTraitImpl)
    (hResolve :
      resolvesTraitMethod traitName methodName receiverType implEntry = true) :
    resolveTraitTarget (implEntry :: tail) traitName methodName receiverType =
      some implEntry.targetName := by
  simp [resolveTraitTarget, hResolve]

theorem negative_impl_does_not_resolve
    (implEntry : ReducedTraitImpl)
    (hNegative : implEntry.negative = true) :
    resolvesTraitMethod traitName methodName receiverType implEntry = false := by
  simp [resolvesTraitMethod, hNegative]

inductive ReducedOperator where
  | add
  | sub
  | mul
  | div
  | mod
  | matmul
  | neg
  | index
  deriving DecidableEq, Repr

def reducedOperatorTrait : ReducedOperator -> String × String
  | .add => ("Add", "add")
  | .sub => ("Sub", "sub")
  | .mul => ("Mul", "mul")
  | .div => ("Div", "div")
  | .mod => ("Mod", "mod")
  | .matmul => ("MatMul", "matmul")
  | .neg => ("Neg", "neg")
  | .index => ("Index", "index")

theorem neg_operator_maps_to_neg_trait :
    reducedOperatorTrait .neg = ("Neg", "neg") := by
  rfl

structure ReducedTraitMetadata where
  name : String
  isPublic : Bool
  deriving DecidableEq, Repr

def exportedTraitMetadata
    (entries : List ReducedTraitMetadata) : List ReducedTraitMetadata :=
  entries.filter (fun entry => entry.isPublic)

theorem exported_trait_metadata_keeps_public_head
    (entry : ReducedTraitMetadata)
    (tail : List ReducedTraitMetadata)
    (hPublic : entry.isPublic = true) :
    exportedTraitMetadata (entry :: tail) = entry :: exportedTraitMetadata tail := by
  simp [exportedTraitMetadata, hPublic]

theorem exported_trait_metadata_drops_private_head
    (entry : ReducedTraitMetadata)
    (tail : List ReducedTraitMetadata)
    (hPrivate : entry.isPublic = false) :
    exportedTraitMetadata (entry :: tail) = exportedTraitMetadata tail := by
  simp [exportedTraitMetadata, hPrivate]

structure ReducedGenericInstance where
  genericName : String
  concreteTypes : List ReducedTraitType
  monomorphizedName : String
  deriving DecidableEq, Repr

def matchesGenericInstance
    (genericName : String)
    (concreteTypes : List ReducedTraitType)
    (inst : ReducedGenericInstance) : Bool :=
  inst.genericName == genericName && inst.concreteTypes == concreteTypes

def resolveMonomorphizedTarget
    (instances : List ReducedGenericInstance)
    (genericName : String)
    (concreteTypes : List ReducedTraitType) : Option String :=
  (instances.find? (matchesGenericInstance genericName concreteTypes)).map
    (fun inst => inst.monomorphizedName)

theorem monomorphization_preserves_resolved_target
    (inst : ReducedGenericInstance)
    (tail : List ReducedGenericInstance)
    (hResolve : matchesGenericInstance genericName concreteTypes inst = true) :
    resolveMonomorphizedTarget (inst :: tail) genericName concreteTypes =
      some inst.monomorphizedName := by
  simp [resolveMonomorphizedTarget, hResolve]

def repeatedParamPairOverlapsExact
    (leftComponent rightComponent : ReducedTraitType) : Bool :=
  leftComponent == rightComponent

theorem repeated_param_pair_rejects_inconsistent_exact_types :
    repeatedParamPairOverlapsExact "int" "float" = false := by
  rfl

theorem repeated_param_pair_accepts_consistent_exact_type
    (component : ReducedTraitType) :
    repeatedParamPairOverlapsExact component component = true := by
  simp [repeatedParamPairOverlapsExact]

structure ReducedAssocProjection where
  baseType : ReducedTraitType
  ownerTrait : String
  assocName : String
  resolvedType : ReducedTraitType
  deriving DecidableEq, Repr

def matchesAssocProjection
    (baseType ownerTrait assocName : String)
    (entry : ReducedAssocProjection) : Bool :=
  entry.baseType == baseType
    && entry.ownerTrait == ownerTrait
    && entry.assocName == assocName

def resolveAssocProjection
    (entries : List ReducedAssocProjection)
    (baseType ownerTrait assocName : String) : Option ReducedTraitType :=
  (entries.find? (matchesAssocProjection baseType ownerTrait assocName)).map
    (fun entry => entry.resolvedType)

theorem qualified_assoc_projection_preserves_owner_resolution
    (entry : ReducedAssocProjection)
    (tail : List ReducedAssocProjection)
    (hResolve :
      matchesAssocProjection baseType ownerTrait assocName entry = true) :
    resolveAssocProjection (entry :: tail) baseType ownerTrait assocName =
      some entry.resolvedType := by
  simp [resolveAssocProjection, hResolve]

theorem qualified_assoc_projection_ignores_sibling_owner
    (sibling entry : ReducedAssocProjection)
    (tail : List ReducedAssocProjection)
    (hSibling :
      matchesAssocProjection baseType ownerTrait assocName sibling = false)
    (hResolve :
      matchesAssocProjection baseType ownerTrait assocName entry = true) :
    resolveAssocProjection (sibling :: entry :: tail) baseType ownerTrait assocName =
      some entry.resolvedType := by
  simp [resolveAssocProjection, hSibling, hResolve]

end RRProofs
