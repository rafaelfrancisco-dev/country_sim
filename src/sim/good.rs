pub type GoodId = u8;
pub const GOODS_COUNT: usize = 15;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Good {
    pub id: GoodId,
    pub name: &'static str,
    pub necessity_rank: u8,
    pub effort_per_unit: f64,
    pub monthly_demand_factor: f64,
    pub base_unit_cost: f64,
}

const COMMON_GOODS: [Good; GOODS_COUNT] = [
    Good {
        id: 0,
        name: "Water",
        necessity_rank: 1,
        effort_per_unit: 2.5,
        monthly_demand_factor: 4.0,
        base_unit_cost: 6.0,
    },
    Good {
        id: 1,
        name: "Food",
        necessity_rank: 2,
        effort_per_unit: 3.0,
        monthly_demand_factor: 3.2,
        base_unit_cost: 14.0,
    },
    Good {
        id: 2,
        name: "Housing",
        necessity_rank: 3,
        effort_per_unit: 12.0,
        monthly_demand_factor: 1.0,
        base_unit_cost: 520.0,
    },
    Good {
        id: 3,
        name: "Electricity",
        necessity_rank: 4,
        effort_per_unit: 6.5,
        monthly_demand_factor: 1.1,
        base_unit_cost: 95.0,
    },
    Good {
        id: 4,
        name: "Medicine",
        necessity_rank: 5,
        effort_per_unit: 5.0,
        monthly_demand_factor: 0.9,
        base_unit_cost: 60.0,
    },
    Good {
        id: 5,
        name: "Clothing",
        necessity_rank: 6,
        effort_per_unit: 4.0,
        monthly_demand_factor: 0.8,
        base_unit_cost: 38.0,
    },
    Good {
        id: 6,
        name: "Hygiene Products",
        necessity_rank: 7,
        effort_per_unit: 3.3,
        monthly_demand_factor: 1.0,
        base_unit_cost: 22.0,
    },
    Good {
        id: 7,
        name: "Transport",
        necessity_rank: 8,
        effort_per_unit: 5.5,
        monthly_demand_factor: 1.0,
        base_unit_cost: 70.0,
    },
    Good {
        id: 8,
        name: "Internet",
        necessity_rank: 9,
        effort_per_unit: 4.5,
        monthly_demand_factor: 1.0,
        base_unit_cost: 44.0,
    },
    Good {
        id: 9,
        name: "Education Services",
        necessity_rank: 10,
        effort_per_unit: 8.0,
        monthly_demand_factor: 0.5,
        base_unit_cost: 130.0,
    },
    Good {
        id: 10,
        name: "Household Goods",
        necessity_rank: 11,
        effort_per_unit: 4.8,
        monthly_demand_factor: 0.7,
        base_unit_cost: 58.0,
    },
    Good {
        id: 11,
        name: "Electronics",
        necessity_rank: 12,
        effort_per_unit: 9.5,
        monthly_demand_factor: 0.4,
        base_unit_cost: 180.0,
    },
    Good {
        id: 12,
        name: "Appliances",
        necessity_rank: 13,
        effort_per_unit: 11.0,
        monthly_demand_factor: 0.3,
        base_unit_cost: 240.0,
    },
    Good {
        id: 13,
        name: "Cars",
        necessity_rank: 14,
        effort_per_unit: 24.0,
        monthly_demand_factor: 0.12,
        base_unit_cost: 2_000.0,
    },
    Good {
        id: 14,
        name: "Entertainment",
        necessity_rank: 15,
        effort_per_unit: 4.2,
        monthly_demand_factor: 1.2,
        base_unit_cost: 32.0,
    },
];

pub fn common_goods() -> &'static [Good; GOODS_COUNT] {
    &COMMON_GOODS
}

pub fn good_by_id(id: GoodId) -> Option<&'static Good> {
    COMMON_GOODS.iter().find(|good| good.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exactly_fifteen_common_goods_exist() {
        assert_eq!(common_goods().len(), 15);
    }

    #[test]
    fn goods_ids_are_unique() {
        let mut ids = common_goods()
            .iter()
            .map(|good| good.id)
            .collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();

        assert_eq!(ids.len(), 15);
    }

    #[test]
    fn necessity_rank_is_strictly_increasing() {
        let ranks = common_goods()
            .iter()
            .map(|good| good.necessity_rank)
            .collect::<Vec<_>>();
        let mut sorted = ranks.clone();
        sorted.sort_unstable();
        sorted.dedup();

        assert_eq!(ranks, sorted);
        assert_eq!(ranks.first().copied(), Some(1));
        assert_eq!(ranks.last().copied(), Some(15));
    }
}
