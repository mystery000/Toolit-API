use crate::fault::Fault;
use crate::models::{Craftsman, Payment, PaymentState, Task, User};
use crate::push::send_custom_pn;
use crate::util::{log, DataResponse, Empty};
use crate::{
    CRAFTSMAN_COLLECTION, NOTIFICATION_HUB_ACCOUNT, PAYMENT_COLLECTION, SWISH_CERT_PASS,
    SWISH_CERT_PATH, TASK_COLLECTION, USER_COLLECTION,
};
use chrono::Utc;
use cosmos_utils::{get, modify};
use uuid::Uuid;
use warp::reject;

// This endpoint should only ever be called by swish
pub async fn payment_escrow(
    office_id: String,
    task_id: String,
    bid_id: String,
    payment_id: String,
    _untrusted: Option<swish::PaymentObject>,
) -> Result<impl warp::Reply, warp::Rejection> {
    // NOTE: Anyone could call this endpoint with any information whatsoever. We call up Swish to
    // make sure what the actual status of the payment is

    // NOTE: When sending payment IDs to swish the ID has to be a simple UUID with all uppercase
    // characters
    let simple_payment_id = Uuid::parse_str(&payment_id).map_err(|_| {
        warp::reject::custom(Fault::IllegalState(format!(
            "Could not parse payment id as a UUID"
        )))
    })?;
    let mut uuid_encode_buf = Uuid::encode_buffer();
    let simple_payment_id = simple_payment_id
        .to_simple()
        .encode_upper(&mut uuid_encode_buf);
    let swish_client =
        swish::SwishClient::new(SWISH_CERT_PATH.as_str(), &SWISH_CERT_PASS, None, None)
            .await
            .map_err(|e| Fault::from(e))?;
    let swish_payment_object = swish_client
        .payment_retrieve_from_id(&simple_payment_id)
        .await
        .map_err(|e| Fault::from(e))?;

    if swish_payment_object.id.as_str() != simple_payment_id {
        return Err(reject::custom(Fault::IllegalArgument(format!(
            "Payment id does not mach provided payment"
        ))));
    }

    let payment = match swish_payment_object.status {
        swish::PaymentStatus::PAID => {
            // TODO(Jonathan): make this into maybe_modify
            match modify(PAYMENT_COLLECTION, [&office_id], &payment_id, |mut payment: Payment| {
                // TODO(Jonathan): Verify the correct task_id and bid_id
                if payment.amount == swish_payment_object.amount {
                    match payment.payment_state {
                        PaymentState::Initialized => {
                            payment.payment_state = PaymentState::PaidToEscrow;
                            payment.payment_date = Some(Utc::now());
                            payment.swish_payment_id = Some(swish_payment_object.payment_reference.clone());
                            payment.modified = chrono::Utc::now();
                            return Ok(payment);
                        },
                        s => {
                            return Err(reject::custom(Fault::Unspecified(format!(
                                            "Payment could not go through as it was not in the initialzed state but rather in {:?}",
                                            s))));
                        }
                    };
                } else {
                    return Err(reject::custom(Fault::Unspecified(format!(
                                    "Payment was not successful due to incorrect amount"
                                    ))));
                }
            }).await {
                Ok(r) => r,
                Err(e) => {
                    log(format!("Attempt to finish escrow payment failed due to {}", e));
                    // Return OK in order to not require Swish to continually resend.
                    return Ok(warp::reply::json(&DataResponse {
                        data: None::<Empty>,
                        extra: None::<Empty>,
                    }));
                },
            }
        }
        swish::PaymentStatus::CREATED => {
            log(format!("Attempt to finish escrow payment failed due to payment being created but not finished"));
            // Return OK in order to not require Swish to continually resend.
            return Ok(warp::reply::json(&DataResponse {
                data: None::<Empty>,
                extra: None::<Empty>,
            }));
        }
        _ => {
            match modify(PAYMENT_COLLECTION, [&office_id], &payment_id, |mut payment: Payment| {
                match payment.payment_state {
                    PaymentState::Initialized => {
                        payment.payment_state = PaymentState::Failed;
                        payment.modified = chrono::Utc::now();
                        return Ok(payment);
                    },
                    s => {
                        return Err(reject::custom(Fault::Unspecified(format!(
                                        "Payment could not be marked as failed as it was not Initialized but rather {:?}",
                                        s))));
                    }
                };
            }).await {
                Ok(_) => {
                    log(format!("A payment failed with id {}", payment_id));
                    // Return OK in order to not require Swish to continually resend.
                    return Ok(warp::reply::json(&DataResponse {
                        data: None::<Empty>,
                        extra: None::<Empty>,
                    }));
                },
                Err(e) => {
                    log(format!("Attempt to mark payment as failed errored due to {}", e));
                    // Return OK in order to not require Swish to continually resend.
                    return Ok(warp::reply::json(&DataResponse {
                        data: None::<Empty>,
                        extra: None::<Empty>,
                    }));
                },
            }
        }
    };

    // Set the task to have accepted this bid
    match modify(TASK_COLLECTION, [&office_id], &task_id, |mut task: Task| {
        if let Some(prev_accepted) = task.accepted_bid {
            log(format!("Accepted a bid when one was already accepted, new bid: {}, old bid: {}. This should not be possible", bid_id.clone(), prev_accepted));
        }
        task.accepted_bid = Some(bid_id.clone());
        task.payment_id = Some(payment.id.clone());
        task.modified = Utc::now();
        Ok(task)
    }).await {
        Ok(_) => (),
        Err(e) => {
            log(format!("Could not change task after payment has gone through to escrow, this is a critical error in payment_escrow due to {}", e));
            // Return OK in order to not require Swish to continually resend.
            return Ok(warp::reply::json(&DataResponse {
                data: None::<Empty>,
                extra: None::<Empty>,
            }));
        },
    };

    // We run the code necessary to send out a PN in a separate thread in order to quicker give a
    // response. This code should not cause a failure anyway.
    tokio::task::spawn(async move {
        let (c, t) = tokio::join!(
            async {
                let (craftsman, _): (Craftsman, _) =
                    get(CRAFTSMAN_COLLECTION, [&office_id], &payment.craftsman_id).await?;
                let (user, _): (User, _) =
                    get(USER_COLLECTION, [&craftsman.user_id], &craftsman.user_id).await?;
                Result::<_, warp::Rejection>::Ok((craftsman, user))
            },
            async {
                let (task, _): (Task, _) = get(TASK_COLLECTION, [&office_id], &task_id).await?;
                let (user, _): (User, _) =
                    get(USER_COLLECTION, [&task.user_id], &task.user_id).await?;
                Result::<_, warp::Rejection>::Ok((task, user))
            }
        );
        let (_craftsman, craftsman_user): (Craftsman, User) = match c {
            Ok(r) => r,
            Err(e) => {
                log(format!(
                    "Could not send PN in payment_escrow since cosmos get failed with {:?}",
                    e
                ));
                return ();
            }
        };
        let (_task, task_owner): (Task, User) = match t {
            Ok(r) => r,
            Err(e) => {
                log(format!(
                    "Could not send PN in payment_escrow since cosmos get failed with {:?}",
                    e
                ));
                return ();
            }
        };

        // Send PN to the craftsman
        if let Err(e) = send_custom_pn(
            &craftsman_user,
            &format!("Jaa! {} har accepterat ditt bud!", task_owner.name()),
            None,
            &NOTIFICATION_HUB_ACCOUNT,
        )
        .await
        {
            log(format!("Could not send PN in payment_post due to {}", e));
        }
        // Send PN to the payer
        if let Err(e) = send_custom_pn(
            &task_owner,
            &format!(
                "Snyggt! Din betalning till {} gick igenom!",
                craftsman_user.name()
            ),
            None,
            &NOTIFICATION_HUB_ACCOUNT,
        )
        .await
        {
            log(format!("Could not send PN in payment_post due to {}", e));
        }
    });

    //// Make date from IANA location.
    //let tz: Tz = Stockholm;
    //let today = Utc::now();
    //let today = today.with_timezone(&tz);

    //// Send receipt email to task owner.
    //let mut map = SGMap::new();
    //map.insert(String::from("userFirstName"), task_owner.first_name.clone());
    //map.insert(String::from("userLastName"), task_owner.last_name.clone());
    //map.insert(String::from("craftmanCompanyName"), craftsman.company_name.clone());
    //map.insert(String::from("craftmanCompanyNid"), craftsman.org_number.clone());
    //map.insert(String::from("craftsmanFirstName"), craftsman_user.first_name.clone());
    //map.insert(String::from("craftsmanLastName"), craftsman_user.last_name.clone());
    //map.insert(String::from("jobTitle"), task.title.clone());
    //map.insert(String::from("jobTransactionDate"), today.to_rfc3339());

    //map.insert(String::from("jobFinishDate"), task_owner.last_name.clone());
    //map.insert(String::from("jobCraftmanPrice"), task.last_name.clone());
    //map.insert(String::from("craftmanMaterialCost"), task_owner.last_name.clone());
    //map.insert(String::from("rotRutAvdrag"), task_owner.last_name.clone());
    //map.insert(String::from("vat"), task_owner.last_name.clone());
    //map.insert(String::from("jobSum"), task_owner.last_name.clone());

    //let p = Personalization::new(Email::new("support@toolitapp.com"))
    //    .add_to(Email::new(&task_owner.email))
    //    .add_dynamic_template_data(map);

    //let m = Message::new(Email::new("support@toolitapp.com"))
    //    .set_template_id("d-e734aff83ce247a3b6ce0b30306b17c9")
    //    .add_personalization(p);
    //let sender = Sender::new(SENDGRID_API_KEY.to_string());
    //match sender.send(&m).await {
    //    Ok(_) => {}
    //    Err(_) => {}
    //};

    Ok(warp::reply::json(&DataResponse {
        data: None::<Empty>,
        extra: None::<Empty>,
    }))
}
