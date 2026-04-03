use crate::typeck::builtin_sigs::{
    char_like_first_arg_term, first_arg_term, preserved_head_tail_term, vectorized_first_arg_term,
};
use crate::typeck::term::TypeTerm;

pub(crate) fn infer_utils_package_call_term(
    callee: &str,
    arg_terms: &[TypeTerm],
) -> Option<TypeTerm> {
    match callee {
        "utils::head" | "utils::tail" => Some(preserved_head_tail_term(first_arg_term(arg_terms))),
        "utils::packageVersion"
        | "utils::citation"
        | "utils::person"
        | "utils::as.person"
        | "utils::as.personList" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "utils::getAnywhere" => Some(TypeTerm::NamedList(vec![
            ("name".to_string(), TypeTerm::Char),
            ("objs".to_string(), TypeTerm::List(Box::new(TypeTerm::Any))),
            (
                "where".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "visible".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Logical)),
            ),
            (
                "dups".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Logical)),
            ),
        ])),
        "utils::packageDescription" => Some(TypeTerm::NamedList(vec![
            ("Package".to_string(), TypeTerm::Char),
            ("Version".to_string(), TypeTerm::Char),
            ("Priority".to_string(), TypeTerm::Char),
            ("Title".to_string(), TypeTerm::Char),
            ("Author".to_string(), TypeTerm::Char),
            ("Maintainer".to_string(), TypeTerm::Char),
            ("Contact".to_string(), TypeTerm::Char),
            ("Description".to_string(), TypeTerm::Char),
            ("License".to_string(), TypeTerm::Char),
            ("Imports".to_string(), TypeTerm::Char),
            ("Suggests".to_string(), TypeTerm::Char),
            ("NeedsCompilation".to_string(), TypeTerm::Char),
            ("Encoding".to_string(), TypeTerm::Char),
            ("Enhances".to_string(), TypeTerm::Char),
            ("Built".to_string(), TypeTerm::Char),
        ])),
        "utils::sessionInfo" => Some(TypeTerm::NamedList(vec![
            ("R.version".to_string(), TypeTerm::Any),
            ("platform".to_string(), TypeTerm::Char),
            ("locale".to_string(), TypeTerm::Char),
            ("tzone".to_string(), TypeTerm::Char),
            ("tzcode_type".to_string(), TypeTerm::Char),
            ("running".to_string(), TypeTerm::Char),
            (
                "RNGkind".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "basePkgs".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            ("loadedOnly".to_string(), TypeTerm::Any),
            ("matprod".to_string(), TypeTerm::Char),
            ("BLAS".to_string(), TypeTerm::Char),
            ("LAPACK".to_string(), TypeTerm::Char),
            ("LA_version".to_string(), TypeTerm::Char),
        ])),
        "utils::as.roman" => match first_arg_term(arg_terms) {
            TypeTerm::Vector(_)
            | TypeTerm::VectorLen(_, _)
            | TypeTerm::Matrix(_)
            | TypeTerm::MatrixDim(_, _, _)
            | TypeTerm::ArrayDim(_, _) => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
            _ => Some(TypeTerm::Int),
        },
        "utils::hasName" => Some(TypeTerm::Logical),
        "utils::strcapture" => Some(TypeTerm::DataFrame(Vec::new())),
        "utils::contrib.url" => Some(TypeTerm::Char),
        "utils::localeToCharset" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "utils::charClass" => Some(TypeTerm::Vector(Box::new(TypeTerm::Logical))),
        "utils::findMatches" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "utils::fileSnapshot" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "utils::apropos" | "utils::find" | "utils::methods" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Char)))
        }
        "utils::help.search" => Some(TypeTerm::NamedList(vec![
            ("pattern".to_string(), TypeTerm::Char),
            (
                "fields".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            ("type".to_string(), TypeTerm::Char),
            ("agrep".to_string(), TypeTerm::Any),
            ("ignore.case".to_string(), TypeTerm::Logical),
            (
                "types".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            ("package".to_string(), TypeTerm::Any),
            ("lib.loc".to_string(), TypeTerm::Char),
            (
                "matches".to_string(),
                TypeTerm::DataFrameNamed(vec![
                    (
                        "Topic".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char)),
                    ),
                    (
                        "Title".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char)),
                    ),
                    (
                        "Name".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char)),
                    ),
                    ("ID".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Char))),
                    (
                        "Package".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char)),
                    ),
                    (
                        "LibPath".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char)),
                    ),
                    (
                        "Type".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char)),
                    ),
                    (
                        "Field".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char)),
                    ),
                    (
                        "Entry".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char)),
                    ),
                ]),
            ),
        ])),
        "utils::data" => Some(TypeTerm::NamedList(vec![
            ("title".to_string(), TypeTerm::Char),
            ("header".to_string(), TypeTerm::Any),
            (
                "results".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Char)),
            ),
            ("footer".to_string(), TypeTerm::Char),
        ])),
        "utils::argsAnywhere" => Some(TypeTerm::Any),
        "utils::compareVersion" => Some(TypeTerm::Double),
        "utils::capture.output" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "utils::file_test" => match arg_terms.get(1).cloned().unwrap_or(TypeTerm::Any) {
            TypeTerm::Vector(_)
            | TypeTerm::VectorLen(_, _)
            | TypeTerm::Matrix(_)
            | TypeTerm::MatrixDim(_, _, _)
            | TypeTerm::ArrayDim(_, _) => Some(TypeTerm::Vector(Box::new(TypeTerm::Logical))),
            _ => Some(TypeTerm::Logical),
        },
        "utils::URLencode" | "utils::URLdecode" => {
            Some(char_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "utils::head.matrix" | "utils::tail.matrix" => {
            Some(preserved_head_tail_term(first_arg_term(arg_terms)))
        }
        "utils::available.packages" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Char))),
        "utils::stack" | "utils::unstack" => Some(TypeTerm::DataFrame(Vec::new())),
        "utils::strOptions" => Some(TypeTerm::NamedList(vec![
            ("strict.width".to_string(), TypeTerm::Char),
            ("digits.d".to_string(), TypeTerm::Int),
            ("vec.len".to_string(), TypeTerm::Int),
            ("list.len".to_string(), TypeTerm::Int),
            ("deparse.lines".to_string(), TypeTerm::Any),
            ("drop.deparse.attr".to_string(), TypeTerm::Logical),
            ("formatNum".to_string(), TypeTerm::Any),
        ])),
        "utils::txtProgressBar" => Some(TypeTerm::Any),
        "utils::toBibtex" | "utils::toLatex" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "utils::getTxtProgressBar" | "utils::setTxtProgressBar" => Some(TypeTerm::Double),
        "utils::modifyList"
        | "utils::relist"
        | "utils::as.relistable"
        | "utils::personList"
        | "utils::warnErrList"
        | "utils::readCitationFile"
        | "utils::bibentry"
        | "utils::citEntry"
        | "utils::citHeader"
        | "utils::citFooter" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "utils::getSrcref" | "utils::getFromNamespace" | "utils::getS3method" => {
            Some(TypeTerm::Any)
        }
        "utils::getParseData" => Some(TypeTerm::DataFrame(Vec::new())),
        "utils::getParseText" | "utils::globalVariables" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Char)))
        }
        "utils::getSrcFilename" | "utils::getSrcDirectory" => Some(TypeTerm::Char),
        "utils::getSrcLocation" => Some(TypeTerm::Any),
        "utils::hashtab" | "utils::gethash" => Some(TypeTerm::Any),
        "utils::sethash" => Some(TypeTerm::Any),
        "utils::remhash" | "utils::is.hashtab" => Some(TypeTerm::Logical),
        "utils::clrhash" | "utils::maphash" => Some(TypeTerm::Null),
        "utils::numhash" => Some(TypeTerm::Int),
        "utils::typhash" => Some(TypeTerm::Char),
        "utils::asDateBuilt" => Some(TypeTerm::Double),
        "utils::findLineNum" => Some(TypeTerm::Any),
        "utils::getCRANmirrors" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "Name".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Country".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "City".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "URL".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Host".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Maintainer".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            ("OK".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Int))),
            (
                "CountryCode".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Comment".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])),
        "utils::findCRANmirror" => Some(TypeTerm::Char),
        "utils::package.skeleton" | "utils::unzip" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "utils::zip" => Some(TypeTerm::Int),
        "utils::limitedLabels"
        | "utils::formatOL"
        | "utils::formatUL"
        | "utils::ls.str"
        | "utils::lsf.str" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "utils::news" => Some(TypeTerm::Null),
        "utils::vignette" => Some(TypeTerm::NamedList(vec![
            ("type".to_string(), TypeTerm::Char),
            ("title".to_string(), TypeTerm::Char),
            ("header".to_string(), TypeTerm::Any),
            ("results".to_string(), TypeTerm::DataFrame(Vec::new())),
            ("footer".to_string(), TypeTerm::Any),
        ])),
        "utils::hsearch_db" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "utils::hsearch_db_concepts" | "utils::hsearch_db_keywords" => {
            Some(TypeTerm::DataFrame(Vec::new()))
        }
        "utils::browseEnv"
        | "utils::browseURL"
        | "utils::browseVignettes"
        | "utils::bug.report"
        | "utils::checkCRAN"
        | "utils::chooseBioCmirror"
        | "utils::chooseCRANmirror"
        | "utils::create.post"
        | "utils::data.entry"
        | "utils::dataentry"
        | "utils::debugcall"
        | "utils::debugger"
        | "utils::demo"
        | "utils::dump.frames"
        | "utils::edit"
        | "utils::emacs"
        | "utils::example"
        | "utils::file.edit"
        | "utils::fix"
        | "utils::fixInNamespace"
        | "utils::flush.console"
        | "utils::help.request"
        | "utils::help.start"
        | "utils::page"
        | "utils::pico"
        | "utils::process.events"
        | "utils::prompt"
        | "utils::promptData"
        | "utils::promptImport"
        | "utils::promptPackage"
        | "utils::recover"
        | "utils::removeSource"
        | "utils::RShowDoc"
        | "utils::RSiteSearch"
        | "utils::rtags"
        | "utils::setBreakpoint"
        | "utils::suppressForeignCheck"
        | "utils::undebugcall"
        | "utils::url.show"
        | "utils::vi"
        | "utils::View"
        | "utils::xedit"
        | "utils::xemacs" => Some(TypeTerm::Null),
        "utils::tar" => Some(TypeTerm::Int),
        "utils::untar" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "utils::timestamp" => Some(TypeTerm::Char),
        "utils::Rprof" | "utils::Rprofmem" => Some(TypeTerm::Null),
        "utils::summaryRprof" => Some(TypeTerm::NamedList(vec![
            ("by.self".to_string(), TypeTerm::Any),
            ("by.total".to_string(), TypeTerm::Any),
            ("sample.interval".to_string(), TypeTerm::Double),
            ("sampling.time".to_string(), TypeTerm::Double),
        ])),
        "utils::setRepositories" => Some(TypeTerm::NamedList(vec![(
            "repos".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Char)),
        )])),
        "utils::?"
        | "utils::.AtNames"
        | "utils::.DollarNames"
        | "utils::cite"
        | "utils::citeNatbib"
        | "utils::help"
        | "utils::read.socket"
        | "utils::RweaveChunkPrefix" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "utils::.romans" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "utils::askYesNo" | "utils::isS3method" | "utils::isS3stdGeneric" => {
            Some(TypeTerm::Logical)
        }
        "utils::rc.settings" => Some(TypeTerm::Vector(Box::new(TypeTerm::Logical))),
        "utils::download.file" => Some(TypeTerm::Int),
        "utils::alarm" | "utils::rc.getOption" => Some(TypeTerm::Any),
        "utils::close.socket"
        | "utils::history"
        | "utils::loadhistory"
        | "utils::savehistory"
        | "utils::write.socket" => Some(TypeTerm::Null),
        "utils::.checkHT"
        | "utils::.RtangleCodeLabel"
        | "utils::.S3methods"
        | "utils::aregexec"
        | "utils::aspell"
        | "utils::aspell_package_C_files"
        | "utils::aspell_package_R_files"
        | "utils::aspell_package_Rd_files"
        | "utils::aspell_package_vignettes"
        | "utils::aspell_write_personal_dictionary_file"
        | "utils::assignInMyNamespace"
        | "utils::assignInNamespace"
        | "utils::changedFiles"
        | "utils::de"
        | "utils::de.ncols"
        | "utils::de.restore"
        | "utils::de.setup"
        | "utils::download.packages"
        | "utils::install.packages"
        | "utils::make.packages.html"
        | "utils::make.socket"
        | "utils::makeRweaveLatexCodeRunner"
        | "utils::mirror2html"
        | "utils::new.packages"
        | "utils::old.packages"
        | "utils::packageStatus"
        | "utils::rc.options"
        | "utils::rc.status"
        | "utils::remove.packages"
        | "utils::Rtangle"
        | "utils::RtangleFinish"
        | "utils::RtangleRuncode"
        | "utils::RtangleSetup"
        | "utils::RtangleWritedoc"
        | "utils::RweaveEvalWithOpt"
        | "utils::RweaveLatex"
        | "utils::RweaveLatexFinish"
        | "utils::RweaveLatexOptions"
        | "utils::RweaveLatexSetup"
        | "utils::RweaveLatexWritedoc"
        | "utils::RweaveTryStop"
        | "utils::Stangle"
        | "utils::Sweave"
        | "utils::SweaveHooks"
        | "utils::SweaveSyntaxLatex"
        | "utils::SweaveSyntaxNoweb"
        | "utils::SweaveSyntConv"
        | "utils::update.packages"
        | "utils::upgrade" => Some(TypeTerm::Any),
        "utils::is.relistable" => Some(TypeTerm::Logical),
        "utils::packageName" | "utils::osVersion" | "utils::nsl" | "utils::select.list" => {
            Some(TypeTerm::Char)
        }
        "utils::menu" => Some(TypeTerm::Int),
        "utils::glob2rx" => Some(TypeTerm::Char),
        "utils::installed.packages" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Char))),
        "utils::maintainer" => Some(TypeTerm::Char),
        "utils::packageDate"
        | "utils::object.size"
        | "utils::memory.size"
        | "utils::memory.limit" => Some(TypeTerm::Double),
        "utils::read.csv"
        | "utils::read.csv2"
        | "utils::read.table"
        | "utils::read.delim"
        | "utils::read.fwf"
        | "utils::read.delim2"
        | "utils::read.DIF"
        | "utils::read.fortran" => Some(TypeTerm::DataFrame(Vec::new())),
        "utils::write.csv" | "utils::write.csv2" | "utils::write.table" | "utils::str" => {
            Some(TypeTerm::Null)
        }
        "utils::count.fields" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "utils::adist" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "utils::combn" => {
            let elem = match first_arg_term(arg_terms) {
                TypeTerm::Vector(inner)
                | TypeTerm::VectorLen(inner, _)
                | TypeTerm::Matrix(inner)
                | TypeTerm::MatrixDim(inner, _, _)
                | TypeTerm::ArrayDim(inner, _) => inner.as_ref().clone(),
                term => term,
            };
            Some(TypeTerm::Matrix(Box::new(elem)))
        }
        "utils::type.convert" => Some(vectorized_first_arg_term(first_arg_term(arg_terms))),
        _ => None,
    }
}
