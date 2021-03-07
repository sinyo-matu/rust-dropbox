mod test;

use async_trait::async_trait;
use reqwest::{header, header::HeaderMap, Error as reqwest_error, StatusCode};
use serde::Deserialize;
use serde_json::json;
use std::{time};

const CONTENT_END_POINT: &str = "https://content.dropboxapi.com";
const OPERATION_END_POINT: &str = "https://api.dropboxapi.com";

#[derive(Debug, Deserialize)]
struct DbxRequestLimitsErrorSummary {
    error_summary: String,
    error: DbxRequestErrorReason,
}
#[derive(Debug, Deserialize)]
struct DbxRequestErrorSummary {
    error_summary: String,
    error: DbxRequestErrorTag,
}

#[derive(Debug, Deserialize)]
struct DbxRequestErrorReason {
    reason: DbxRequestErrorTag,
    retry_after: u32,
}
#[derive(Debug, Deserialize)]
struct DbxRequestErrorTag {
    #[serde(alias = ".tag")]
    tag: String,
}
#[derive(Debug, Deserialize)]
struct UserCheckResult {
    result: String,
}
#[derive(Debug, Clone)]
pub struct OAuth2Client {
    client: reqwest::Client,
}

impl OAuth2Client {
    pub fn new(token: &str) -> Self {
        let mut auth_value = header::HeaderValue::from_str(&format!("Bearer {}", token)).unwrap();
        auth_value.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, auth_value);
        let client = reqwest::ClientBuilder::new()
            .connect_timeout(time::Duration::from_secs(100))
            .default_headers(headers)
            .build()
            .unwrap();
        Self { client }
    }

    pub async fn check_user(&self, ping_str: &str) -> DropboxResult<()> {
        let url = format!("{}{}", OPERATION_END_POINT, "/2/check/user");
        let res = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(
                json!(
                {
                    "query":ping_str,
                }
                )
                .to_string(),
            )
            .send()
            .await?;
        match res.status() {
            reqwest::StatusCode::BAD_REQUEST => {
                let text = res.text().await?;
                return Err(DropboxError::DbxInvalidTokenError(text));
            }
            _ => handle_dbx_request_response::<()>(res).await,
        }
    }
    ///binding /upload
    pub async fn upload(&self, file: Vec<u8>, path: &str, mode: UploadMode) -> DropboxResult<()> {
        println!("uploading {}", path);
        let mode = match mode {
            UploadMode::Add => "add",
            UploadMode::Overwrite => "overwrite",
        };
        let url = format!("{}{}", CONTENT_END_POINT, "/2/files/upload");
        let res = self
            .client
            .post(&url)
            .header("Content-Type", "application/octet-stream")
            .header(
                "Dropbox-API-Arg",
                json!({"path":path,"mode":mode}).to_string(),
            )
            .body(file)
            .send()
            .await?;

        handle_dbx_request_response::<()>(res).await
    }

    ///binding /download
    pub async fn download(&self, path: &str) -> DropboxResult<Vec<u8>> {
        let url = format!("{}{}", CONTENT_END_POINT, "/2/files/download");
        let res = self
            .client
            .post(&url)
            .header("Dropbox-API-Arg", json!({ "path": path }).to_string())
            .send()
            .await?;

        handle_dbx_request_response::<Vec<u8>>(res).await
    }

    // binding /move_v2
    pub async fn move_file(
        &self,
        from_path: &str,
        to_path: &str,
        option: MoveOption,
    ) -> DropboxResult<()> {
        println!("moving {} to {}", from_path, to_path);
        let url = format!("{}{}", OPERATION_END_POINT, "/2/files/move_v2");
        let res = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(
                json!(
                {
                    "from_path":from_path,
                    "to_path":to_path,
                    "allow_shared_folder": option.allow_shared_folder,
                    "autorename": option.auto_rename,
                    "allow_ownership_transfer": option.allow_ownership_transfer
                }
                )
                .to_string(),
            )
            .send()
            .await?;
        handle_dbx_request_response::<()>(res).await
    }
}

