use super::*;

#[test]
fn monthly_salary_comes_from_job_catalog_when_employed() {
    let person = Person::new(1, 9, 0, 42);

    assert!((person.monthly_salary() - 8_500.0).abs() < f64::EPSILON);
}

#[test]
fn monthly_salary_is_zero_when_unemployed() {
    let person = Person::new_with_profile(
        1,
        EmploymentStatus::Unemployed,
        42,
        22 * DAYS_PER_YEAR,
        EducationTrack::Bachelor,
    );

    assert!((person.monthly_salary() - 0.0).abs() < f64::EPSILON);
}

#[test]
fn monthly_trackers_reset_without_touching_cash() {
    let mut person = Person::new(1, 0, 0, 42);
    person.cash_balance = 50.0;
    person.last_month_gross_pay = 10.0;
    person.last_month_tax_paid = 2.0;
    person.last_month_net_pay = 8.0;
    person.last_month_goods_spending = 33.0;
    person.superficial_stock_units[0] = 3;

    person.reset_monthly_trackers();

    assert!((person.cash_balance - 50.0).abs() < f64::EPSILON);
    assert!((person.last_month_gross_pay - 0.0).abs() < f64::EPSILON);
    assert!((person.last_month_tax_paid - 0.0).abs() < f64::EPSILON);
    assert!((person.last_month_net_pay - 0.0).abs() < f64::EPSILON);
    assert!((person.last_completed_month_gross_pay - 10.0).abs() < f64::EPSILON);
    assert!((person.last_completed_month_tax_paid - 2.0).abs() < f64::EPSILON);
    assert!((person.last_completed_month_net_pay - 8.0).abs() < f64::EPSILON);
    assert!((person.last_month_goods_spending - 0.0).abs() < f64::EPSILON);
    assert_eq!(person.superficial_stock_units[0], 3);
}

#[test]
fn employment_start_day_tracks_employment_lifecycle() {
    let mut person = Person::new_with_profile(
        1,
        EmploymentStatus::Unemployed,
        42,
        22 * DAYS_PER_YEAR,
        EducationTrack::Bachelor,
    );

    assert_eq!(person.employment_start_day, None);

    person.set_employment_with_salary(3, 1, 4_200.0, 88);
    assert_eq!(person.employment_start_day, Some(88));

    person.clear_employment();
    assert_eq!(person.employment_start_day, None);
}

#[test]
fn salary_update_keeps_existing_employment_start_day() {
    let mut person = Person::new_with_profile(
        1,
        EmploymentStatus::Unemployed,
        42,
        22 * DAYS_PER_YEAR,
        EducationTrack::Bachelor,
    );

    person.set_employment_with_salary(3, 1, 4_200.0, 20);
    person.set_employment_with_salary(3, 1, 4_500.0, 90);

    assert_eq!(person.employment_start_day, Some(20));
}

#[test]
fn can_seek_job_uses_track_specific_age_gate() {
    let basic = Person::new_with_profile(
        1,
        EmploymentStatus::Unemployed,
        1,
        18 * DAYS_PER_YEAR,
        EducationTrack::BasicOnly,
    );
    let bachelor = Person::new_with_profile(
        2,
        EmploymentStatus::Unemployed,
        2,
        20 * DAYS_PER_YEAR,
        EducationTrack::Bachelor,
    );
    let doctor = Person::new_with_profile(
        3,
        EmploymentStatus::Unemployed,
        3,
        22 * DAYS_PER_YEAR,
        EducationTrack::Doctor,
    );

    assert!(basic.can_seek_job());
    assert!(!bachelor.can_seek_job());
    assert!(!doctor.can_seek_job());
}

#[test]
fn doctor_track_reaches_higher_learning_at_age_twenty_three() {
    let mut person = Person::new_with_profile(
        10,
        EmploymentStatus::Unemployed,
        11,
        22 * DAYS_PER_YEAR,
        EducationTrack::Doctor,
    );
    assert_eq!(person.habilitation, HabilitationLevel::HighSchool);

    person.age_days = 23 * DAYS_PER_YEAR;
    person.refresh_habilitation_from_age();

    assert_eq!(person.habilitation, HabilitationLevel::HigherLearning);
}

#[test]
fn habilitation_progresses_with_age_and_track() {
    let mut person = Person::new_with_profile(
        9,
        EmploymentStatus::Unemployed,
        10,
        11 * DAYS_PER_YEAR,
        EducationTrack::Bachelor,
    );
    assert_eq!(person.habilitation, HabilitationLevel::NoSchoolYet);

    person.age_days = 17 * DAYS_PER_YEAR;
    person.refresh_habilitation_from_age();
    assert_eq!(person.habilitation, HabilitationLevel::BasicSchool);

    person.age_days = 18 * DAYS_PER_YEAR;
    person.refresh_habilitation_from_age();
    assert_eq!(person.habilitation, HabilitationLevel::HighSchool);

    person.age_days = 21 * DAYS_PER_YEAR;
    person.refresh_habilitation_from_age();
    assert_eq!(person.habilitation, HabilitationLevel::HigherLearning);
}
