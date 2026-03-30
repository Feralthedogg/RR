# Base / Data

Base package direct interop surface.
Part of the [R Interop](../r-interop.md) reference.

Unlisted regex-safe `base::...` names are not automatically "unsupported":
`base` also has a conservative package-wide direct fallback for awkward
operator/replacement exports and other names that are not cleanly represented in
this quoted surface list.

## Direct Surface

- `base::data.frame`
- `base::globalenv`
- `base::environment`
- `base::unlink`
- `base::file.path`
- `base::basename`
- `base::dirname`
- `base::normalizePath`
- `base::dir.exists`
- `base::file.exists`
- `base::eval`
- `base::evalq`
- `base::do.call`
- `base::parse`
- `base::getOption`
- `base::file`
- `base::list.files`
- `base::path.expand`
- `base::readRDS`
- `base::save`
- `base::get0`
- `base::getNamespace`
- `base::asNamespace`
- `base::isNamespace`
- `base::find.package`
- `base::package_version`
- `base::is.name`
- `base::getElement`
- `base::unname`
- `base::baseenv`
- `base::emptyenv`
- `base::new.env`
- `base::parent.env`
- `base::as.environment`
- `base::is.environment`
- `base::environmentName`
- `base::list2env`
- `base::as.list.environment`
- `base::environmentIsLocked`
- `base::topenv`
- `base::loadedNamespaces`
- `base::isNamespaceLoaded`
- `base::getNamespaceName`
- `base::getNamespaceExports`
- `base::getNamespaceImports`
- `base::getNamespaceUsers`
- `base::getNamespaceVersion`
- `base::requireNamespace`
- `base::library`
- `base::require`
- `base::loadNamespace`
- `base::packageHasNamespace`
- `base::searchpaths`
- `base::getLoadedDLLs`
- `base::is.loaded`
- `base::dyn.load`
- `base::dyn.unload`
- `base::readLines`
- `base::writeLines`
- `base::writeChar`
- `base::writeBin`
- `base::flush`
- `base::seek`
- `base::truncate.connection`
- `base::Sys.getenv`
- `base::Sys.setenv`
- `base::Sys.unsetenv`
- `base::Sys.which`
- `base::Sys.readlink`
- `base::Sys.getpid`
- `base::Sys.time`
- `base::Sys.Date`
- `base::Sys.info`
- `base::Sys.getlocale`
- `base::Sys.glob`
- `base::system`
- `base::system2`
- `base::system.time`
- `base::Sys.sleep`
- `base::Sys.setlocale`
- `base::Sys.timezone`
- `base::Sys.localeconv`
- `base::Sys.setFileTime`
- `base::Sys.chmod`
- `base::Sys.umask`
- `base::sys.call`
- `base::sys.calls`
- `base::sys.function`
- `base::sys.frame`
- `base::sys.frames`
- `base::sys.parent`
- `base::sys.parents`
- `base::sys.nframe`
- `base::sys.status`
- `base::sys.source`
- `base::source`
- `base::search`
- `base::options`
- `base::geterrmessage`
- `base::gettext`
- `base::gettextf`
- `base::ngettext`
- `base::message`
- `base::warning`
- `base::warningCondition`
- `base::packageStartupMessage`
- `base::.packageStartupMessage`
- `base::packageNotFoundError`
- `base::packageEvent`
- `base::stdin`
- `base::stdout`
- `base::stderr`
- `base::textConnection`
- `base::textConnectionValue`
- `base::rawConnection`
- `base::rawConnectionValue`
- `base::socketConnection`
- `base::url`
- `base::pipe`
- `base::open`
- `base::close`
- `base::closeAllConnections`
- `base::isOpen`
- `base::isIncomplete`
- `base::summary.connection`
- `base::pushBack`
- `base::pushBackLength`
- `base::clearPushBack`
- `base::socketSelect`
- `base::scan`
- `base::read.table`
- `base::read.csv`
- `base::read.csv2`
- `base::read.delim`
- `base::read.delim2`
- `base::write.table`
- `base::write.csv`
- `base::write.csv2`
- `base::saveRDS`
- `base::dput`
- `base::dump`
- `base::count.fields`
- `base::sink`
- `base::sink.number`
- `base::capture.output`
- `base::lapply`
- `base::sapply`
- `base::vapply`
- `base::mapply`
- `base::Map`
- `base::Reduce`
- `base::Filter`
- `base::Find`
- `base::Position`
- `base::split`
- `base::unsplit`
- `base::tapply`
- `base::apply`
- `base::by`
- `base::within`
- `base::transform`
- `base::expand.grid`
- `base::merge`
- `base::as.Date`
- `base::as.Date.character`
- `base::as.Date.default`
- `base::as.Date.factor`
- `base::as.Date.numeric`
- `base::as.Date.POSIXct`
- `base::as.Date.POSIXlt`
- `base::as.POSIXct`
- `base::as.POSIXct.Date`
- `base::as.POSIXct.default`
- `base::as.POSIXct.numeric`
- `base::as.POSIXct.POSIXlt`
- `base::as.POSIXlt`
- `base::as.POSIXlt.character`
- `base::as.POSIXlt.Date`
- `base::as.POSIXlt.default`
- `base::as.POSIXlt.factor`
- `base::as.POSIXlt.numeric`
- `base::as.POSIXlt.POSIXct`
- `base::as.difftime`
- `base::as.double.difftime`
- `base::as.double.POSIXlt`
- `base::alist`
- `base::as.call`
- `base::as.expression`
- `base::as.expression.default`
- `base::as.function`
- `base::as.function.default`
- `base::as.name`
- `base::as.null`
- `base::as.null.default`
- `base::Arg`
- `base::as.package_version`
- `base::as.pairlist`
- `base::as.raw`
- `base::as.single`
- `base::as.single.default`
- `base::as.symbol`
- `base::as.table`
- `base::as.table.default`
- `base::as.complex`
- `base::as.hexmode`
- `base::as.numeric_version`
- `base::as.octmode`
- `base::as.ordered`
- `base::as.qr`
- `base::asplit`
- `base::asS3`
- `base::asS4`
- `base::abbreviate`
- `base::all.equal`
- `base::all.equal.default`
- `base::all.equal.character`
- `base::all.equal.environment`
- `base::all.equal.envRefClass`
- `base::all.equal.factor`
- `base::all.equal.formula`
- `base::all.equal.function`
- `base::all.equal.language`
- `base::all.equal.list`
- `base::all.equal.numeric`
- `base::all.equal.POSIXt`
- `base::all.equal.raw`
- `base::all.names`
- `base::all.vars`
- `base::anyDuplicated.array`
- `base::anyDuplicated.data.frame`
- `base::anyDuplicated.default`
- `base::anyDuplicated.matrix`
- `base::anyNA`
- `base::anyNA.data.frame`
- `base::anyNA.numeric_version`
- `base::anyNA.POSIXlt`
- `base::activeBindingFunction`
- `base::addTaskCallback`
- `base::allowInterrupts`
- `base::args`
- `base::attach`
- `base::attachNamespace`
- `base::autoload`
- `base::autoloader`
- `base::body`
- `base::bindingIsActive`
- `base::bindingIsLocked`
- `base::bindtextdomain`
- `base::break`
- `base::call`
- `base::bquote`
- `base::browser`
- `base::browserSetDebug`
- `base::browserText`
- `base::browserCondition`
- `base::builtins`
- `base::backsolve`
- `base::balancePOSIXlt`
- `base::besselI`
- `base::besselJ`
- `base::besselK`
- `base::besselY`
- `base::beta`
- `base::bitwAnd`
- `base::bitwNot`
- `base::bitwOr`
- `base::bitwShiftL`
- `base::bitwShiftR`
- `base::bitwXor`
- `base::by.data.frame`
- `base::by.default`
- `base::c.Date`
- `base::c.difftime`
- `base::c.factor`
- `base::c.noquote`
- `base::c.numeric_version`
- `base::c.POSIXct`
- `base::c.POSIXlt`
- `base::c.warnings`
- `base::callCC`
- `base::cbind.data.frame`
- `base::close.connection`
- `base::close.srcfile`
- `base::close.srcfilealias`
- `base::comment`
- `base::computeRestarts`
- `base::conditionCall`
- `base::conditionCall.condition`
- `base::conditionMessage`
- `base::conditionMessage.condition`
- `base::conflictRules`
- `base::conflicts`
- `base::contributors`
- `base::Cstack_info`
- `base::curlGetHeaders`
- `base::casefold`
- `base::char.expand`
- `base::charmatch`
- `base::charToRaw`
- `base::chkDots`
- `base::chol`
- `base::chol.default`
- `base::chol2inv`
- `base::choose`
- `base::chooseOpsMethod`
- `base::chooseOpsMethod.default`
- `base::Conj`
- `base::complete.cases`
- `base::complex`
- `base::cummax`
- `base::cummin`
- `base::cumprod`
- `base::cumsum`
- `base::diff`
- `base::diff.default`
- `base::cut.Date`
- `base::cut.POSIXt`
- `base::cut.default`
- `base::diff.Date`
- `base::diff.POSIXt`
- `base::diff.difftime`
- `base::commandArgs`
- `base::data.class`
- `base::date`
- `base::deparse`
- `base::deparse1`
- `base::data.matrix`
- `base::det`
- `base::determinant`
- `base::determinant.matrix`
- `base::debug`
- `base::debuggingState`
- `base::debugonce`
- `base::declare`
- `base::delayedAssign`
- `base::detach`
- `base::dget`
- `base::digamma`
- `base::dontCheck`
- `base::drop`
- `base::droplevels`
- `base::droplevels.data.frame`
- `base::droplevels.factor`
- `base::duplicated.default`
- `base::duplicated.array`
- `base::duplicated.data.frame`
- `base::duplicated.matrix`
- `base::duplicated.numeric_version`
- `base::duplicated.POSIXlt`
- `base::duplicated.warnings`
- `base::dQuote`
- `base::enc2native`
- `base::enc2utf8`
- `base::encodeString`
- `base::Encoding`
- `base::enquote`
- `base::env.profile`
- `base::errorCondition`
- `base::eval.parent`
- `base::Exec`
- `base::exists`
- `base::expression`
- `base::eigen`
- `base::expm1`
- `base::extSoftVersion`
- `base::factorial`
- `base::findInterval`
- `base::file.choose`
- `base::file.show`
- `base::format`
- `base::format.AsIs`
- `base::format.data.frame`
- `base::format.default`
- `base::format.difftime`
- `base::format.factor`
- `base::format.hexmode`
- `base::format.info`
- `base::format.libraryIQR`
- `base::format.numeric_version`
- `base::format.octmode`
- `base::format.packageInfo`
- `base::format.pval`
- `base::format.summaryDefault`
- `base::formatC`
- `base::formatDL`
- `base::gl`
- `base::acosh`
- `base::asinh`
- `base::atanh`
- `base::cospi`
- `base::capabilities`
- `base::append`
- `base::array`
- `base::as.array`
- `base::as.array.default`
- `base::as.character.condition`
- `base::as.character.default`
- `base::as.character.error`
- `base::as.character.factor`
- `base::as.character.hexmode`
- `base::as.character.numeric_version`
- `base::as.character.octmode`
- `base::as.character.srcref`
- `base::as.data.frame`
- `base::as.data.frame.array`
- `base::as.data.frame.AsIs`
- `base::as.data.frame.character`
- `base::as.data.frame.complex`
- `base::as.data.frame.data.frame`
- `base::as.data.frame.Date`
- `base::as.data.frame.default`
- `base::as.data.frame.difftime`
- `base::as.data.frame.factor`
- `base::as.data.frame.integer`
- `base::as.data.frame.list`
- `base::as.data.frame.logical`
- `base::as.data.frame.matrix`
- `base::as.data.frame.model.matrix`
- `base::as.data.frame.noquote`
- `base::as.data.frame.numeric`
- `base::as.data.frame.numeric_version`
- `base::as.data.frame.ordered`
- `base::as.data.frame.POSIXct`
- `base::as.data.frame.POSIXlt`
- `base::as.data.frame.raw`
- `base::as.data.frame.table`
- `base::as.data.frame.ts`
- `base::as.data.frame.vector`
- `base::as.list`
- `base::as.list.data.frame`
- `base::as.list.Date`
- `base::as.list.default`
- `base::as.list.difftime`
- `base::as.list.factor`
- `base::as.list.function`
- `base::as.list.numeric_version`
- `base::as.list.POSIXct`
- `base::as.list.POSIXlt`
- `base::as.logical.factor`
- `base::as.matrix`
- `base::as.matrix.data.frame`
- `base::as.matrix.default`
- `base::as.matrix.noquote`
- `base::as.matrix.POSIXlt`
- `base::array2DF`
- `base::arrayInd`
- `base::aperm`
- `base::aperm.default`
- `base::aperm.table`
- `base::as.character`
- `base::as.double`
- `base::as.factor`
- `base::as.integer`
- `base::as.logical`
- `base::as.numeric`
- `base::as.vector`
- `base::as.vector.data.frame`
- `base::as.vector.factor`
- `base::as.vector.POSIXlt`
- `base::col`
- `base::row`
- `base::colMeans`
- `base::rowMeans`
- `base::class`
- `base::attr`
- `base::attributes`
- `base::levels`
- `base::ordered`
- `base::addNA`
- `base::as.character.Date`
- `base::as.character.POSIXt`
- `base::format.Date`
- `base::format.POSIXct`
- `base::format.POSIXlt`
- `base::strptime`
- `base::difftime`
- `base::months`
- `base::quarters`
- `base::weekdays`
- `base::julian`
- `base::OlsonNames`
- `base::ISOdate`
- `base::ISOdatetime`
- `base::seq.Date`
- `base::seq.POSIXt`
- `base::readBin`
- `base::readChar`
- `base::serialize`
- `base::unserialize`
- `base::load`
- `base::fifo`
- `base::gzcon`
- `base::getwd`
- `base::tempdir`
- `base::tempfile`
- `base::dir`
- `base::list.dirs`
- `base::dir.create`
- `base::file.create`
- `base::file.remove`
- `base::file.rename`
- `base::file.copy`
- `base::file.append`
- `base::file.link`
- `base::file.symlink`
- `base::file.access`
- `base::file.info`
- `base::file.size`
- `base::file.mtime`
- `base::file.mode`
- `base::system.file`
- `base::path.package`
- `base::.packages`
- `base::length`
- `base::c`
- `base::list`
- `base::sum`
- `base::mean`
- `base::vector`
- `base::seq`
- `base::ifelse`
- `base::abs`
- `base::min`
- `base::max`
- `base::pmax`
- `base::pmin`
- `base::sqrt`
- `base::log`
- `base::log10`
- `base::log2`
- `base::exp`
- `base::atan2`
- `base::sin`
- `base::cos`
- `base::tan`
- `base::asin`
- `base::acos`
- `base::atan`
- `base::sinh`
- `base::cosh`
- `base::tanh`
- `base::sign`
- `base::gamma`
- `base::lgamma`
- `base::floor`
- `base::ceiling`
- `base::trunc`
- `base::round`
- `base::is.na`
- `base::is.finite`
- `base::identical`
- `base::inherits`
- `base::interactive`
- `base::is.R`
- `base::is.array`
- `base::is.atomic`
- `base::is.call`
- `base::is.character`
- `base::is.complex`
- `base::is.data.frame`
- `base::is.double`
- `base::is.element`
- `base::is.expression`
- `base::is.factor`
- `base::is.finite.POSIXlt`
- `base::is.function`
- `base::is.infinite`
- `base::is.infinite.POSIXlt`
- `base::is.integer`
- `base::is.language`
- `base::is.list`
- `base::is.logical`
- `base::is.na.POSIXlt`
- `base::is.na.data.frame`
- `base::is.na.numeric_version`
- `base::is.nan`
- `base::is.nan.POSIXlt`
- `base::is.null`
- `base::is.numeric`
- `base::is.numeric.Date`
- `base::is.numeric.POSIXt`
- `base::is.numeric.difftime`
- `base::is.numeric_version`
- `base::is.object`
- `base::is.ordered`
- `base::is.package_version`
- `base::is.pairlist`
- `base::is.primitive`
- `base::is.qr`
- `base::is.raw`
- `base::is.recursive`
- `base::is.single`
- `base::is.symbol`
- `base::is.table`
- `base::is.unsorted`
- `base::is.vector`
- `base::print`
- `base::print.AsIs`
- `base::print.DLLInfo`
- `base::print.DLLInfoList`
- `base::print.DLLRegisteredRoutines`
- `base::print.Date`
- `base::print.Dlist`
- `base::print.NativeRoutineList`
- `base::print.POSIXct`
- `base::print.POSIXlt`
- `base::print.by`
- `base::print.condition`
- `base::print.connection`
- `base::print.data.frame`
- `base::print.default`
- `base::print.difftime`
- `base::print.eigen`
- `base::print.factor`
- `base::print.function`
- `base::print.hexmode`
- `base::print.libraryIQR`
- `base::print.listof`
- `base::print.noquote`
- `base::print.numeric_version`
- `base::print.octmode`
- `base::print.packageInfo`
- `base::print.proc_time`
- `base::print.restart`
- `base::print.rle`
- `base::print.simple.list`
- `base::print.srcfile`
- `base::print.srcref`
- `base::print.summary.table`
- `base::print.summary.warnings`
- `base::print.summaryDefault`
- `base::print.table`
- `base::print.warnings`
- `base::numeric`
- `base::matrix`
- `base::dim`
- `base::dimnames`
- `base::nrow`
- `base::ncol`
- `base::seq_len`
- `base::seq_along`
- `base::diag`
- `base::t`
- `base::rbind`
- `base::cbind`
- `base::rowSums`
- `base::colSums`
- `base::crossprod`
- `base::tcrossprod`
- `base::character`
- `base::logical`
- `base::integer`
- `base::double`
- `base::rep`
- `base::any`
- `base::all`
- `base::which`
- `base::prod`
- `base::paste`
- `base::paste0`
- `base::sprintf`
- `base::cat`
- `base::rep.int`
- `base::tolower`
- `base::toupper`
- `base::substr`
- `base::sub`
- `base::gsub`
- `base::nchar`
- `base::nzchar`
- `base::grepl`
- `base::grep`
- `base::startsWith`
- `base::endsWith`
- `base::which.min`
- `base::which.max`
- `base::isTRUE`
- `base::isFALSE`
- `base::lengths`
- `base::union`
- `base::intersect`
- `base::setdiff`
- `base::sample`
- `base::sample.int`
- `base::rank`
- `base::factor`
- `base::cut`
- `base::table`
- `base::trimws`
- `base::chartr`
- `base::strsplit`
- `base::regexpr`
- `base::gregexpr`
- `base::regexec`
- `base::agrep`
- `base::agrepl`
- `base::names`
- `base::rownames`
- `base::colnames`
- `base::sort`
- `base::order`
- `base::match`
- `base::unique`
- `base::duplicated`
- `base::anyDuplicated`
- `base::summary`
- `base::summary.Date`
- `base::summary.POSIXct`
- `base::summary.POSIXlt`
- `base::summary.data.frame`
- `base::summary.default`
- `base::summary.difftime`
- `base::summary.factor`
- `base::summary.matrix`
- `base::summary.proc_time`
- `base::summary.srcfile`
- `base::summary.srcref`
- `base::summary.table`
- `base::summary.warnings`

