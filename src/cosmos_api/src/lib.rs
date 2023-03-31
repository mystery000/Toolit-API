lazy_static::lazy_static! {
    static ref COSMOS_ACCOUNT: String = std::env::var("COSMOS_ACCOUNT").unwrap();
    static ref COSMOS_DATABASE: String = std::env::var("COSMOS_DATABASE").unwrap();
    static ref COSMOS_MASTER_KEY: String = std::env::var("COSMOS_MASTER_KEY").unwrap();
    static ref STORAGE_ACCOUNT: String = std::env::var("STORAGE_ACCOUNT").unwrap();
    static ref STORAGE_MASTER_KEY: String = std::env::var("STORAGE_MASTER_KEY").unwrap();
    static ref IMAGES_STORAGE_CONTAINER: String = std::env::var("IMAGES_STORAGE_CONTAINER").unwrap();
    static ref VIDEOS_STORAGE_CONTAINER: String = std::env::var("VIDEOS_STORAGE_CONTAINER").unwrap();
}

use azure_core::{prelude::IfMatchCondition, HttpClient};
use azure_cosmos::prelude::*;
use azure_storage::clients::*;
use futures::StreamExt;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use uuid::Uuid;
use warp::{filters::multipart::FormData, Buf as OtherBuf};

type CosmosError = CosmosErrorStruct;

#[derive(Debug)]
pub struct CosmosErrorStruct {
    pub err: String,
    pub kind: CosmosErrorKind,
}

impl std::fmt::Display for CosmosErrorStruct {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_fmt(format_args!("kind: {}, err: {}", self.kind, self.err))?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum CosmosErrorKind {
    PreconditionFailed,
    NotFound,
    BadRequest,
    InternalError,
    Conflict,
    BlobError,
    ModificationError(warp::Rejection),
}

impl std::fmt::Display for CosmosErrorKind {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CosmosErrorKind::PreconditionFailed => {
                fmt.write_str("PreconditionFailed")?;
            }
            CosmosErrorKind::NotFound => {
                fmt.write_str("NotFound")?;
            }
            CosmosErrorKind::BadRequest => {
                fmt.write_str("BadRequest")?;
            }
            CosmosErrorKind::InternalError => {
                fmt.write_str("InternalError")?;
            }
            CosmosErrorKind::Conflict => {
                fmt.write_str("Conflict")?;
            }
            CosmosErrorKind::BlobError => {
                fmt.write_str("BlobError")?;
            }
            CosmosErrorKind::ModificationError(rej) => {
                fmt.write_fmt(format_args!("ModificationError({:?})", rej))?;
            }
        }
        Ok(())
    }
}

impl warp::reject::Reject for CosmosErrorStruct {}

/// Utility function that returns a closure that converts whatever error into a Reject error.
/// Usage is: function_call_which_returns_non_reject_result().map_err(into_cosmos_error("custom error message"))?;
fn into_cosmos_error<E: ToString, S: ToString>(message: S) -> impl FnOnce(E) -> CosmosError {
    move |e: E| new_cosmos_error(format!("{} : {:?}", message.to_string(), e.to_string()))
}

/// Creates a new cosmos error from a given error
fn new_cosmos_error<E: ToString>(err: E) -> CosmosError {
    let err = err.to_string();
    let kind = if err.contains("412") {
        CosmosErrorKind::PreconditionFailed
    } else if err.contains("404") {
        CosmosErrorKind::NotFound
    } else if err.contains("400") {
        CosmosErrorKind::BadRequest
    } else if err.contains("409") {
        CosmosErrorKind::Conflict
    } else {
        CosmosErrorKind::InternalError
    };
    CosmosErrorStruct { kind, err }
}

/// Creates a new cosmos error from a given error and the error kind
fn new_cosmos_error_kind<E: ToString>(err: E, kind: CosmosErrorKind) -> CosmosError {
    let err = err.to_string();
    CosmosErrorStruct { kind, err }
}

async fn insert_internal<D: Serialize, P: Into<PartitionKeys>, C: ToString>(
    collection_name: C,
    pk: P,
    document: &D,
    etag: Option<&str>,
    upsert: bool,
) -> Result<String, CosmosError> {
    let collection_name = collection_name.to_string();
    let authorization_token = AuthorizationToken::primary_from_base64(&COSMOS_MASTER_KEY)
        .map_err(into_cosmos_error("Could not get authorization token"))?;

    let pk = pk.into();
    let http_client: Arc<Box<dyn HttpClient>> = Arc::new(Box::new(reqwest::Client::new()));
    let client = CosmosClient::new(http_client, COSMOS_ACCOUNT.to_string(), authorization_token);
    let database_client = client.into_database_client(&*COSMOS_DATABASE);
    let collection_client = database_client.into_collection_client(collection_name);

    let c = collection_client
        .create_document()
        .partition_keys(pk)
        .is_upsert(upsert);
    let c = match etag {
        Some(etag) => c.if_match_condition(IfMatchCondition::Match(etag)),
        None => c,
    };

    let resp = retry_loop(MAX_RETRY_LOOPS, || async {
        match c.execute(&Document::new(document)).await {
            Ok(t) => Ok(t),
            Err(err) => {
                return Err(RetryLoopError::Permanent(new_cosmos_error(format!(
                    "Cosmos db error: {:?}",
                    err
                ))));
            }
        }
    })
    .await?;
    let etag = resp.etag;
    Ok(etag)
}

