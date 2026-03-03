use country_sim::sim::manual_harness::{
    run_export_csv_for_one_year_headless, run_export_stock_csv_for_one_year_headless,
};

fn main() {
    run_export_csv_for_one_year_headless();
    run_export_stock_csv_for_one_year_headless();
}
