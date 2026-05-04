use super::type_precision_regression_common::*;

#[test]
pub(crate) fn datasets_package_loads_refine_known_dataframe_shapes() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let iris = add_load(&mut fn_ir, "datasets::iris");
    let mtcars = add_load(&mut fn_ir, "datasets::mtcars");
    let airquality = add_load(&mut fn_ir, "datasets::airquality");
    let tooth_growth = add_load(&mut fn_ir, "datasets::ToothGrowth");
    let co2 = add_load(&mut fn_ir, "datasets::CO2");
    let us_arrests = add_load(&mut fn_ir, "datasets::USArrests");
    let cars = add_load(&mut fn_ir, "datasets::cars");
    let pressure = add_load(&mut fn_ir, "datasets::pressure");
    let faithful = add_load(&mut fn_ir, "datasets::faithful");
    let women = add_load(&mut fn_ir, "datasets::women");
    let bod = add_load(&mut fn_ir, "datasets::BOD");
    let attitude = add_load(&mut fn_ir, "datasets::attitude");
    let plant_growth = add_load(&mut fn_ir, "datasets::PlantGrowth");
    let insect_sprays = add_load(&mut fn_ir, "datasets::InsectSprays");
    let sleep = add_load(&mut fn_ir, "datasets::sleep");
    let orange = add_load(&mut fn_ir, "datasets::Orange");
    let rock = add_load(&mut fn_ir, "datasets::rock");
    let trees = add_load(&mut fn_ir, "datasets::trees");
    let esoph = add_load(&mut fn_ir, "datasets::esoph");
    let stackloss = add_load(&mut fn_ir, "datasets::stackloss");
    let warpbreaks = add_load(&mut fn_ir, "datasets::warpbreaks");
    let quakes = add_load(&mut fn_ir, "datasets::quakes");
    let life_cycle_savings = add_load(&mut fn_ir, "datasets::LifeCycleSavings");
    let chick_weight = add_load(&mut fn_ir, "datasets::ChickWeight");
    let dnase = add_load(&mut fn_ir, "datasets::DNase");
    let formaldehyde = add_load(&mut fn_ir, "datasets::Formaldehyde");
    let indometh = add_load(&mut fn_ir, "datasets::Indometh");
    let loblolly = add_load(&mut fn_ir, "datasets::Loblolly");
    let puromycin = add_load(&mut fn_ir, "datasets::Puromycin");
    let us_judge_ratings = add_load(&mut fn_ir, "datasets::USJudgeRatings");
    let anscombe = add_load(&mut fn_ir, "datasets::anscombe");
    let attenu = add_load(&mut fn_ir, "datasets::attenu");
    let chickwts = add_load(&mut fn_ir, "datasets::chickwts");
    let infert = add_load(&mut fn_ir, "datasets::infert");
    let longley = add_load(&mut fn_ir, "datasets::longley");
    let morley = add_load(&mut fn_ir, "datasets::morley");
    let npk = add_load(&mut fn_ir, "datasets::npk");
    let swiss = add_load(&mut fn_ir, "datasets::swiss");
    let species_name = add_str(&mut fn_ir, "Species");
    let mpg_name = add_str(&mut fn_ir, "mpg");
    let wind_name = add_str(&mut fn_ir, "Wind");
    let supp_name = add_str(&mut fn_ir, "supp");
    let uptake_name = add_str(&mut fn_ir, "uptake");
    let murder_name = add_str(&mut fn_ir, "Murder");
    let speed_name = add_str(&mut fn_ir, "speed");
    let pressure_name = add_str(&mut fn_ir, "pressure");
    let eruptions_name = add_str(&mut fn_ir, "eruptions");
    let weight_name = add_str(&mut fn_ir, "weight");
    let demand_name = add_str(&mut fn_ir, "demand");
    let rating_name = add_str(&mut fn_ir, "rating");
    let group_name = add_str(&mut fn_ir, "group");
    let count_name = add_str(&mut fn_ir, "count");
    let extra_name = add_str(&mut fn_ir, "extra");
    let tree_name = add_str(&mut fn_ir, "Tree");
    let area_name = add_str(&mut fn_ir, "area");
    let girth_name = add_str(&mut fn_ir, "Girth");
    let agegp_name = add_str(&mut fn_ir, "agegp");
    let stack_loss_name = add_str(&mut fn_ir, "stack.loss");
    let wool_name = add_str(&mut fn_ir, "wool");
    let mag_name = add_str(&mut fn_ir, "mag");
    let sr_name = add_str(&mut fn_ir, "sr");
    let chick_name = add_str(&mut fn_ir, "Chick");
    let run_name = add_str(&mut fn_ir, "Run");
    let carb_name = add_str(&mut fn_ir, "carb");
    let subject_name = add_str(&mut fn_ir, "Subject");
    let seed_name = add_str(&mut fn_ir, "Seed");
    let rate_name = add_str(&mut fn_ir, "rate");
    let cont_name = add_str(&mut fn_ir, "CONT");
    let x1_name = add_str(&mut fn_ir, "x1");
    let station_name = add_str(&mut fn_ir, "station");
    let feed_name = add_str(&mut fn_ir, "feed");
    let education_name = add_str(&mut fn_ir, "education");
    let year_name = add_str(&mut fn_ir, "Year");
    let morley_speed_name = add_str(&mut fn_ir, "Speed");
    let yield_name = add_str(&mut fn_ir, "yield");
    let fertility_name = add_str(&mut fn_ir, "Fertility");
    let species = add_call(&mut fn_ir, "rr_field_get", vec![iris, species_name]);
    let mpg = add_call(&mut fn_ir, "rr_field_get", vec![mtcars, mpg_name]);
    let wind = add_call(&mut fn_ir, "rr_field_get", vec![airquality, wind_name]);
    let supp = add_call(&mut fn_ir, "rr_field_get", vec![tooth_growth, supp_name]);
    let uptake = add_call(&mut fn_ir, "rr_field_get", vec![co2, uptake_name]);
    let murder = add_call(&mut fn_ir, "rr_field_get", vec![us_arrests, murder_name]);
    let speed = add_call(&mut fn_ir, "rr_field_get", vec![cars, speed_name]);
    let pressure_col = add_call(&mut fn_ir, "rr_field_get", vec![pressure, pressure_name]);
    let eruptions = add_call(&mut fn_ir, "rr_field_get", vec![faithful, eruptions_name]);
    let weight = add_call(&mut fn_ir, "rr_field_get", vec![women, weight_name]);
    let demand = add_call(&mut fn_ir, "rr_field_get", vec![bod, demand_name]);
    let rating = add_call(&mut fn_ir, "rr_field_get", vec![attitude, rating_name]);
    let pg_group = add_call(&mut fn_ir, "rr_field_get", vec![plant_growth, group_name]);
    let spray_count = add_call(&mut fn_ir, "rr_field_get", vec![insect_sprays, count_name]);
    let sleep_extra = add_call(&mut fn_ir, "rr_field_get", vec![sleep, extra_name]);
    let orange_tree = add_call(&mut fn_ir, "rr_field_get", vec![orange, tree_name]);
    let rock_area = add_call(&mut fn_ir, "rr_field_get", vec![rock, area_name]);
    let tree_girth = add_call(&mut fn_ir, "rr_field_get", vec![trees, girth_name]);
    let esoph_agegp = add_call(&mut fn_ir, "rr_field_get", vec![esoph, agegp_name]);
    let stack_loss = add_call(&mut fn_ir, "rr_field_get", vec![stackloss, stack_loss_name]);
    let warpbreaks_wool = add_call(&mut fn_ir, "rr_field_get", vec![warpbreaks, wool_name]);
    let quake_mag = add_call(&mut fn_ir, "rr_field_get", vec![quakes, mag_name]);
    let savings_sr = add_call(
        &mut fn_ir,
        "rr_field_get",
        vec![life_cycle_savings, sr_name],
    );
    let chick_weight_chick = add_call(&mut fn_ir, "rr_field_get", vec![chick_weight, chick_name]);
    let dnase_run = add_call(&mut fn_ir, "rr_field_get", vec![dnase, run_name]);
    let formaldehyde_carb = add_call(&mut fn_ir, "rr_field_get", vec![formaldehyde, carb_name]);
    let indometh_subject = add_call(&mut fn_ir, "rr_field_get", vec![indometh, subject_name]);
    let loblolly_seed = add_call(&mut fn_ir, "rr_field_get", vec![loblolly, seed_name]);
    let puromycin_rate = add_call(&mut fn_ir, "rr_field_get", vec![puromycin, rate_name]);
    let judge_cont = add_call(
        &mut fn_ir,
        "rr_field_get",
        vec![us_judge_ratings, cont_name],
    );
    let anscombe_x1 = add_call(&mut fn_ir, "rr_field_get", vec![anscombe, x1_name]);
    let attenu_station = add_call(&mut fn_ir, "rr_field_get", vec![attenu, station_name]);
    let chickwts_feed = add_call(&mut fn_ir, "rr_field_get", vec![chickwts, feed_name]);
    let infert_education = add_call(&mut fn_ir, "rr_field_get", vec![infert, education_name]);
    let longley_year = add_call(&mut fn_ir, "rr_field_get", vec![longley, year_name]);
    let morley_speed = add_call(&mut fn_ir, "rr_field_get", vec![morley, morley_speed_name]);
    let npk_yield = add_call(&mut fn_ir, "rr_field_get", vec![npk, yield_name]);
    let swiss_fertility = add_call(&mut fn_ir, "rr_field_get", vec![swiss, fertility_name]);
    let iris_names = add_call(&mut fn_ir, "names", vec![iris]);
    let mtcars_colnames = add_call(&mut fn_ir, "colnames", vec![mtcars]);
    let co2_colnames = add_call(&mut fn_ir, "colnames", vec![co2]);
    fn_ir.blocks[b0].term = Terminator::Return(Some(mpg));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    assert_eq!(out.values[iris].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[iris].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[mtcars].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[mtcars].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[airquality].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[airquality].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[tooth_growth].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[tooth_growth].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[co2].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[co2].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[us_arrests].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[us_arrests].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[cars].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[cars].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[pressure].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[pressure].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[faithful].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[faithful].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[women].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[women].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[bod].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[bod].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[attitude].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[attitude].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[plant_growth].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[plant_growth].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[insect_sprays].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[insect_sprays].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[sleep].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[sleep].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[orange].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[orange].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[rock].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[rock].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[trees].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[trees].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[esoph].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[stackloss].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[stackloss].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[warpbreaks].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[quakes].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[quakes].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[life_cycle_savings].value_ty.shape,
        ShapeTy::Matrix
    );
    assert_eq!(out.values[life_cycle_savings].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[chick_weight].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[dnase].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[formaldehyde].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[indometh].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[loblolly].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[puromycin].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[us_judge_ratings].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[anscombe].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[attenu].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[chickwts].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[infert].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[longley].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[morley].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[npk].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[swiss].value_ty.shape, ShapeTy::Matrix);

    assert_eq!(out.values[species].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[species].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[species].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );

    assert_eq!(out.values[mpg].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[mpg].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[mpg].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );

    assert_eq!(out.values[wind].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[wind].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[wind].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );

    assert_eq!(out.values[supp].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[supp].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[supp].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );

    assert_eq!(out.values[uptake].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[uptake].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[uptake].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );

    assert_eq!(out.values[murder].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[murder].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[murder].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );

    for vid in [speed, pressure_col, eruptions, weight, demand] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }

    assert_eq!(out.values[rating].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[rating].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[rating].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );

    assert_eq!(out.values[pg_group].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[pg_group].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[pg_group].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );

    for vid in [spray_count, sleep_extra, rock_area] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }

    assert_eq!(out.values[orange_tree].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[orange_tree].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[orange_tree].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );

    for vid in [tree_girth, stack_loss, quake_mag, savings_sr] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }

    for vid in [
        esoph_agegp,
        warpbreaks_wool,
        chick_weight_chick,
        dnase_run,
        indometh_subject,
        loblolly_seed,
        attenu_station,
        chickwts_feed,
        infert_education,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    for vid in [
        formaldehyde_carb,
        puromycin_rate,
        judge_cont,
        anscombe_x1,
        npk_yield,
        swiss_fertility,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }

    for vid in [longley_year, morley_speed] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Int))
        );
    }

    for vid in [iris_names, mtcars_colnames, co2_colnames] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
    }
    assert_eq!(
        out.values[iris_names].value_term,
        TypeTerm::VectorLen(Box::new(TypeTerm::Char), Some(5))
    );
    assert_eq!(
        out.values[mtcars_colnames].value_term,
        TypeTerm::VectorLen(Box::new(TypeTerm::Char), Some(11))
    );
    assert_eq!(
        out.values[co2_colnames].value_term,
        TypeTerm::VectorLen(Box::new(TypeTerm::Char), Some(5))
    );
}

