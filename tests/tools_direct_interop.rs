mod common;

use common::{compile_rr, normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::PathBuf;

#[test]
fn tools_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping tools direct interop runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("tools_direct_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let a_txt = out_dir.join("a.txt");
    let b_csv = out_dir.join("b.csv");
    let c_r = out_dir.join("c.R");
    let sample_rd = out_dir.join("sample.Rd");
    let sample_index = out_dir.join("00Index");
    let non_ascii_txt = out_dir.join("non_ascii.txt");
    let sample_html = out_dir.join("sample.html");

    let src = r#"
import r * as base from "base"
import r default from "tools"
import r * as utils from "utils"

fn use_tools() -> char {
  let titled = tools.toTitleCase("hello world")
  let abs = tools.file_path_as_absolute("__A_TXT__")
  let user_dir = tools.R_user_dir("RR", "cache")
  let hash = tools.md5sum(abs)
  let tools_exports = base.getNamespaceExports("tools")
  let has_sha256 = base.any(tools_exports == "sha256sum")
  let has_parse_uri_reference = base.any(tools_exports == "parse_URI_reference")
  let ext = tools.file_ext(c("a.txt", "b.csv"))
  let sans = tools.file_path_sans_ext(c("a.txt", "b.csv"))
  let listed_exts = tools.list_files_with_exts("__ROOT__", c("txt", "csv"))
  let listed_type = tools.list_files_with_type("__ROOT__", "code")
  let depends = tools.dependsOnPkgs("stats")
  let vignette_info = tools.getVignetteInfo("stats")
  let pkg_vigs = tools.pkgVignettes(package = "stats")
  let match_pos = tools.delimMatch("a[b[c]d]e", c("[", "]"))
  let parsed_rd = tools.parse_Rd("__SAMPLE_RD__")
  let rd_text = tools.Rd2txt(parsed_rd)
  let rd_html = utils.capture.output(tools.Rd2HTML(parsed_rd))
  let rd_latex = utils.capture.output(tools.Rd2latex(parsed_rd))
  let rd_ex = utils.capture.output(tools.Rd2ex(parsed_rd))
  let rd_index = tools.Rdindex("__SAMPLE_RD__")
  let checked_rd = tools.checkRd("__SAMPLE_RD__")
  let rd_filtered = tools.RdTextFilter("__SAMPLE_RD__")
  let rd_opts = tools.Rd2txt_options()
  let rd_width = rd_opts.width
  let encoded = tools.encoded_text_to_latex("cafe", "UTF-8")
  let parsed_latex = tools.parseLatex("alpha_beta")
  let bibstyle = tools.getBibstyle()
  let deparsed_latex = tools.deparseLatex(parsed_latex)
  let parsed_utf8 = tools.parseLatex("caf\\'e")
  let utf8_latex = tools.latexToUtf8(parsed_utf8)
  let gs_cmd = tools.find_gs_cmd()
  let mv_site = tools.makevars_site()
  let mv_user = tools.makevars_user()
  let html_header = tools.HTMLheader("RR")
  let html_links = tools.findHTMLlinks("__SAMPLE_HTML__")
  let vignette_engine = tools.vignetteEngine("utils::Sweave")
  let nonascii = tools.showNonASCII("__NON_ASCII__")
  let nonascii_file = tools.showNonASCIIfile("__NON_ASCII__")
  let tools_ns = base.asNamespace("tools")
  let has_standard_package_names = base.exists("standard_package_names", envir = tools_ns, inherits = false)
  let has_base_aliases_db = base.exists("base_aliases_db", envir = tools_ns, inherits = false)
  let has_base_rdxrefs_db = base.exists("base_rdxrefs_db", envir = tools_ns, inherits = false)
  let installed = utils.installed.packages()
  let deps = tools.package_dependencies("stats", db = installed, recursive = false)
  let rd = tools.Rd_db("stats")
  let vig_names = pkg_vigs.names
  let vig_dir = pkg_vigs.dir
  print(hash)
  if (has_sha256) {
    print(tools.sha256sum(c("__A_TXT__", "__B_CSV__")))
  } else {
    print(base.character(0L))
  }
  print(user_dir)
  print(ext)
  print(sans)
  print(length(listed_exts))
  print(length(listed_type))
  print(length(depends))
  print(dim(vignette_info))
  print(length(pkg_vigs))
  print(match_pos)
  if (has_parse_uri_reference) {
    print(dim(tools.parse_URI_reference("https://example.com/path?a=1#frag")))
  } else {
    print(base.c(0L, 0L))
  }
  print(length(parsed_rd))
  print(length(rd_text))
  print(length(rd_html))
  print(length(rd_latex))
  print(length(rd_ex))
  print(length(rd_index))
  print(length(checked_rd))
  print(length(rd_filtered))
  print(length(rd_opts))
  print(rd_width)
  print(length(encoded))
  print(length(parsed_latex))
  print(length(bibstyle))
  print(length(deparsed_latex))
  print(length(utf8_latex))
  print(length(gs_cmd))
  print(length(mv_site))
  print(length(mv_user))
  print(length(html_header))
  print(length(html_links))
  print(length(vignette_engine))
  print(length(nonascii))
  print(length(nonascii_file))
  if (has_standard_package_names) { print(length(tools.standard_package_names())) } else { print(0L) }
  if (has_base_aliases_db) { print(length(tools.base_aliases_db())) } else { print(0L) }
  if (has_base_rdxrefs_db) { print(length(tools.base_rdxrefs_db())) } else { print(0L) }
  print(0L)
  print(0L)
  print(0L)
  print(base.c(0L, 0L))
  print(base.c(0L, 0L))
  print(base.c(0L, 0L))
  print(base.c(0L, 0L))
  print(base.c(0L, 0L))
  print(dim(installed))
  print(length(deps))
  print(length(rd))
  print(length(vig_names))
  print(length(vig_dir))
  return titled
}

print(use_tools())
"#
    .replace("__ROOT__", &out_dir.to_string_lossy())
    .replace("__A_TXT__", &a_txt.to_string_lossy())
    .replace("__B_CSV__", &b_csv.to_string_lossy())
    .replace("__SAMPLE_RD__", &sample_rd.to_string_lossy())
    .replace("__SAMPLE_INDEX__", &sample_index.to_string_lossy())
    .replace("__SAMPLE_HTML__", &sample_html.to_string_lossy())
    .replace("__NON_ASCII__", &non_ascii_txt.to_string_lossy());

    let rr_path = out_dir.join("tools_direct_interop.rr");
    let o0 = out_dir.join("tools_direct_interop_o0.R");
    let o2 = out_dir.join("tools_direct_interop_o2.R");

    fs::write(&a_txt, "alpha\n").expect("failed to write txt sample");
    fs::write(&b_csv, "beta\n").expect("failed to write csv sample");
    fs::write(&c_r, "f <- function() 1\n").expect("failed to write code sample");
    fs::write(
        &sample_rd,
        "\\name{rrtmp}\n\\title{RR Tmp}\n\\description{Example.}\n",
    )
    .expect("failed to write Rd sample");
    fs::write(
        &sample_index,
        "rrtmp\tExample title\nrrtmp2\tAnother title\n",
    )
    .expect("failed to write 00Index sample");
    fs::write(
        &sample_html,
        "<html><body><a href=\"https://example.com\">x</a></body></html>\n",
    )
    .expect("failed to write html sample");
    fs::write(&non_ascii_txt, "ascii\ncafé\n").expect("failed to write non-ascii sample");
    fs::write(&rr_path, src).expect("failed to write source");
    compile_rr(&rr_bin, &rr_path, &o0, "-O0");
    compile_rr(&rr_bin, &rr_path, &o2, "-O2");

    let run_o0 = run_rscript(&rscript, &o0);
    let run_o2 = run_rscript(&rscript, &o2);

    assert_eq!(run_o0.status, 0, "O0 runtime failed:\n{}", run_o0.stderr);
    assert_eq!(run_o2.status, 0, "O2 runtime failed:\n{}", run_o2.stderr);
    assert_eq!(
        normalize(&run_o0.stdout),
        normalize(&run_o2.stdout),
        "stdout mismatch O0 vs O2"
    );
    assert_eq!(
        normalize(&run_o0.stderr),
        normalize(&run_o2.stderr),
        "stderr mismatch O0 vs O2"
    );
}
