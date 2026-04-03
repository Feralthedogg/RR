use crate::typeck::builtin_sigs::*;
use crate::typeck::term::TypeTerm;

pub(crate) fn infer_base_extra_package_call_term_env_io(
    callee: &str,
    arg_terms: &[TypeTerm],
) -> Option<TypeTerm> {
    match callee {
        "base::data.frame" => Some(TypeTerm::DataFrame(Vec::new())),
        "base::globalenv" | "base::environment" => Some(TypeTerm::Any),
        "base::unlink" => Some(TypeTerm::Int),
        "base::file.path" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::basename" | "base::dirname" | "base::normalizePath" => {
            Some(char_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::dir.exists" | "base::file.exists" => {
            Some(logical_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::eval" | "base::evalq" | "base::do.call" | "base::parse" | "base::readRDS"
        | "base::get0" | "base::getOption" | "base::file" => Some(TypeTerm::Any),
        "base::save" => Some(TypeTerm::Null),
        "base::list.files" | "base::path.expand" => {
            Some(char_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::getNamespace" | "base::asNamespace" => Some(TypeTerm::Any),
        "base::isNamespace" | "base::is.name" => Some(TypeTerm::Logical),
        "base::find.package" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::package_version" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "base::getElement" | "base::unname" => Some(first_arg_term(arg_terms)),
        "base::baseenv"
        | "base::emptyenv"
        | "base::new.env"
        | "base::parent.env"
        | "base::as.environment"
        | "base::list2env"
        | "base::topenv" => Some(TypeTerm::Any),
        "base::is.environment"
        | "base::environmentIsLocked"
        | "base::isNamespaceLoaded"
        | "base::requireNamespace" => Some(TypeTerm::Logical),
        "base::environmentName" | "base::getNamespaceName" | "base::getNamespaceVersion" => {
            Some(TypeTerm::Char)
        }
        "base::loadedNamespaces" | "base::getNamespaceExports" | "base::getNamespaceUsers" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Char)))
        }
        "base::as.list.environment" | "base::getNamespaceImports" => {
            Some(TypeTerm::List(Box::new(TypeTerm::Any)))
        }
        "base::library" | "base::searchpaths" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::require" | "base::packageHasNamespace" | "base::is.loaded" => {
            Some(TypeTerm::Logical)
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
        | "base::is.vector" => Some(TypeTerm::Logical),
        "base::is.element"
        | "base::is.finite.POSIXlt"
        | "base::is.infinite"
        | "base::is.infinite.POSIXlt"
        | "base::is.na.POSIXlt"
        | "base::is.na.data.frame"
        | "base::is.na.numeric_version"
        | "base::is.nan"
        | "base::is.nan.POSIXlt" => Some(logical_like_first_arg_term(first_arg_term(arg_terms))),
        "base::loadNamespace" | "base::getLoadedDLLs" | "base::dyn.load" => Some(TypeTerm::Any),
        "base::dyn.unload" => Some(TypeTerm::Null),
        "base::readLines" | "base::Sys.getenv" | "base::Sys.which" | "base::Sys.readlink"
        | "base::Sys.info" | "base::Sys.glob" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::writeLines"
        | "base::writeChar"
        | "base::writeBin"
        | "base::flush"
        | "base::truncate.connection" => Some(TypeTerm::Null),
        "base::seek" => Some(TypeTerm::Double),
        "base::Sys.setenv" | "base::Sys.unsetenv" => Some(TypeTerm::Logical),
        "base::Sys.getpid" => Some(TypeTerm::Int),
        "base::Sys.time" | "base::Sys.Date" => Some(TypeTerm::Double),
        "base::Sys.getlocale" => Some(TypeTerm::Char),
        "base::system" | "base::system2" => Some(TypeTerm::Any),
        "base::system.time" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "base::Sys.sleep" => Some(TypeTerm::Null),
        "base::Sys.setlocale" | "base::Sys.timezone" => Some(TypeTerm::Char),
        "base::Sys.localeconv" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::Sys.setFileTime" | "base::Sys.chmod" => {
            Some(logical_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::Sys.umask" => Some(TypeTerm::Int),
        "base::sys.parent" | "base::sys.nframe" => Some(TypeTerm::Int),
        "base::sys.parents" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "base::search" | "base::gettext" | "base::gettextf" | "base::ngettext" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Char)))
        }
        "base::geterrmessage" => Some(TypeTerm::Char),
        "base::message" | "base::packageStartupMessage" | "base::.packageStartupMessage" => {
            Some(TypeTerm::Null)
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
        | "base::packageEvent" => Some(TypeTerm::Any),
        "base::stdin"
        | "base::stdout"
        | "base::stderr"
        | "base::textConnection"
        | "base::rawConnection"
        | "base::socketConnection"
        | "base::url"
        | "base::pipe"
        | "base::open"
        | "base::summary.connection" => Some(TypeTerm::Any),
        "base::textConnectionValue" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::rawConnectionValue" => Some(TypeTerm::Vector(Box::new(TypeTerm::Any))),
        "base::close" | "base::closeAllConnections" | "base::pushBack" | "base::clearPushBack" => {
            Some(TypeTerm::Null)
        }
        "base::close.connection" | "base::close.srcfile" | "base::close.srcfilealias" => {
            Some(TypeTerm::Null)
        }
        "base::isOpen" | "base::isIncomplete" => Some(TypeTerm::Logical),
        "base::pushBackLength" => Some(TypeTerm::Int),
        "base::socketSelect" => Some(TypeTerm::Vector(Box::new(TypeTerm::Logical))),
        "base::scan" => Some(TypeTerm::Vector(Box::new(TypeTerm::Any))),
        "base::read.table" | "base::read.csv" | "base::read.csv2" | "base::read.delim"
        | "base::read.delim2" => Some(TypeTerm::DataFrame(Vec::new())),
        "base::write.table" | "base::write.csv" | "base::write.csv2" | "base::saveRDS"
        | "base::dput" | "base::dump" | "base::sink" => Some(TypeTerm::Null),
        "base::count.fields" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "base::sink.number" => Some(TypeTerm::Int),
        "base::capture.output" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        _ => None,
    }
}
