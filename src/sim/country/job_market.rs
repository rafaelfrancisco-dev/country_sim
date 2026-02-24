#[derive(Debug, Copy, Clone)]
struct JobSeekIntent {
    person_idx: usize,
    seek_seed: u64,
    qualification: HabilitationLevel,
    preferred: Option<(usize, JobTypeId)>,
}

fn select_matching_opening_from_snapshot(
    company_openings: &[Vec<JobOpening>],
    qualification: HabilitationLevel,
    seed: u64,
) -> Option<(usize, JobTypeId)> {
    let mut chosen: Option<(usize, JobTypeId, u64)> = None;
    let mut seen_slots = 0_u64;

    for (company_idx, openings) in company_openings.iter().enumerate() {
        for (opening_idx, opening) in openings.iter().enumerate() {
            if opening.slots == 0
                || !job_is_eligible_for_habilitation(opening.job_type_id, qualification)
            {
                continue;
            }

            let slots = u64::from(opening.slots);
            seen_slots = seen_slots.saturating_add(slots);
            let local_seed =
                seed ^ (u64::from(company_idx as u32) << 32) ^ u64::from(opening_idx as u32);
            if hash64(local_seed) % seen_slots < slots {
                chosen = Some((company_idx, opening.job_type_id, seen_slots));
            }
        }
    }

    let (company_idx, job_type_id, _) = chosen?;
    Some((company_idx, job_type_id))
}

fn consume_opening_by_job_type(company: &mut Company, job_type_id: JobTypeId) -> Option<JobTypeId> {
    let opening_idx = company
        .open_positions
        .iter()
        .position(|opening| opening.slots > 0 && opening.job_type_id == job_type_id)?;
    company.consume_opening(opening_idx)
}

fn consume_matching_opening(
    companies: &mut [Company],
    qualification: HabilitationLevel,
    seed: u64,
) -> Option<(u16, JobTypeId)> {
    let mut chosen: Option<(usize, usize, u64)> = None;
    let mut seen_slots = 0_u64;

    for (company_idx, company) in companies.iter().enumerate() {
        for (opening_idx, opening) in company.open_positions.iter().enumerate() {
            if opening.slots == 0
                || !job_is_eligible_for_habilitation(opening.job_type_id, qualification)
            {
                continue;
            }

            let slots = u64::from(opening.slots);
            seen_slots = seen_slots.saturating_add(slots);
            let local_seed =
                seed ^ (u64::from(company_idx as u32) << 32) ^ u64::from(opening_idx as u32);
            if hash64(local_seed) % seen_slots < slots {
                chosen = Some((company_idx, opening_idx, seen_slots));
            }
        }
    }

    let (company_idx, opening_idx, _) = chosen?;
    let company = companies.get_mut(company_idx)?;
    let job_id = company.consume_opening(opening_idx)?;
    Some((company_idx as u16, job_id))
}
