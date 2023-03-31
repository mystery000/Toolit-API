use crate::util::{is_false, log};
use chrono::{DateTime, Utc};
use rust_decimal::prelude::{Decimal, RoundingStrategy, Zero};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Bid {
    #[serde(default)]
    pub id: String,

    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    pub deleted: bool,

    pub office_id: String,

    pub task_id: String,

    pub craftsman_id: String,

    pub bid_message: String,

    // The total price including the tax and removing the root deduction. This is what the customer
    // is supposed to swish
    pub final_bid: Decimal,

    // The total root deduction
    pub root_deduction: Decimal,

    // Material cost without the tax added
    pub material_cost: Decimal,

    // Labour cost without the tax added
    pub labour_cost: Decimal,

    // Total added tax
    pub vat: Decimal,

    pub is_cancelled: bool,

    pub modified: DateTime<Utc>,
}

impl Bid {
    // NOTE: Make sure the final price is correct
    // We round to 2 decimals at every step
    pub fn cost_is_correct(&self, use_rut_root: bool) -> bool {
        // MAGIC NUMBER: 25% for the vat tax
        let vat_percentage = Decimal::new(25, 2);
        // MAGIC NUMBER: 30% for the root deduction
        let root_percentage: Decimal = if use_rut_root {
            Decimal::new(3, 1)
        } else {
            Decimal::zero()
        };

        let labour_cost = self
            .labour_cost
            .round_dp_with_strategy(2, RoundingStrategy::BankersRounding);
        let material_cost = self
            .material_cost
            .round_dp_with_strategy(2, RoundingStrategy::BankersRounding);

        let labour_cost_vat = (labour_cost * vat_percentage)
            .round_dp_with_strategy(2, RoundingStrategy::BankersRounding);
        let labour_cost_inc_vat = labour_cost + labour_cost_vat;

        let material_cost_vat = (material_cost * vat_percentage)
            .round_dp_with_strategy(2, RoundingStrategy::BankersRounding);
        let material_cost_inc_vat = material_cost + material_cost_vat;

        let root_deduction = (labour_cost_inc_vat * root_percentage)
            .round_dp_with_strategy(2, RoundingStrategy::BankersRounding);
        let vat = (labour_cost_vat - (labour_cost_vat * root_percentage)) + material_cost_vat;
        let final_bid = labour_cost_inc_vat + material_cost_inc_vat - root_deduction;

        if self.final_bid != final_bid {
            log(format!(
                "The final bid should have been {} but it was {}",
                final_bid, self.final_bid
            ));
            return false;
        }

        if self.vat != vat {
            log(format!(
                "The vat should have been {} but it was {}",
                vat, self.vat
            ));
            return false;
        }

        if self.root_deduction != root_deduction {
            log(format!(
                "The root deduction should have been {} but it was {}",
                root_deduction, self.root_deduction
            ));
            return false;
        }

        true
    }
}
