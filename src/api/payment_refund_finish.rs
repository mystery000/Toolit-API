use crate::fault::Fault;
use crate::models::{Payment, PaymentState};
use crate::util::{log, DataResponse, Empty};
use crate::{PAYMENT_COLLECTION, SWISH_CERT_PASS, SWISH_CERT_PATH};
use chrono::Utc;
use cosmos_utils::modify;
use swish::PaymentStatus;
use uuid::Uuid;

// This endpoint marks a payment as refunded to the task owner
pub async fn payment_refund_finish(
    office_id: String,
    _task_id: String,
    _bid_id: String,
    payment_id: String,
    refund_id: String,
    _untrusted: swish::RefundObject,
) -> Result<impl warp::Reply, warp::Rejection> {
    // NOTE: Anyone could call this endpoint with any information whatsoever. We call up Swish to
    // make sure what the actual status of the payment is
    let swish_client =
        swish::SwishClient::new(SWISH_CERT_PATH.as_str(), &SWISH_CERT_PASS, None, None)
            .await
            .map_err(|e| Fault::from(e))?;
    let swish_refund_object = match swish_client.refund_retrieve_from_id(&refund_id).await {
        Ok(r) => r,
        Err(e) => {
            log(format!("Could not recieve the refund id from swish"));
            return Err(warp::reject::custom(Fault::from(e)));
        }
    };
    // NOTE: Make sure that this refund is for the same payment as is provided in the url. This is
    // security critical.
    // The payment reference is in the form of a uppercase simple UUID
    match swish_refund_object.payer_payment_reference {
        Some(p) => {
            let p = match Uuid::parse_str(&p) {
                Ok(p) => p,
                Err(_) => {
                    log(format!(
                        "Could not parse the refund payer payment reference as a UUID"
                    ));
                    return Ok(warp::reply::json(&DataResponse {
                        data: None::<Empty>,
                        extra: None::<Empty>,
                    }));
                }
            };
            let p = p.to_string();
            if payment_id != p {
                log(format!(
                    "Swish refund payment ID: {} does not match the provided payment ID: {}",
                    p, payment_id
                ));
                return Ok(warp::reply::json(&DataResponse {
                    data: None::<Empty>,
                    extra: None::<Empty>,
                }));
            }
        }
        None => {
            log(format!(
            "Could not find a payer_payment_reference in the swish refund object. Internal error likely originating in the refund creation."
            ));
            return Ok(warp::reply::json(&DataResponse {
                data: None::<Empty>,
                extra: None::<Empty>,
            }));
        }
    };

    match swish_refund_object.status {
        PaymentStatus::PAID => {
            modify(
                PAYMENT_COLLECTION,
                [&office_id],
                &payment_id,
                |mut payment: Payment| {
                    payment.payment_state = PaymentState::Refunded;
                    payment.modified = Utc::now();
                    Ok(payment)
                },
            )
            .await?;
        }
        PaymentStatus::DECLINED => {
            modify(
                PAYMENT_COLLECTION,
                [&office_id],
                &payment_id,
                |mut payment: Payment| {
                    payment.payment_state = PaymentState::RefundFailed;
                    payment.modified = Utc::now();
                    Ok(payment)
                },
            )
            .await?;
        }
        PaymentStatus::ERROR => {
            modify(
                PAYMENT_COLLECTION,
                [&office_id],
                &payment_id,
                |mut payment: Payment| {
                    payment.payment_state = PaymentState::RefundFailed;
                    payment.modified = Utc::now();
                    Ok(payment)
                },
            )
            .await?;
        }
        PaymentStatus::CREATED => {}
    };
    Ok(warp::reply::json(&DataResponse {
        data: None::<Empty>,
        extra: None::<Empty>,
    }))
}
