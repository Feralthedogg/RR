# Parallel

Parallel package direct interop surface.
Part of the [R Interop](../r-interop.md) reference.

## Direct Surface

- `parallel::detectCores`
- `parallel::makeCluster`
- `parallel::stopCluster`
- `parallel::parLapply`
- `parallel::clusterExport`
- `parallel::clusterEvalQ`
- `parallel::clusterMap`
- `parallel::clusterApply`
- `parallel::clusterCall`
- `parallel::mclapply`
- `parallel::clusterSplit`
- `parallel::splitIndices`
- `parallel::clusterApplyLB`
- `parallel::parSapply`
- `parallel::parSapplyLB`
- `parallel::parApply`
- `parallel::mcparallel`
- `parallel::mccollect`

Selected parallel calls also keep direct type information:

- `parallel::detectCores` -> scalar int
- `parallel::makeCluster`, `parallel::parLapply`, `parallel::clusterEvalQ`, `parallel::clusterMap`, `parallel::clusterApply`, `parallel::clusterCall`, `parallel::mclapply`, `parallel::clusterSplit`, `parallel::splitIndices`, `parallel::clusterApplyLB`, `parallel::mcparallel`, `parallel::mccollect` -> list-like opaque object
- `parallel::parSapply`, `parallel::parSapplyLB`, `parallel::parApply` -> vector-like opaque object
- `parallel::stopCluster`, `parallel::clusterExport` -> null

