use crate::typeck::lattice::{PrimTy, TypeState};
use crate::typeck::term::TypeTerm;

pub(crate) fn infer_parallel_package_call(
    callee: &str,
    _arg_tys: &[TypeState],
) -> Option<TypeState> {
    match callee {
        "parallel::detectCores" => Some(TypeState::scalar(PrimTy::Int, false)),
        "parallel::makeCluster"
        | "parallel::makeForkCluster"
        | "parallel::makePSOCKcluster"
        | "parallel::parLapply"
        | "parallel::parLapplyLB"
        | "parallel::clusterEvalQ"
        | "parallel::clusterMap"
        | "parallel::clusterApply"
        | "parallel::clusterCall"
        | "parallel::mclapply"
        | "parallel::mcMap"
        | "parallel::clusterSplit"
        | "parallel::splitIndices"
        | "parallel::getDefaultCluster"
        | "parallel::recvData"
        | "parallel::recvOneData"
        | "parallel::clusterApplyLB" => Some(TypeState::vector(PrimTy::Any, false)),
        "parallel::parSapply"
        | "parallel::parSapplyLB"
        | "parallel::parApply"
        | "parallel::parCapply"
        | "parallel::parRapply"
        | "parallel::pvec"
        | "parallel::mcmapply" => Some(TypeState::vector(PrimTy::Any, false)),
        "parallel::nextRNGStream" | "parallel::nextRNGSubStream" | "parallel::mcaffinity" => {
            Some(TypeState::vector(PrimTy::Int, false))
        }
        "parallel::mcparallel" | "parallel::mccollect" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "parallel::stopCluster"
        | "parallel::clusterExport"
        | "parallel::closeNode"
        | "parallel::clusterSetRNGStream"
        | "parallel::mc.reset.stream"
        | "parallel::sendData"
        | "parallel::registerClusterType"
        | "parallel::setDefaultCluster" => Some(TypeState::null()),
        _ => None,
    }
}

pub(crate) fn infer_parallel_package_call_term(
    callee: &str,
    _arg_terms: &[TypeTerm],
) -> Option<TypeTerm> {
    match callee {
        "parallel::detectCores" => Some(TypeTerm::Int),
        "parallel::makeCluster"
        | "parallel::makeForkCluster"
        | "parallel::makePSOCKcluster"
        | "parallel::parLapply"
        | "parallel::parLapplyLB"
        | "parallel::clusterEvalQ"
        | "parallel::clusterMap"
        | "parallel::clusterApply"
        | "parallel::clusterCall"
        | "parallel::mclapply"
        | "parallel::mcMap"
        | "parallel::clusterSplit"
        | "parallel::splitIndices"
        | "parallel::getDefaultCluster"
        | "parallel::recvData"
        | "parallel::recvOneData"
        | "parallel::clusterApplyLB"
        | "parallel::mcparallel"
        | "parallel::mccollect" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "parallel::parSapply"
        | "parallel::parSapplyLB"
        | "parallel::parApply"
        | "parallel::parCapply"
        | "parallel::parRapply"
        | "parallel::pvec"
        | "parallel::mcmapply" => Some(TypeTerm::Vector(Box::new(TypeTerm::Any))),
        "parallel::nextRNGStream" | "parallel::nextRNGSubStream" | "parallel::mcaffinity" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Int)))
        }
        "parallel::stopCluster"
        | "parallel::clusterExport"
        | "parallel::closeNode"
        | "parallel::clusterSetRNGStream"
        | "parallel::mc.reset.stream"
        | "parallel::sendData"
        | "parallel::registerClusterType"
        | "parallel::setDefaultCluster" => Some(TypeTerm::Null),
        _ => None,
    }
}