- `base::F`
- `base::I`
- `base::Im`
- `base::LETTERS`
- `base::La.svd`
- `base::La_library`
- `base::La_version`
- `base::Math.Date`
- `base::Math.POSIXt`
- `base::Math.data.frame`
- `base::Math.difftime`
- `base::Math.factor`
- `base::Mod`
- `base::NCOL`
- `base::NROW`
- `base::Negate`
- `base::NextMethod`
- `base::Ops.Date`
- `base::Ops.POSIXt`
- `base::Ops.data.frame`
- `base::Ops.difftime`
- `base::Ops.factor`
- `base::Ops.numeric_version`
- `base::Ops.ordered`
- `base::R.Version`
- `base::R.home`
- `base::R.version`
- `base::R.version.string`
- `base::RNGkind`
- `base::RNGversion`
- `base::R_compiled_by`
- `base::R_system_version`
- `base::Re`
- `base::Recall`
- `base::Summary.Date`
- `base::Summary.POSIXct`
- `base::Summary.POSIXlt`
- `base::Summary.data.frame`
- `base::Summary.difftime`
- `base::Summary.factor`
- `base::Summary.numeric_version`
- `base::Summary.ordered`
- `base::Sys.setLanguage`
- `base::T`
- `base::Tailcall`
- `base::UseMethod`
- `base::Vectorize`
- `base::assign`
- `base::attr.all.equal`
- `base::bzfile`
- `base::default.stringsAsFactors`
- `base::dim.data.frame`
- `base::dimnames.data.frame`
- `base::dynGet`
- `base::eapply`
- `base::findPackageEnv`
- `base::findRestart`
- `base::flush.connection`
- `base::for`
- `base::force`
- `base::forceAndCall`
- `base::formals`
- `base::forwardsolve`
- `base::function`
- `base::gc`
- `base::gc.time`
- `base::gcinfo`
- `base::gctorture`
- `base::gctorture2`
- `base::get`
- `base::getAllConnections`
- `base::getCallingDLL`
- `base::getCallingDLLe`
- `base::getConnection`
- `base::getDLLRegisteredRoutines`
- `base::getDLLRegisteredRoutines.DLLInfo`
- `base::getDLLRegisteredRoutines.character`
- `base::getExportedValue`
- `base::getHook`
- `base::getNamespaceInfo`
- `base::getNativeSymbolInfo`
- `base::getRversion`
- `base::getSrcLines`
- `base::getTaskCallbackNames`
- `base::globalCallingHandlers`
- `base::gregexec`
- `base::grepRaw`
- `base::grepv`
- `base::grouping`
- `base::gzfile`
- `base::iconv`
- `base::iconvlist`
- `base::icuGetCollate`
- `base::icuSetCollate`
- `base::identity`
- `base::if`
- `base::importIntoEnv`
- `base::infoRDS`
- `base::intToBits`
- `base::intToUtf8`
- `base::interaction`
- `base::inverse.rle`
- `base::invisible`
- `base::invokeRestart`
- `base::invokeRestartInteractively`
- `base::is.matrix`
- `base::isBaseNamespace`
- `base::isRestart`
- `base::isS4`
- `base::isSeekable`
- `base::isSymmetric`
- `base::isSymmetric.matrix`
- `base::isa`
- `base::isatty`
- `base::isdebugged`
- `base::jitter`
- `base::julian.Date`
- `base::julian.POSIXt`
- `base::kappa`
- `base::kappa.default`
- `base::kappa.lm`
- `base::kappa.qr`
- `base::kronecker`
- `base::l10n_info`
- `base::labels`
- `base::labels.default`
- `base::lazyLoad`
- `base::lazyLoadDBexec`
- `base::lazyLoadDBfetch`
- `base::lbeta`
- `base::lchoose`
- `base::length.POSIXlt`
- `base::letters`
- `base::levels.default`
- `base::lfactorial`
- `base::libcurlVersion`
- `base::library.dynam`
- `base::library.dynam.unload`
- `base::licence`
- `base::license`
- `base::list2DF`
- `base::loadingNamespaceInfo`
- `base::local`
- `base::lockBinding`
- `base::lockEnvironment`
- `base::log1p`
- `base::logb`
- `base::lower.tri`
- `base::ls`
- `base::make.names`
- `base::make.unique`
- `base::makeActiveBinding`
- `base::margin.table`
- `base::marginSums`
- `base::mat.or.vec`
- `base::match.arg`
- `base::match.call`
- `base::match.fun`
- `base::max.col`
- `base::mean.Date`
- `base::mean.POSIXct`
- `base::mean.POSIXlt`
- `base::mean.default`
- `base::mean.difftime`
- `base::mem.maxNSize`
- `base::mem.maxVSize`
- `base::memCompress`
- `base::memDecompress`
- `base::memory.profile`
- `base::merge.data.frame`
- `base::merge.default`
- `base::mget`
- `base::missing`
- `base::mode`
- `base::month.abb`
- `base::month.name`
- `base::months.Date`
- `base::months.POSIXt`
- `base::mtfrm`
- `base::mtfrm.POSIXct`
- `base::mtfrm.POSIXlt`
- `base::mtfrm.default`
- `base::nameOfClass`
- `base::nameOfClass.default`
- `base::names.POSIXlt`
- `base::namespaceExport`
- `base::namespaceImport`
- `base::namespaceImportClasses`
- `base::namespaceImportFrom`
- `base::namespaceImportMethods`
- `base::nargs`
- `base::next`
- `base::nlevels`
- `base::noquote`
- `base::norm`
- `base::nullfile`
- `base::numToBits`
- `base::numToInts`
- `base::numeric_version`
- `base::objects`
- `base::oldClass`
- `base::on.exit`
- `base::open.connection`
- `base::open.srcfile`
- `base::open.srcfilealias`
- `base::open.srcfilecopy`
- `base::outer`
- `base::packBits`
- `base::pairlist`
- `base::parent.frame`
- `base::parseNamespaceFile`
- `base::pcre_config`
- `base::pi`
- `base::plot`
- `base::pmatch`
- `base::pmax.int`
- `base::pmin.int`
- `base::polyroot`
- `base::pos.to.env`
- `base::pretty`
- `base::pretty.default`
- `base::prettyNum`
- `base::prmatrix`
- `base::proc.time`
- `base::prop.table`
- `base::proportions`
- `base::provideDimnames`
- `base::psigamma`
- `base::q`
- `base::qr`
- `base::qr.Q`
- `base::qr.R`
- `base::qr.X`
- `base::qr.coef`
- `base::qr.default`
- `base::qr.fitted`
- `base::qr.qty`
- `base::qr.qy`
- `base::qr.resid`
- `base::qr.solve`
- `base::quarters.Date`
- `base::quarters.POSIXt`
- `base::quit`
- `base::quote`
- `base::range`
- `base::range.Date`
- `base::range.POSIXct`
- `base::range.default`
- `base::rapply`
- `base::raw`
- `base::rawShift`
- `base::rawToBits`
- `base::rawToChar`
- `base::rbind.data.frame`
- `base::rcond`
- `base::read.dcf`
- `base::readRenviron`
- `base::readline`
- `base::reg.finalizer`
- `base::registerS3method`
- `base::registerS3methods`
- `base::regmatches`
- `base::remove`
- `base::removeTaskCallback`
- `base::rep.Date`
- `base::rep.POSIXct`
- `base::rep.POSIXlt`
- `base::rep.difftime`
- `base::rep.factor`
- `base::rep.numeric_version`
- `base::rep_len`
- `base::repeat`
- `base::replace`
- `base::replicate`
- `base::restartDescription`
- `base::restartFormals`
- `base::retracemem`
- `base::return`
- `base::returnValue`
- `base::rev`
- `base::rev.default`
- `base::rle`
- `base::rm`
- `base::round.Date`
- `base::round.POSIXt`
- `base::row.names`
- `base::row.names.data.frame`
- `base::row.names.default`
- `base::rowsum`
- `base::rowsum.data.frame`
- `base::rowsum.default`
- `base::sQuote`
- `base::save.image`
- `base::scale`
- `base::scale.default`
- `base::seek.connection`
- `base::seq.default`
- `base::seq.int`
- `base::sequence`
- `base::sequence.default`
- `base::serverSocket`
- `base::set.seed`
- `base::setHook`
- `base::setNamespaceInfo`
- `base::setSessionTimeLimit`
- `base::setTimeLimit`
- `base::setequal`
- `base::setwd`
- `base::shQuote`
- `base::showConnections`
- `base::signalCondition`
- `base::signif`
- `base::simpleCondition`
- `base::simpleError`
- `base::simpleMessage`
- `base::simpleWarning`
- `base::simplify2array`
- `base::single`
- `base::sinpi`
- `base::slice.index`
- `base::socketAccept`
- `base::socketTimeout`
- `base::solve`
- `base::solve.default`
- `base::solve.qr`
- `base::sort.POSIXlt`
- `base::sort.default`
- `base::sort.int`
- `base::sort.list`
- `base::sort_by`
- `base::sort_by.data.frame`
- `base::sort_by.default`
- `base::split.Date`
- `base::split.POSIXct`
- `base::split.data.frame`
- `base::split.default`
- `base::srcfile`
- `base::srcfilealias`
- `base::srcfilecopy`
- `base::srcref`
- `base::standardGeneric`
- `base::stop`
- `base::stopifnot`
- `base::storage.mode`
- `base::str2expression`
- `base::str2lang`
- `base::strftime`
- `base::strrep`
- `base::strtoi`
- `base::strtrim`
- `base::structure`
- `base::strwrap`
- `base::subset`
- `base::subset.data.frame`
- `base::subset.default`
- `base::subset.matrix`
- `base::substitute`
- `base::substring`
- `base::suppressMessages`
- `base::suppressPackageStartupMessages`
- `base::suppressWarnings`
- `base::suspendInterrupts`
- `base::svd`
- `base::sweep`
- `base::switch`
- `base::sys.load.image`
- `base::sys.on.exit`
- `base::sys.save.image`
- `base::t.data.frame`
- `base::t.default`
- `base::tabulate`
- `base::tanpi`
- `base::taskCallbackManager`
- `base::toString`
- `base::toString.default`
- `base::trace`
- `base::traceback`
- `base::tracemem`
- `base::tracingState`
- `base::transform.data.frame`
- `base::transform.default`
- `base::trigamma`
- `base::trunc.Date`
- `base::trunc.POSIXt`
- `base::truncate`
- `base::try`
- `base::tryCatch`
- `base::tryInvokeRestart`
- `base::typeof`
- `base::unCfillPOSIXlt`
- `base::unclass`
- `base::undebug`
- `base::unique.POSIXlt`
- `base::unique.array`
- `base::unique.data.frame`
- `base::unique.default`
- `base::unique.matrix`
- `base::unique.numeric_version`
- `base::unique.warnings`
- `base::units`
- `base::units.difftime`
- `base::unix.time`
- `base::unlist`
- `base::unloadNamespace`
- `base::unlockBinding`
- `base::untrace`
- `base::untracemem`
- `base::unz`
- `base::upper.tri`
- `base::use`
- `base::utf8ToInt`
- `base::validEnc`
- `base::validUTF8`
- `base::version`
- `base::warnings`
- `base::weekdays.Date`
- `base::weekdays.POSIXt`
- `base::while`
- `base::with`
- `base::with.default`
- `base::withAutoprint`
- `base::withCallingHandlers`
- `base::withRestarts`
- `base::withVisible`
- `base::within.data.frame`
- `base::within.list`
- `base::write`
- `base::write.dcf`
- `base::xor`
- `base::xpdrows.data.frame`
- `base::xtfrm`
- `base::xtfrm.AsIs`
- `base::xtfrm.Date`
- `base::xtfrm.POSIXct`
- `base::xtfrm.POSIXlt`
- `base::xtfrm.data.frame`
- `base::xtfrm.default`
- `base::xtfrm.difftime`
- `base::xtfrm.factor`
- `base::xtfrm.numeric_version`
- `base::xzfile`
- `base::zapsmall`
- `base::zstdfile`
`base::data.frame` also preserves a shared symbolic row-count when RR can prove it from the input columns.
`base::globalenv` and `base::environment` stay on the direct surface as opaque environment objects.
`base::unlink` -> scalar int
`base::file.path`, `base::basename`, `base::dirname`, `base::normalizePath` -> char-like output following the path input shape
`base::dir.exists`, `base::file.exists` -> logical-like output following the path input shape
`base::eval`, `base::evalq`, `base::do.call`, `base::parse`, `base::getOption`, `base::file`, `base::readRDS`, `base::get0` -> opaque/object-like result
`base::save` -> null
`base::list.files`, `base::path.expand` -> char-like output following the path input shape
`base::getNamespace`, `base::asNamespace` -> opaque environment object
`base::isNamespace`, `base::is.name` -> scalar logical
`base::find.package` -> vector char
`base::package_version` -> list-like version object
`base::getElement`, `base::unname` -> preserve the first argument's broad shape
`base::baseenv`, `base::emptyenv`, `base::new.env`, `base::parent.env`, `base::as.environment`, `base::list2env`, `base::topenv` -> opaque environment object
`base::is.environment`, `base::environmentIsLocked`, `base::isNamespaceLoaded`, `base::requireNamespace` -> scalar logical
`base::environmentName`, `base::getNamespaceName`, `base::getNamespaceVersion` -> scalar char
`base::loadedNamespaces`, `base::getNamespaceExports`, `base::getNamespaceUsers` -> vector char
`base::as.list.environment`, `base::getNamespaceImports` -> list-like opaque object
`base::library`, `base::searchpaths` -> vector char
`base::require`, `base::packageHasNamespace`, `base::is.loaded` -> scalar logical
`base::loadNamespace`, `base::getLoadedDLLs`, `base::dyn.load` -> opaque/object-like result
`base::dyn.unload` -> null
`base::readLines`, `base::Sys.getenv`, `base::Sys.which`, `base::Sys.readlink`, `base::Sys.info`, `base::Sys.glob` -> vector char
`base::writeLines`, `base::writeChar`, `base::writeBin`, `base::flush`, `base::truncate.connection` -> null
`base::seek` -> scalar double
`base::Sys.setenv`, `base::Sys.unsetenv` -> scalar logical
`base::Sys.getpid` -> scalar int
`base::Sys.time`, `base::Sys.Date` -> scalar double
`base::Sys.getlocale` -> scalar char
`base::system`, `base::system2` -> opaque/object-like result
`base::system.time` -> vector double
`base::Sys.sleep` -> null
`base::Sys.setlocale`, `base::Sys.timezone` -> scalar char
`base::Sys.localeconv` -> vector char
`base::Sys.setFileTime`, `base::Sys.chmod` -> logical-like output following the path input shape
`base::Sys.umask` -> scalar int
`base::sys.parent`, `base::sys.nframe` -> scalar int
`base::sys.parents` -> vector int
`base::search`, `base::gettext`, `base::gettextf`, `base::ngettext` -> vector char
`base::geterrmessage` -> scalar char
`base::message`, `base::packageStartupMessage`, `base::.packageStartupMessage` -> null
`base::sys.call`, `base::sys.calls`, `base::sys.function`, `base::sys.frame`, `base::sys.frames`, `base::sys.status`, `base::sys.source`, `base::source`, `base::options`, `base::warning`, `base::warningCondition`, `base::packageNotFoundError`, `base::packageEvent` -> opaque/object-like result
`base::stdin`, `base::stdout`, `base::stderr`, `base::textConnection`, `base::rawConnection`, `base::socketConnection`, `base::url`, `base::pipe`, `base::open`, `base::summary.connection` -> opaque/object-like result
`base::textConnectionValue` -> vector char
`base::rawConnectionValue` -> broad vector-like opaque result
`base::close`, `base::closeAllConnections`, `base::pushBack`, `base::clearPushBack` -> null
`base::isOpen`, `base::isIncomplete` -> scalar logical
`base::pushBackLength` -> scalar int
`base::socketSelect` -> vector logical
`base::scan` -> broad vector-like opaque result
`base::read.table`, `base::read.csv`, `base::read.csv2`, `base::read.delim`, `base::read.delim2` -> dataframe-like table
`base::write.table`, `base::write.csv`, `base::write.csv2`, `base::saveRDS`, `base::dput`, `base::dump`, `base::sink` -> null
`base::count.fields` -> vector int
`base::sink.number` -> scalar int
`base::capture.output` -> vector char
`base::lapply`, `base::Map`, `base::split`, `base::by` -> list-like opaque result
`base::sapply`, `base::vapply`, `base::mapply`, `base::tapply`, `base::apply` -> broad vector-like opaque result
`base::Reduce`, `base::Find` -> opaque/object-like result
`base::Filter`, `base::unsplit`, `base::within`, `base::transform` -> preserve the first argument's broad shape
`base::Position` -> scalar int
`base::expand.grid`, `base::merge` -> dataframe-like table
`base::as.Date*`, `base::as.POSIXct*`, `base::as.POSIXlt*`, `base::as.difftime`, `base::as.double.difftime`, `base::as.double.POSIXlt`, `base::strptime`, `base::difftime`, `base::julian` -> double-like output following the first argument's broad shape
`base::alist`, `base::as.expression`, `base::as.expression.default`, `base::as.package_version`, `base::as.pairlist` -> list-like opaque result
`base::as.call`, `base::as.function`, `base::as.function.default`, `base::as.name`, `base::as.symbol` -> opaque/object-like result
`base::Arg` -> double-like output following the first argument's broad shape
`base::as.complex` -> broad vector-like opaque result
`base::as.hexmode`, `base::as.octmode`, `base::as.ordered`, `base::gl` -> int-like output following the first argument's broad shape
`base::as.numeric_version`, `base::asplit` -> list-like opaque result
`base::as.qr`, `base::asS3`, `base::asS4` -> opaque/object-like result
`base::as.null`, `base::as.null.default` -> null
`base::as.raw` -> broad vector-like opaque result
`base::as.single`, `base::as.single.default` -> double-like output following the first argument's broad shape
`base::as.table`, `base::as.table.default` -> matrix-like output following the first argument's broad shape
`base::abbreviate`, `base::as.character` -> char-like output following the first argument's broad shape
`base::as.character.condition`, `base::as.character.default`, `base::as.character.error`, `base::as.character.factor`, `base::as.character.hexmode`, `base::as.character.numeric_version`, `base::as.character.octmode`, `base::as.character.srcref` -> char-like output following the first argument's broad shape
`base::all.names`, `base::all.vars` -> vector char
`base::anyDuplicated.array`, `base::anyDuplicated.data.frame`, `base::anyDuplicated.default`, `base::anyDuplicated.matrix` -> scalar int
`base::anyNA`, `base::anyNA.data.frame` -> scalar logical
`base::anyNA.numeric_version`, `base::anyNA.POSIXlt` -> scalar logical
`base::addTaskCallback` -> scalar int
`base::backsolve` -> double-like output following the second argument's broad shape
`base::balancePOSIXlt` -> list-like opaque result
`base::besselI`, `base::besselJ`, `base::besselK`, `base::besselY`, `base::beta`, `base::choose` -> double-like output following the first argument's broad shape
`base::bitwAnd`, `base::bitwNot`, `base::bitwOr`, `base::bitwShiftL`, `base::bitwShiftR`, `base::bitwXor` -> int-like output following the first argument's broad shape
`base::by.data.frame`, `base::by.default`, `base::computeRestarts`, `base::c.numeric_version`, `base::c.POSIXlt`, `base::c.warnings` -> list-like opaque result
`base::c.Date`, `base::c.difftime`, `base::c.POSIXct` -> vector double
`base::c.factor` -> vector int
`base::c.noquote` -> vector char
`base::callCC`, `base::comment`, `base::conditionCall`, `base::conditionCall.condition`, `base::conflictRules` -> opaque/object-like result
`base::cbind.data.frame` -> dataframe-like table
`base::close.connection`, `base::close.srcfile`, `base::close.srcfilealias` -> null
`base::conditionMessage`, `base::conditionMessage.condition`, `base::conflicts`, `base::curlGetHeaders` -> vector char
`base::contributors` -> null
`base::Cstack_info` -> named int object
`base::casefold`, `base::char.expand` -> char-like output following the first argument's broad shape
`base::charmatch` -> int-like output following the first argument's broad shape
`base::charToRaw` -> broad vector-like opaque result
`base::chkDots` -> null
`base::chol`, `base::chol.default`, `base::chol2inv` -> matrix double
`base::chooseOpsMethod`, `base::chooseOpsMethod.default` -> scalar logical
`base::Conj` -> preserve the first argument's broad shape
`base::complete.cases` -> logical-like output following the first argument's broad shape
`base::complex` -> broad vector-like opaque result
`base::cummax`, `base::cummin`, `base::cumsum` -> vectorized first-argument output
`base::cumprod` -> double-like output following the first argument's broad shape
`base::cut.Date`, `base::cut.POSIXt`, `base::cut.default` -> int-like output following the first argument's broad shape
`base::diff`, `base::diff.default` -> preserve the first argument's broad shape
`base::diff.Date`, `base::diff.POSIXt`, `base::diff.difftime` -> double-like output following the first argument's broad shape
`base::commandArgs`, `base::data.class`, `base::deparse`, `base::extSoftVersion` -> vector char
`base::data.matrix` -> matrix double
`base::det` -> scalar double
`base::determinant`, `base::determinant.matrix` -> named object with modulus/sign
`base::debuggingState` -> scalar logical
`base::dget` -> opaque/object-like result
`base::date`, `base::deparse1`, `base::file.choose` -> scalar char
`base::dQuote`, `base::enc2native`, `base::enc2utf8`, `base::encodeString`, `base::Encoding` -> char-like output following the first argument's broad shape
`base::dontCheck` -> preserve the first argument's broad shape
`base::digamma`, `base::expm1`, `base::factorial`, `base::acosh`, `base::asinh`, `base::atanh`, `base::cospi` -> double-like output following the first argument's broad shape
`base::debug`, `base::debugonce`, `base::declare`, `base::delayedAssign`, `base::detach`, `base::enquote`, `base::env.profile`, `base::environment<-`, `base::errorCondition`, `base::eval.parent`, `base::Exec`, `base::expression` -> opaque/object-like result
`base::eigen` -> named object with values/vectors
`base::exists` -> scalar logical
`base::findInterval` -> int-like output following the first argument's broad shape
`base::file.show` -> null
`base::format.data.frame`, `base::format.info` -> dataframe-like table of chars
`base::format`, `base::format.AsIs`, `base::format.default`, `base::format.difftime`, `base::format.factor`, `base::format.hexmode`, `base::format.libraryIQR`, `base::format.numeric_version`, `base::format.octmode`, `base::format.packageInfo`, `base::format.pval`, `base::format.summaryDefault`, `base::formatC`, `base::formatDL` -> vector char
`base::print.*` -> preserve the first argument's broad shape
`base::summary.*` -> opaque/object-like result
`base::drop` -> opaque/object-like result
`base::droplevels`, `base::droplevels.data.frame` -> preserve the first argument's broad shape
`base::droplevels.factor` -> int-like output following the first argument's broad shape
`base::duplicated.default` -> logical-like output following the first argument's broad shape
`base::duplicated.array`, `base::duplicated.data.frame`, `base::duplicated.matrix`, `base::duplicated.numeric_version`, `base::duplicated.POSIXlt`, `base::duplicated.warnings` -> logical-like output following the first argument's broad shape
`base::identical`, `base::inherits`, `base::interactive`, `base::is.R`, `base::is.array`, `base::is.atomic`, `base::is.call`, `base::is.character`, `base::is.complex`, `base::is.data.frame`, `base::is.double`, `base::is.expression`, `base::is.factor`, `base::is.function`, `base::is.integer`, `base::is.language`, `base::is.list`, `base::is.logical`, `base::is.null`, `base::is.numeric`, `base::is.numeric.Date`, `base::is.numeric.POSIXt`, `base::is.numeric.difftime`, `base::is.numeric_version`, `base::is.object`, `base::is.ordered`, `base::is.package_version`, `base::is.pairlist`, `base::is.primitive`, `base::is.qr`, `base::is.raw`, `base::is.recursive`, `base::is.single`, `base::is.symbol`, `base::is.table`, `base::is.unsorted`, `base::is.vector` -> scalar logical
`base::is.element`, `base::is.finite.POSIXlt`, `base::is.infinite`, `base::is.infinite.POSIXlt`, `base::is.na.POSIXlt`, `base::is.na.data.frame`, `base::is.na.numeric_version`, `base::is.nan`, `base::is.nan.POSIXlt` -> logical-like output following the first argument's broad shape
`base::attr<-`, `base::attributes<-`, `base::class<-`, `base::colnames<-`, `base::comment<-`, `base::dimnames<-`, `base::levels<-`, `base::names<-`, `base::row.names<-`, `base::rownames<-` -> preserve the first argument's broad shape
`base::body<-` -> opaque/object-like result
`base::bindingIsActive`, `base::bindingIsLocked` -> scalar logical
`base::bindtextdomain` -> scalar char
`base::browserText` -> scalar char
`base::builtins` -> vector char
`base::capabilities` -> vector logical
`base::all.equal`, `base::all.equal.default`, `base::all.equal.character`, `base::all.equal.environment`, `base::all.equal.envRefClass`, `base::all.equal.factor`, `base::all.equal.formula`, `base::all.equal.function`, `base::all.equal.language`, `base::all.equal.list`, `base::all.equal.numeric`, `base::all.equal.POSIXt`, `base::all.equal.raw`, `base::activeBindingFunction`, `base::allowInterrupts`, `base::args`, `base::attach`, `base::attachNamespace`, `base::autoload`, `base::autoloader`, `base::body`, `base::break`, `base::call`, `base::bquote`, `base::browser`, `base::browserCondition`, `base::browserSetDebug` -> opaque/object-like result
`base::array`, `base::as.array`, `base::as.array.default`, `base::as.matrix`, `base::as.matrix.data.frame`, `base::as.matrix.default`, `base::as.matrix.noquote`, `base::as.matrix.POSIXlt`, `base::aperm` -> matrix-like output following the first argument's broad shape
`base::as.data.frame`, `base::as.data.frame.array`, `base::as.data.frame.AsIs`, `base::as.data.frame.character`, `base::as.data.frame.complex`, `base::as.data.frame.data.frame`, `base::as.data.frame.Date`, `base::as.data.frame.default`, `base::as.data.frame.difftime`, `base::as.data.frame.factor`, `base::as.data.frame.integer`, `base::as.data.frame.list`, `base::as.data.frame.logical`, `base::as.data.frame.matrix`, `base::as.data.frame.model.matrix`, `base::as.data.frame.noquote`, `base::as.data.frame.numeric`, `base::as.data.frame.numeric_version`, `base::as.data.frame.ordered`, `base::as.data.frame.POSIXct`, `base::as.data.frame.POSIXlt`, `base::as.data.frame.raw`, `base::as.data.frame.table`, `base::as.data.frame.ts`, `base::as.data.frame.vector`, `base::array2DF` -> dataframe-like table
`base::arrayInd`, `base::col`, `base::row` -> matrix int
`base::append`, `base::addNA` -> preserve the first argument's broad shape
`base::as.list`, `base::as.list.data.frame`, `base::as.list.Date`, `base::as.list.default`, `base::as.list.difftime`, `base::as.list.factor`, `base::as.list.function`, `base::as.list.numeric_version`, `base::as.list.POSIXct`, `base::as.list.POSIXlt` -> list-like opaque result
`base::as.double`, `base::as.numeric` -> double-like output following the first argument's broad shape
`base::as.factor`, `base::as.integer`, `base::ordered` -> int-like output following the first argument's broad shape
`base::as.logical`, `base::as.logical.factor` -> logical-like output following the first argument's broad shape
`base::as.vector`, `base::as.vector.data.frame`, `base::as.vector.factor`, `base::as.vector.POSIXlt` -> vectorized first-argument output
`base::colMeans`, `base::rowMeans` -> vector double
`base::class`, `base::levels` -> vector char
`base::attr` -> opaque/object-like result
`base::attributes` -> list-like opaque result
`base::as.character.Date`, `base::as.character.POSIXt`, `base::format.Date`, `base::format.POSIXct`, `base::format.POSIXlt`, `base::months`, `base::quarters`, `base::weekdays` -> char-like output following the first argument's broad shape
`base::OlsonNames` -> vector char
`base::ISOdate`, `base::ISOdatetime`, `base::seq.Date`, `base::seq.POSIXt` -> vector double
`base::readBin`, `base::serialize` -> broad vector-like opaque result
`base::readChar`, `base::load` -> vector char
`base::unserialize`, `base::fifo`, `base::gzcon` -> opaque/object-like result
`base::getwd`, `base::tempdir`, `base::tempfile`, `base::system.file` -> scalar char
`base::dir`, `base::list.dirs`, `base::path.package`, `base::.packages` -> vector char
`base::dir.create`, `base::file.create`, `base::file.remove`, `base::file.rename`, `base::file.copy`, `base::file.append`, `base::file.link`, `base::file.symlink` -> logical-like output following the path input shape
`base::file.access`, `base::file.mode` -> vector int
`base::file.info` -> dataframe-like table
`base::file.size`, `base::file.mtime` -> vector double
Core namespaced helpers such as `base::length`, `base::c`, `base::list`, `base::sum`, `base::mean`, `base::vector`, `base::seq`, `base::ifelse`, `base::abs`, `base::min`, `base::max`, `base::pmax`, `base::pmin`, `base::sqrt`, `base::log`, `base::log10`, `base::log2`, `base::exp`, `base::atan2`, `base::sin`, `base::cos`, `base::tan`, `base::asin`, `base::acos`, `base::atan`, `base::sinh`, `base::cosh`, `base::tanh`, `base::sign`, `base::gamma`, `base::lgamma`, `base::floor`, `base::ceiling`, `base::trunc`, `base::round`, `base::is.na`, `base::is.finite`, `base::print`, `base::numeric`, `base::matrix`, `base::dim`, `base::dimnames`, `base::nrow`, `base::ncol`, `base::seq_len`, `base::seq_along`, `base::diag`, `base::t`, `base::rbind`, `base::cbind`, `base::rowSums`, `base::colSums`, `base::crossprod`, `base::tcrossprod`, `base::character`, `base::logical`, `base::integer`, `base::double`, `base::rep`, `base::rep.int`, `base::any`, `base::all`, `base::which`, `base::prod`, `base::paste`, `base::paste0`, `base::sprintf`, `base::cat`, `base::tolower`, `base::toupper`, `base::substr`, `base::sub`, `base::gsub`, `base::nchar`, `base::nzchar`, `base::grepl`, `base::grep`, `base::startsWith`, `base::endsWith`, `base::which.min`, `base::which.max`, `base::isTRUE`, `base::isFALSE`, `base::lengths`, `base::union`, `base::intersect`, `base::setdiff`, `base::sample`, `base::sample.int`, `base::rank`, `base::factor`, `base::cut`, `base::table`, `base::trimws`, `base::chartr`, `base::strsplit`, `base::regexpr`, `base::gregexpr`, `base::regexec`, `base::agrep`, and `base::agrepl` now reuse the same typed builtin understanding RR already had for their unqualified forms.
