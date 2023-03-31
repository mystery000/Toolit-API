use crate::fault::Fault;
use crate::models::{Bid, Claims, Currency, Payment, PaymentMethod, PaymentState, Task, User};
use crate::util::{DataResponse, Empty};
use crate::{
    BASE_CALLBACK_URL, BID_COLLECTION, PAYMENT_COLLECTION, SWISH_CERT_PASS, SWISH_CERT_PATH,
    SWISH_INTERMEDIATE_ACCOUNT_NUMBER, TASK_COLLECTION, USER_COLLECTION,
};
use chrono::Utc;
use cosmos_utils::{get, insert, query};
use serde::Serialize;
use swish::SwishClient;
use tokio::join;
use uuid::Uuid;
use warp::reject;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    #[serde(skip_serializing_if = "crate::util::is_none")]
    payment_request_token: Option<String>,
    payment_id: String,
}

pub async fn bid_accept(
    office_id: String,
    task_id: String,
    bid_id: String,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let q = format!(
        r#"SELECT * FROM {} p WHERE p.bidId = "{}""#,
        PAYMENT_COLLECTION, bid_id
    );
    let (t, b, p) = join!(
        get(TASK_COLLECTION, [&office_id], &task_id),
        get(BID_COLLECTION, [&office_id], &bid_id),
        query(PAYMENT_COLLECTION, [&office_id], q, -1)
    );
    let (task, _): (Task, _) = t?;
    let (bid, _): (Bid, _) = b?;
    let payments: Vec<Payment> = p?;
    let (task_owner, _): (User, _) = get(USER_COLLECTION, [&task.user_id], &task.user_id).await?;
    for payment in payments {
        match payment.payment_state {
            PaymentState::Error(_) => (),
            PaymentState::Failed => (),
            _ => {
                return Err(reject::custom(Fault::Duplicate(format!(
                    "A payment for this bid is either already in progress or has been completed",
                ))));
            }
        }
    }

    if claims.sub != task.user_id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Only the task poster may accept bids",
        ))));
    }
    if let Some(bid_id) = task.accepted_bid {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Already accepted a bid from bid id {}",
            bid_id
        ))));
    }

    // NOTE: We need to format the ID as a simple string without hyphens in order for swish to
    // accept it. It also *MUST* be upper-case letters.
    let payment_id = Uuid::new_v4();

    let payment = Payment {
        id: payment_id.to_string(),
        office_id: office_id.clone(),
        task_id: task_id.clone(),
        bid_id: bid_id.clone(),
        craftsman_id: bid.craftsman_id,
        swish_payment_id: None,
        payment_date: None,
        payment_state: PaymentState::Initialized,
        payment_method: PaymentMethod::Swish,
        amount: bid.final_bid,
        currency: Currency::SEK,
        modified: Utc::now(),
        deleted: false,
    };

    let mut uuid_encode_buf = Uuid::encode_buffer();
    let id = &payment_id.to_simple().encode_upper(&mut uuid_encode_buf);
    // Make payment to intermediate account, will create a payment on success
    // https://toolit-api-play.azurewebsites.net/offices/<id>/tasks/<id>/bids/<id>/payments/<id>
    let callback_url = format!(
        "{}/offices/{}/tasks/{}/bids/{}/payments/{}/escrow",
        BASE_CALLBACK_URL.as_str(),
        office_id,
        task_id,
        bid_id,
        payment_id.to_string()
    );

    // FIXME(Jonathan): This is old code from when we sent a phone number
    //// Swish requires a special format, like MSISDN but without a plus sign.
    //let mut payer_alias = task_owner.phone.clone();
    //if payer_alias.starts_with('+') {
    //    payer_alias = payer_alias.chars().into_iter().skip(1).collect();
    //}

    // Initialize the swish payment
    let swish_client = SwishClient::new(SWISH_CERT_PATH.as_str(), &SWISH_CERT_PASS, None, None)
        .await
        .map_err(|e| Fault::from(e))?;
    // NOTE: We need to format the ID as a simple string without hyphens in order for swish to
    // accept it. It also *MUST* be upper-case letters.
    let payment_req = swish::PaymentRequest::V2(swish::PaymentRequestV2 {
        id,
        callback_url: &callback_url,
        payee_alias: &SWISH_INTERMEDIATE_ACCOUNT_NUMBER,
        amount: bid.final_bid,
        currency: swish::Currency::SEK,
        payee_payment_reference: None,
        // NOTE: We are setting payer-alias to None in order to use the m-commerce feature and get
        // a token back
        // TODO(Jonathan): Remove the commented line below, it's only useful if we don't want the
        // autostarttoken
        //payer_alias: Some(&task_owner.phone),
        payer_alias: None,
        payer_ssn: Some(&task_owner.nid),
        //payer_ssn: &payer.nid,
        payer_age_limit: None,
        message: Some(format!("Betalning för avslutat toolit arbete")),
    });

    let resp = swish_client
        .payment_request(payment_req)
        .await
        .map_err(|e| Fault::from(e))?;

    // Insert initialized payment
    insert(PAYMENT_COLLECTION, [&payment.office_id], &payment, None).await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(&Response {
            payment_request_token: resp.payment_request_token,
            payment_id: payment_id.to_string(),
        }),
        extra: None::<Empty>,
    }))
}