/// Insert a document into the cosmos database and returning an etag from the response if
/// successful
pub async fn insert<D: Serialize, P: Into<PartitionKeys>, C: ToString>(
    collection_name: C,
    pk: P,
    document: &D,
    etag: Option<&str>,
) -> Result<String, CosmosError> {
    insert_internal(collection_name, pk, document, etag, false).await
}

/// Upsert a document into the cosmos database and returning an etag from the response if
/// successful
pub async fn upsert<
    D: Serialize,
    P: Into<PartitionKeys>,
    C: ToString,
>(
    collection_name: C,
    pk: P,
    document: &D,
    etag: Option<&str>,
) -> Result<String, CosmosError> {
    insert_internal(collection_name, pk, document, etag, true).await
}

/// Returns a specific document from the cosmos DB together with a corresponding etag
pub async fn get<
    D: DeserializeOwned,
    P: Into<PartitionKeys>,
    C: ToString,
    S: ToString,
>(
    collection_name: C,
    pk: P,
    document_id: S,
) -> Result<(D, String), CosmosError> {
    let collection_name = collection_name.to_string();
    let document_id = document_id.to_string();
    let authorization_token = AuthorizationToken::primary_from_base64(&COSMOS_MASTER_KEY)
        .map_err(into_cosmos_error("Could not get authorization token"))?;

    let pk = pk.into();
    let http_client: Arc<Box<dyn HttpClient>> = Arc::new(Box::new(reqwest::Client::new()));
    let client = CosmosClient::new(http_client, COSMOS_ACCOUNT.to_string(), authorization_token);
    let database_client = client.into_database_client(&*COSMOS_DATABASE);
    let collection_client = database_client.into_collection_client(collection_name);
    let document_client = collection_client
        .clone()
        .into_document_client(document_id, pk.clone());
    let resp = match document_client
        .get_document()
        .execute::<D>()
        .await
        .map_err(into_cosmos_error("Could not get document"))?
    {
        azure_cosmos::responses::GetDocumentResponse::Found(resp) => resp,
        azure_cosmos::responses::GetDocumentResponse::NotFound(resp) => {
            return Err(new_cosmos_error_kind(
                format!("Document not found: {:?}", resp),
                CosmosErrorKind::NotFound,
            ));
        }
    };
    let doc: D = resp.document.document;
    let etag = resp.etag;
    Ok((doc, etag))
}

/// Modifies a document in cosmos by applying `transform` async closure on the existing document and then
/// inserting the returned document and returning both the transformed document, the old
/// document and the etag if successful.
/// If the transform closure fails then no insertion is performed and the error it fails with
/// is returned
pub async fn modify_async_get_old<
D: Serialize + DeserializeOwned + Clone,
P: Into<PartitionKeys>,
F: Fn(D) -> Fut,
C: ToString,
S: ToString,
Fut: futures::Future<Output = Result<D, warp::Rejection>>,
>(
    collection_name: C,
    pk: P,
    document_id: S,
    transform: F,
    ) -> Result<(D, D, String), CosmosError> {
    let collection_name = collection_name.to_string();
    let document_id = document_id.to_string();
    let authorization_token = AuthorizationToken::primary_from_base64(&COSMOS_MASTER_KEY)
        .map_err(into_cosmos_error("Could not get authorization token"))?;

    let pk = pk.into();
    let http_client: Arc<Box<dyn HttpClient>> = Arc::new(Box::new(reqwest::Client::new()));
    let client =
        CosmosClient::new(http_client, COSMOS_ACCOUNT.to_string(), authorization_token);
    let database_client = client.into_database_client(&*COSMOS_DATABASE);
    let collection_client = database_client.into_collection_client(collection_name);
    let document_client = collection_client
        .clone()
        .into_document_client(document_id, pk.clone());
    let (doc, old_doc, etag) = retry_loop(MAX_RETRY_LOOPS, || async {
        let resp = match document_client
            .get_document()
            .execute::<D>()
            .await
            .map_err(|e| {
                RetryLoopError::Permanent(new_cosmos_error(format!(
                            "Could not get document {:?}",
                            e
                            )))
            })? {
                azure_cosmos::responses::GetDocumentResponse::Found(resp) => resp,
                azure_cosmos::responses::GetDocumentResponse::NotFound(resp) => {
                    return Err(RetryLoopError::Permanent(new_cosmos_error_kind(
                                format!("Document not found: {:?}", resp),
                                CosmosErrorKind::NotFound,
                                )));
                }
            };
        let doc: D = resp.document.document;
        let old_doc = doc.clone();

        // Perform changes to the document
        let doc = transform(doc).await.map_err(|e| {
            RetryLoopError::Permanent(new_cosmos_error_kind(
                    format!("Modification not possible: {:?}", e),
                    CosmosErrorKind::InternalError,
                    ))
        })?;
        let c = collection_client
            .create_document()
            .partition_keys(pk.clone())
            .is_upsert(true)
            .if_match_condition(IfMatchCondition::Match(&resp.etag));

        match c.execute(&Document::new(&doc)).await {
            Ok(resp) => {
                let etag = resp.etag;
                return Result::Ok::<_, RetryLoopError<CosmosError>>((doc, old_doc, etag))
            }
            Err(err) => {
                let s = err.to_string();
                //NOTE: 412 means the document has been edited between read and write so it
                //means we need to retry the entire read/write block
                if s.contains("412") {
                    return Err(RetryLoopError::Transient(new_cosmos_error_kind(
                                format!("Cosmos db error: {:?}", s),
                                CosmosErrorKind::PreconditionFailed,
                                )));
                } else {
                    return Err(RetryLoopError::Permanent(new_cosmos_error(format!(
                                    "Cosmos db error: {:?}",
                                    s
                                    ))));
                }
            }
        };
    })
    .await?;
    Ok((doc, old_doc, etag))
}

