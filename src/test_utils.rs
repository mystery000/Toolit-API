// #![cfg(test)]
// use super::models::*;
// use super::*;
// use serde::Deserialize;

// const GLOBAL_ADMIN_USER: &str = "admin.adminson@admin.com";
// const GLOBAL_ADMIN_PASS: &str = "adminpw";

// pub fn test_env() {
//     if !cfg!(debug_assertions) {
//         panic!("Tests can not safely be run in release mode");
//     }
//     assert_eq!(*PRODUCTION_ENVIRONMENT, false);
//     assert_eq!(*ACCESS_TOKEN_SECRET, "access-token-secret");
//     assert_eq!(*REFRESH_TOKEN_SECRET, "refresh-token-secret");
//     assert_eq!(
//         *COSMOS_MASTER_KEY,
//         "jBqbt1R780nirFckloRlOXa0qMj3qSVPg1sdZlC9Zak0qutQqVEXNdn7Sk9CNalilU1U8ZmEiz92doHbaa8rsw=="
//     );
//     assert_eq!(*COSMOS_ACCOUNT, "toolit-play");
//     assert_eq!(*STORAGE_ACCOUNT, "toolitplay");
//     assert_eq!(*STORAGE_MASTER_KEY, "storage-master-key");
//     assert_eq!(COSMOS_DATABASE, "toolit");
//     assert_eq!(IMAGES_STORAGE_CONTAINER, "images");
//     assert_eq!(*SENDGRID_API_KEY, "sendgrid-api-key");
// }

// pub async fn user_signup(email: &str, pass: &str) {
//     test_env();
//     let p = "/users";
//     let routes = routes();
//     let body = format!(
//         r#"{{
//             "data": {{
//                 "id": "",
//                 "firstName": "Moot",
//                 "lastName": "Mudkipz",
//                 "email": "{}",
//                 "phone": "+6047641183",
//                 "started": "2021-01-01T17:41:18Z",
//                 "at": "2021-01-01T17:41:18Z",
//                 "test": true
//             }},
//             "extra": "{}"
//         }}"#,
//         email, pass
//     );
//     let res = warp::test::request()
//         .path(p)
//         .method("POST")
//         .body(body.clone())
//         .header("Accept", "application/vnd.toolit.v1+json")
//         .header("Content-Type", "application/json")
//         .reply(&routes)
//         .await;
//     if res.status() != 200 {
//         dbg!("Could not signup", res.body());
//         dbg!("Sent body", &body);
//     }
//     assert_eq!(res.status(), 200);
//     assert!(res.body().starts_with(br#"{"data":{"accessToken""#));
// }

// pub fn get_access_refresh_id(body: Vec<u8>) -> (String, String, String) {
//     #[derive(Deserialize)]
//     #[serde(rename_all = "camelCase")]
//     #[allow(non_camel_case_types)]
//     struct Data {
//         access_token: String,
//         refresh_token: String,
//         user_id: String,
//     }
//     #[derive(Deserialize)]
//     #[serde(rename_all = "camelCase")]
//     struct Resp {
//         data: Data,
//     }
//     let body = String::from_utf8(body).unwrap();
//     let body: Resp = serde_json::from_str(&body).unwrap();
//     let access: String = body.data.access_token;
//     let refresh: String = body.data.refresh_token;
//     let id: String = body.data.user_id;
//     println!("Access: {}", access);
//     println!("Refresh: {}", refresh);
//     println!("Id: {}", id);
//     (access, refresh, id)
// }

// pub async fn signin_access_refresh_id(user: &str, pass: &str) -> (String, String, String) {
//     test_env();
//     let p = "/users/signin/";
//     let body = format!("{{\"data\": \"{}\",\"extra\":\"{}\"}}", user, pass);
//     let routes = routes();
//     let res = warp::test::request()
//         .path(p)
//         .method("POST")
//         .body(body)
//         .header("Accept", "application/vnd.toolit.v1+json")
//         .header("Content-Type", "application/json")
//         .reply(&routes)
//         .await;
//     if res.status() != 200 {
//         user_signup(user, pass).await;
//     }
//     let p = "/users/signin/";
//     let body = format!("{{\"data\": \"{}\",\"extra\":\"{}\"}}", user, pass);
//     let res = warp::test::request()
//         .path(p)
//         .method("POST")
//         .body(body)
//         .header("Accept", "application/vnd.toolit.v1+json")
//         .header("Content-Type", "application/json")
//         .reply(&routes)
//         .await;
//     assert_eq!(res.status(), 200);
//     get_access_refresh_id(res.body().to_vec())
// }

