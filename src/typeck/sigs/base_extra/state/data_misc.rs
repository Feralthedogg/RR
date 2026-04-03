use crate::typeck::builtin_sigs::*;
use crate::typeck::lattice::{PrimTy, TypeState};

pub(crate) fn infer_base_extra_package_call_data_misc(
    callee: &str,
    arg_tys: &[TypeState],
) -> Option<TypeState> {
    match callee {
        "base::lapply" | "base::Map" | "base::split" | "base::by" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "base::sapply" | "base::vapply" | "base::mapply" | "base::tapply" | "base::apply" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "base::Reduce" | "base::Find" => Some(TypeState::unknown()),
        "base::Filter" | "base::unsplit" | "base::within" | "base::transform" => Some(
            preserved_first_arg_type_without_len(first_arg_type_state(arg_tys)),
        ),
        "base::Position" => Some(TypeState::scalar(PrimTy::Int, false)),
        "base::expand.grid" | "base::merge" => Some(TypeState::matrix(PrimTy::Any, false)),
        "base::as.Date"
        | "base::as.Date.character"
        | "base::as.Date.default"
        | "base::as.Date.factor"
        | "base::as.Date.numeric"
        | "base::as.Date.POSIXct"
        | "base::as.Date.POSIXlt"
        | "base::as.POSIXct"
        | "base::as.POSIXct.Date"
        | "base::as.POSIXct.default"
        | "base::as.POSIXct.numeric"
        | "base::as.POSIXct.POSIXlt"
        | "base::as.POSIXlt"
        | "base::as.POSIXlt.character"
        | "base::as.POSIXlt.Date"
        | "base::as.POSIXlt.default"
        | "base::as.POSIXlt.factor"
        | "base::as.POSIXlt.numeric"
        | "base::as.POSIXlt.POSIXct"
        | "base::as.difftime"
        | "base::as.double.difftime"
        | "base::as.double.POSIXlt"
        | "base::strptime"
        | "base::difftime"
        | "base::julian" => Some(double_like_first_arg_type(first_arg_type_state(arg_tys))),
        "base::as.character.Date"
        | "base::as.character.POSIXt"
        | "base::format.Date"
        | "base::format.POSIXct"
        | "base::format.POSIXlt"
        | "base::months"
        | "base::quarters"
        | "base::weekdays" => Some(char_like_first_arg_type(first_arg_type_state(arg_tys))),
        "base::OlsonNames" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::ISOdate" | "base::ISOdatetime" | "base::seq.Date" | "base::seq.POSIXt" => {
            Some(TypeState::vector(PrimTy::Double, false))
        }
        "base::all.names" | "base::all.vars" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::anyDuplicated.array"
        | "base::anyDuplicated.data.frame"
        | "base::anyDuplicated.default"
        | "base::anyDuplicated.matrix" => Some(TypeState::scalar(PrimTy::Int, false)),
        "base::anyNA" | "base::anyNA.data.frame" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "base::anyNA.numeric_version" | "base::anyNA.POSIXlt" => {
            Some(TypeState::scalar(PrimTy::Logical, false))
        }
        "base::addTaskCallback" => Some(TypeState::scalar(PrimTy::Int, false)),
        "base::bindingIsActive" | "base::bindingIsLocked" => {
            Some(TypeState::scalar(PrimTy::Logical, false))
        }
        "base::backsolve" => Some(double_like_first_arg_type(second_arg_type_state(arg_tys))),
        "base::balancePOSIXlt" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::besselI" | "base::besselJ" | "base::besselK" | "base::besselY" | "base::beta"
        | "base::choose" => Some(double_like_first_arg_type(first_arg_type_state(arg_tys))),
        "base::casefold" | "base::char.expand" => {
            Some(char_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::charmatch" => Some(int_like_first_arg_type(first_arg_type_state(arg_tys))),
        "base::charToRaw" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::chkDots" => Some(TypeState::null()),
        "base::chol" | "base::chol.default" | "base::chol2inv" => {
            Some(TypeState::matrix(PrimTy::Double, false))
        }
        "base::chooseOpsMethod" | "base::chooseOpsMethod.default" => {
            Some(TypeState::scalar(PrimTy::Logical, false))
        }
        "base::complete.cases" => Some(logical_like_first_arg_type(first_arg_type_state(arg_tys))),
        "base::cut.Date" | "base::cut.POSIXt" | "base::cut.default" => {
            Some(int_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::complex" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::cummax" | "base::cummin" | "base::cumsum" => {
            Some(vectorized_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::cumprod" => Some(double_like_first_arg_type(first_arg_type_state(arg_tys))),
        "base::diff" | "base::diff.default" => Some(preserved_first_arg_type_without_len(
            first_arg_type_state(arg_tys),
        )),
        "base::diff.Date" | "base::diff.POSIXt" | "base::diff.difftime" => {
            Some(double_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::commandArgs" | "base::data.class" | "base::deparse" | "base::extSoftVersion" => {
            Some(TypeState::vector(PrimTy::Char, false))
        }
        "base::data.matrix" => Some(TypeState::matrix(PrimTy::Double, false)),
        "base::det" => Some(TypeState::scalar(PrimTy::Double, false)),
        "base::determinant" | "base::determinant.matrix" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "base::debuggingState" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "base::dget" => Some(TypeState::unknown()),
        "base::debug"
        | "base::debugonce"
        | "base::declare"
        | "base::delayedAssign"
        | "base::detach"
        | "base::enquote"
        | "base::env.profile"
        | "base::environment<-"
        | "base::errorCondition"
        | "base::eval.parent"
        | "base::Exec"
        | "base::expression" => Some(TypeState::unknown()),
        "base::date" | "base::deparse1" | "base::file.choose" => {
            Some(TypeState::scalar(PrimTy::Char, false))
        }
        "base::dQuote" | "base::enc2native" | "base::enc2utf8" | "base::encodeString"
        | "base::Encoding" => Some(char_like_first_arg_type(first_arg_type_state(arg_tys))),
        "base::dontCheck" => Some(preserved_first_arg_type_without_len(first_arg_type_state(
            arg_tys,
        ))),
        "base::digamma" | "base::expm1" | "base::factorial" | "base::acosh" | "base::asinh"
        | "base::atanh" | "base::cospi" => {
            Some(double_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::eigen" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::exists" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "base::findInterval" => Some(int_like_first_arg_type(first_arg_type_state(arg_tys))),
        "base::file.show" => Some(TypeState::null()),
        "base::format.data.frame" | "base::format.info" => {
            Some(TypeState::matrix(PrimTy::Char, false))
        }
        "base::format"
        | "base::format.AsIs"
        | "base::format.default"
        | "base::format.difftime"
        | "base::format.factor"
        | "base::format.hexmode"
        | "base::format.libraryIQR"
        | "base::format.numeric_version"
        | "base::format.octmode"
        | "base::format.packageInfo"
        | "base::format.pval"
        | "base::format.summaryDefault"
        | "base::formatC"
        | "base::formatDL" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::drop" => Some(TypeState::unknown()),
        "base::droplevels" | "base::droplevels.data.frame" => Some(
            preserved_first_arg_type_without_len(first_arg_type_state(arg_tys)),
        ),
        "base::droplevels.factor" => Some(int_like_first_arg_type(first_arg_type_state(arg_tys))),
        "base::duplicated.default" => infer_builtin("duplicated", arg_tys),
        "base::duplicated.array"
        | "base::duplicated.data.frame"
        | "base::duplicated.matrix"
        | "base::duplicated.numeric_version"
        | "base::duplicated.POSIXlt"
        | "base::duplicated.warnings" => infer_builtin("duplicated", arg_tys),
        "base::attr<-" | "base::attributes<-" | "base::class<-" | "base::colnames<-"
        | "base::comment<-" | "base::dimnames<-" | "base::levels<-" | "base::names<-"
        | "base::row.names<-" | "base::rownames<-" => Some(preserved_first_arg_type_without_len(
            first_arg_type_state(arg_tys),
        )),
        "base::body<-" => Some(TypeState::unknown()),
        "base::bindtextdomain" => Some(TypeState::scalar(PrimTy::Char, false)),
        "base::builtins" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::alist"
        | "base::as.expression"
        | "base::as.expression.default"
        | "base::as.package_version"
        | "base::as.pairlist" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::as.call"
        | "base::as.function"
        | "base::as.function.default"
        | "base::as.name"
        | "base::as.symbol"
        | "base::activeBindingFunction"
        | "base::allowInterrupts"
        | "base::attach"
        | "base::attachNamespace"
        | "base::autoload"
        | "base::autoloader"
        | "base::break"
        | "base::browser"
        | "base::browserSetDebug"
        | "base::as.qr"
        | "base::asS3"
        | "base::asS4" => Some(TypeState::unknown()),
        "base::Arg" => Some(double_like_first_arg_type(first_arg_type_state(arg_tys))),
        "base::aperm.default" | "base::aperm.table" => {
            Some(matrix_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::as.complex" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::as.hexmode" | "base::as.octmode" | "base::as.ordered" | "base::gl" => {
            Some(int_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::as.numeric_version" | "base::asplit" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::as.null" | "base::as.null.default" => Some(TypeState::null()),
        "base::as.raw" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::as.single" | "base::as.single.default" => {
            Some(double_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::as.table" | "base::as.table.default" => {
            Some(matrix_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::bitwAnd" | "base::bitwNot" | "base::bitwOr" | "base::bitwShiftL"
        | "base::bitwShiftR" | "base::bitwXor" => {
            Some(int_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::by.data.frame"
        | "base::by.default"
        | "base::computeRestarts"
        | "base::c.numeric_version"
        | "base::c.POSIXlt"
        | "base::c.warnings" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::c.Date" | "base::c.difftime" | "base::c.POSIXct" => {
            Some(TypeState::vector(PrimTy::Double, false))
        }
        "base::c.factor" => Some(TypeState::vector(PrimTy::Int, false)),
        "base::c.noquote" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::callCC"
        | "base::comment"
        | "base::conditionCall"
        | "base::conditionCall.condition"
        | "base::conflictRules" => Some(TypeState::unknown()),
        "base::cbind.data.frame" => Some(TypeState::matrix(PrimTy::Any, false)),
        "base::conditionMessage"
        | "base::conditionMessage.condition"
        | "base::conflicts"
        | "base::curlGetHeaders" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::contributors" => Some(TypeState::null()),
        "base::Cstack_info" => Some(TypeState::vector(PrimTy::Int, false)),
        "base::browserText" => Some(TypeState::scalar(PrimTy::Char, false)),
        "base::capabilities" => Some(TypeState::vector(PrimTy::Logical, false)),
        "base::Conj" => Some(preserved_first_arg_type_without_len(first_arg_type_state(
            arg_tys,
        ))),
        "base::abbreviate"
        | "base::as.character"
        | "base::as.character.condition"
        | "base::as.character.default"
        | "base::as.character.error"
        | "base::as.character.factor"
        | "base::as.character.hexmode"
        | "base::as.character.numeric_version"
        | "base::as.character.octmode"
        | "base::as.character.srcref" => {
            Some(char_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::all.equal"
        | "base::all.equal.default"
        | "base::all.equal.character"
        | "base::all.equal.environment"
        | "base::all.equal.envRefClass"
        | "base::all.equal.factor"
        | "base::all.equal.formula"
        | "base::all.equal.function"
        | "base::all.equal.language"
        | "base::all.equal.list"
        | "base::all.equal.numeric"
        | "base::all.equal.POSIXt"
        | "base::all.equal.raw"
        | "base::args"
        | "base::body"
        | "base::call"
        | "base::bquote"
        | "base::browserCondition" => Some(TypeState::unknown()),
        "base::array"
        | "base::as.array"
        | "base::as.array.default"
        | "base::as.matrix"
        | "base::as.matrix.data.frame"
        | "base::as.matrix.default"
        | "base::as.matrix.noquote"
        | "base::as.matrix.POSIXlt"
        | "base::aperm" => Some(matrix_like_first_arg_type(first_arg_type_state(arg_tys))),
        "base::as.data.frame"
        | "base::as.data.frame.array"
        | "base::as.data.frame.AsIs"
        | "base::as.data.frame.character"
        | "base::as.data.frame.complex"
        | "base::as.data.frame.data.frame"
        | "base::as.data.frame.Date"
        | "base::as.data.frame.default"
        | "base::as.data.frame.difftime"
        | "base::as.data.frame.factor"
        | "base::as.data.frame.integer"
        | "base::as.data.frame.list"
        | "base::as.data.frame.logical"
        | "base::as.data.frame.matrix"
        | "base::as.data.frame.model.matrix"
        | "base::as.data.frame.noquote"
        | "base::as.data.frame.numeric"
        | "base::as.data.frame.numeric_version"
        | "base::as.data.frame.ordered"
        | "base::as.data.frame.POSIXct"
        | "base::as.data.frame.POSIXlt"
        | "base::as.data.frame.raw"
        | "base::as.data.frame.table"
        | "base::as.data.frame.ts"
        | "base::as.data.frame.vector"
        | "base::array2DF" => Some(TypeState::matrix(PrimTy::Any, false)),
        "base::as.list"
        | "base::as.list.data.frame"
        | "base::as.list.Date"
        | "base::as.list.default"
        | "base::as.list.difftime"
        | "base::as.list.factor"
        | "base::as.list.function"
        | "base::as.list.numeric_version"
        | "base::as.list.POSIXct"
        | "base::as.list.POSIXlt" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::arrayInd" | "base::col" | "base::row" => Some(TypeState::matrix(PrimTy::Int, false)),
        "base::colMeans" | "base::rowMeans" => Some(TypeState::vector(PrimTy::Double, false)),
        "base::append" | "base::addNA" => Some(preserved_first_arg_type_without_len(
            first_arg_type_state(arg_tys),
        )),
        "base::as.double" | "base::as.numeric" => {
            Some(double_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::as.factor" | "base::as.integer" | "base::ordered" => {
            Some(int_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::as.logical" | "base::as.logical.factor" => {
            Some(logical_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::as.vector"
        | "base::as.vector.data.frame"
        | "base::as.vector.factor"
        | "base::as.vector.POSIXlt" => {
            Some(vectorized_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::class" | "base::levels" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::attr" => Some(TypeState::unknown()),
        "base::attributes" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::readBin" | "base::serialize" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::readChar" | "base::load" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::unserialize" | "base::fifo" | "base::gzcon" => Some(TypeState::unknown()),
        "base::getwd" | "base::tempdir" | "base::tempfile" | "base::system.file" => {
            Some(TypeState::scalar(PrimTy::Char, false))
        }
        "base::dir" | "base::list.dirs" | "base::path.package" | "base::.packages" => {
            Some(TypeState::vector(PrimTy::Char, false))
        }
        "base::dir.create" | "base::file.create" | "base::file.remove" | "base::file.rename"
        | "base::file.copy" | "base::file.append" | "base::file.link" | "base::file.symlink" => {
            Some(logical_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::file.access" | "base::file.mode" => Some(TypeState::vector(PrimTy::Int, false)),
        "base::file.info" => Some(TypeState::matrix(PrimTy::Any, false)),
        "base::file.size" | "base::file.mtime" => Some(TypeState::vector(PrimTy::Double, false)),
        _ => None,
    }
}
