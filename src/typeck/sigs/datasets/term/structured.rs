use crate::typeck::term::TypeTerm;

pub(crate) fn infer_datasets_structured_binding_term(var: &str) -> Option<TypeTerm> {
    match var {
        "datasets::gait" => Some(TypeTerm::ArrayDim(
            Box::new(TypeTerm::Double),
            vec![Some(20), Some(39), Some(2)],
        )),
        "datasets::crimtab" => Some(TypeTerm::ArrayDim(
            Box::new(TypeTerm::Int),
            vec![Some(42), Some(22)],
        )),
        "datasets::occupationalStatus" => Some(TypeTerm::ArrayDim(
            Box::new(TypeTerm::Int),
            vec![Some(8), Some(8)],
        )),
        "datasets::ability.cov" => Some(TypeTerm::NamedList(vec![
            (
                "cov".to_string(),
                TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(6), Some(6)),
            ),
            (
                "center".to_string(),
                TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(6)),
            ),
            ("n.obs".to_string(), TypeTerm::Double),
        ])),
        "datasets::Harman23.cor" => Some(TypeTerm::NamedList(vec![
            (
                "cov".to_string(),
                TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(8), Some(8)),
            ),
            (
                "center".to_string(),
                TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(8)),
            ),
            ("n.obs".to_string(), TypeTerm::Double),
        ])),
        "datasets::Harman74.cor" => Some(TypeTerm::NamedList(vec![
            (
                "cov".to_string(),
                TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(24), Some(24)),
            ),
            (
                "center".to_string(),
                TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(24)),
            ),
            ("n.obs".to_string(), TypeTerm::Double),
        ])),
        "datasets::state.center" => Some(TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(50)),
            ),
            (
                "y".to_string(),
                TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(50)),
            ),
        ])),
        "datasets::iris3" => Some(TypeTerm::ArrayDim(
            Box::new(TypeTerm::Double),
            vec![Some(50), Some(4), Some(3)],
        )),
        "datasets::Titanic" => Some(TypeTerm::ArrayDim(
            Box::new(TypeTerm::Double),
            vec![Some(4), Some(2), Some(2), Some(2)],
        )),
        "datasets::UCBAdmissions" => Some(TypeTerm::ArrayDim(
            Box::new(TypeTerm::Double),
            vec![Some(2), Some(2), Some(6)],
        )),
        "datasets::HairEyeColor" => Some(TypeTerm::ArrayDim(
            Box::new(TypeTerm::Double),
            vec![Some(4), Some(4), Some(2)],
        )),
        _ => None,
    }
}
