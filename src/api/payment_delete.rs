use crate::fault::Fault;
use crate::models::{Claims, Payment, RoleFlags};
use crate::util::{has_role, DataResponse, Empty};
use crate::PAYMENT_COLLECTION;
use chrono::Utc;
use cosmos_utils::modify;
use warp::reject;

pub async fn payment_delete(
    office_id: String,
    task_id: String,
    payment_id: String,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let deleted_payment = modify(
        PAYMENT_COLLECTION,
        [&office_id],
        &payment_id,
        |mut payment: Payment| {
            if payment.office_id != office_id {
                return Err(reject::custom(Fault::IllegalArgument(format!(
                    "office_id does not match url ({} != {}).",
                    payment.office_id, office_id
                ))));
            }
            if payment.task_id != task_id {
                return Err(reject::custom(Fault::IllegalArgument(format!(
                    "task_id does not match url ({} != {}).",
                    payment.task_id, task_id
                ))));
            }

            if payment.id != payment_id {
                return Err(reject::custom(Fault::IllegalArgument(format!(
                    "payment_id does not match url ({} != {}).",
                    payment.id, payment_id
                ))));
            }

            if !has_role(Some(&office_id), &claims, RoleFlags::OFFICE_BILLING_ADMIN) {
                return Err(reject::custom(Fault::Forbidden(format!(
                    "User does not have sufficient roles."
                ))));
            }
            payment.deleted = true;
            payment.modified = Utc::now();
            Ok(payment)
        },
    )
    .await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(deleted_payment),
        extra: None::<Empty>,
    }))
}
