use crate::fault::Fault;
use crate::models::{Bid, Claims, Currency, Payment, PaymentState, RoleFlags};
use crate::util::{has_role, DataResponse, Empty};
use crate::{
    BASE_CALLBACK_URL, BID_COLLECTION, PAYMENT_COLLECTION, SWISH_CERT_PASS, SWISH_CERT_PATH,
    SWISH_INTERMEDIATE_ACCOUNT_NUMBER,
};
use cosmos_utils::get;
use swish::SwishClient;
use uuid::Uuid;
use warp::reject::custom;

pub async fn payment_refund_init(
    office_id: String,
    task_id: String,
    bid_id: String,
    payment_id: String,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    if !has_role(Some(&office_id), &claims, RoleFlags::OFFICE_BILLING_ADMIN) {
        return Err(custom(Fault::Forbidden(format!(
            "Needs to be an office billing admin to refund payment",
        ))));
    }

    let (bid, _): (Bid, _) = get(BID_COLLECTION, [&office_id], &bid_id).await?;
    let (payment, _): (Payment, _) = get(PAYMENT_COLLECTION, [&office_id], &payment_id).await?;
    if !bid.is_cancelled {
        return Err(custom(Fault::Forbidden(format!(
            "May only refund payments for cancelled bids"
        ))));
    }

    match payment.payment_state {
        PaymentState::PaidToEscrow => {}
        s => {
            return Err(custom(Fault::Forbidden(format!(
                "Payment can not be refunded if it has not been sent to escrow instead it is {:?}",
                s
            ))));
        }
    }

    let mut uuid_encode_buf = Uuid::encode_buffer();
    let refund_id = Uuid::new_v4();
    let refund_id = refund_id.to_simple().encode_upper(&mut uuid_encode_buf);

    // NOTE we embed the refund id in the url in order to allow lookup by the finish function
    let callback_url = format!(
        "{}/offices/{}/tasks/{}/bids/{}/payments/{}/refund/{}/finish",
        &*BASE_CALLBACK_URL, office_id, task_id, bid_id, payment_id, refund_id
    );

    let simple_payment_id = Uuid::parse_str(&payment_id).map_err(|_| {
        custom(Fault::IllegalState(format!(
            "Could not parse payment id as a UUID"
        )))
    })?;
    let mut uuid_encode_buf = Uuid::encode_buffer();
    let simple_payment_id = simple_payment_id
        .to_simple()
        .encode_upper(&mut uuid_encode_buf);

    // Init the refund process in swish
    let swish_client = SwishClient::new(SWISH_CERT_PATH.as_str(), &SWISH_CERT_PASS, None, None)
        .await
        .map_err(|e| warp::reject::custom(Fault::from(e)))?;
    let refund_req = swish::RefundRequest {
        // NOTE: We set the payer_payment_reference to the payment id in order to use this in the
        // finishing step of the refund
        payer_payment_reference: Some(&simple_payment_id),
        original_payment_reference: &payment.swish_payment_id.ok_or_else(|| {
            warp::reject::custom(Fault::IllegalState(format!(
                "No swish ID not found in payment"
            )))
        })?,
        callback_url: &callback_url,
        payer_alias: &*SWISH_INTERMEDIATE_ACCOUNT_NUMBER,
        payee_alias: None,
        amount: payment.amount,
        currency: match payment.currency {
            Currency::SEK => swish::Currency::SEK,
        },
        message: Some(format!("Refund due to cancelled job")),
        instruction_uuid: Some(&refund_id),
    };
    swish_client
        .refund_request(refund_req, swish::Version::V2)
        .await
        .map_err(|e| Fault::from(e))?;

    Ok(warp::reply::json(&DataResponse {
        data: None::<Empty>,
        extra: None::<Empty>,
    }))
}
