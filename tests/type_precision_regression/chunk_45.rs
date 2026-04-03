use super::type_precision_regression_common::*;

#[test]
fn datasets_package_loads_refine_known_dataframe_shapes() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let iris = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::iris".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mtcars = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::mtcars".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let airquality = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::airquality".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tooth_growth = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::ToothGrowth".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let co2 = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::CO2".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let us_arrests = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::USArrests".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cars = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::cars".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pressure = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::pressure".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let faithful = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::faithful".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let women = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::women".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bod = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::BOD".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let attitude = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::attitude".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let plant_growth = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::PlantGrowth".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let insect_sprays = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::InsectSprays".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sleep = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::sleep".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let orange = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::Orange".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rock = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::rock".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let trees = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::trees".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let esoph = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::esoph".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let stackloss = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::stackloss".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let warpbreaks = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::warpbreaks".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let quakes = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::quakes".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let life_cycle_savings = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::LifeCycleSavings".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let chick_weight = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::ChickWeight".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dnase = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::DNase".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formaldehyde = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::Formaldehyde".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let indometh = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::Indometh".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let loblolly = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::Loblolly".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let puromycin = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::Puromycin".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let us_judge_ratings = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::USJudgeRatings".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let anscombe = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::anscombe".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let attenu = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::attenu".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let chickwts = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::chickwts".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let infert = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::infert".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let longley = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::longley".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let morley = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::morley".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let npk = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::npk".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let swiss = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::swiss".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let species_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("Species".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mpg_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("mpg".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let wind_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("Wind".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let supp_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("supp".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let uptake_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("uptake".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let murder_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("Murder".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let speed_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("speed".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pressure_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("pressure".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let eruptions_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("eruptions".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let weight_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("weight".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let demand_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("demand".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rating_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("rating".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let group_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("group".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let count_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("count".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let extra_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("extra".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tree_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("Tree".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let area_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("area".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let girth_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("Girth".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let agegp_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("agegp".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let stack_loss_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("stack.loss".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let wool_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("wool".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mag_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("mag".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sr_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("sr".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let chick_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("Chick".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let run_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("Run".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let carb_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("carb".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let subject_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("Subject".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let seed_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("Seed".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rate_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("rate".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cont_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("CONT".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let x1_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("x1".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let station_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("station".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let feed_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("feed".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let education_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("education".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let year_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("Year".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let morley_speed_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("Speed".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let yield_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("yield".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fertility_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("Fertility".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let species = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![iris, species_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mpg = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![mtcars, mpg_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let wind = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![airquality, wind_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let supp = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![tooth_growth, supp_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let uptake = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![co2, uptake_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let murder = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![us_arrests, murder_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let speed = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![cars, speed_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pressure_col = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![pressure, pressure_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let eruptions = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![faithful, eruptions_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let weight = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![women, weight_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let demand = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![bod, demand_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rating = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![attitude, rating_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pg_group = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![plant_growth, group_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let spray_count = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![insect_sprays, count_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sleep_extra = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![sleep, extra_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let orange_tree = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![orange, tree_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rock_area = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![rock, area_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tree_girth = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![trees, girth_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let esoph_agegp = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![esoph, agegp_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let stack_loss = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![stackloss, stack_loss_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let warpbreaks_wool = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![warpbreaks, wool_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let quake_mag = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![quakes, mag_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let savings_sr = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![life_cycle_savings, sr_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let chick_weight_chick = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![chick_weight, chick_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dnase_run = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![dnase, run_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formaldehyde_carb = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![formaldehyde, carb_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let indometh_subject = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![indometh, subject_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let loblolly_seed = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![loblolly, seed_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let puromycin_rate = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![puromycin, rate_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let judge_cont = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![us_judge_ratings, cont_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let anscombe_x1 = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![anscombe, x1_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let attenu_station = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![attenu, station_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let chickwts_feed = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![chickwts, feed_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let infert_education = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![infert, education_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let longley_year = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![longley, year_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let morley_speed = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![morley, morley_speed_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let npk_yield = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![npk, yield_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let swiss_fertility = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![swiss, fertility_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let iris_names = fn_ir.add_value(
        ValueKind::Call {
            callee: "names".to_string(),
            args: vec![iris],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mtcars_colnames = fn_ir.add_value(
        ValueKind::Call {
            callee: "colnames".to_string(),
            args: vec![mtcars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let co2_colnames = fn_ir.add_value(
        ValueKind::Call {
            callee: "colnames".to_string(),
            args: vec![co2],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
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
fn datasets_package_matrix_loads_preserve_known_matrix_dims() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let volcano = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::volcano".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let state_x77 = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::state.x77".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let us_personal_expenditure = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::USPersonalExpenditure".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let world_phones = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::WorldPhones".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let eu_stock_markets = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::EuStockMarkets".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let va_deaths = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::VADeaths".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let volcano_rows = fn_ir.add_value(
        ValueKind::Call {
            callee: "nrow".to_string(),
            args: vec![volcano],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let volcano_cols = fn_ir.add_value(
        ValueKind::Call {
            callee: "ncol".to_string(),
            args: vec![volcano],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let state_cols = fn_ir.add_value(
        ValueKind::Call {
            callee: "ncol".to_string(),
            args: vec![state_x77],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let expenditure_rows = fn_ir.add_value(
        ValueKind::Call {
            callee: "nrow".to_string(),
            args: vec![us_personal_expenditure],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let world_phones_cols = fn_ir.add_value(
        ValueKind::Call {
            callee: "ncol".to_string(),
            args: vec![world_phones],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let stock_rows = fn_ir.add_value(
        ValueKind::Call {
            callee: "nrow".to_string(),
            args: vec![eu_stock_markets],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let va_deaths_cols = fn_ir.add_value(
        ValueKind::Call {
            callee: "ncol".to_string(),
            args: vec![va_deaths],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
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
fn datasets_package_vector_loads_refine_known_vector_shapes() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let air_passengers = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::AirPassengers".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let johnson_johnson = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::JohnsonJohnson".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nile = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::Nile".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let lynx = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::lynx".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nottem = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::nottem".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sunspot_year = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::sunspot.year".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let precip = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::precip".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let islands = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::islands".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let state_area = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::state.area".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let state_abb = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::state.abb".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let state_name = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::state.name".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let state_region = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::state.region".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let state_division = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::state.division".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let euro = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::euro".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let stack_loss_series = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::stack.loss".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sunspot_m2014 = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::sunspot.m2014".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sunspot_month = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::sunspot.month".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let lake_huron = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::LakeHuron".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let lh = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::lh".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let presidents = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::presidents".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let airmiles = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::airmiles".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let austres = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::austres".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let co2_series = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::co2".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let discoveries = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::discoveries".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fdeaths = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::fdeaths".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ldeaths = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::ldeaths".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mdeaths = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::mdeaths".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nhtemp = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::nhtemp".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sunspots = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::sunspots".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let treering = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::treering".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let uspop = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::uspop".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rivers = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::rivers".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let uk_driver_deaths = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::UKDriverDeaths".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ukgas = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::UKgas".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let us_acc_deaths = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::USAccDeaths".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let wwwusage = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::WWWusage".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let eurodist = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::eurodist".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let uscities_d = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::UScitiesD".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
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
