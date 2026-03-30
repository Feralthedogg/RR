mod common;

use common::{normalize, rscript_available, rscript_path};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn utils_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping utils direct interop runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("utils_direct_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r * as utils from "utils"
import r * as base from "base"
import r default from "stats"

fn roundtrip() -> int {
  let srcfile = "utils_direct_input.csv"
  let srcfile2 = "sample_sc.csv"
  let fwffile = "sample_fwf.txt"
  let outfile = "utils_direct_output.csv"
  let outfile2 = "utils_direct_output_sc.csv"
  let tablefile = "utils_direct_table.tsv"
  let df = base.data.frame(
    x = c(1.0, 2.0, 3.0),
    y = c("a", "b", "c")
  )
  utils.write.csv(df, srcfile)
  let loaded = utils.read.csv(srcfile)
  let loaded2 = utils.read.csv2(srcfile2)
  let table_loaded = utils.read.table("sample.csv", sep = ",", header = true)
  let delim_loaded = utils.read.delim("sample.tsv", sep = ";")
  let fwf_loaded = utils.read.fwf(fwffile, widths = c(3, 2, 2), header = false)
  print(utils.head(loaded.x, 2))
  print(utils.tail(loaded.y, 2))
  utils.str(loaded)
  let version = utils.packageVersion("stats")
  let maint = utils.maintainer("stats")
  let built = utils.packageDate("stats")
  let obj_size = utils.object.size(c(1.0, 2.0, 3.0))
  let mem_size = utils.memory.size()
  let mem_limit = utils.memory.limit()
  let cmp = utils.compareVersion("1.2.0", "1.1.9")
  let captured = utils.capture.output(print(c(1.0, 2.0, 3.0)))
  let desc = utils.packageDescription("stats")
  let session = utils.sessionInfo()
  let desc_pkg = desc.Package
  let session_pkgs = session.basePkgs
  let cite = utils.citation("stats")
  let who = utils.person("A", "B")
  let who_one = utils.as.person("Jane Doe <jane@example.com> [aut]")
  let who_many = utils.as.personList(c("Jane Doe <jane@example.com> [aut]", "John Roe <john@example.com> [ctb]"))
  let roman = utils.as.roman(c(12, 14))
  let has_x = utils.hasName(list(x = 1.0, y = 2.0), "x")
  let proto = base.data.frame(a = integer(), b = character())
  let captured_df = utils.strcapture("([0-9]+)-([A-Za-z]+)", c("1-one", "2-two"), proto)
  let hits = utils.apropos("mean")
  let found = utils.find("mean")
  let matches = utils.findMatches("me", c("mean", "median", "mode"))
  let methods_list = utils.methods("mean")
  let help_hits = utils.help.search("lm")
  let data_iqr = utils.data()
  let anywhere = utils.getAnywhere("mean")
  let anywhere_where = anywhere.where
  let help_matches = help_hits.matches
  let data_results = data_iqr.results
  let arglist = utils.argsAnywhere("mean")
  let contrib = utils.contrib.url("https://cloud.r-project.org")
  let charset = utils.localeToCharset()
  let cls = utils.charClass("abc123", "alpha")
  let snap = utils.fileSnapshot("sample.csv")
  let encoded = utils.URLencode("a b/c")
  let decoded = utils.URLdecode("a%20b%2Fc")
  let mat = base.matrix(c(1L, 2L, 3L, 4L, 5L, 6L), nrow = 2L)
  let hmat = utils.head.matrix(mat, 1L)
  let tmat = utils.tail.matrix(mat, 1L)
  let stacked = utils.stack(list(a = c(1.0, 2.0), b = c(3.0, 4.0)))
  let unstacked = utils.unstack(base.data.frame(values = c(1.0, 2.0, 3.0, 4.0), ind = base.factor(c("a", "a", "b", "b"))))
  let strops = utils.strOptions()
  let bib = utils.toBibtex(utils.citation())
  let bibentry_one = utils.bibentry(bibtype = "Manual", title = "T", author = utils.person("A", "B"))
  let citentry_one = utils.citEntry(entry = "Manual", title = "T", author = "A")
  let citheader_one = utils.citHeader("Header")
  let citfooter_one = utils.citFooter("Footer")
  let pkg_name = utils.packageName(base.environment(stats.lm))
  let osv = utils.osVersion
  let host = utils.nsl("localhost")
  let modified = utils.modifyList(list(a = 1.0, b = 2.0), list(b = 3.0, c = 4.0))
  let relisted = utils.relist(c(1.0, 2.0, 3.0, 4.0), skeleton = list(a = c(1.0, 2.0), b = c(3.0, 4.0)))
  let relistable = utils.as.relistable(list(a = c(1.0, 2.0), b = c(3.0, 4.0)))
  let is_relistable = utils.is.relistable(relistable)
  let plist = utils.personList(utils.person("A", "B"), utils.person("C", "D"))
  let warns = utils.warnErrList(list(a = "oops", b = "bad"))
  let citation_file = utils.readCitationFile("stats_citation.bib")
  let h = utils.hashtab()
  let seth = utils.sethash(h, "a", c(1.0, 2.0, 3.0))
  let geth = utils.gethash(h, "a")
  let hash_type = utils.typhash(h)
  let hash_count = utils.numhash(h)
  let hash_ok = utils.is.hashtab(h)
  let removed = utils.remhash(h, "a")
  let mapped = utils.maphash(h, function(k, v) { return(k) })
  let cleared = utils.clrhash(h)
  let built_date = utils.asDateBuilt("R 4.5.0; ; 2025-01-01; unix")
  let zip_clean = base.unlink("archive.zip")
  let glob = utils.glob2rx("*.csv")
  let mirrors = utils.getCRANmirrors(true)
  let mirror = utils.findCRANmirror(type = "web")
  let zip_status = utils.zip("archive.zip", files = c("sample.csv"))
  let unzip_listing = utils.unzip("archive.zip", list = true)
  let tar_status = utils.tar("archive.tar", files = c("sample.csv"), compression = "none", tar = "internal")
  let untar_listing = utils.untar("archive.tar", list = true, tar = "internal")
  let repos = utils.setRepositories(ind = c(1L, 2L))
  let labels = utils.limitedLabels(c(1.0, 2.0, 3.0, 4.0))
  let ordered = utils.formatOL(c("a", "b"))
  let unordered = utils.formatUL(c("a", "b"))
  let vig = utils.vignette(package = "stats")
  let hdb = utils.hsearch_db()
  let hdb_concepts = utils.hsearch_db_concepts()
  let hdb_keywords = utils.hsearch_db_keywords()
  let ft_one = utils.file_test("-f", "sample.csv")
  let ft_many = utils.file_test("-f", c("sample.csv", "sample.tsv"))
  let combos = utils.combn(c(1.0, 2.0, 3.0), 2)
  let dists = utils.adist(c("cat", "dog"), c("cot", "dig"))
  let fields = utils.count.fields("sample.csv", sep = ",")
  let converted = utils.type.convert(c("1", "2"), "NA", true)
  let fortran_loaded = utils.read.fortran(fwffile, c("A3", "I2", "A2"))
  print(length(version))
  print(length(maint))
  print(length(built))
  print(length(obj_size))
  print(mem_size)
  print(mem_limit)
  print(cmp)
  print(length(captured))
  print(length(desc))
  print(length(session))
  print(length(desc_pkg))
  print(length(session_pkgs))
  print(length(cite))
  print(length(who))
  print(length(who_one))
  print(length(who_many))
  print(length(roman))
  print(has_x)
  print(dim(captured_df))
  print(length(hits))
  print(length(found))
  print(length(matches))
  print(length(methods_list))
  print(length(help_hits))
  print(length(data_iqr))
  print(length(anywhere))
  print(length(anywhere_where))
  print(dim(help_matches))
  print(dim(data_results))
  print(length(arglist))
  print(contrib)
  print(length(charset))
  print(length(cls))
  print(length(snap))
  print(encoded)
  print(decoded)
  print(dim(hmat))
  print(dim(tmat))
  print(dim(stacked))
  print(dim(unstacked))
  print(length(strops))
  print(length(bib))
  print(length(bibentry_one))
  print(length(citentry_one))
  print(length(citheader_one))
  print(length(citfooter_one))
  print(pkg_name)
  print(osv)
  print(host)
  print(length(modified))
  print(length(relisted))
  print(length(relistable))
  print(is_relistable)
  print(length(plist))
  print(length(warns))
  print(length(citation_file))
  print(seth)
  print(geth)
  print(hash_type)
  print(hash_count)
  print(hash_ok)
  print(removed)
  print(mapped)
  print(cleared)
  print(built_date)
  print(zip_clean)
  print(dim(mirrors))
  print(mirror)
  print(zip_status)
  print(dim(unzip_listing))
  print(tar_status)
  print(length(untar_listing))
  print(length(repos))
  print(length(labels))
  print(length(ordered))
  print(length(unordered))
  print(length(vig))
  print(length(hdb))
  print(dim(hdb_concepts))
  print(dim(hdb_keywords))
  print(glob)
  print(ft_one)
  print(length(ft_many))
  print(dim(loaded2))
  print(dim(table_loaded))
  print(dim(delim_loaded))
  print(dim(fwf_loaded))
  print(dim(combos))
  print(dim(dists))
  print(length(fields))
  print(length(converted))
  print(dim(fortran_loaded))
  utils.write.csv(loaded, outfile)
  utils.write.csv2(loaded2, outfile2)
  utils.write.table(loaded, tablefile, sep = "\t")
  return length(utils.head(loaded.x, 2))
}

print(roundtrip())
"#;

    let rr_path = out_dir.join("utils_direct_interop.rr");
    let o0 = out_dir.join("utils_direct_interop_o0.R");
    let o2 = out_dir.join("utils_direct_interop_o2.R");

    fs::write(out_dir.join("sample.csv"), "a,b,c\n1,2,3\n4,5,6\n")
        .expect("failed to write sample csv");
    fs::write(out_dir.join("sample_sc.csv"), "a;b;c\n1;2;3\n4;5;6\n")
        .expect("failed to write sample csv2");
    fs::write(out_dir.join("sample.tsv"), "x;y\n1;a\n2;b\n").expect("failed to write sample tsv");
    fs::write(out_dir.join("sample_fwf.txt"), "abc12xy\ndef34zz\n")
        .expect("failed to write sample fwf");
    fs::write(
        out_dir.join("stats_citation.bib"),
        "citHeader(\"RR Tmp\")\n\
citEntry(entry = \"Manual\", title = \"RR Tmp\", author = \"RR Team\", year = \"2025\")\n",
    )
    .expect("failed to write citation file");
    fs::write(&rr_path, src).expect("failed to write source");

    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&o0)
        .arg("-O0")
        .status()
        .expect("failed to compile O0");
    assert!(status.success(), "O0 compile failed");

    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&o2)
        .arg("-O2")
        .status()
        .expect("failed to compile O2");
    assert!(status.success(), "O2 compile failed");

    let run_o0 = Command::new(&rscript)
        .current_dir(&out_dir)
        .arg("--vanilla")
        .arg(&o0)
        .output()
        .expect("failed to execute O0 Rscript");
    let run_o2 = Command::new(&rscript)
        .current_dir(&out_dir)
        .arg("--vanilla")
        .arg(&o2)
        .output()
        .expect("failed to execute O2 Rscript");

    assert_eq!(
        run_o0.status.code().unwrap_or(-1),
        0,
        "O0 runtime failed:\n{}",
        String::from_utf8_lossy(&run_o0.stderr)
    );
    assert_eq!(
        run_o2.status.code().unwrap_or(-1),
        0,
        "O2 runtime failed:\n{}",
        String::from_utf8_lossy(&run_o2.stderr)
    );

    let stdout_o0 = normalize(&String::from_utf8_lossy(&run_o0.stdout));
    let stdout_o2 = normalize(&String::from_utf8_lossy(&run_o2.stdout));
    let stderr_o0 = normalize(&String::from_utf8_lossy(&run_o0.stderr));
    let stderr_o2 = normalize(&String::from_utf8_lossy(&run_o2.stderr));

    assert_eq!(stdout_o0, stdout_o2, "stdout mismatch O0 vs O2");
    assert_eq!(stderr_o0, stderr_o2, "stderr mismatch O0 vs O2");

    for name in [
        "utils_direct_input.csv",
        "utils_direct_output.csv",
        "utils_direct_output_sc.csv",
        "utils_direct_table.tsv",
    ] {
        let path = out_dir.join(name);
        let meta = fs::metadata(&path).expect("expected utils CSV output");
        assert!(meta.len() > 0, "expected non-empty utils CSV output");
    }
}
