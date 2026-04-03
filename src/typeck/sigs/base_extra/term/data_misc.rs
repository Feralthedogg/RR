use crate::typeck::builtin_sigs::*;
use crate::typeck::term::TypeTerm;

pub(crate) fn infer_base_extra_package_call_term_data_misc(
    callee: &str,
    arg_terms: &[TypeTerm],
) -> Option<TypeTerm> {
    match callee {
        "base::lapply" | "base::Map" | "base::split" | "base::by" => {
            Some(TypeTerm::List(Box::new(TypeTerm::Any)))
        }
        "base::sapply" | "base::vapply" | "base::mapply" | "base::tapply" | "base::apply" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Any)))
        }
        "base::Reduce" | "base::Find" => Some(TypeTerm::Any),
        "base::Filter" | "base::unsplit" | "base::within" | "base::transform" => {
            Some(first_arg_term(arg_terms))
        }
        "base::Position" => Some(TypeTerm::Int),
        "base::expand.grid" | "base::merge" => Some(TypeTerm::DataFrame(Vec::new())),
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
        | "base::julian" => Some(double_like_first_arg_term(first_arg_term(arg_terms))),
        "base::as.character.Date"
        | "base::as.character.POSIXt"
        | "base::format.Date"
        | "base::format.POSIXct"
        | "base::format.POSIXlt"
        | "base::months"
        | "base::quarters"
        | "base::weekdays" => Some(char_like_first_arg_term(first_arg_term(arg_terms))),
        "base::OlsonNames" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::ISOdate" | "base::ISOdatetime" | "base::seq.Date" | "base::seq.POSIXt" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Double)))
        }
        "base::all.names" | "base::all.vars" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::anyDuplicated.array"
        | "base::anyDuplicated.data.frame"
        | "base::anyDuplicated.default"
        | "base::anyDuplicated.matrix" => Some(TypeTerm::Int),
        "base::anyNA" | "base::anyNA.data.frame" => Some(TypeTerm::Logical),
        "base::anyNA.numeric_version" | "base::anyNA.POSIXlt" => Some(TypeTerm::Logical),
        "base::addTaskCallback" => Some(TypeTerm::Int),
        "base::bindingIsActive" | "base::bindingIsLocked" => Some(TypeTerm::Logical),
        "base::backsolve" => Some(double_like_first_arg_term(second_arg_term(arg_terms))),
        "base::balancePOSIXlt" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "base::besselI" | "base::besselJ" | "base::besselK" | "base::besselY" | "base::beta"
        | "base::choose" => Some(double_like_first_arg_term(first_arg_term(arg_terms))),
        "base::casefold" | "base::char.expand" => {
            Some(char_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::charmatch" => Some(int_like_first_arg_term(first_arg_term(arg_terms))),
        "base::charToRaw" => Some(TypeTerm::Vector(Box::new(TypeTerm::Any))),
        "base::chkDots" => Some(TypeTerm::Null),
        "base::chol" | "base::chol.default" | "base::chol2inv" => {
            Some(TypeTerm::Matrix(Box::new(TypeTerm::Double)))
        }
        "base::chooseOpsMethod" | "base::chooseOpsMethod.default" => Some(TypeTerm::Logical),
        "base::complete.cases" => Some(logical_like_first_arg_term(first_arg_term(arg_terms))),
        "base::complex" => Some(TypeTerm::Vector(Box::new(TypeTerm::Any))),
        "base::cut.Date" | "base::cut.POSIXt" | "base::cut.default" => {
            Some(int_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::cummax" | "base::cummin" | "base::cumsum" => {
            Some(vectorized_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::cumprod" => Some(double_like_first_arg_term(first_arg_term(arg_terms))),
        "base::diff" | "base::diff.default" => {
            Some(preserved_head_tail_term(first_arg_term(arg_terms)))
        }
        "base::diff.Date" | "base::diff.POSIXt" | "base::diff.difftime" => {
            Some(double_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::commandArgs" | "base::data.class" | "base::deparse" | "base::extSoftVersion" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Char)))
        }
        "base::data.matrix" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "base::det" => Some(TypeTerm::Double),
        "base::determinant" | "base::determinant.matrix" => Some(TypeTerm::NamedList(vec![
            ("modulus".to_string(), TypeTerm::Double),
            ("sign".to_string(), TypeTerm::Int),
        ])),
        "base::debuggingState" => Some(TypeTerm::Logical),
        "base::dget" => Some(TypeTerm::Any),
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
        | "base::expression" => Some(TypeTerm::Any),
        "base::date" | "base::deparse1" | "base::file.choose" => Some(TypeTerm::Char),
        "base::dQuote" | "base::enc2native" | "base::enc2utf8" | "base::encodeString"
        | "base::Encoding" => Some(char_like_first_arg_term(first_arg_term(arg_terms))),
        "base::dontCheck" => Some(first_arg_term(arg_terms)),
        "base::digamma" | "base::expm1" | "base::factorial" | "base::acosh" | "base::asinh"
        | "base::atanh" | "base::cospi" => {
            Some(double_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::eigen" => Some(TypeTerm::NamedList(vec![
            (
                "values".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "vectors".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
        ])),
        "base::exists" => Some(TypeTerm::Logical),
        "base::findInterval" => Some(int_like_first_arg_term(first_arg_term(arg_terms))),
        "base::file.show" => Some(TypeTerm::Null),
        "base::format.data.frame" | "base::format.info" => Some(TypeTerm::DataFrame(Vec::new())),
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
        | "base::formatDL" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::drop" => Some(TypeTerm::Any),
        "base::droplevels" | "base::droplevels.data.frame" => Some(first_arg_term(arg_terms)),
        "base::droplevels.factor" => Some(int_like_first_arg_term(first_arg_term(arg_terms))),
        "base::duplicated.default" => infer_builtin_term("duplicated", arg_terms),
        "base::duplicated.array"
        | "base::duplicated.data.frame"
        | "base::duplicated.matrix"
        | "base::duplicated.numeric_version"
        | "base::duplicated.POSIXlt"
        | "base::duplicated.warnings" => infer_builtin_term("duplicated", arg_terms),
        "base::attr<-" | "base::attributes<-" | "base::class<-" | "base::colnames<-"
        | "base::comment<-" | "base::dimnames<-" | "base::levels<-" | "base::names<-"
        | "base::row.names<-" | "base::rownames<-" => Some(first_arg_term(arg_terms)),
        "base::body<-" => Some(TypeTerm::Any),
        "base::bindtextdomain" => Some(TypeTerm::Char),
        "base::builtins" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::alist"
        | "base::as.expression"
        | "base::as.expression.default"
        | "base::as.package_version"
        | "base::as.pairlist" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
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
        | "base::asS4" => Some(TypeTerm::Any),
        "base::Arg" => Some(double_like_first_arg_term(first_arg_term(arg_terms))),
        "base::aperm.default" | "base::aperm.table" => {
            Some(matrix_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::as.complex" => Some(TypeTerm::Vector(Box::new(TypeTerm::Any))),
        "base::as.hexmode" | "base::as.octmode" | "base::as.ordered" | "base::gl" => {
            Some(int_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::as.numeric_version" | "base::asplit" => {
            Some(TypeTerm::List(Box::new(TypeTerm::Any)))
        }
        "base::as.null" | "base::as.null.default" => Some(TypeTerm::Null),
        "base::as.raw" => Some(TypeTerm::Vector(Box::new(TypeTerm::Any))),
        "base::as.single" | "base::as.single.default" => {
            Some(double_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::as.table" | "base::as.table.default" => {
            Some(matrix_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::bitwAnd" | "base::bitwNot" | "base::bitwOr" | "base::bitwShiftL"
        | "base::bitwShiftR" | "base::bitwXor" => {
            Some(int_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::by.data.frame"
        | "base::by.default"
        | "base::computeRestarts"
        | "base::c.numeric_version"
        | "base::c.POSIXlt"
        | "base::c.warnings" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "base::c.Date" | "base::c.difftime" | "base::c.POSIXct" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Double)))
        }
        "base::c.factor" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "base::c.noquote" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::callCC"
        | "base::comment"
        | "base::conditionCall"
        | "base::conditionCall.condition"
        | "base::conflictRules" => Some(TypeTerm::Any),
        "base::cbind.data.frame" => Some(TypeTerm::DataFrame(Vec::new())),
        "base::conditionMessage"
        | "base::conditionMessage.condition"
        | "base::conflicts"
        | "base::curlGetHeaders" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::contributors" => Some(TypeTerm::Null),
        "base::Cstack_info" => Some(TypeTerm::NamedList(vec![
            ("size".to_string(), TypeTerm::Int),
            ("current".to_string(), TypeTerm::Int),
            ("direction".to_string(), TypeTerm::Int),
            ("eval_depth".to_string(), TypeTerm::Int),
        ])),
        "base::browserText" => Some(TypeTerm::Char),
        "base::capabilities" => Some(TypeTerm::Vector(Box::new(TypeTerm::Logical))),
        "base::Conj" => Some(first_arg_term(arg_terms)),
        "base::abbreviate"
        | "base::as.character"
        | "base::as.character.condition"
        | "base::as.character.default"
        | "base::as.character.error"
        | "base::as.character.factor"
        | "base::as.character.hexmode"
        | "base::as.character.numeric_version"
        | "base::as.character.octmode"
        | "base::as.character.srcref" => Some(char_like_first_arg_term(first_arg_term(arg_terms))),
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
        | "base::browserCondition" => Some(TypeTerm::Any),
        "base::array"
        | "base::as.array"
        | "base::as.array.default"
        | "base::as.matrix"
        | "base::as.matrix.data.frame"
        | "base::as.matrix.default"
        | "base::as.matrix.noquote"
        | "base::as.matrix.POSIXlt"
        | "base::aperm" => Some(matrix_like_first_arg_term(first_arg_term(arg_terms))),
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
        | "base::array2DF" => Some(TypeTerm::DataFrame(Vec::new())),
        "base::as.list"
        | "base::as.list.data.frame"
        | "base::as.list.Date"
        | "base::as.list.default"
        | "base::as.list.difftime"
        | "base::as.list.factor"
        | "base::as.list.function"
        | "base::as.list.numeric_version"
        | "base::as.list.POSIXct"
        | "base::as.list.POSIXlt" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "base::arrayInd" | "base::col" | "base::row" => {
            Some(TypeTerm::Matrix(Box::new(TypeTerm::Int)))
        }
        "base::colMeans" | "base::rowMeans" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "base::append" | "base::addNA" => Some(first_arg_term(arg_terms)),
        "base::as.double" | "base::as.numeric" => {
            Some(double_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::as.factor" | "base::as.integer" | "base::ordered" => {
            Some(int_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::as.logical" | "base::as.logical.factor" => {
            Some(logical_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::as.vector"
        | "base::as.vector.data.frame"
        | "base::as.vector.factor"
        | "base::as.vector.POSIXlt" => Some(vectorized_first_arg_term(first_arg_term(arg_terms))),
        "base::class" | "base::levels" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::attr" => Some(TypeTerm::Any),
        "base::attributes" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "base::readBin" | "base::serialize" => Some(TypeTerm::Vector(Box::new(TypeTerm::Any))),
        "base::readChar" | "base::load" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::unserialize" | "base::fifo" | "base::gzcon" => Some(TypeTerm::Any),
        "base::getwd" | "base::tempdir" | "base::tempfile" | "base::system.file" => {
            Some(TypeTerm::Char)
        }
        "base::dir" | "base::list.dirs" | "base::path.package" | "base::.packages" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Char)))
        }
        "base::dir.create" | "base::file.create" | "base::file.remove" | "base::file.rename"
        | "base::file.copy" | "base::file.append" | "base::file.link" | "base::file.symlink" => {
            Some(logical_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::file.access" | "base::file.mode" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "base::file.info" => Some(TypeTerm::DataFrame(Vec::new())),
        "base::file.size" | "base::file.mtime" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Double)))
        }
        _ => None,
    }
}