/// Modifies a document in cosmos by applying `transform` async closure on the existing document and then
/// inserting the returned document and returning the transformed document if successful
/// If the transform closure fails then no insertion is performed and the error it fails with
/// is returned
pub async fn modify_async<
    D: Serialize + DeserializeOwned,
    P: Into<PartitionKeys>,
    F: Fn(D) -> Fut,
    C: ToString,
    S: ToString,
    Fut: futures::Future<Output = Result<D, warp::Rejection>>,
>(
    collection_name: C,
    pk: P,
    document_id: S,
    transform: F,
) -> Result<D, CosmosError> {
    let collection_name = collection_name.to_string();
    let document_id = document_id.to_string();
    let authorization_token = AuthorizationToken::primary_from_base64(&COSMOS_MASTER_KEY)
        .map_err(into_cosmos_error("Could not get authorization token"))?;

    let pk = pk.into();
    let http_client: Arc<Box<dyn HttpClient>> = Arc::new(Box::new(reqwest::Client::new()));
    let client = CosmosClient::new(http_client, COSMOS_ACCOUNT.to_string(), authorization_token);
    let database_client = client.into_database_client(&*COSMOS_DATABASE);
    let collection_client = database_client.into_collection_client(collection_name);
    let document_client = collection_client
        .clone()
        .into_document_client(document_id, pk.clone());
    let doc = retry_loop(MAX_RETRY_LOOPS, || async {
        let resp = match document_client
            .get_document()
            .execute::<D>()
            .await
            .map_err(|e| {
                RetryLoopError::Permanent(new_cosmos_error(format!(
                    "Could not get document {:?}",
                    e
                )))
            })? {
            azure_cosmos::responses::GetDocumentResponse::Found(resp) => resp,
            azure_cosmos::responses::GetDocumentResponse::NotFound(resp) => {
                return Err(RetryLoopError::Permanent(new_cosmos_error_kind(
                    format!("Document not found: {:?}", resp),
                    CosmosErrorKind::NotFound,
                )));
            }
        };
        let doc: D = resp.document.document;

        // Perform changes to the document
        let doc = transform(doc).await.map_err(|e| {
            RetryLoopError::Permanent(new_cosmos_error_kind(
                format!("Modification not possible: {:?}", e),
                CosmosErrorKind::InternalError,
            ))
        })?;
        let c = collection_client
            .create_document()
            .partition_keys(pk.clone())
            .is_upsert(true)
            .if_match_condition(IfMatchCondition::Match(&resp.etag));

        match c.execute(&Document::new(&doc)).await {
            Ok(_) => return Result::Ok::<_, RetryLoopError<CosmosError>>(doc),
            Err(err) => {
                let s = err.to_string();
                //NOTE: 412 means the document has been edited between read and write so it
                //means we need to retry the entire read/write block
                if s.contains("412") {
                    return Err(RetryLoopError::Transient(new_cosmos_error_kind(
                        format!("Cosmos db error: {:?}", s),
                        CosmosErrorKind::PreconditionFailed,
                    )));
                } else {
                    return Err(RetryLoopError::Permanent(new_cosmos_error(format!(
                        "Cosmos db error: {:?}",
                        s
                    ))));
                }
            }
        };
    })
    .await?;
    Ok(doc)
}

#[derive(Debug, Clone)]
pub enum ModifyReturn<D> {
    Replace(D),
    DontReplace(D),
}

impl<D> ModifyReturn<D> {
    /// No risk of panicing, gets the inner value
    pub fn inner(self) -> D {
        match self {
            ModifyReturn::Replace(d) => d,
            ModifyReturn::DontReplace(d) => d,
        }
    }
}