// pub async fn filled_user_poll(access: &str, id: &str) -> UserPollDataResponse {
//     test_env();
//     let up = user_poll(access, id).await;
//     let office = match up.offices.get(0) {
//         Some(e) => e.clone(),
//         None => {
//             let access = global_admin_access().await;
//             let e = create_office(&access).await;
//             e
//         }
//     };
//     let task = match up.tasks.get(0) {
//         Some(e) => e,
//         None => {
//             let e = create_task(&access, &office.id).await;
//             e
//         }
//     };
//     let craftsman = match up.craftsmen.get(0) {
//         Some(e) => e,
//         None => {
//             let name = util::random_string(10);
//             let pass = util::random_string(10);
//             user_signup(name, pass).await;
//             let (access, refresh, id) = signin_access_refresh_id(name, pass).await;
//             let e = create_craftsman(&access, new_craftsman_user.id).await;
//             e
//         }
//     };
//     up
// }

// #[derive(Deserialize)]
// #[serde(rename_all = "camelCase")]
// struct UserPollData {
//     data: UserPollDataResponse,
// }

// pub async fn user_poll(access: &str, id: &str) -> UserPollDataResponse {
//     test_env();
//     let p = format!("/users/{}/poll", id);
//     let routes = routes();
//     let res = warp::test::request()
//         .path(&p)
//         .method("GET")
//         .header("Authorization", format!("Bearer {}", access))
//         .header("Range", "items=0-")
//         .reply(&routes)
//         .await;
//     assert_eq!(res.status(), 200);
//     let body = String::from_utf8(res.body().to_vec()).unwrap();
//     assert!(res.body().starts_with(
//         format!("{{\"data\":{{\"user\":{{\"id\":\"{}\",\"firstName\":\"", id).as_bytes()
//     ));

//     let poll_data: UserPollData = serde_json::from_str(&body).unwrap();
//     poll_data.data
// }

// pub async fn refresh_token(user_id: &str, refresh: &str) -> String {
//     let p = format!("/users/{}/token/refresh", user_id);
//     let routes = routes();
//     let body = format!(
//         r#"{{
//             "data": "{}"
//         }}"#,
//         refresh,
//     );
//     let res = warp::test::request()
//         .path(&p)
//         .method("POST")
//         .body(body)
//         .header("Accept", "application/vnd.toolit.v1+json")
//         .header("Content-Type", "application/json")
//         .reply(&routes)
//         .await;
//     assert_eq!(res.status(), 200);
//     let body = res_to_string(res.body());
//     #[derive(Deserialize)]
//     #[serde(rename_all = "camelCase")]
//     struct AccessToken {
//         data: String,
//     }
//     let ac: AccessToken = serde_json::from_str(&body).unwrap();
//     ac.data
// }

// async fn make_area_admin(access: &str, user_id: &str, area_id: &str) {
//     let p = format!("/users/{}/roles", user_id);
//     let body = format!(r#"{{"data": [{{"flg": 80,"sub": "{}"}}]}}"#, area_id);
//     let routes = routes();
//     let res = warp::test::request()
//         .path(&p)
//         .method("POST")
//         .body(body)
//         .header("Accept", "application/vnd.toolit.v1+json")
//         .header("Content-Type", "application/json")
//         .header("Range", "items=0-")
//         .header("Authorization", format!("Bearer {}", access))
//         .reply(&routes)
//         .await;
//     assert_eq!(res.status(), 200);
// }

