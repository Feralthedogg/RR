# Splines

Splines package direct interop surface.
Part of the [R Interop](../r-interop.md) reference.

## Direct Surface

- `splines::bs`
- `splines::ns`
- `splines::splineDesign`
- `splines::interpSpline`
- `splines::periodicSpline`
- `splines::backSpline`
- `splines::spline.des`
- `splines::as.polySpline`
- `splines::polySpline`
- `splines::asVector`
- `splines::splineKnots`
- `splines::splineOrder`
- `splines::xyVector`

Selected splines calls also keep direct type information:

- `splines::bs`, `splines::ns`, `splines::splineDesign` -> matrix double
- `splines::interpSpline`, `splines::periodicSpline`, `splines::backSpline`, `splines::xyVector` -> list-like opaque object
- `splines::spline.des`, `splines::as.polySpline`, `splines::polySpline` -> list-like opaque object
- `splines::asVector` -> vector double
- `splines::splineKnots` -> vector double
- `splines::splineOrder` -> scalar int