/// Modifies a document in cosmos by applying `transform` closure on the existing document and
/// either inserting the returned value or just returning it to the caller
pub async fn maybe_modify<
D: Serialize + DeserializeOwned + std::fmt::Debug,
P: Into<PartitionKeys>,
F: Fn(D) -> Result<ModifyReturn<D>, warp::Rejection>,
C: ToString,
S: ToString,
>(
    collection_name: C,
    pk: P,
    document_id: S,
    transform: F,
    ) -> Result<ModifyReturn<D>, CosmosError> {
    let collection_name = collection_name.to_string();
    let document_id = document_id.to_string();
    let authorization_token = AuthorizationToken::primary_from_base64(&COSMOS_MASTER_KEY)
        .map_err(into_cosmos_error("Could not get authorization token"))?;

    let pk = pk.into();
    let http_client: Arc<Box<dyn HttpClient>> = Arc::new(Box::new(reqwest::Client::new()));
    let client =
        CosmosClient::new(http_client, COSMOS_ACCOUNT.to_string(), authorization_token);
    let database_client = client.into_database_client(&*COSMOS_DATABASE);
    let collection_client = database_client.into_collection_client(collection_name);
    let document_client = collection_client
        .clone()
        .into_document_client(document_id, pk.clone());
    let doc = retry_loop(MAX_RETRY_LOOPS, || async {
        let resp = match document_client
            .get_document()
            .execute::<D>()
            .await
            .map_err(|e| RetryLoopError::Permanent(new_cosmos_error(format!("{:?}", e))))?
            {
                azure_cosmos::responses::GetDocumentResponse::Found(resp) => resp,
                azure_cosmos::responses::GetDocumentResponse::NotFound(resp) => {
                    return Err(RetryLoopError::Permanent(new_cosmos_error_kind(
                                format!("Document not found: {:?}", resp),
                                CosmosErrorKind::NotFound,
                                )));
                }
            };
        let doc: D = resp.document.document;

        // Perform changes to the document
        let doc = transform(doc).map_err(|e| {
            RetryLoopError::Permanent(new_cosmos_error_kind(
                    format!("Modification error: {:?}", e),
                    CosmosErrorKind::InternalError,
                    ))
        })?;

        let doc = match doc {
            ModifyReturn::Replace(doc) => doc,
            ModifyReturn::DontReplace(doc) => return Ok(ModifyReturn::DontReplace(doc)),
        };

        let c = collection_client
            .create_document()
            .partition_keys(pk.clone())
            .is_upsert(true)
            .if_match_condition(IfMatchCondition::Match(&resp.etag));

        match c.execute(&Document::new(&doc)).await {
            Ok(_) => return Result::Ok::<_, RetryLoopError<CosmosError>>(ModifyReturn::Replace(doc)),
            Err(err) => {
                let s = err.to_string();
                //NOTE: 412 means the document has been edited between read and write so it
                //means we need to retry the entire read/write block
                if s.contains("412") {
                    return Err(RetryLoopError::Transient(new_cosmos_error_kind(
                                format!("Cosmos db error: {:?}", s),
                                CosmosErrorKind::PreconditionFailed,
                                )));
                } else {
                    return Err(RetryLoopError::Transient(new_cosmos_error(format!(
                                    "Cosmos db error: {:?}",
                                    s
                                    ))));
                }
            }
        };
    })
    .await?;
    Ok(doc)
}

/// Modifies a document in cosmos by applying `transform` closure on the existing document and
/// either inserting the returned value or just returning it to the caller
pub async fn maybe_modify_async<
D: Serialize + DeserializeOwned + std::fmt::Debug,
P: Into<PartitionKeys>,
F: Fn(D) -> Fut,
C: ToString,
S: ToString,
Fut: futures::Future<Output = Result<ModifyReturn<D>, warp::Rejection>>,
>(
    collection_name: C,
    pk: P,
    document_id: S,
    transform: F,
    ) -> Result<ModifyReturn<D>, CosmosError> {
    let collection_name = collection_name.to_string();
    let document_id = document_id.to_string();
    let authorization_token = AuthorizationToken::primary_from_base64(&COSMOS_MASTER_KEY)
        .map_err(into_cosmos_error("Could not get authorization token"))?;

    let pk = pk.into();
    let http_client: Arc<Box<dyn HttpClient>> = Arc::new(Box::new(reqwest::Client::new()));
    let client =
        CosmosClient::new(http_client, COSMOS_ACCOUNT.to_string(), authorization_token);
    let database_client = client.into_database_client(&*COSMOS_DATABASE);
    let collection_client = database_client.into_collection_client(collection_name);
    let document_client = collection_client
        .clone()
        .into_document_client(document_id, pk.clone());
    let doc = retry_loop(MAX_RETRY_LOOPS, || async {
        let resp = match document_client
            .get_document()
            .execute::<D>()
            .await
            .map_err(|e| RetryLoopError::Permanent(new_cosmos_error(format!("{:?}", e))))?
            {
                azure_cosmos::responses::GetDocumentResponse::Found(resp) => resp,
                azure_cosmos::responses::GetDocumentResponse::NotFound(resp) => {
                    return Err(RetryLoopError::Permanent(new_cosmos_error_kind(
                                format!("Document not found: {:?}", resp),
                                CosmosErrorKind::NotFound,
                                )));
                }
            };
        let doc: D = resp.document.document;

        // Perform changes to the document
        let doc = transform(doc).await.map_err(|e| {
            RetryLoopError::Permanent(new_cosmos_error_kind(
                    format!("Modification error: {:?}", e),
                    CosmosErrorKind::InternalError,
                    ))
        })?;

        let doc = match doc {
            ModifyReturn::Replace(doc) => doc,
            ModifyReturn::DontReplace(doc) => return Ok(ModifyReturn::DontReplace(doc)),
        };

        let c = collection_client
            .create_document()
            .partition_keys(pk.clone())
            .is_upsert(true)
            .if_match_condition(IfMatchCondition::Match(&resp.etag));

        match c.execute(&Document::new(&doc)).await {
            Ok(_) => return Result::Ok::<_, RetryLoopError<CosmosError>>(ModifyReturn::Replace(doc)),
            Err(err) => {
                let s = err.to_string();
                //NOTE: 412 means the document has been edited between read and write so it
                //means we need to retry the entire read/write block
                if s.contains("412") {
                    return Err(RetryLoopError::Transient(new_cosmos_error_kind(
                                format!("Cosmos db error: {:?}", s),
                                CosmosErrorKind::PreconditionFailed,
                                )));
                } else {
                    return Err(RetryLoopError::Transient(new_cosmos_error(format!(
                                    "Cosmos db error: {:?}",
                                    s
                                    ))));
                }
            }
        };
    })
    .await?;
    Ok(doc)
}