#[test]
pub(crate) fn datasets_package_matrix_loads_preserve_known_matrix_dims() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let volcano = add_load(&mut fn_ir, "datasets::volcano");
    let state_x77 = add_load(&mut fn_ir, "datasets::state.x77");
    let us_personal_expenditure = add_load(&mut fn_ir, "datasets::USPersonalExpenditure");
    let world_phones = add_load(&mut fn_ir, "datasets::WorldPhones");
    let eu_stock_markets = add_load(&mut fn_ir, "datasets::EuStockMarkets");
    let va_deaths = add_load(&mut fn_ir, "datasets::VADeaths");

    let volcano_rows = add_call(&mut fn_ir, "nrow", vec![volcano]);
    let volcano_cols = add_call(&mut fn_ir, "ncol", vec![volcano]);
    let state_cols = add_call(&mut fn_ir, "ncol", vec![state_x77]);
    let expenditure_rows = add_call(&mut fn_ir, "nrow", vec![us_personal_expenditure]);
    let world_phones_cols = add_call(&mut fn_ir, "ncol", vec![world_phones]);
    let stock_rows = add_call(&mut fn_ir, "nrow", vec![eu_stock_markets]);
    let va_deaths_cols = add_call(&mut fn_ir, "ncol", vec![va_deaths]);
    fn_ir.blocks[b0].term = Terminator::Return(Some(volcano_rows));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in [
        volcano,
        state_x77,
        us_personal_expenditure,
        world_phones,
        eu_stock_markets,
        va_deaths,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
    }

    assert_eq!(
        out.values[volcano].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(87), Some(61))
    );
    assert_eq!(
        out.values[state_x77].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(50), Some(8))
    );
    assert_eq!(
        out.values[us_personal_expenditure].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(5), Some(5))
    );
    assert_eq!(
        out.values[world_phones].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(7), Some(7))
    );
    assert_eq!(
        out.values[eu_stock_markets].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(1860), Some(4))
    );
    assert_eq!(
        out.values[va_deaths].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(5), Some(4))
    );

    for vid in [
        volcano_rows,
        volcano_cols,
        state_cols,
        expenditure_rows,
        world_phones_cols,
        stock_rows,
        va_deaths_cols,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(out.values[vid].value_term, TypeTerm::Int);
    }
}

