//use crate::fault::Fault;
//use crate::models::{Payment, PaymentState};
//use crate::util::{log, DataResponse, Empty};
//use crate::{PAYMENT_COLLECTION, SWISH_CERT_PASS, SWISH_CERT_PATH};
//use chrono::Utc;
//use cosmos_utils::modify;
//use swish::PayoutStatus;
//use uuid::Uuid;
//use warp::reject;
//
//// This endpoint should only ever be called by swish
//pub async fn payment_finalize(
//    office_id: String,
//    _task_id: String,
//    _bid_id: String,
//    payment_id: String,
//    _untrusted: swish::PayoutObject,
//) -> Result<impl warp::Reply, warp::Rejection> {
//    // TODO(Jonathan): This log can be removed later, it's here for debugging purposes
//    log(format!("payment_finalize called with {:?}", _untrusted));
//
//    // Get the payout object
//
//    // NOTE: We need to format the ID as a simple string without hyphens in order for swish to
//    // handle it. It also *MUST* be upper-case letters. We create two versions of the payment ID,
//    // one to handle in our own database and one to handle when comparing with swish.
//    let simple_payment_id = Uuid::parse_str(&payment_id).map_err(|_| {
//        warp::reject::custom(Fault::IllegalState(format!(
//            "Could not parse payment id as a UUID"
//        )))
//    })?;
//    let mut uuid_encode_buf = Uuid::encode_buffer();
//    let simple_payment_id = simple_payment_id
//        .to_simple()
//        .encode_upper(&mut uuid_encode_buf);
//    // NOTE: Anyone could call this endpoint with any information whatsoever. We call up Swish to
//    // make sure what the actual status of the payment is
//    let swish_client = swish::SwishClient::new(SWISH_CERT_PATH.as_str(), &SWISH_CERT_PASS, None, None)
//        .await
//        .map_err(|e| Fault::from(e))?;
//    let swish_payout_object = match swish_client
//        .payout_retrieve_from_id(&simple_payment_id)
//        .await
//    {
//        Ok(r) => r,
//        Err(e) => {
//            log(format!(
//                "Could not recieve payout object from swish due to {}",
//                e
//            ));
//            return Err(warp::reject::custom(Fault::from(e)));
//        }
//    };
//
//    // Validate the payout object
//
//    if &swish_payout_object.payout_instruction_uuid != simple_payment_id {
//        return Err(reject::custom(Fault::IllegalArgument(format!(
//            "Payout id does not mach provided payment"
//        ))));
//    }
//
//    match swish_payout_object.status {
//        PayoutStatus::PAID => {}
//        _ => {
//            log(format!(
//                "Payout does not have PAID status, instead it has {:?}",
//                swish_payout_object.status
//            ));
//            return Ok(warp::reply::json(&DataResponse {
//                data: None::<Empty>,
//                extra: None::<Empty>,
//            }));
//        }
//    }
//
//    modify(
//        PAYMENT_COLLECTION,
//        [&office_id],
//        &payment_id,
//        |mut payment: Payment| {
//            // Validate the 
//            if swish_payout_object.amount != payment.amount {
//                log(format!(
//                        "Payout is for the wrong amount, this should be impossible."
//                        ));
//                return Err(reject::custom(Fault::Duplicate(format!(
//                            "Payout is for the wrong amount, this should be impossible."))));
//            }
//
//            // Update the database payment
//            match payment.payment_state {
//                PaymentState::PaidToFinal => {
//                    payment.payment_state = PaymentState::RecievedAtFinal;
//                    payment.modified = Utc::now();
//                    Ok(payment)
//                }
//                s => {
//                    log(format!(
//                            "Tried to finalize payment that is not in the PaidToFinal state but instead is in {:?}.", s
//                            ));
//                    return Err(reject::custom(Fault::Duplicate(format!(
//                            "Tried to finalize payment that is not in the PaidToFinal state but instead is in {:?}.", s
//                            ))));
//                }
//            }
//        },
//    )
//    .await?;
//
//    Ok(warp::reply::json(&DataResponse {
//        data: None::<Empty>,
//        extra: None::<Empty>,
//    }))
//}