/// Modify in cosmos but does not retry on failure which is useful when one needs to pass in a
/// `FnOnce` closure
pub async fn modify_no_retry<
D: Serialize + DeserializeOwned + std::fmt::Debug,
P: Into<PartitionKeys>,
F: FnOnce(D) -> Result<D, warp::Rejection>,
C: ToString,
S: ToString,
>(
    collection_name: C,
    pk: P,
    document_id: S,
    transform: F,
    ) -> Result<D, CosmosError> {
    let collection_name = collection_name.to_string();
    let document_id = document_id.to_string();
    let authorization_token = AuthorizationToken::primary_from_base64(&COSMOS_MASTER_KEY)
        .map_err(into_cosmos_error("Could not get authorization token"))?;

    let pk = pk.into();
    let http_client: Arc<Box<dyn HttpClient>> = Arc::new(Box::new(reqwest::Client::new()));
    let client =
        CosmosClient::new(http_client, COSMOS_ACCOUNT.to_string(), authorization_token);
    let database_client = client.into_database_client(&*COSMOS_DATABASE);
    let collection_client = database_client.into_collection_client(collection_name);
    let document_client = collection_client
        .clone()
        .into_document_client(document_id, pk.clone());
    let resp = match document_client
        .get_document()
        .execute::<D>()
        .await
        .map_err(|e| new_cosmos_error(format!("{:?}", e)))?
        {
            azure_cosmos::responses::GetDocumentResponse::Found(resp) => resp,
            azure_cosmos::responses::GetDocumentResponse::NotFound(resp) => {
                return Err(new_cosmos_error_kind(
                        format!("Document not found: {:?}", resp),
                        CosmosErrorKind::NotFound,
                        ));
            }
        };
    let doc: D = resp.document.document;

    // Perform changes to the document
    let doc = transform(doc).map_err(|e| {
        new_cosmos_error_kind(
            format!("Modification error: {:?}", e),
            CosmosErrorKind::InternalError,
            )
    })?;
    let c = collection_client
        .create_document()
        .partition_keys(pk.clone())
        .is_upsert(true)
        .if_match_condition(IfMatchCondition::Match(&resp.etag));

    let doc = match c.execute(&Document::new(&doc)).await {
        Ok(_) => doc,
        Err(err) => {
            return Err(new_cosmos_error(format!("Cosmos db error: {:?}", err)));
        }
    };
    Ok(doc)
}


/// Modifies a document in cosmos by applying `transform` closure on the existing document and then
/// inserting the returned document and returning the transformed document if successful
/// If the transform closure fails then no insertion is performed and the error it fails with
/// is returned
pub async fn modify<
    D: Serialize + DeserializeOwned + std::fmt::Debug,
    P: Into<PartitionKeys>,
    F: Fn(D) -> Result<D, warp::Rejection>,
    C: ToString,
    S: ToString,
>(
    collection_name: C,
    pk: P,
    document_id: S,
    transform: F,
) -> Result<D, CosmosError> {
    modify_async(collection_name, pk, document_id, |d| async { transform(d) }).await
}

pub async fn delete<C: ToString, S: ToString, P: Into<PartitionKeys>>(
    collection_name: C,
    pk: P,
    document_id: S,
    etag: Option<String>,
) -> Result<(), CosmosError> {
    let collection_name = collection_name.to_string();
    let document_id = document_id.to_string();
    let authorization_token = AuthorizationToken::primary_from_base64(&COSMOS_MASTER_KEY)
        .map_err(into_cosmos_error("Could not get authorization token"))?;

    let pk = pk.into();
    let http_client: Arc<Box<dyn HttpClient>> = Arc::new(Box::new(reqwest::Client::new()));
    let client = CosmosClient::new(http_client, COSMOS_ACCOUNT.to_string(), authorization_token);
    let database_client = client.into_database_client(&*COSMOS_DATABASE);
    let collection_client = database_client.into_collection_client(collection_name);
    let document_client = collection_client
        .clone()
        .into_document_client(document_id, pk.clone());
    let del_doc = document_client.delete_document();
    if let Some(etag) = etag {
        del_doc
            .if_match_condition(IfMatchCondition::Match(&etag))
            .execute()
            .await
            .map_err(into_cosmos_error("Could not delete document"))?;
    } else {
        del_doc
            .execute()
            .await
            .map_err(into_cosmos_error("Could not delete document"))?;
    }
    Ok(())
}

pub async fn query_crosspartition_etag<
    D: DeserializeOwned,
    P: Into<PartitionKeys>,
    C: ToString,
