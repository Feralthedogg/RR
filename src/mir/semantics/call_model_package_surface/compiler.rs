pub(crate) fn contains(name: &str) -> bool {
    matches!(
        name,
        "compiler::enableJIT"
            | "compiler::getCompilerOption"
            | "compiler::setCompilerOptions"
            | "compiler::compile"
            | "compiler::compilePKGS"
            | "compiler::cmpfun"
            | "compiler::disassemble"
            | "compiler::cmpfile"
            | "compiler::loadcmp"
    )
}
