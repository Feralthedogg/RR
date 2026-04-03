pub(crate) fn contains(name: &str) -> bool {
    matches!(
        name,
        "splines::bs"
            | "splines::ns"
            | "splines::splineDesign"
            | "splines::interpSpline"
            | "splines::periodicSpline"
            | "splines::backSpline"
            | "splines::splineKnots"
            | "splines::splineOrder"
            | "splines::xyVector"
            | "splines::spline.des"
            | "splines::as.polySpline"
            | "splines::polySpline"
            | "splines::asVector"
    )
}