>(
    collection_name: C,
    pk: P,
    query: String,
    max_count: i32,
    cross_partition: bool,
) -> Result<Vec<(D, String)>, CosmosError> {
    let collection_name = collection_name.to_string();
    let authorization_token = AuthorizationToken::primary_from_base64(&COSMOS_MASTER_KEY)
        .map_err(into_cosmos_error("Could not get authorization token"))?;
    let pk: PartitionKeys = pk.into();

    let http_client: Arc<Box<dyn HttpClient>> = Arc::new(Box::new(reqwest::Client::new()));
    let client = CosmosClient::new(http_client, COSMOS_ACCOUNT.to_string(), authorization_token);
    let database_client = client.into_database_client(&*COSMOS_DATABASE);
    let collection_client = database_client.into_collection_client(collection_name);

    let mut documents: Vec<(D, String)> = vec![];
    let mut continuation_token = String::from("");
    loop {
        let query = Query::new(&query);
        let mut query_documents_builder = collection_client
            .query_documents()
            .max_item_count(max_count);
        if cross_partition {
            query_documents_builder = query_documents_builder.query_cross_partition(true);
        } else {
            query_documents_builder = query_documents_builder.partition_keys(&pk);
        }

        if continuation_token != "" {
            query_documents_builder =
                query_documents_builder.continuation(continuation_token.as_str());
        }

        let query_documents_response = query_documents_builder
            .execute::<D, _>(query)
            .await
            .map_err(into_cosmos_error("Could not get query documents"))?;

        let query_documents_response =
            query_documents_response
                .into_documents()
                .map_err(into_cosmos_error(
                    "Could not get cosmos db query document response",
                ))?;

        let mut fetched_documents: Vec<(D, String)> = query_documents_response
            .results
            .into_iter()
            .map(|document| {
                (
                    document.result,
                    document.document_attributes.etag().to_string(),
                )
            })
            .collect();

        documents.append(&mut fetched_documents);

        continuation_token = match query_documents_response.continuation_token {
            Some(token) => token.clone(),
            None => {
                break;
            }
        };
    }

    Ok(documents)
}

pub async fn query_crosspartition<
    D: DeserializeOwned,
    P: Into<PartitionKeys>,
    C: ToString,
>(
    collection_name: C,
    pk: P,
    query: String,
    max_count: i32,
    cross_partition: bool,
) -> Result<Vec<D>, CosmosError> {
    let v = query_crosspartition_etag(
        collection_name,
        pk,
        query,
        max_count,
        cross_partition,
    )
    .await?;
    Ok(v.into_iter().map(|(d, _)| d).collect())
}

pub async fn query<D: DeserializeOwned, P: Into<PartitionKeys>, C: ToString>(
    collection_name: C,
    pk: P,
    query: String,
    max_count: i32,
) -> Result<Vec<D>, CosmosError> {
    query_crosspartition(collection_name, pk, query, max_count, false).await
}

/// Uploads a new form data image to the blob storage and returns the image_id
pub async fn upload_image(f: FormData) -> Result<String, CosmosError> {
    upload_blob(f, "image", "image", &*IMAGES_STORAGE_CONTAINER).await
}

/// Uploads a new form data video to the blob storage and returns the video_id
pub async fn upload_video(f: FormData) -> Result<String, CosmosError> {
    // TODO(Jonathan): Currently the client will send videos as "image" content types, this should be changed
    upload_blob(f, "video", "video", &*VIDEOS_STORAGE_CONTAINER).await
}