async fn handle_dbx_request_response<T: FromRes>(res: reqwest::Response) -> DropboxResult<T::Item> {
    if res.status() != StatusCode::OK {
        match res.status() {
            StatusCode::BAD_REQUEST => {
                let text = res.text().await?;
                return Err(DropboxError::DbxPathError(text));
            }
            StatusCode::UNAUTHORIZED => {
                let error_summary = res.json::<DbxRequestErrorSummary>().await?;
                return Err(DropboxError::DbxInvalidTokenError(
                    error_summary.error_summary,
                ));
            }
            StatusCode::FORBIDDEN => {
                let error_summary = res.json::<DbxRequestErrorSummary>().await?;
                return Err(DropboxError::DbxAccessError(error_summary.error_summary));
            }
            StatusCode::CONFLICT => {
                let error_summary = res.json::<DbxRequestErrorSummary>().await?;
                let content: Vec<&str> = error_summary.error_summary.split("/").collect();
                match content[0] == "path" {
                    true => return Err(DropboxError::DbxPathError(content[1].to_string())),
                    false => match content[0] == "from_lookup" {
                        true => {
                            return Err(DropboxError::DbxFromLookUpError(content[1].to_string()))
                        }
                        false => match content[0] == "to" {
                            true => {
                                return Err(DropboxError::DbxExistedError(content[1].to_string()))
                            }
                            false => {
                                return Err(DropboxError::DbxConflictError(
                                    error_summary.error_summary,
                                ))
                            }
                        },
                    },
                }
            }
            StatusCode::TOO_MANY_REQUESTS => {
                let text = res.text().await?;
                match serde_json::from_str::<DbxRequestLimitsErrorSummary>(&text) {
                    Ok(error_summary) => {
                        return Err(DropboxError::DbxRequestLimitsError(format!(
                            "{} , retry after {}",
                            error_summary.error_summary, error_summary.error.retry_after
                        )));
                    }
                    Err(_) => {
                        return Err(DropboxError::DbxRequestLimitsError(text));
                    }
                }
            }
            StatusCode::INTERNAL_SERVER_ERROR | StatusCode::SERVICE_UNAVAILABLE => {
                let text = res.text().await?;
                return Err(DropboxError::DbxServerError(text));
            }
            _ => {
                let text = res.text().await?;
                match serde_json::from_str::<DbxRequestErrorSummary>(&text) {
                    Ok(error_summary) => {
                        return Err(DropboxError::OtherError(error_summary.error_summary));
                    }
                    Err(_) => {
                        return Err(DropboxError::OtherError(text));
                    }
                }
            }
        }
    }
    T::from_res(res).await
}

pub struct MoveOption {
    allow_shared_folder: bool,
    auto_rename: bool,
    allow_ownership_transfer: bool,
}

impl MoveOption {
    pub fn new() -> Self {
        Self {
            allow_shared_folder: false,
            auto_rename: false,
            allow_ownership_transfer: false,
        }
    }

    pub fn allow_shared_folder(mut self) -> Self {
        self.allow_shared_folder = true;
        self
    }

    pub fn auto_rename(mut self) -> Self {
        self.auto_rename = true;
        self
    }

    pub fn allow_ownership_transfer(mut self) -> Self {
        self.allow_ownership_transfer = true;
        self
    }
}
pub enum UploadMode {
    Add,
    Overwrite,
}

pub type DropboxResult<T> = std::result::Result<T, DropboxError>;
#[derive(Debug)]
pub enum DropboxError {
    HttpRequestError(reqwest_error),
    DbxUserCheckError(String),
    DbxPathError(String),
    DbxExistedError(String),
    DbxInvalidTokenError(String),
    DbxFromLookUpError(String),
    DbxRequestLimitsError(String),
    DbxAccessError(String),
    DbxConflictError(String),
    DbxServerError(String),
    OtherError(String),
}

impl From<reqwest_error> for DropboxError {
    fn from(e: reqwest_error) -> Self {
        Self::HttpRequestError(e)
    }
}

#[async_trait]
trait FromRes {
    type Item;
    async fn from_res(res: reqwest::Response) -> DropboxResult<Self::Item>;
}

#[async_trait]
impl FromRes for Vec<u8> {
    type Item = Self;
    async fn from_res(res: reqwest::Response) -> DropboxResult<Self> {
        res.bytes().await.map(|b| b.to_vec()).map_err(|e| DropboxError::HttpRequestError(e))
    }
}

#[async_trait]
impl FromRes for () {
    type Item = Self;
    async fn from_res(_res: reqwest::Response) -> DropboxResult<Self> {
        DropboxResult::Ok(())
    }
}
