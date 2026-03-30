# Stats4

Stats4 package direct interop surface.
Part of the [R Interop](../r-interop.md) reference.

## Direct Surface

- `stats4::mle`
- `stats4::coef`
- `stats4::vcov`
- `stats4::confint`
- `stats4::logLik`
- `stats4::AIC`
- `stats4::BIC`
- `stats4::nobs`
- `stats4::update`
- `stats4::summary`
- `stats4::profile`
- `stats4::plot`
- `stats4::show`

Selected stats4 calls also keep direct type information:

- `stats4::mle`, `stats4::update`, `stats4::summary`, `stats4::profile`, `stats4::plot` -> list-like opaque object
- `stats4::coef`, `stats4::confint` -> vector double
- `stats4::vcov` -> matrix double
- `stats4::logLik`, `stats4::AIC`, `stats4::BIC`, `stats4::show` -> scalar double
- `stats4::nobs` -> scalar int