/// Uploads a new form data image to the blob storage and returns the blob id
pub async fn upload_blob(mut f: FormData, blob_type: &str, expected_content_type: &str, storage_container: &str) -> Result<String, CosmosError> {
    while let Some(r) = f.next().await {
        match r {
            Ok(part) => {
                if part.name() == blob_type {
                    if let Some(g) = part.content_type() {
                        let content_type = String::from(g);
                        if content_type.starts_with(&expected_content_type) {
                            match part.filename() {
                                Some(n) => {
                                    let pos = n.find('.');
                                    if let Some(pos) = pos {
                                        let ext = String::from(n);
                                        let ext = &ext[pos..];

                                        //FIXME(Jonathan): Here we are using a Vec<u8> when we probably want to
                                        //use a bytes::Bytes. Additionally we are copying the buffer
                                        //when moving or referencing is prerferrable.
                                        //The reason we're doing it this way is because there are
                                        //currently some versioning issues with azure
                                        let mut buf: Vec<u8> = Vec::with_capacity(64);
                                        let mut s = part.stream();
                                        while let Some(r) = s.next().await {
                                            match r {
                                                Ok(mut b) => {
                                                    buf.extend(b.copy_to_bytes(b.remaining()));
                                                }
                                                Err(err) => {
                                                    return Err(new_cosmos_error_kind(
                                                            format!(
                                                                "Error getting {} data: {:?}",
                                                                blob_type, err
                                                            ),
                                                            CosmosErrorKind::BlobError,
                                                    ));
                                                }
                                            }
                                        }

                                        let mut blob_id = Uuid::new_v4().to_string();

                                        // Add extension to blob id.
                                        blob_id.push_str(ext);
                                        let http_client: Arc<Box<dyn HttpClient>> =
                                            Arc::new(Box::new(reqwest::Client::new()));

                                        let blob_client = StorageAccountClient::new_access_key(
                                            http_client.clone(),
                                            STORAGE_ACCOUNT.to_string(),
                                            STORAGE_MASTER_KEY.to_string(),
                                        )
                                            .as_storage_client()
                                            .as_container_client(storage_container)
                                            .as_blob_client(&blob_id);

                                        // Helps preventing spurious data to be uploaded.
                                        let digest = md5::compute(&buf[..]).into();
                                        {
                                            match blob_client
                                                .put_block_blob(buf)
                                                .content_type(content_type.as_str())
                                                .hash(&digest)
                                                .execute()
                                                .await
                                                {
                                                    Ok(r) => r,
                                                    Err(err) => {
                                                        return Err(new_cosmos_error_kind(
                                                                format!("Could not add blob to storage account {:?}", err),
                                                                CosmosErrorKind::BlobError));
                                                    }
                                                };
                                        }

                                        // Set blob id.
                                        return Ok(blob_id);
                                    } else {
                                        return Err(new_cosmos_error_kind(
                                                "Could not get filename for blob.",
                                                CosmosErrorKind::BlobError,
                                        ));
                                    }
                                }
                                None => {
                                    return Err(new_cosmos_error_kind(
                                            "Could not get filename for blob.",
                                            CosmosErrorKind::BlobError,
                                    ));
                                }
                            }
                        } else {
                            return Err(new_cosmos_error_kind(
                                    format!("Blob does not have a {} content-type.", expected_content_type),
                                    CosmosErrorKind::BlobError,
                            ));
                        }
                    }
                }
            }
            Err(err) => {
                return Err(new_cosmos_error_kind(
                        format!("Error getting multipart data {:?}", err),
                        CosmosErrorKind::BlobError,
                ));
            }
        }
    }
    return Err(new_cosmos_error_kind(
            "No blob provided",
            CosmosErrorKind::BlobError,
    ));
}

pub struct CosmosSaga {
    operation_stack: Vec<Operation>,
}

enum Operation {
    Upsert {
        old_document: Box<dyn erased_serde::Serialize + Send + Sync>,
        col_name: String,
        pk: PartitionKeys,
        etag: String,
    },
    Insert {
        document_id: String,
        col_name: String,
        pk: PartitionKeys,
        etag: String,
    },
    Modify {
        old_document: Box<dyn erased_serde::Serialize + Send + Sync>,
        col_name: String,
        pk: PartitionKeys,
        etag: String,
    },
    Delete {
        old_document: Box<dyn erased_serde::Serialize + Send + Sync>,
        col_name: String,
        pk: PartitionKeys,
    },
}

impl Operation {
    async fn reverse(self) -> Result<(), CosmosError> {
        match self {
            Operation::Upsert {
                old_document,
                col_name,
                pk,
                etag,
            } => {
                upsert(col_name, pk, &old_document, Some(&etag)).await?;
            }
            Operation::Insert {
                document_id,
                col_name,
                pk,
                etag,
            } => {
                delete(col_name, pk, document_id, Some(etag)).await?;
            }
            Operation::Modify {
                old_document,
                col_name,
                pk,
                etag,
            } => {
                upsert(col_name, pk, &old_document, Some(&etag)).await?;
            }
            Operation::Delete {
                old_document,
                col_name,
                pk,
            } => {
                insert(col_name, pk, &old_document, None).await?;
            }
        }
        Ok(())
    }

}

impl CosmosSaga {
    /// Constructs a new saga object which can be used to perform several cosmos operations in
    /// sequence. If any operation fails then all previous operations performed with the saga
    /// object will be reversed.
    pub fn new() -> Self {
        Self {
            operation_stack: Vec::new(),
        }
    }

    pub async fn delete<
        D: DeserializeOwned + Serialize + Send + Sync + 'static,
        C: ToString + Clone,
        S: ToString + Clone,
        P: Into<PartitionKeys> + Clone,
    >(
        &mut self,
        collection_name: C,
        pk: P,
        document_id: S,
        etag: Option<String>,
    ) -> Result<(), CosmosError> {
        let (document, _): (D, _) =
            match get(collection_name.clone(), pk.clone(), document_id.clone()).await {
                Ok(d) => d,
                Err(e) => {
                    self.abort().await?;
                    return Err(e);
                }
            };
        match delete(collection_name.clone(), pk.clone(), document_id, etag).await {
            Ok(r) => r,
            Err(e) => {
                self.abort().await?;
                return Err(e);
            }
        };
        self.operation_stack.push(Operation::Delete {
            old_document: Box::new(document),
            col_name: collection_name.to_string(),
            pk: pk.into(),
        });
        Ok(())
    }