#[test]
pub(crate) fn datasets_package_vector_loads_refine_known_vector_shapes() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let air_passengers = add_load(&mut fn_ir, "datasets::AirPassengers");
    let johnson_johnson = add_load(&mut fn_ir, "datasets::JohnsonJohnson");
    let nile = add_load(&mut fn_ir, "datasets::Nile");
    let lynx = add_load(&mut fn_ir, "datasets::lynx");
    let nottem = add_load(&mut fn_ir, "datasets::nottem");
    let sunspot_year = add_load(&mut fn_ir, "datasets::sunspot.year");
    let precip = add_load(&mut fn_ir, "datasets::precip");
    let islands = add_load(&mut fn_ir, "datasets::islands");
    let state_area = add_load(&mut fn_ir, "datasets::state.area");
    let state_abb = add_load(&mut fn_ir, "datasets::state.abb");
    let state_name = add_load(&mut fn_ir, "datasets::state.name");
    let state_region = add_load(&mut fn_ir, "datasets::state.region");
    let state_division = add_load(&mut fn_ir, "datasets::state.division");
    let euro = add_load(&mut fn_ir, "datasets::euro");
    let stack_loss_series = add_load(&mut fn_ir, "datasets::stack.loss");
    let sunspot_m2014 = add_load(&mut fn_ir, "datasets::sunspot.m2014");
    let sunspot_month = add_load(&mut fn_ir, "datasets::sunspot.month");
    let lake_huron = add_load(&mut fn_ir, "datasets::LakeHuron");
    let lh = add_load(&mut fn_ir, "datasets::lh");
    let presidents = add_load(&mut fn_ir, "datasets::presidents");
    let airmiles = add_load(&mut fn_ir, "datasets::airmiles");
    let austres = add_load(&mut fn_ir, "datasets::austres");
    let co2_series = add_load(&mut fn_ir, "datasets::co2");
    let discoveries = add_load(&mut fn_ir, "datasets::discoveries");
    let fdeaths = add_load(&mut fn_ir, "datasets::fdeaths");
    let ldeaths = add_load(&mut fn_ir, "datasets::ldeaths");
    let mdeaths = add_load(&mut fn_ir, "datasets::mdeaths");
    let nhtemp = add_load(&mut fn_ir, "datasets::nhtemp");
    let sunspots = add_load(&mut fn_ir, "datasets::sunspots");
    let treering = add_load(&mut fn_ir, "datasets::treering");
    let uspop = add_load(&mut fn_ir, "datasets::uspop");
    let rivers = add_load(&mut fn_ir, "datasets::rivers");
    let uk_driver_deaths = add_load(&mut fn_ir, "datasets::UKDriverDeaths");
    let ukgas = add_load(&mut fn_ir, "datasets::UKgas");
    let us_acc_deaths = add_load(&mut fn_ir, "datasets::USAccDeaths");
    let wwwusage = add_load(&mut fn_ir, "datasets::WWWusage");
    let eurodist = add_load(&mut fn_ir, "datasets::eurodist");
    let uscities_d = add_load(&mut fn_ir, "datasets::UScitiesD");
    fn_ir.blocks[b0].term = Terminator::Return(Some(air_passengers));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in [
        air_passengers,
        johnson_johnson,
        nile,
        lynx,
        nottem,
        sunspot_year,
        precip,
        islands,
        state_area,
        euro,
        stack_loss_series,
        sunspot_m2014,
        sunspot_month,
        lake_huron,
        lh,
        presidents,
        airmiles,
        austres,
        co2_series,
        discoveries,
        fdeaths,
        ldeaths,
        mdeaths,
        nhtemp,
        sunspots,
        treering,
        uspop,
        rivers,
        uk_driver_deaths,
        ukgas,
        us_acc_deaths,
        wwwusage,
        eurodist,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }

    for vid in [state_abb, state_name, state_region, state_division] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    assert_eq!(out.values[uscities_d].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[uscities_d].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[uscities_d].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
}
