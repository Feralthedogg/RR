use crate::typeck::builtin_sigs::*;
use crate::typeck::lattice::{PrimTy, TypeState};

pub(crate) fn infer_base_extra_package_call_env_io(
    callee: &str,
    arg_tys: &[TypeState],
) -> Option<TypeState> {
    match callee {
        "base::data.frame" => {
            Some(TypeState::matrix(PrimTy::Any, false).with_len(shared_vector_len_sym(arg_tys)))
        }
        "base::globalenv" | "base::environment" => Some(TypeState::unknown()),
        "base::unlink" => Some(TypeState::scalar(PrimTy::Int, false)),
        "base::file.path" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::basename" | "base::dirname" | "base::normalizePath" => {
            Some(char_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::dir.exists" | "base::file.exists" => {
            Some(logical_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::eval" | "base::evalq" | "base::do.call" | "base::parse" | "base::readRDS"
        | "base::get0" | "base::getOption" | "base::file" => Some(TypeState::unknown()),
        "base::save" => Some(TypeState::null()),
        "base::list.files" | "base::path.expand" => {
            Some(char_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::getNamespace" | "base::asNamespace" => Some(TypeState::unknown()),
        "base::isNamespace" | "base::is.name" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "base::find.package" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::package_version" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::getElement" | "base::unname" => Some(preserved_first_arg_type_without_len(
            first_arg_type_state(arg_tys),
        )),
        "base::baseenv"
        | "base::emptyenv"
        | "base::new.env"
        | "base::parent.env"
        | "base::as.environment"
        | "base::list2env"
        | "base::topenv" => Some(TypeState::unknown()),
        "base::is.environment"
        | "base::environmentIsLocked"
        | "base::isNamespaceLoaded"
        | "base::requireNamespace" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "base::environmentName" | "base::getNamespaceName" | "base::getNamespaceVersion" => {
            Some(TypeState::scalar(PrimTy::Char, false))
        }
        "base::loadedNamespaces" | "base::getNamespaceExports" | "base::getNamespaceUsers" => {
            Some(TypeState::vector(PrimTy::Char, false))
        }
        "base::as.list.environment" | "base::getNamespaceImports" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "base::library" | "base::searchpaths" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::require" | "base::packageHasNamespace" | "base::is.loaded" => {
            Some(TypeState::scalar(PrimTy::Logical, false))
        }
        "base::identical"
        | "base::inherits"
        | "base::interactive"
        | "base::is.R"
        | "base::is.array"
        | "base::is.atomic"
        | "base::is.call"
        | "base::is.character"
        | "base::is.complex"
        | "base::is.data.frame"
        | "base::is.double"
        | "base::is.expression"
        | "base::is.factor"
        | "base::is.function"
        | "base::is.integer"
        | "base::is.language"
        | "base::is.list"
        | "base::is.logical"
        | "base::is.null"
        | "base::is.numeric"
        | "base::is.numeric.Date"
        | "base::is.numeric.POSIXt"
        | "base::is.numeric.difftime"
        | "base::is.numeric_version"
        | "base::is.object"
        | "base::is.ordered"
        | "base::is.package_version"
        | "base::is.pairlist"
        | "base::is.primitive"
        | "base::is.qr"
        | "base::is.raw"
        | "base::is.recursive"
        | "base::is.single"
        | "base::is.symbol"
        | "base::is.table"
        | "base::is.unsorted"
        | "base::is.vector" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "base::is.element"
        | "base::is.finite.POSIXlt"
        | "base::is.infinite"
        | "base::is.infinite.POSIXlt"
        | "base::is.na.POSIXlt"
        | "base::is.na.data.frame"
        | "base::is.na.numeric_version"
        | "base::is.nan"
        | "base::is.nan.POSIXlt" => {
            Some(logical_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::loadNamespace" | "base::getLoadedDLLs" | "base::dyn.load" => {
            Some(TypeState::unknown())
        }
        "base::dyn.unload" => Some(TypeState::null()),
        "base::readLines" | "base::Sys.getenv" | "base::Sys.which" | "base::Sys.readlink"
        | "base::Sys.info" | "base::Sys.glob" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::writeLines"
        | "base::writeChar"
        | "base::writeBin"
        | "base::flush"
        | "base::truncate.connection" => Some(TypeState::null()),
        "base::seek" => Some(TypeState::scalar(PrimTy::Double, false)),
        "base::Sys.setenv" | "base::Sys.unsetenv" => {
            Some(TypeState::scalar(PrimTy::Logical, false))
        }
        "base::Sys.getpid" => Some(TypeState::scalar(PrimTy::Int, false)),
        "base::Sys.time" | "base::Sys.Date" => Some(TypeState::scalar(PrimTy::Double, false)),
        "base::Sys.getlocale" => Some(TypeState::scalar(PrimTy::Char, false)),
        "base::system" | "base::system2" => Some(TypeState::unknown()),
        "base::system.time" => Some(TypeState::vector(PrimTy::Double, false)),
        "base::Sys.sleep" => Some(TypeState::null()),
        "base::Sys.setlocale" | "base::Sys.timezone" => {
            Some(TypeState::scalar(PrimTy::Char, false))
        }
        "base::Sys.localeconv" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::Sys.setFileTime" | "base::Sys.chmod" => {
            Some(logical_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::Sys.umask" => Some(TypeState::scalar(PrimTy::Int, false)),
        "base::sys.parent" | "base::sys.nframe" => Some(TypeState::scalar(PrimTy::Int, false)),
        "base::sys.parents" => Some(TypeState::vector(PrimTy::Int, false)),
        "base::search" | "base::gettext" | "base::gettextf" | "base::ngettext" => {
            Some(TypeState::vector(PrimTy::Char, false))
        }
        "base::geterrmessage" => Some(TypeState::scalar(PrimTy::Char, false)),
        "base::message" | "base::packageStartupMessage" | "base::.packageStartupMessage" => {
            Some(TypeState::null())
        }
        "base::sys.call"
        | "base::sys.calls"
        | "base::sys.function"
        | "base::sys.frame"
        | "base::sys.frames"
        | "base::sys.status"
        | "base::sys.source"
        | "base::source"
        | "base::options"
        | "base::warning"
        | "base::warningCondition"
        | "base::packageNotFoundError"
        | "base::packageEvent" => Some(TypeState::unknown()),
        "base::stdin"
        | "base::stdout"
        | "base::stderr"
        | "base::textConnection"
        | "base::rawConnection"
        | "base::socketConnection"
        | "base::url"
        | "base::pipe"
        | "base::open"
        | "base::summary.connection" => Some(TypeState::unknown()),
        "base::textConnectionValue" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::rawConnectionValue" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::close" | "base::closeAllConnections" | "base::pushBack" | "base::clearPushBack" => {
            Some(TypeState::null())
        }
        "base::close.connection" | "base::close.srcfile" | "base::close.srcfilealias" => {
            Some(TypeState::null())
        }
        "base::isOpen" | "base::isIncomplete" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "base::pushBackLength" => Some(TypeState::scalar(PrimTy::Int, false)),
        "base::socketSelect" => Some(TypeState::vector(PrimTy::Logical, false)),
        "base::scan" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::read.table" | "base::read.csv" | "base::read.csv2" | "base::read.delim"
        | "base::read.delim2" => Some(TypeState::matrix(PrimTy::Any, false)),
        "base::write.table" | "base::write.csv" | "base::write.csv2" | "base::saveRDS"
        | "base::dput" | "base::dump" | "base::sink" => Some(TypeState::null()),
        "base::count.fields" => Some(TypeState::vector(PrimTy::Int, false)),
        "base::sink.number" => Some(TypeState::scalar(PrimTy::Int, false)),
        "base::capture.output" => Some(TypeState::vector(PrimTy::Char, false)),
        _ => None,
    }
}