    pub async fn modify<
        D: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
        P: Into<PartitionKeys> + Clone,
        F: Fn(D) -> Fut,
        C: ToString + Clone,
        S: ToString + Clone,
        Fut: futures::Future<Output = Result<D, warp::Rejection>>,
    >(
        &mut self,
        collection_name: C,
        pk: P,
        document_id: S,
        transform: F,
    ) -> Result<D, CosmosError> {
        let (document, old_document, etag) =
            match modify_async_get_old(collection_name.clone(), pk.clone(), document_id, transform)
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    self.abort().await?;
                    return Err(e);
                }
            };
        self.operation_stack.push(Operation::Modify {
            old_document: Box::new(old_document),
            col_name: collection_name.to_string(),
            pk: pk.into(),
            etag,
        });
        Ok(document)
    }

    pub async fn upsert<
        'de,
        D: Serialize + DeserializeOwned + Send + Sync + 'static,
        P: Into<PartitionKeys> + Clone,
        I: ToString,
        C: ToString + Clone,
    >(
        &mut self,
        collection_name: C,
        pk: P,
        document: &'de D,
        document_id: I,
        etag: Option<&str>,
    ) -> Result<String, CosmosError> {
        let (old_document, _): (D, _) =
            get(collection_name.clone(), pk.clone(), document_id).await?;
        let etag =
            match upsert(collection_name.clone(), pk.clone(), document, etag)
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    self.abort().await?;
                    return Err(e);
                }
            };
        self.operation_stack.push(Operation::Upsert {
            old_document: Box::new(old_document),
            col_name: collection_name.to_string(),
            pk: pk.into(),
            etag: etag.clone(),
        });
        Ok(etag)
    }

    pub async fn insert<
        'de,
        D: Serialize + DeserializeOwned + Send + Sync + 'static,
        P: Into<PartitionKeys> + Clone,
        I: ToString,
        C: ToString + Clone,
    >(
        &mut self,
        collection_name: C,
        pk: P,
        document: &'de D,
        document_id: I,
        etag: Option<&str>,
    ) -> Result<String, CosmosError> {
        let etag =
            match insert(collection_name.clone(), pk.clone(), document, etag)
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    self.abort().await?;
                    return Err(e);
                }
            };
        self.operation_stack.push(Operation::Insert {
            document_id: document_id.to_string(),
            col_name: collection_name.to_string(),
            pk: pk.into(),
            etag: etag.clone(),
        });
        Ok(etag)
    }

    pub async fn abort(&mut self) -> Result<(), CosmosError> {
        while let Some(operation) = self.operation_stack.pop() {
            operation.reverse().await?;
        }
        Ok(())
    }

    /// Consumes the saga and makes adding new things to it impossible
    /// This currently does no execution but is async in order to allow async operations to be
    /// done here in the future. Currently sagas are eagerly executed, meaning that the
    /// operations is performed when the method is called.
    pub async fn finalize(self) {}
}

/// The default value for amount of retry loops we do
const MAX_RETRY_LOOPS: usize = 5;
/// `retry_loop` is utilized in order to combat transient errors. A closure which generates a future
/// is used to generate the same future and run this untill completion, if the future returns success
/// with r then Ok(r) is returned. If the future returns an error then exponential backoff is tried
/// until eventually a max amount of tries is reached at which point the function returns the
/// latest error. The function runs at least once.
const RETRY_MAX_RANDOM: u64 = 200;
const RETRY_START_WAIT: u64 = 50;
pub enum RetryLoopError<E> {
    Permanent(E),
    Transient(E),
}
pub async fn retry_loop<F, A, R, E>(tries: usize, mut f: F) -> Result<R, E>
where
    F: FnMut() -> A,
    A: std::future::Future<Output = Result<R, RetryLoopError<E>>>,
{
    let mut counter = 0;
    let mut wait = RETRY_START_WAIT;
    loop {
        match f().await {
            Ok(r) => {
                return Ok(r);
            }
            Err(e) => {
                match e {
                    RetryLoopError::Permanent(e) => return Err(e),
                    RetryLoopError::Transient(e) => {
                        counter += 1;
                        let random_wait: u64 = rand::random::<u64>() % RETRY_MAX_RANDOM;
                        // Wait for an exponential backoff time + some random wait
                        sleep(Duration::from_millis(wait) + Duration::from_millis(random_wait))
                            .await;
                        if counter >= tries {
                            return Err(e);
                        }
                        //Exponential backoff
                        wait *= 2;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod util_tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn retry_loop_test() {
        let calls = std::cell::RefCell::new(vec![]);
        retry_loop(8, || async {
            calls.borrow_mut().push(Instant::now());
            if calls.borrow().len() >= 4 {
                Ok(())
            } else {
                Err(RetryLoopError::Transient(()))
            }
        })
        .await
        .unwrap();
        let mut calls = calls.borrow_mut();
        assert_eq!(calls.len(), 4);
        let t = calls.pop().unwrap().elapsed().as_millis();
        assert!(t == 0);
        let t = calls.pop().unwrap().elapsed().as_millis();
        assert!(t >= 200 && t <= 1200);
        let t = calls.pop().unwrap().elapsed().as_millis();
        assert!(t >= 300 && t <= 2200);
        let t = calls.pop().unwrap().elapsed().as_millis();
        assert!(t >= 350 && t <= 3200);
    }

    #[tokio::test]
    #[ignore]
    // Ignored since it requires quite a bit of time to retry several times
    async fn rety_loop_failure() {
        let calls = std::cell::RefCell::new(vec![]);
        let result = retry_loop(8, || async {
            calls.borrow_mut().push(Instant::now());
            if 1 == 2 {
                return Ok(());
            }
            Err(RetryLoopError::Transient(()))
        })
        .await;
        if let Ok(_) = result {
            panic!();
        }
        let calls = calls.borrow();
        assert_eq!(calls.len(), 8);
    }
}
