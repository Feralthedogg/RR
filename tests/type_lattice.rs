use rr::compiler::internal::typeck::{NaTy, PrimTy, ShapeTy, TypeState};

#[test]
fn lattice_join_promotes_numeric_and_merges_na() {
    let a = TypeState::scalar(PrimTy::Int, true);
    let b = TypeState::scalar(PrimTy::Double, false);
    let j = a.join(b);
    assert_eq!(j.prim, PrimTy::Double);
    assert_eq!(j.shape, ShapeTy::Scalar);
    assert_eq!(j.na, NaTy::Maybe);
}

#[test]
fn lattice_join_mismatched_shapes_becomes_unknown_shape() {
    let a = TypeState::scalar(PrimTy::Double, true);
    let b = TypeState::vector(PrimTy::Double, true);
    let j = a.join(b);
    assert_eq!(j.prim, PrimTy::Double);
    assert_eq!(j.shape, ShapeTy::Unknown);
}

#[test]
fn lattice_join_unknown_yields_other_side() {
    let a = TypeState::unknown();
    let b = TypeState::scalar(PrimTy::Logical, true);
    assert_eq!(a.join(b), b);
    assert_eq!(b.join(a), b);
}