// async fn global_admin_access() -> String {
//     let p = "/users/signin/";
//     let body = format!(
//         "{{\"data\": \"{}\",\"extra\":\"{}\"}}",
//         GLOBAL_ADMIN_USER, GLOBAL_ADMIN_PASS
//     );
//     let routes = routes();
//     let res = warp::test::request()
//         .path(p)
//         .method("POST")
//         .body(body)
//         .header("Accept", "application/vnd.toolit.v1+json")
//         .header("Content-Type", "application/json")
//         .reply(&routes)
//         .await;
//     assert_eq!(res.status(), 200);
//     let (access, _, _) = get_access_refresh_id(res.body().to_vec());
//     access
// }

// pub fn res_to_string(body: &[u8]) -> String {
//     String::from_utf8(body.to_vec()).unwrap()
// }

// pub async fn http_to(path: &str, body: &str, access: &str, method: &str) -> String {
//     let routes = routes();
//     let res = warp::test::request()
//         .path(&path)
//         .method(method)
//         .body(body)
//         .header("Accept", "application/vnd.toolit.v1+json")
//         .header("Content-Type", "application/json")
//         .header("Authorization", format!("Bearer {}", access))
//         .reply(&routes)
//         .await;
//     let res_body = res_to_string(res.body());
//     if res.status() != 200 {
//         dbg!(&body);
//         dbg!(&res_body);
//     }
//     assert_eq!(res.status(), 200);
//     res_body
// }

// pub async fn post_to(path: &str, body: &str, access: &str) -> String {
//     http_to(path, body, access, "POST").await
// }

// #[derive(Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct UserPollDataResponse {
//     #[serde(skip_serializing_if = "util::is_none")]
//     pub user: Option<User>,
//     pub offices: Vec<Office>,
//     pub tasks: Vec<Task>,
//     pub craftsmen: Vec<Craftsman>,
//     pub chat_messages: Vec<ChatMessage>,
//     pub payments: Vec<Payment>,
//     pub ratings: Vec<Rating>,
//     pub bids: Vec<Bid>,
// }

// async fn create_office(access: &str) -> Office {
//     let p = format!("/",);
//     let routes = routes();
//     let body = format!(
//         r#"{{
//             "data": {{

//             }}
//         }}"#,
//     );
//     let res = warp::test::request()
//         .path(&p)
//         .method("POST")
//         .body(body)
//         .header("Accept", "application/vnd.toolit.v1+json")
//         .header("Content-Type", "application/json")
//         .header("Authorization", format!("Bearer {}", access))
//         .reply(&routes)
//         .await;
//     assert_eq!(res.status(), 200);
//     let body = res_to_string(res.body());
//     #[derive(Deserialize)]
//     #[serde(rename_all = "camelCase")]
//     struct OfficeResp {
//         data: Office,
//     }
//     let office: OfficeResp = serde_json::from_str(&body).unwrap();
//     office.data
// }
// async fn create_craftsman(access: &str, user_id: &str) -> Craftsman {
//     let p = format!("/user/{}/", user_id,);
//     let routes = routes();
//     let body = format!(
//         r#"{{
//             "data": {{
//                 user_id: {},
//                 office: "TEST_STRING"

//             }}
//         }}"#,
//         user_id,
//     );
//     let res = warp::test::request()
//         .path(&p)
//         .method("POST")
//         .body(body)
//         .header("Accept", "application/vnd.toolit.v1+json")
//         .header("Content-Type", "application/json")
//         .header("Authorization", format!("Bearer {}", access))
//         .reply(&routes)
//         .await;
//     assert_eq!(res.status(), 200);
//     let body = res_to_string(res.body());
//     #[derive(Deserialize)]
//     #[serde(rename_all = "camelCase")]
//     struct CraftsmanResp {
//         data: Craftsman,
//     }
//     let craftsman: CraftsmanResp = serde_json::from_str(&body).unwrap();
//     craftsman.data
// }
// async fn create_rating(access: &str, user_id: &str, craftsman_id: &str) -> Rating {
//     let p = format!("/users/{}/craftsmen/{}/", user_id, craftsman_id,);
//     let routes = routes();
//     let body = format!(
//         r#"{{
//             "data": {{
//                 user_id: {},craftsman: {},
//                 office_id: "TEST_STRING"
// craftsman: "TEST_STRING"

