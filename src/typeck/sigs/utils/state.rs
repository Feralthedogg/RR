use crate::typeck::builtin_sigs::{
    char_like_first_arg_type, first_arg_type_state, preserved_first_arg_type_without_len,
    vectorized_first_arg_type,
};
use crate::typeck::lattice::{PrimTy, ShapeTy, TypeState};

pub(crate) fn infer_utils_package_call(callee: &str, arg_tys: &[TypeState]) -> Option<TypeState> {
    match callee {
        "utils::head" | "utils::tail" => Some(preserved_first_arg_type_without_len(
            first_arg_type_state(arg_tys),
        )),
        "utils::packageVersion"
        | "utils::packageDescription"
        | "utils::sessionInfo"
        | "utils::citation"
        | "utils::person"
        | "utils::as.person"
        | "utils::as.personList"
        | "utils::getAnywhere" => Some(TypeState::vector(PrimTy::Any, false)),
        "utils::as.roman" => {
            let first = first_arg_type_state(arg_tys);
            if matches!(first.shape, ShapeTy::Scalar) {
                Some(TypeState::scalar(PrimTy::Int, false))
            } else {
                Some(TypeState::vector(PrimTy::Int, false).with_len(first.len_sym))
            }
        }
        "utils::hasName" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "utils::strcapture" => Some(
            TypeState::matrix(PrimTy::Any, false)
                .with_len(arg_tys.get(1).copied().and_then(|ty| ty.len_sym)),
        ),
        "utils::contrib.url" => Some(TypeState::scalar(PrimTy::Char, false)),
        "utils::localeToCharset" => Some(TypeState::vector(PrimTy::Char, false)),
        "utils::charClass" => Some(TypeState::vector(PrimTy::Logical, false)),
        "utils::findMatches" => Some(TypeState::vector(PrimTy::Char, false)),
        "utils::fileSnapshot" => Some(TypeState::vector(PrimTy::Any, false)),
        "utils::apropos" | "utils::find" | "utils::methods" => {
            Some(TypeState::vector(PrimTy::Char, false))
        }
        "utils::help.search" | "utils::data" => Some(TypeState::vector(PrimTy::Any, false)),
        "utils::argsAnywhere" => Some(TypeState::unknown()),
        "utils::compareVersion" => Some(TypeState::scalar(PrimTy::Double, false)),
        "utils::capture.output" => Some(TypeState::vector(PrimTy::Char, false)),
        "utils::file_test" => {
            let probe = arg_tys.get(1).copied().unwrap_or(TypeState::unknown());
            if matches!(probe.shape, ShapeTy::Scalar) {
                Some(TypeState::scalar(PrimTy::Logical, false))
            } else {
                Some(TypeState::vector(PrimTy::Logical, false).with_len(probe.len_sym))
            }
        }
        "utils::URLencode" | "utils::URLdecode" => {
            Some(char_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "utils::head.matrix" | "utils::tail.matrix" => Some(preserved_first_arg_type_without_len(
            first_arg_type_state(arg_tys),
        )),
        "utils::available.packages" => Some(TypeState::matrix(PrimTy::Char, false)),
        "utils::stack" | "utils::unstack" => Some(TypeState::matrix(PrimTy::Any, false)),
        "utils::strOptions" | "utils::txtProgressBar" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "utils::toBibtex" | "utils::toLatex" => Some(TypeState::vector(PrimTy::Char, false)),
        "utils::getTxtProgressBar" | "utils::setTxtProgressBar" => {
            Some(TypeState::scalar(PrimTy::Double, false))
        }
        "utils::modifyList"
        | "utils::relist"
        | "utils::as.relistable"
        | "utils::personList"
        | "utils::warnErrList"
        | "utils::readCitationFile"
        | "utils::bibentry"
        | "utils::citEntry"
        | "utils::citHeader"
        | "utils::citFooter" => Some(TypeState::vector(PrimTy::Any, false)),
        "utils::getSrcref" | "utils::getFromNamespace" | "utils::getS3method" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "utils::getParseData" => Some(TypeState::matrix(PrimTy::Any, false)),
        "utils::getParseText" | "utils::globalVariables" => {
            Some(TypeState::vector(PrimTy::Char, false))
        }
        "utils::getSrcFilename" | "utils::getSrcDirectory" => {
            Some(TypeState::scalar(PrimTy::Char, false))
        }
        "utils::getSrcLocation" => Some(TypeState::vector(PrimTy::Any, false)),
        "utils::hashtab" | "utils::gethash" => Some(TypeState::vector(PrimTy::Any, false)),
        "utils::sethash" => Some(TypeState::unknown()),
        "utils::remhash" | "utils::is.hashtab" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "utils::clrhash" | "utils::maphash" => Some(TypeState::null()),
        "utils::numhash" => Some(TypeState::scalar(PrimTy::Int, false)),
        "utils::typhash" => Some(TypeState::scalar(PrimTy::Char, false)),
        "utils::asDateBuilt" => Some(TypeState::scalar(PrimTy::Double, false)),
        "utils::findLineNum" => Some(TypeState::vector(PrimTy::Any, false)),
        "utils::getCRANmirrors" => Some(TypeState::matrix(PrimTy::Char, false)),
        "utils::findCRANmirror" => Some(TypeState::scalar(PrimTy::Char, false)),
        "utils::package.skeleton" | "utils::unzip" => Some(TypeState::vector(PrimTy::Any, false)),
        "utils::zip" => Some(TypeState::scalar(PrimTy::Int, false)),
        "utils::limitedLabels"
        | "utils::formatOL"
        | "utils::formatUL"
        | "utils::ls.str"
        | "utils::lsf.str" => Some(TypeState::vector(PrimTy::Char, false)),
        "utils::news" => Some(TypeState::null()),
        "utils::vignette" | "utils::hsearch_db" => Some(TypeState::vector(PrimTy::Any, false)),
        "utils::hsearch_db_concepts" | "utils::hsearch_db_keywords" => {
            Some(TypeState::matrix(PrimTy::Any, false))
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
        | "utils::xemacs" => Some(TypeState::null()),
        "utils::tar" => Some(TypeState::scalar(PrimTy::Int, false)),
        "utils::untar" => Some(TypeState::vector(PrimTy::Char, false)),
        "utils::timestamp" => Some(TypeState::scalar(PrimTy::Char, false)),
        "utils::Rprof" | "utils::Rprofmem" => Some(TypeState::null()),
        "utils::summaryRprof" | "utils::setRepositories" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "utils::?"
        | "utils::.AtNames"
        | "utils::.DollarNames"
        | "utils::cite"
        | "utils::citeNatbib"
        | "utils::help"
        | "utils::read.socket"
        | "utils::RweaveChunkPrefix" => Some(TypeState::vector(PrimTy::Char, false)),
        "utils::.romans" => Some(TypeState::vector(PrimTy::Int, false)),
        "utils::askYesNo" | "utils::isS3method" | "utils::isS3stdGeneric" => {
            Some(TypeState::scalar(PrimTy::Logical, false))
        }
        "utils::rc.settings" => Some(TypeState::vector(PrimTy::Logical, false)),
        "utils::download.file" => Some(TypeState::scalar(PrimTy::Int, false)),
        "utils::alarm" | "utils::rc.getOption" => Some(TypeState::scalar(PrimTy::Any, false)),
        "utils::close.socket"
        | "utils::history"
        | "utils::loadhistory"
        | "utils::savehistory"
        | "utils::write.socket" => Some(TypeState::null()),
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
        | "utils::upgrade" => Some(TypeState::vector(PrimTy::Any, false)),
        "utils::is.relistable" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "utils::packageName" | "utils::osVersion" | "utils::nsl" | "utils::select.list" => {
            Some(TypeState::scalar(PrimTy::Char, false))
        }
        "utils::menu" => Some(TypeState::scalar(PrimTy::Int, false)),
        "utils::glob2rx" => Some(TypeState::scalar(PrimTy::Char, false)),
        "utils::installed.packages" => Some(TypeState::matrix(PrimTy::Char, false)),
        "utils::maintainer" => Some(TypeState::scalar(PrimTy::Char, false)),
        "utils::packageDate"
        | "utils::object.size"
        | "utils::memory.size"
        | "utils::memory.limit" => Some(TypeState::scalar(PrimTy::Double, false)),
        "utils::read.csv"
        | "utils::read.csv2"
        | "utils::read.table"
        | "utils::read.delim"
        | "utils::read.fwf"
        | "utils::read.delim2"
        | "utils::read.DIF"
        | "utils::read.fortran" => Some(TypeState::matrix(PrimTy::Any, false)),
        "utils::write.csv" | "utils::write.csv2" | "utils::write.table" | "utils::str" => {
            Some(TypeState::null())
        }
        "utils::count.fields" => Some(TypeState::vector(PrimTy::Int, false)),
        "utils::adist" => Some(TypeState::matrix(PrimTy::Double, false)),
        "utils::combn" => {
            let first = first_arg_type_state(arg_tys);
            let prim = match first.shape {
                ShapeTy::Matrix | ShapeTy::Vector | ShapeTy::Scalar => first.prim,
                ShapeTy::Unknown => PrimTy::Any,
            };
            Some(TypeState::matrix(prim, false))
        }
        "utils::type.convert" => Some(vectorized_first_arg_type(first_arg_type_state(arg_tys))),
        _ => None,
    }
}
