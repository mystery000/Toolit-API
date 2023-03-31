//use crate::fault::Fault;
//use crate::models::{Bid, Claims, Payment, Task};
//use cosmos_utils::get;
//use crate::util::{DataRequest, DataResponse, Empty};
//use crate::{
//    BASE_CALLBACK_URL, BID_COLLECTION, SWISH_CERT_PASS, SWISH_CERT_PATH,
//    SWISH_RECEIVE_PAYMENT_NUMBER, TASK_COLLECTION, USER_COLLECTION,
//};
//use serde::Serialize;
//use swish::SwishClient;
//use uuid::Uuid;
//use warp::reject;
//
////#[derive(Serialize)]
////struct Response {
////    token: String,
////    payment: Payment,
////}
//
//pub async fn payment_init(
//    office_id: String,
//    task_id: String,
//    bid_id: String,
//    r: DataRequest<Payment, Empty>,
//    claims: Claims,
//    _v: u8,
//) -> Result<impl warp::Reply, warp::Rejection> {
//    let mut payment;
//    if let Some(q) = r.data {
//        payment = q;
//    } else {
//        return Err(reject::custom(Fault::NoData));
//    }
//
//    if payment.office_id != office_id {
//        return Err(reject::custom(Fault::IllegalArgument(format!(
//            "office_id does not match url ({} != {}).",
//            payment.office_id, office_id
//        ))));
//    }
//
//    if payment.task_id != task_id {
//        return Err(reject::custom(Fault::IllegalArgument(format!(
//            "task_id does not match url ({} != {}).",
//            payment.task_id, task_id
//        ))));
//    }
//
//    if payment.bid_id != bid_id {
//        return Err(reject::custom(Fault::IllegalArgument(format!(
//            "bid_id does not match url ({} != {}).",
//            payment.bid_id, bid_id
//        ))));
//    }
//
//    let (task_r, bid_r) = tokio::join!(
//        get(TASK_COLLECTION, [&office_id], &task_id),
//        get(BID_COLLECTION, [&office_id], &bid_id)
//    );
//    let (task, _): (Task, _) = task_r?;
//    let (bid, _): (Bid, _) = bid_r?;
//    //FIXME(Jonathan): Do we want to use the payer in order to get e-commerce rather than
//    //m-commerce?
//    //let (payer, _): (User, _) = get(USER_COLLECTION, [&task.user_id], &task.user_id).await?;
//
//    // Verification
//    if bid.task_id != task.id {
//        return Err(reject::custom(Fault::IllegalArgument(format!(
//            "Bid does not point at that task",
//        ))));
//    }
//    if let Some(bid_id) = task.accepted_bid {
//        if bid_id != bid.id {
//            return Err(reject::custom(Fault::IllegalArgument(format!(
//                "Bid does not point at that task",
//            ))));
//        }
//    } else {
//        return Err(reject::custom(Fault::IllegalArgument(format!(
//            "Task has not accepted a bid",
//        ))));
//    }
//    if !task.finished {
//        return Err(reject::custom(Fault::IllegalArgument(format!(
//            "Task is not finished and you can not pay until it is",
//        ))));
//    }
//    //NOTE(Jonathan): Right now you are only allowed to pay if you're the bidder. However this might be
//    //changed. Additionally, it should be possible for anyone to pay by swish regardless.
//    //This is mostly here to have as small of a whitelist as possible around payment in order to
//    //minimize exploitable bugs.
//    if task.user_id != claims.sub {
//        return Err(reject::custom(Fault::IllegalArgument(format!(
//            "Payer is not the original task-owner",
//        ))));
//    }
//
//    payment.is_paid = false;
//    let uuid = Uuid::new_v4();
//    payment.id = uuid.to_string();
//    payment.modified = chrono::Utc::now();
//
//    // https://toolit-api-play.azurewebsites.net/offices/<id>/tasks/<id>/bids/<id>/payments/<id>
//    let callback_url = format!(
//        "{}/offices/{}/tasks/{}/bids/{}/payments/{}",
//        BASE_CALLBACK_URL.as_str(),
//        office_id,
//        task_id,
//        bid_id,
//        payment.id
//    );
//
//    // Initialize the swish payment
//    let swish_client = SwishClient::new(SWISH_CERT_PATH.as_str(), &SWISH_CERT_PASS)
//        .await
//        .map_err(|e| Fault::from(e))?;
//    // NOTE: We need to format the ID as a simple string without hyphens in order for swish to
//    // accept it. It also *MUST* be upper-case letters.
//    let mut uuid_encode_buf = Uuid::encode_buffer();
//    let id = &uuid.to_simple().encode_upper(&mut uuid_encode_buf);
//    let payment_req = swish::PaymentRequest::V2(swish::PaymentRequestV2 {
//        id,
//        callback_url: &callback_url,
//        payee_alias: &SWISH_RECEIVE_PAYMENT_NUMBER,
//        amount: bid.final_bid,
//        currency: swish::Currency::SEK,
//        payee_payment_reference: None,
//        // NOTE: We are setting payer-alias to None in order to use the m-commerce feature and get
//        // a token back
//        payer_alias: None,
//        payer_ssn: None,
//        //payer_ssn: &payer.nid,
//        payer_age_limit: None,
//        message: Some(format!("Betalning för avslutat toolit arbete")),
//    });
//    let resp = swish_client
//        .payment_request(payment_req)
//        .await
//        .map_err(|e| Fault::from(e))?;
//    if resp.payment_request_token.is_none() {
//        return Err(warp::reject::custom(Fault::IllegalState(format!(
//            "Did not receive a payment request token from swish"
//        ))));
//    }
//    let payment_request_token = resp.payment_request_token.unwrap();
//
//    Ok(warp::reply::json(&DataResponse {
//        //data: Some(Response {
//        //    token: payment_request_token,
//        //}),
//        data: Some(payment_request_token),
//        extra: None::<Empty>,
//    }))
//}
