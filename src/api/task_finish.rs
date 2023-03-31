use crate::fault::Fault;
use crate::models::{Bid, Claims, Craftsman, Office, Payment, PaymentState, Task, User};
use crate::push::send_custom_pn;
use crate::util::{log, DataResponse, Empty};
use crate::{
    BID_COLLECTION, CRAFTSMAN_COLLECTION, NOTIFICATION_HUB_ACCOUNT, OFFICE_COLLECTION,
    PAYMENT_COLLECTION, SENDGRID_API_KEY, TASK_COLLECTION, USER_COLLECTION,
};
use chrono::Utc;
use chrono_tz::{Europe::Stockholm, Tz};
use cosmos_utils::{get, CosmosSaga};
use rust_decimal::{prelude::Zero, Decimal};
use sendgrid::v3::*;
use warp::reject;

// This endpoint is callable only by the task-owner
pub async fn task_finish(
    office_id: String,
    task_id: String,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    // TODO(Jonathan): Should be maybe_modify
    let mut saga = CosmosSaga::new();
    let task = saga
        .modify(
            TASK_COLLECTION,
            [&office_id],
            &task_id,
            |mut task: Task| async {
                if claims.sub != task.user_id {
                    return Err(reject::custom(Fault::Forbidden(format!(
                        "User is not the task owner"
                    ))));
                }
                if task.accepted_bid.is_none() {
                    return Err(reject::custom(Fault::Forbidden(format!(
                        "No bid has been accepted for this task"
                    ))));
                }
                // Idempotancy
                if !task.finished {
                    task.finished = true;
                    task.date_done = Some(chrono::Utc::now());
                    task.modified = chrono::Utc::now();
                }
                Ok(task)
            },
        )
        .await?;

    let bid_id = match &task.accepted_bid {
        Some(r) => r,
        None => {
            saga.abort().await?;
            return Err(warp::reject::custom(Fault::IllegalState(
                "Unreachable state".to_string(),
            )));
        }
    };

    let payment_date;
    if let Some(payment_id) = &task.payment_id {
        let payment = saga.modify(
            PAYMENT_COLLECTION,
            [&office_id],
            &payment_id,
            |mut payment: Payment| async {
                match payment.payment_state {
                    PaymentState::PaidToEscrow => {
                        payment.payment_state = PaymentState::Finalized;
                        payment.modified = chrono::Utc::now();
                        return Ok(payment);
                    },
                    s => {
                        return Err(reject::custom(Fault::Unspecified(format!(
                                        "Could not finish task as payment was not PaidToEscrow but {:?} instead",
                                        s))));
                    },
                };
            },
        )
        .await?;
        payment_date = match payment.payment_date {
            Some(date) => date,
            None => {
                saga.abort().await?;
                log(format!("Payment does not have a payment date set after payment has been paid to escrow, this should be impossible in task_finish"));
                return Err(reject::custom(Fault::IllegalState(format!("Payment does not have a payment date set after payment has been paid to escrow, this should be impossible in task_finish"))));
            }
        };
    } else {
        saga.abort().await?;
        log(format!("A task with an accepted bid does not have a payment id, this should be impossible in task_finish"));
        return Err(reject::custom(Fault::IllegalState(format!("A task with an accepted bid does not have a payment id, this should be impossible in task_finish"))));
    }
    saga.finalize().await;

    let (cm_r, to_r, office) = tokio::join!(
        async {
            let (bid, _): (Bid, _) = get(BID_COLLECTION, [&office_id], &bid_id).await?;
            let (craftsman, _): (Craftsman, _) =
                get(CRAFTSMAN_COLLECTION, [&office_id], &bid.craftsman_id).await?;
            let (craftsman_user, _): (User, _) =
                get(USER_COLLECTION, [&craftsman.user_id], &craftsman.user_id).await?;
            Result::<_, warp::Rejection>::Ok((bid, craftsman, craftsman_user))
        },
        async {
            let (task_owner, _): (User, _) =
                get(USER_COLLECTION, [&task.user_id], &task.user_id).await?;
            Result::<_, warp::Rejection>::Ok(task_owner)
        },
        async {
            let (office, _): (Office, _) = get(OFFICE_COLLECTION, [&office_id], &office_id).await?;
            Result::<_, warp::Rejection>::Ok(office)
        }
    );
    // TODO(Jonathan): Bump the "completed jobs" for craftsman
    let (bid, craftsman, craftsman_user): (Bid, Craftsman, User) = cm_r?;
    let task_owner = to_r?;
    let office = office?;

    // Send PN to the task owner
    if let Err(e) = send_custom_pn(
        &task_owner,
        &format!("Härligt, du har nu godkänt och avslutat ett av dina jobb!"),
        None,
        &NOTIFICATION_HUB_ACCOUNT,
    )
    .await
    {
        log(format!("Could not send PN in task_finish due to {}", e));
    }

    // Send PN to the craftsman
    if let Err(e) = send_custom_pn(
        &craftsman_user,
        &format!(
            "Bra jobbat! Nu har {} godkänt ett av jobben du hållit på med!",
            task_owner.name()
        ),
        None,
        &NOTIFICATION_HUB_ACCOUNT,
    )
    .await
    {
        log(format!("Could not send PN in task_finish due to {}", e));
    }

    // Make todays Stockholm date from IANA location.
    let tz: Tz = Stockholm;
    let today = Utc::now();
    let today = today.with_timezone(&tz);
    let today = today.to_rfc3339_opts(chrono::SecondsFormat::Secs, false);
    let payment_date = payment_date.with_timezone(&tz);
    let payment_date = payment_date.to_rfc3339_opts(chrono::SecondsFormat::Secs, false);

    // Send receipt email to task owner.
    let mut map = SGMap::new();
    map.insert(String::from("userFirstName"), task_owner.first_name.clone());
    map.insert(String::from("userLastName"), task_owner.last_name.clone());
    map.insert(
        String::from("craftmanCompanyName"),
        craftsman.company_name.clone(),
    );
    map.insert(
        String::from("craftmanCompanyNid"),
        craftsman.org_number.clone(),
    );
    map.insert(
        String::from("craftsmanFirstName"),
        craftsman_user.first_name.clone(),
    );
    map.insert(
        String::from("craftsmanLastName"),
        craftsman_user.last_name.clone(),
    );
    map.insert(String::from("jobTitle"), task.title.clone());
    map.insert(String::from("jobTransactionDate"), payment_date.clone());
    map.insert(String::from("jobFinishDate"), today.clone());
    map.insert(
        String::from("jobCraftmanPrice"),
        bid.labour_cost.to_string(),
    );
    map.insert(
        String::from("craftmanMaterialCost"),
        bid.material_cost.to_string(),
    );
    map.insert(
        String::from("rotavdrag"),
        if bid.root_deduction == Decimal::zero() {
            String::from("Ej applicerbart")
        } else {
            bid.root_deduction.to_string()
        },
    );
    map.insert(String::from("vat"), bid.vat.to_string());
    map.insert(String::from("jobSum"), bid.final_bid.to_string());

    let p = Personalization::new(Email::new("support@toolitapp.com"))
        .add_to(Email::new(&task_owner.email))
        .add_dynamic_template_data(map);

    let to_email = Message::new(Email::new("support@toolitapp.com"))
        .set_template_id("d-e734aff83ce247a3b6ce0b30306b17c9")
        .add_personalization(p);

    // Turn the crafts into a string of names
    let mut object_types = match task.crafts.get(0) {
        Some(c) => c.swedish_name(),
        None => String::new(),
    };
    for craft in task.crafts.iter().skip(1) {
        object_types.push_str(&format!(", {}", craft.swedish_name()));
    }

    // Send receipt email to craftsman
    let mut map = SGMap::new();
    map.insert(String::from("userFirstName"), task_owner.first_name.clone());
    map.insert(String::from("userLastName"), task_owner.last_name.clone());
    map.insert(String::from("userNid"), task_owner.nid.clone());
    map.insert(String::from("jobAddress"), task.address.clone());
    map.insert(String::from("jobPostalCode"), task.postcode.clone());
    map.insert(String::from("jobCity"), task.city.clone());
    map.insert(String::from("objectType"), object_types.clone());
    map.insert(
        String::from("rotRutAvdragChoice"),
        if task.use_rot_rut {
            String::from("ja")
        } else {
            String::from("nej")
        },
    );
    map.insert(
        String::from("houseDesignation"),
        task.property_designation
            .clone()
            .unwrap_or(String::from("N/A")),
    );
    map.insert(
        String::from("apartmentCooperative"),
        task.realestate_union.clone().unwrap_or(String::from("N/A")),
    );
    map.insert(
        String::from("apartmentNr"),
        task.apartment_number.clone().unwrap_or(String::from("N/A")),
    );
    map.insert(String::from("jobTitle"), task.title.clone());
    map.insert(String::from("jobTransactionDate"), payment_date.clone());
    map.insert(String::from("jobFinishDate"), today.clone());
    map.insert(
        String::from("jobCraftmanPrice"),
        bid.labour_cost.to_string(),
    );
    map.insert(
        String::from("craftmanMaterialCost"),
        bid.material_cost.to_string(),
    );
    map.insert(
        String::from("rotavdrag"),
        if bid.root_deduction == Decimal::zero() {
            String::from("Ej applicerbart")
        } else {
            bid.root_deduction.to_string()
        },
    );
    map.insert(String::from("jobSum"), bid.final_bid.to_string());

    // FIXME(Jonathan): How do I calculate the vat, brokerageFeeExVat, brokerageFeeVat,
    // brokerageFeeSum?
    map.insert(String::from("vat"), bid.vat.to_string());
    map.insert(
        String::from("brokerageFeeExVat"),
        ((bid.material_cost + bid.labour_cost) * office.brokerage_percentage).to_string(),
    );
    map.insert(
        String::from("brokerageFeeVat"),
        (bid.vat * office.brokerage_percentage).to_string(),
    );
    map.insert(
        String::from("brokerageFeeSum"),
        ((bid.material_cost + bid.labour_cost + bid.vat) * office.brokerage_percentage).to_string(),
    );

    let p = Personalization::new(Email::new("support@toolitapp.com"))
        .add_to(Email::new(&craftsman_user.email))
        .add_dynamic_template_data(map);

    let craftsman_email = Message::new(Email::new("support@toolitapp.com"))
        .set_template_id("d-1b5e3f4d1a9a46149b58a6d3c5b19daa")
        .add_personalization(p);

    // Send receipt email to toolit
    let mut map = SGMap::new();
    map.insert(String::from("userFirstName"), task_owner.first_name.clone());
    map.insert(String::from("userLastName"), task_owner.last_name.clone());
    map.insert(String::from("userNid"), task_owner.nid.clone());
    map.insert(String::from("jobAddress"), task.address.clone());
    map.insert(String::from("jobPostalCode"), task.postcode.clone());
    map.insert(String::from("jobCity"), task.city.clone());
    map.insert(String::from("objectType"), object_types);
    map.insert(
        String::from("rotRutAvdragChoice"),
        if task.use_rot_rut {
            String::from("ja")
        } else {
            String::from("nej")
        },
    );
    map.insert(
        String::from("houseDesignation"),
        task.property_designation
            .clone()
            .unwrap_or(String::from("N/A")),
    );
    map.insert(
        String::from("apartmentCooperative"),
        task.realestate_union.clone().unwrap_or(String::from("N/A")),
    );
    map.insert(
        String::from("apartmentNr"),
        task.apartment_number.clone().unwrap_or(String::from("N/A")),
    );
    map.insert(String::from("jobTitle"), task.title.clone());
    map.insert(String::from("jobTransactionDate"), payment_date);
    map.insert(String::from("jobFinishDate"), today);
    map.insert(
        String::from("jobCraftmanPrice"),
        bid.labour_cost.to_string(),
    );
    map.insert(
        String::from("craftmanMaterialCost"),
        bid.material_cost.to_string(),
    );
    map.insert(
        String::from("rotavdrag"),
        if bid.root_deduction == Decimal::zero() {
            String::from("Ej applicerbart")
        } else {
            bid.root_deduction.to_string()
        },
    );
    map.insert(String::from("jobSum"), bid.final_bid.to_string());
    map.insert(
        String::from("craftsmanAccountNumber"),
        craftsman.account_number.to_string(),
    );
    map.insert(String::from("vat"), bid.vat.to_string());
    map.insert(
        String::from("brokerageFeeSum"),
        ((bid.material_cost + bid.labour_cost + bid.vat) * office.brokerage_percentage).to_string(),
    );

    let p = Personalization::new(Email::new("support@toolitapp.com"))
        .add_to(Email::new(&craftsman_user.email))
        .add_dynamic_template_data(map);

    let toolit_email = Message::new(Email::new("support@toolitapp.com"))
        .set_template_id("d-17ab5ef7123848e18dea987d1bc40d20")
        .add_personalization(p);
    let sender = Sender::new(SENDGRID_API_KEY.to_string());
    let (to_r, cr_r, tool_r) = tokio::join!(
        sender.send(&to_email),
        sender.send(&craftsman_email),
        sender.send(&toolit_email)
    );

    match to_r {
        Ok(_) => (),
        Err(e) => {
            log(format!(
                "Could not send email to task owner for finishing task {:?}",
                e
            ));
        }
    }

    match cr_r {
        Ok(_) => (),
        Err(e) => {
            log(format!(
                "Could not send email to craftsman for finishing task {:?}",
                e
            ));
        }
    }

    match tool_r {
        Ok(_) => (),
        Err(e) => {
            log(format!(
                "Could not send email to toolit for finishing task {:?}",
                e
            ));
        }
    }

    Ok(warp::reply::json(&DataResponse {
        data: Some(&task),
        extra: None::<Empty>,
    }))
}
