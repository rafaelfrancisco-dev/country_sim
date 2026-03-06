use super::*;
use crate::sim::{
    country::Country,
    generator::{GenerationConfig, generate_countries},
    person::{DAYS_PER_YEAR, EducationTrack, EmploymentStatus, Person},
};
use rayon::ThreadPoolBuilder;

pub(super) fn build_people(count: usize) -> Vec<Person> {
    (0..count)
        .map(|idx| {
            Person::new(
                idx as u32,
                (idx % 10) as u8,
                (idx % 4) as u16,
                idx as u32 + 10,
            )
        })
        .collect()
}

pub(super) fn unemployed_person(id: u32, cash_balance: f64) -> Person {
    let mut person = Person::new_with_profile(
        id,
        EmploymentStatus::Unemployed,
        id + 100,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    person.cash_balance = cash_balance;
    person
}

pub(super) fn employed_person(
    id: u32,
    job_type_id: u8,
    employer_company_id: u16,
    cash_balance: f64,
) -> Person {
    let mut person = Person::new_with_profile(
        id,
        EmploymentStatus::Employed {
            job_type_id,
            employer_company_id,
        },
        id + 100,
        30 * DAYS_PER_YEAR,
        EducationTrack::HighSchool,
    );
    person.cash_balance = cash_balance;
    person
}

pub(super) fn run_world_in_pool(
    country_count: usize,
    people_per_country: usize,
    days: u64,
    threads: usize,
) -> World {
    let pool = ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("thread pool should build");

    pool.install(|| {
        let countries = (0..country_count)
            .map(|idx| {
                Country::new(
                    (idx + 1) as u16,
                    format!("C{idx}"),
                    build_people(people_per_country),
                )
            })
            .collect::<Vec<_>>();
        let mut world = World::new(countries);
        for _ in 0..days {
            world.tick_day();
        }
        world
    })
}

pub(super) fn run_generated_world_in_pool(
    seed: u64,
    country_count: u16,
    population_per_country: u32,
    days: u64,
    threads: usize,
) -> World {
    let pool = ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("thread pool should build");

    pool.install(|| {
        let countries = generate_countries(&GenerationConfig::with_defaults(
            country_count,
            population_per_country,
            seed,
        ));
        let mut world = World::new_with_generation_seed(countries, seed);
        for _ in 0..days {
            world.tick_day();
        }
        world
    })
}

pub(super) fn mix_hash(hash: &mut u64, value: u64) {
    *hash ^= value
        .wrapping_add(0x9e37_79b9_7f4a_7c15)
        .wrapping_add(*hash << 6)
        .wrapping_add(*hash >> 2);
}

pub(super) fn world_state_hash(world: &World) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;

    mix_hash(&mut hash, world.day);
    mix_hash(&mut hash, world.generation_seed);
    mix_hash(&mut hash, world.countries.len() as u64);

    for country in &world.countries {
        mix_hash(&mut hash, u64::from(country.id));
        mix_hash(&mut hash, country.day);
        mix_hash(&mut hash, country.month);
        mix_hash(&mut hash, country.people.len() as u64);
        mix_hash(&mut hash, country.companies.len() as u64);
        mix_hash(&mut hash, country.monthly_unmet_need_events);
        mix_hash(&mut hash, country.monthly_household_spending.to_bits());
        mix_hash(&mut hash, country.monthly_tariff_collected.to_bits());
        mix_hash(&mut hash, country.monthly_goods_import_spending.to_bits());
        mix_hash(&mut hash, country.monthly_goods_export_revenue.to_bits());
        mix_hash(&mut hash, country.monthly_essential_debt_spending.to_bits());
        mix_hash(&mut hash, country.total_money_supply.to_bits());
        mix_hash(
            &mut hash,
            country.primary_bank().created_money_outstanding.to_bits(),
        );

        for tariff in country.export_tariffs_by_good {
            mix_hash(&mut hash, tariff.to_bits());
        }

        for company in &country.companies {
            mix_hash(&mut hash, u64::from(company.id));
            mix_hash(&mut hash, company.produced_good_id as u64);
            mix_hash(&mut hash, company.inventory_units as u64);
            mix_hash(&mut hash, company.unit_price.to_bits());
            mix_hash(&mut hash, company.cash_balance.to_bits());
            mix_hash(&mut hash, company.last_month_sales.to_bits());
            mix_hash(&mut hash, company.last_month_units_sold as u64);
            mix_hash(&mut hash, company.last_month_export_units_sold as u64);
            mix_hash(&mut hash, company.last_month_domestic_units_sold as u64);
        }

        for person in &country.people {
            mix_hash(&mut hash, u64::from(person.id));
            mix_hash(&mut hash, person.cash_balance.to_bits());
            mix_hash(&mut hash, u64::from(person.name_seed));
            mix_hash(&mut hash, u64::from(person.age_days));
            mix_hash(&mut hash, u64::from(person.last_month_goods_units_bought));
            mix_hash(&mut hash, u64::from(person.last_month_needs_met_mask));
            mix_hash(&mut hash, u64::from(person.last_month_unmet_needs_mask));
            for stock_units in person.superficial_stock_units {
                mix_hash(&mut hash, u64::from(stock_units));
            }
            for carry in person.superficial_stock_consumption_carry {
                mix_hash(&mut hash, u64::from(carry.to_bits()));
            }
        }
    }

    hash
}
