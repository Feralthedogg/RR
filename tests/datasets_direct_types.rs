mod common;

use common::{compile_rr, normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::PathBuf;

#[test]
fn datasets_package_data_have_usable_types_in_strict_mode() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping datasets direct types runtime test: Rscript unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("datasets_direct_types");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r * as datasets from "datasets"
import r * as base from "base"
import r * as utils from "utils"
import r {
  state.x77 as state_x77,
  sunspot.year as sunspot_year,
  state.area as state_area,
  state.abb as state_abb,
  state.name as state_name,
  state.region as state_region,
  state.division as state_division,
  ability.cov as ability_cov,
  Harman23.cor as harman23_cor,
  Harman74.cor as harman74_cor,
  state.center as state_center,
  BJsales.lead as bj_sales_lead,
  euro.cross as euro_cross,
  stack.loss as stack_loss_series,
  sunspot.m2014 as sunspot_m2014,
  sunspot.month as sunspot_month,
  stack.x as stack_x,
  freeny.x as freeny_x,
  freeny.y as freeny_y
} from "datasets"

fn has_datasets_entry(name: char) -> bool {
  let items = c(utils.data(package = "datasets").results)
  return any(items == name) || any(base.startsWith(items, paste0(name, " (")))
}

fn mean_mpg() -> float {
  let cars = datasets.mtcars
  return mean(cars.mpg)
}

fn species_count() -> int {
  let flowers = datasets.iris
  return length(flowers.Species)
}

fn mean_wind() -> float {
  let aq = datasets.airquality
  return mean(aq.Wind)
}

fn supp_count() -> int {
  let tg = datasets.ToothGrowth
  return length(tg.supp)
}

fn mean_uptake() -> float {
  let co2 = datasets.CO2
  return mean(co2.uptake)
}

fn mean_murder() -> float {
  let arrests = datasets.USArrests
  return mean(arrests.Murder)
}

fn mean_speed() -> float {
  let cars = datasets.cars
  return mean(cars.speed)
}

fn mean_pressure() -> float {
  let p = datasets.pressure
  return mean(p.pressure)
}

fn mean_eruptions() -> float {
  let f = datasets.faithful
  return mean(f.eruptions)
}

fn mean_weight() -> float {
  let w = datasets.women
  return mean(w.weight)
}

fn mean_demand() -> float {
  let bod = datasets.BOD
  return mean(bod.demand)
}

fn mean_rating() -> float {
  let a = datasets.attitude
  return mean(a.rating)
}

fn group_count() -> int {
  let pg = datasets.PlantGrowth
  return length(pg.group)
}

fn mean_count() -> float {
  let ins = datasets.InsectSprays
  return mean(ins.count)
}

fn mean_extra() -> float {
  let s = datasets.sleep
  return mean(s.extra)
}

fn tree_count() -> int {
  let o = datasets.Orange
  return length(o.Tree)
}

fn mean_area() -> float {
  let r = datasets.rock
  return mean(r.area)
}

fn mean_girth() -> float {
  let t = datasets.trees
  return mean(t.Girth)
}

fn agegp_count() -> int {
  let e = datasets.esoph
  return length(e.agegp)
}

fn mean_stack_loss() -> float {
  let s = datasets.stackloss
  return mean(s.stack.loss)
}

fn wool_count() -> int {
  let w = datasets.warpbreaks
  return length(w.wool)
}

fn mean_mag() -> float {
  let q = datasets.quakes
  return mean(q.mag)
}

fn mean_sr() -> float {
  let l = datasets.LifeCycleSavings
  return mean(l.sr)
}

fn chick_count() -> int {
  let cw = datasets.ChickWeight
  return length(cw.Chick)
}

fn dnase_run_count() -> int {
  let d = datasets.DNase
  return length(d.Run)
}

fn mean_carb() -> float {
  let f = datasets.Formaldehyde
  return mean(f.carb)
}

fn subject_count() -> int {
  let i = datasets.Indometh
  return length(i.Subject)
}

fn seed_count() -> int {
  let l = datasets.Loblolly
  return length(l.Seed)
}

fn mean_rate() -> float {
  let p = datasets.Puromycin
  return mean(p.rate)
}

fn mean_cont() -> float {
  let j = datasets.USJudgeRatings
  return mean(j.CONT)
}

fn mean_x1() -> float {
  let a = datasets.anscombe
  return mean(a.x1)
}

fn station_count() -> int {
  let a = datasets.attenu
  return length(a.station)
}

fn feed_count() -> int {
  let c = datasets.chickwts
  return length(c.feed)
}

fn education_count() -> int {
  let i = datasets.infert
  return length(i.education)
}

fn mean_year() -> float {
  let l = datasets.longley
  return mean(l.Year)
}

fn mean_speed_morley() -> float {
  let m = datasets.morley
  return mean(m.Speed)
}

fn mean_yield() -> float {
  let n = datasets.npk
  return mean(n.yield)
}

fn mean_fertility() -> float {
  let s = datasets.swiss
  return mean(s.Fertility)
}

fn volcano_rows() -> int {
  let v = datasets.volcano
  return nrow(v)
}

fn volcano_cols() -> int {
  let v = datasets.volcano
  return ncol(v)
}

fn state_x77_cols() -> int {
  return ncol(state_x77)
}

fn personal_expenditure_rows() -> int {
  let p = datasets.USPersonalExpenditure
  return nrow(p)
}

fn worldphones_cols() -> int {
  let w = datasets.WorldPhones
  return ncol(w)
}

fn stock_market_rows() -> int {
  let s = datasets.EuStockMarkets
  return nrow(s)
}

fn va_deaths_cols() -> int {
  let v = datasets.VADeaths
  return ncol(v)
}

fn mean_air_passengers() -> float {
  let a = datasets.AirPassengers
  return mean(a)
}

fn mean_johnson_johnson() -> float {
  let j = datasets.JohnsonJohnson
  return mean(j)
}

fn mean_nile() -> float {
  let n = datasets.Nile
  return mean(n)
}

fn mean_lynx() -> float {
  let l = datasets.lynx
  return mean(l)
}

fn mean_nottem() -> float {
  let n = datasets.nottem
  return mean(n)
}

fn sunspot_year_count() -> int {
  return length(sunspot_year)
}

fn mean_precip() -> float {
  let p = datasets.precip
  return mean(p)
}

fn mean_islands() -> float {
  let i = datasets.islands
  return mean(i)
}

fn state_area_count() -> int {
  return length(state_area)
}

fn state_abb_count() -> int {
  return length(state_abb)
}

fn state_name_count() -> int {
  return length(state_name)
}

fn state_region_count() -> int {
  return length(state_region)
}

fn state_division_count() -> int {
  return length(state_division)
}

fn mean_airmiles() -> float {
  let x = datasets.airmiles
  return mean(x)
}

fn mean_austres() -> float {
  let x = datasets.austres
  return mean(x)
}

fn mean_co2_series() -> float {
  let x = datasets.co2
  return mean(x)
}

fn mean_discoveries() -> float {
  let x = datasets.discoveries
  return mean(x)
}

fn mean_fdeaths() -> float {
  let x = datasets.fdeaths
  return mean(x)
}

fn mean_ldeaths() -> float {
  let x = datasets.ldeaths
  return mean(x)
}

fn mean_mdeaths() -> float {
  let x = datasets.mdeaths
  return mean(x)
}

fn mean_nhtemp() -> float {
  let x = datasets.nhtemp
  return mean(x)
}

fn mean_sunspots() -> float {
  let x = datasets.sunspots
  return mean(x)
}

fn mean_treering() -> float {
  let x = datasets.treering
  return mean(x)
}

fn mean_uspop() -> float {
  let x = datasets.uspop
  return mean(x)
}

fn mean_rivers() -> float {
  let x = datasets.rivers
  return mean(x)
}

fn mean_uk_driver_deaths() -> float {
  let x = datasets.UKDriverDeaths
  return mean(x)
}

fn mean_ukgas() -> float {
  let x = datasets.UKgas
  return mean(x)
}

fn mean_us_acc_deaths() -> float {
  let x = datasets.USAccDeaths
  return mean(x)
}

fn mean_wwwusage() -> float {
  let x = datasets.WWWusage
  return mean(x)
}

fn mean_eurodist() -> float {
  let x = datasets.eurodist
  return mean(x)
}

fn uscitiesd_count() -> int {
  let x = datasets.UScitiesD
  return length(x)
}

fn mean_euro() -> float {
  let x = datasets.euro
  return mean(x)
}

fn sum_stack_loss_series() -> float {
  return sum(stack_loss_series)
}

fn sum_sunspot_m2014() -> float {
  if (!has_datasets_entry("sunspot.m2014")) {
    return 0.0
  }
  return sum(sunspot_m2014)
}

fn sum_sunspot_month() -> float {
  return sum(sunspot_month)
}

fn mean_lake_huron() -> float {
  let x = datasets.LakeHuron
  return mean(x)
}

fn mean_lh() -> float {
  let x = datasets.lh
  return mean(x)
}

fn mean_presidents() -> float {
  let x = datasets.presidents
  return mean(x)
}

fn seatbelts_cols() -> int {
  let x = datasets.Seatbelts
  return ncol(x)
}

fn orchard_treatment_count() -> int {
  let x = datasets.OrchardSprays
  return length(x.treatment)
}

fn mean_theoph_conc() -> float {
  let x = datasets.Theoph
  return mean(x.conc)
}

fn penguin_species_count() -> int {
  if (!has_datasets_entry("penguins")) {
    return 0L
  }
  let x = datasets.penguins
  return length(x.species)
}

fn mean_penguin_year() -> float {
  if (!has_datasets_entry("penguins")) {
    return 0.0
  }
  let x = datasets.penguins
  return mean(x.year)
}

fn penguins_raw_col_count() -> int {
  if (!has_datasets_entry("penguins_raw")) {
    return 0L
  }
  let x = datasets.penguins_raw
  return length(colnames(x))
}

fn gait_dim_count() -> int {
  if (!has_datasets_entry("gait")) {
    return 0L
  }
  let x = datasets.gait
  return length(dim(x))
}

fn crimtab_nrow() -> int {
  let x = datasets.crimtab
  return nrow(x)
}

fn occupational_status_ncol() -> int {
  let x = datasets.occupationalStatus
  return ncol(x)
}

fn titanic_nrow() -> int {
  let x = datasets.Titanic
  return nrow(x)
}

fn titanic_dim_count() -> int {
  let x = datasets.Titanic
  return length(dim(x))
}

fn titanic_total() -> float {
  let x = datasets.Titanic
  return sum(x)
}

fn ucb_ncol() -> int {
  let x = datasets.UCBAdmissions
  return ncol(x)
}

fn ucb_dim_count() -> int {
  let x = datasets.UCBAdmissions
  return length(dim(x))
}

fn hair_eye_nrow() -> int {
  let x = datasets.HairEyeColor
  return nrow(x)
}

fn hair_eye_dim_count() -> int {
  let x = datasets.HairEyeColor
  return length(dim(x))
}

fn ability_cov_count() -> int {
  return length(ability_cov)
}

fn ability_cov_rows() -> int {
  let x = ability_cov
  return nrow(x.cov)
}

fn harman23_count() -> int {
  return length(harman23_cor)
}

fn harman23_center_count() -> int {
  let x = harman23_cor
  return length(x.center)
}

fn harman74_count() -> int {
  return length(harman74_cor)
}

fn state_center_count() -> int {
  return length(state_center)
}

fn mean_state_center_x() -> float {
  let x = state_center
  return mean(x.x)
}

fn mean_bjsales() -> float {
  let x = datasets.BJsales
  return mean(x)
}

fn sum_bjsales_lead() -> float {
  return sum(bj_sales_lead)
}

fn mean_beaver1_temp() -> float {
  let x = datasets.beaver1
  return mean(x.temp)
}

fn mean_beaver2_temp() -> float {
  let x = datasets.beaver2
  return mean(x.temp)
}

fn euro_cross_rows() -> int {
  return nrow(euro_cross)
}

fn mean_randu_x() -> float {
  let x = datasets.randu
  return mean(x.x)
}

fn mean_freeny_y_df() -> float {
  let x = datasets.freeny
  return mean(x.y)
}

fn stack_x_cols() -> int {
  return ncol(stack_x)
}

fn freeny_x_cols() -> int {
  return ncol(freeny_x)
}

fn sum_freeny_y() -> float {
  return sum(freeny_y)
}

fn iris3_dim_count() -> int {
  let x = datasets.iris3
  return length(dim(x))
}

fn iris_name_count() -> int {
  let x = datasets.iris
  return length(names(x))
}

fn mtcars_colname_count() -> int {
  let x = datasets.mtcars
  return length(colnames(x))
}

fn co2_colname_count() -> int {
  let x = datasets.CO2
  return length(colnames(x))
}

print(mean_mpg())
print(species_count())
print(mean_wind())
print(supp_count())
print(mean_uptake())
print(mean_murder())
print(mean_speed())
print(mean_pressure())
print(mean_eruptions())
print(mean_weight())
print(mean_demand())
print(mean_rating())
print(group_count())
print(mean_count())
print(mean_extra())
print(tree_count())
print(mean_area())
print(mean_girth())
print(agegp_count())
print(mean_stack_loss())
print(wool_count())
print(mean_mag())
print(mean_sr())
print(chick_count())
print(dnase_run_count())
print(mean_carb())
print(subject_count())
print(seed_count())
print(mean_rate())
print(mean_cont())
print(mean_x1())
print(station_count())
print(feed_count())
print(education_count())
print(mean_year())
print(mean_speed_morley())
print(mean_yield())
print(mean_fertility())
print(volcano_rows())
print(volcano_cols())
print(state_x77_cols())
print(personal_expenditure_rows())
print(worldphones_cols())
print(stock_market_rows())
print(va_deaths_cols())
print(mean_air_passengers())
print(mean_johnson_johnson())
print(mean_nile())
print(mean_lynx())
print(mean_nottem())
print(sunspot_year_count())
print(mean_precip())
print(mean_islands())
print(state_area_count())
print(state_abb_count())
print(state_name_count())
print(state_region_count())
print(state_division_count())
print(mean_airmiles())
print(mean_austres())
print(mean_co2_series())
print(mean_discoveries())
print(mean_fdeaths())
print(mean_ldeaths())
print(mean_mdeaths())
print(mean_nhtemp())
print(mean_sunspots())
print(mean_treering())
print(mean_uspop())
print(mean_rivers())
print(mean_uk_driver_deaths())
print(mean_ukgas())
print(mean_us_acc_deaths())
print(mean_wwwusage())
print(mean_eurodist())
print(uscitiesd_count())
print(mean_euro())
print(sum_stack_loss_series())
print(sum_sunspot_m2014())
print(sum_sunspot_month())
print(mean_lake_huron())
print(mean_lh())
print(mean_presidents())
print(seatbelts_cols())
print(orchard_treatment_count())
print(mean_theoph_conc())
print(penguin_species_count())
print(mean_penguin_year())
print(penguins_raw_col_count())
print(gait_dim_count())
print(crimtab_nrow())
print(occupational_status_ncol())
print(titanic_nrow())
print(titanic_dim_count())
print(titanic_total())
print(ucb_ncol())
print(ucb_dim_count())
print(hair_eye_nrow())
print(hair_eye_dim_count())
print(ability_cov_count())
print(ability_cov_rows())
print(harman23_count())
print(harman23_center_count())
print(harman74_count())
print(state_center_count())
print(mean_state_center_x())
print(mean_bjsales())
print(sum_bjsales_lead())
print(mean_beaver1_temp())
print(mean_beaver2_temp())
print(euro_cross_rows())
print(mean_randu_x())
print(mean_freeny_y_df())
print(stack_x_cols())
print(freeny_x_cols())
print(sum_freeny_y())
print(iris3_dim_count())
print(iris_name_count())
print(mtcars_colname_count())
print(co2_colname_count())
"#;

    let rr_path = out_dir.join("datasets_direct_types.rr");
    let o0 = out_dir.join("datasets_direct_types_o0.R");
    let o2 = out_dir.join("datasets_direct_types_o2.R");

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