//             }}
//         }}"#,
//         user_id, craftsman_id,
//     );
//     let res = warp::test::request()
//         .path(&p)
//         .method("POST")
//         .body(body)
//         .header("Accept", "application/vnd.toolit.v1+json")
//         .header("Content-Type", "application/json")
//         .header("Authorization", format!("Bearer {}", access))
//         .reply(&routes)
//         .await;
//     assert_eq!(res.status(), 200);
//     let body = res_to_string(res.body());
//     #[derive(Deserialize)]
//     #[serde(rename_all = "camelCase")]
//     struct RatingResp {
//         data: Rating,
//     }
//     let rating: RatingResp = serde_json::from_str(&body).unwrap();
//     rating.data
// }
// async fn create_chat(access: &str, office_id: &str, task_id: &str) -> ChatMessage {
//     let p = format!("/offices/{}/tasks/{}/", office_id, task_id,);
//     let routes = routes();
//     let body = format!(
//         r#"{{
//             "data": {{
//                 office: {},task: {},
//                 office: "TEST_STRING"
// task: "TEST_STRING"

//             }}
//         }}"#,
//         office_id, task_id,
//     );
//     let res = warp::test::request()
//         .path(&p)
//         .method("POST")
//         .body(body)
//         .header("Accept", "application/vnd.toolit.v1+json")
//         .header("Content-Type", "application/json")
//         .header("Authorization", format!("Bearer {}", access))
//         .reply(&routes)
//         .await;
//     assert_eq!(res.status(), 200);
//     let body = res_to_string(res.body());
//     #[derive(Deserialize)]
//     #[serde(rename_all = "camelCase")]
//     struct ChatResp {
//         data: ChatMessage,
//     }
//     let chat: ChatResp = serde_json::from_str(&body).unwrap();
//     chat.data
// }
// async fn create_task(access: &str, office_id: &str) -> Task {
//     test_env();
//     let p = format!("/offices/{}/", office_id,);
//     let routes = routes();
//     let body = format!(
//         r#"{{
//             "data": {{
//                 office: {},
//                 office: "TEST_STRING"

//             }}
//         }}"#,
//         office_id,
//     );
//     let res = warp::test::request()
//         .path(&p)
//         .method("POST")
//         .body(body)
//         .header("Accept", "application/vnd.toolit.v1+json")
//         .header("Content-Type", "application/json")
//         .header("Authorization", format!("Bearer {}", access))
//         .reply(&routes)
//         .await;
//     assert_eq!(res.status(), 200);
//     let body = res_to_string(res.body());
//     #[derive(Deserialize)]
//     #[serde(rename_all = "camelCase")]
//     struct TaskResp {
//         data: Task,
//     }
//     let task: TaskResp = serde_json::from_str(&body).unwrap();
//     task.data
// }
// async fn create_bid(access: &str, office_id: &str, task_id: &str) -> Bid {
//     test_env();
//     let p = format!("/offices/{}/tasks/{}/", office_id, task_id,);
//     let routes = routes();
//     let body = format!(
//         r#"{{
//             "data": {{
//                 office: {},task: {},
//                 office: "TEST_STRING"
// task: "TEST_STRING"

//             }}
//         }}"#,
//         office_id, task_id,
//     );
//     let res = warp::test::request()
//         .path(&p)
//         .method("POST")
//         .body(body)
//         .header("Accept", "application/vnd.toolit.v1+json")
//         .header("Content-Type", "application/json")
//         .header("Authorization", format!("Bearer {}", access))
//         .reply(&routes)
//         .await;
//     assert_eq!(res.status(), 200);
//     let body = res_to_string(res.body());
//     #[derive(Deserialize)]
//     #[serde(rename_all = "camelCase")]
//     struct BidResp {
//         data: Bid,
//     }
//     let bid: BidResp = serde_json::from_str(&body).unwrap();
//     bid.data
// }
