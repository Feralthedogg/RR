use crate::typeck::builtin_sigs::{
    char_like_first_arg_term, char_like_first_arg_type, first_arg_term, first_arg_type_state,
};
use crate::typeck::lattice::{PrimTy, TypeState};
use crate::typeck::term::TypeTerm;

pub(crate) fn infer_tools_package_call(callee: &str, arg_tys: &[TypeState]) -> Option<TypeState> {
    match callee {
        "tools::toTitleCase" | "tools::file_path_as_absolute" | "tools::R_user_dir" => {
            Some(TypeState::scalar(PrimTy::Char, false))
        }
        "tools::md5sum" | "tools::sha256sum" => Some(TypeState::vector(PrimTy::Char, false)),
        "tools::file_ext" | "tools::file_path_sans_ext" => {
            Some(char_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "tools::list_files_with_exts" | "tools::list_files_with_type" | "tools::dependsOnPkgs" => {
            Some(TypeState::vector(PrimTy::Char, false))
        }
        "tools::getVignetteInfo" => Some(TypeState::matrix(PrimTy::Char, false)),
        "tools::pkgVignettes" => Some(TypeState::vector(PrimTy::Any, false)),
        "tools::delimMatch" => Some(TypeState::scalar(PrimTy::Int, false)),
        "tools::parse_URI_reference" => Some(TypeState::matrix(PrimTy::Char, false)),
        "tools::encoded_text_to_latex" => {
            Some(char_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "tools::parse_Rd" | "tools::Rd2txt_options" => Some(TypeState::vector(PrimTy::Any, false)),
        "tools::Rd2HTML" | "tools::Rd2latex" | "tools::Rd2ex" => {
            Some(TypeState::scalar(PrimTy::Char, false))
        }
        "tools::Rd2txt"
        | "tools::RdTextFilter"
        | "tools::checkRd"
        | "tools::showNonASCII"
        | "tools::showNonASCIIfile" => Some(TypeState::vector(PrimTy::Char, false)),
        "tools::find_gs_cmd"
        | "tools::findHTMLlinks"
        | "tools::makevars_site"
        | "tools::makevars_user"
        | "tools::HTMLheader"
        | "tools::SweaveTeXFilter"
        | "tools::toHTML"
        | "tools::toRd"
        | "tools::charset_to_Unicode" => Some(TypeState::vector(PrimTy::Char, false)),
        "tools::Rdindex" => Some(TypeState::null()),
        "tools::read.00Index" => Some(TypeState::matrix(PrimTy::Char, false)),
        "tools::parseLatex" => Some(TypeState::vector(PrimTy::Any, false)),
        "tools::getBibstyle" => Some(TypeState::scalar(PrimTy::Char, false)),
        "tools::deparseLatex" => Some(TypeState::scalar(PrimTy::Char, false)),
        "tools::latexToUtf8" => Some(TypeState::vector(PrimTy::Any, false)),
        "tools::SIGCHLD" | "tools::SIGCONT" | "tools::SIGHUP" | "tools::SIGINT"
        | "tools::SIGKILL" | "tools::SIGQUIT" | "tools::SIGSTOP" | "tools::SIGTERM"
        | "tools::SIGTSTP" | "tools::SIGUSR1" | "tools::SIGUSR2" => {
            Some(TypeState::scalar(PrimTy::Int, false))
        }
        "tools::assertCondition"
        | "tools::assertError"
        | "tools::assertWarning"
        | "tools::add_datalist"
        | "tools::buildVignette"
        | "tools::buildVignettes"
        | "tools::compactPDF"
        | "tools::installFoundDepends"
        | "tools::make_translations_pkg"
        | "tools::package_native_routine_registration_skeleton"
        | "tools::pskill"
        | "tools::psnice"
        | "tools::resaveRdaFiles"
        | "tools::startDynamicHelp"
        | "tools::testInstalledBasic"
        | "tools::testInstalledPackage"
        | "tools::testInstalledPackages"
        | "tools::texi2dvi"
        | "tools::texi2pdf"
        | "tools::update_PACKAGES"
        | "tools::update_pkg_po"
        | "tools::write_PACKAGES"
        | "tools::xgettext"
        | "tools::xgettext2pot"
        | "tools::xngettext" => Some(TypeState::null()),
        "tools::Adobe_glyphs" => Some(TypeState::matrix(PrimTy::Char, false)),
        "tools::.print.via.format"
        | "tools::analyze_license"
        | "tools::as.Rconcordance"
        | "tools::bibstyle"
        | "tools::check_package_dois"
        | "tools::check_package_urls"
        | "tools::check_packages_in_dir"
        | "tools::check_packages_in_dir_changes"
        | "tools::check_packages_in_dir_details"
        | "tools::checkDocFiles"
        | "tools::checkDocStyle"
        | "tools::checkFF"
        | "tools::checkMD5sums"
        | "tools::checkPoFile"
        | "tools::checkPoFiles"
        | "tools::checkRdaFiles"
        | "tools::checkRdContents"
        | "tools::checkReplaceFuns"
        | "tools::checkS3methods"
        | "tools::checkTnF"
        | "tools::checkVignettes"
        | "tools::codoc"
        | "tools::codocClasses"
        | "tools::codocData"
        | "tools::followConcordance"
        | "tools::getDepList"
        | "tools::langElts"
        | "tools::loadPkgRdMacros"
        | "tools::loadRdMacros"
        | "tools::matchConcordance"
        | "tools::nonS3methods"
        | "tools::package.dependencies"
        | "tools::pkg2HTML"
        | "tools::pkgDepends"
        | "tools::R"
        | "tools::Rcmd"
        | "tools::Rdiff"
        | "tools::summarize_check_packages_in_dir_depends"
        | "tools::summarize_check_packages_in_dir_results"
        | "tools::summarize_check_packages_in_dir_timings"
        | "tools::undoc"
        | "tools::vignetteDepends"
        | "tools::vignetteEngine"
        | "tools::vignetteInfo" => Some(TypeState::vector(PrimTy::Any, false)),
        "tools::standard_package_names"
        | "tools::base_aliases_db"
        | "tools::base_rdxrefs_db"
        | "tools::CRAN_aliases_db"
        | "tools::CRAN_archive_db"
        | "tools::CRAN_rdxrefs_db" => Some(TypeState::vector(PrimTy::Any, false)),
        "tools::CRAN_package_db"
        | "tools::CRAN_authors_db"
        | "tools::CRAN_current_db"
        | "tools::CRAN_check_results"
        | "tools::CRAN_check_details"
        | "tools::CRAN_check_issues" => Some(TypeState::matrix(PrimTy::Any, false)),
        "tools::summarize_CRAN_check_status" => Some(TypeState::vector(PrimTy::Char, false)),
        "tools::package_dependencies" | "tools::Rd_db" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        _ => None,
    }
}

pub(crate) fn infer_tools_package_call_term(
    callee: &str,
    arg_terms: &[TypeTerm],
) -> Option<TypeTerm> {
    match callee {
        "tools::toTitleCase" | "tools::file_path_as_absolute" | "tools::R_user_dir" => {
            Some(TypeTerm::Char)
        }
        "tools::md5sum"
        | "tools::sha256sum"
        | "tools::list_files_with_exts"
        | "tools::list_files_with_type"
        | "tools::dependsOnPkgs"
        | "tools::find_gs_cmd"
        | "tools::findHTMLlinks"
        | "tools::makevars_site"
        | "tools::makevars_user"
        | "tools::HTMLheader"
        | "tools::SweaveTeXFilter"
        | "tools::toHTML"
        | "tools::toRd"
        | "tools::charset_to_Unicode"
        | "tools::Rd2txt"
        | "tools::RdTextFilter"
        | "tools::checkRd"
        | "tools::showNonASCII"
        | "tools::showNonASCIIfile"
        | "tools::summarize_CRAN_check_status" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "tools::file_ext" | "tools::file_path_sans_ext" | "tools::encoded_text_to_latex" => {
            Some(char_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "tools::getVignetteInfo" | "tools::read.00Index" => {
            Some(TypeTerm::Matrix(Box::new(TypeTerm::Char)))
        }
        "tools::pkgVignettes" => Some(TypeTerm::NamedList(vec![
            (
                "docs".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "names".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "engines".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "patterns".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "encodings".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            ("dir".to_string(), TypeTerm::Char),
            ("pkgdir".to_string(), TypeTerm::Char),
            (
                "msg".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])),
        "tools::Rd2txt_options" => Some(TypeTerm::NamedList(vec![
            ("width".to_string(), TypeTerm::Int),
            ("minIndent".to_string(), TypeTerm::Int),
            ("extraIndent".to_string(), TypeTerm::Int),
            ("sectionIndent".to_string(), TypeTerm::Int),
            ("sectionExtra".to_string(), TypeTerm::Int),
            ("itemBullet".to_string(), TypeTerm::Char),
            ("enumFormat".to_string(), TypeTerm::Any),
            ("showURLs".to_string(), TypeTerm::Logical),
            ("code_quote".to_string(), TypeTerm::Logical),
            ("underline_titles".to_string(), TypeTerm::Logical),
        ])),
        "tools::standard_package_names"
        | "tools::base_aliases_db"
        | "tools::base_rdxrefs_db"
        | "tools::CRAN_aliases_db"
        | "tools::CRAN_archive_db"
        | "tools::CRAN_rdxrefs_db"
        | "tools::checkS3methods"
        | "tools::parse_Rd"
        | "tools::parseLatex"
        | "tools::package.dependencies"
        | "tools::vignetteEngine"
        | "tools::vignetteInfo" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "tools::delimMatch" => Some(TypeTerm::Int),
        "tools::parse_URI_reference" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "scheme".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "authority".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "path".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "query".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "fragment".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])),
        "tools::Rd2HTML"
        | "tools::Rd2latex"
        | "tools::Rd2ex"
        | "tools::getBibstyle"
        | "tools::deparseLatex" => Some(TypeTerm::Char),
        "tools::Rdindex"
        | "tools::assertCondition"
        | "tools::assertError"
        | "tools::assertWarning"
        | "tools::add_datalist"
        | "tools::buildVignette"
        | "tools::buildVignettes"
        | "tools::compactPDF"
        | "tools::installFoundDepends"
        | "tools::make_translations_pkg"
        | "tools::package_native_routine_registration_skeleton"
        | "tools::pskill"
        | "tools::psnice"
        | "tools::resaveRdaFiles"
        | "tools::startDynamicHelp"
        | "tools::testInstalledBasic"
        | "tools::testInstalledPackage"
        | "tools::testInstalledPackages"
        | "tools::texi2dvi"
        | "tools::texi2pdf"
        | "tools::update_PACKAGES"
        | "tools::update_pkg_po"
        | "tools::write_PACKAGES"
        | "tools::xgettext"
        | "tools::xgettext2pot"
        | "tools::xngettext" => Some(TypeTerm::Null),
        "tools::Adobe_glyphs" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "adobe".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "unicode".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])),
        "tools::CRAN_package_db"
        | "tools::CRAN_authors_db"
        | "tools::CRAN_current_db"
        | "tools::CRAN_check_results"
        | "tools::CRAN_check_details"
        | "tools::CRAN_check_issues" => Some(TypeTerm::DataFrame(Vec::new())),
        "tools::package_dependencies" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "tools::Rd_db" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "tools::latexToUtf8" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "tools::SIGCHLD" | "tools::SIGCONT" | "tools::SIGHUP" | "tools::SIGINT"
        | "tools::SIGKILL" | "tools::SIGQUIT" | "tools::SIGSTOP" | "tools::SIGTERM"
        | "tools::SIGTSTP" | "tools::SIGUSR1" | "tools::SIGUSR2" => Some(TypeTerm::Int),
        _ => None,
    }
}
