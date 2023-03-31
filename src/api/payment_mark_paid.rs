use crate::fault::Fault;
use crate::models::{Claims, Payment, PaymentState, RoleFlags};
use crate::util::{has_role, DataResponse, Empty};
use crate::PAYMENT_COLLECTION;
use chrono::Utc;
use cosmos_utils::modify;
use warp::reject::custom;

// This endpoint marks a payment as paid to the toolit craftsman
pub async fn payment_mark_paid(
    office_id: String,
    _task_id: String,
    _bid_id: String,
    payment_id: String,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    if !has_role(Some(&office_id), &claims, RoleFlags::OFFICE_BILLING_ADMIN) {
        return Err(custom(Fault::Forbidden(format!(
            "Needs to be an office billing admin to mark a payment as paid",
        ))));
    }

    let payment = modify(PAYMENT_COLLECTION, [&office_id], &payment_id, |mut payment: Payment| {
        match payment.payment_state {
            PaymentState::Finalized => {
                payment.payment_state = PaymentState::PaidToCraftsman;
                payment.modified = Utc::now();
                return Ok(payment);
            },
            s => {
                return Err(custom(Fault::Forbidden(
                        format!(
                            "Payment can not be marked as paid to craftsman before having been confirmed as recieved at final, current state {:?}",
                            s))));
            }
        };
    }).await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(&payment),
        extra: None::<Empty>,
    }))
}
