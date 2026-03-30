# Compiler

Compiler package direct interop surface.
Part of the [R Interop](../r-interop.md) reference.

## Direct Surface

- `compiler::enableJIT`
- `compiler::getCompilerOption`
- `compiler::setCompilerOptions`
- `compiler::compile`
- `compiler::compilePKGS`
- `compiler::cmpfun`
- `compiler::disassemble`
- `compiler::cmpfile`
- `compiler::loadcmp`

Selected compiler calls also keep direct type information:

- `compiler::enableJIT` -> scalar int
- `compiler::compilePKGS` -> scalar logical
- `compiler::getCompilerOption` -> scalar opaque value by default; literal options like `"optimize"` and `"suppressAll"` narrow to scalar int/logical, `"suppressUndefined"` narrows to vector char, and `"suppressNoSuperAssignVar"` narrows to scalar logical
- `compiler::setCompilerOptions` -> named list of previous option values for the provided named options, preserving typed fields such as `optimize`, `suppressAll`, `suppressUndefined`, and `suppressNoSuperAssignVar`
- `compiler::compile`, `compiler::disassemble` -> list-like opaque object
- `compiler::cmpfile`, `compiler::loadcmp` -> null
- `compiler::cmpfun` -> opaque callable object

