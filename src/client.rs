use crate::{
    DbxRequestErrorSummary, DbxRequestLimitsErrorSummary, DropboxError, DropboxResult, MoveOption,
    UploadMode,
};
#[cfg(feature = "non-blocking")]
use async_trait::async_trait;
#[cfg(feature = "non-blocking")]
use reqwest::{header, header::HeaderMap, StatusCode};
use serde_json::json;
#[cfg(feature = "blocking")]
use std::io::Read;
use std::time;

const CONTENT_END_POINT: &str = "https://content.dropboxapi.com";
const OPERATION_END_POINT: &str = "https://api.dropboxapi.com";

#[cfg(feature = "non-blocking")]
#[derive(Debug, Clone)]
pub struct AsyncDBXClient {
    client: reqwest::Client,
}

#[cfg(feature = "non-blocking")]
impl AsyncDBXClient {
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
            _ => handle_async_dbx_request_response(res).await,
        }
    }
    ///binding /upload
    pub async fn upload(&self, file: Vec<u8>, path: &str, mode: &UploadMode) -> DropboxResult<()> {
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
        handle_async_dbx_request_response(res).await
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
        handle_async_dbx_request_response(res).await
    }

    // binding /move_v2
    pub async fn move_file(
        &self,
        from_path: &str,
        to_path: &str,
        option: &MoveOption,
    ) -> DropboxResult<()> {
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
        handle_async_dbx_request_response(res).await
    }
}

#[inline]
#[cfg(feature = "non-blocking")]
async fn handle_async_dbx_request_response<T: AsyncFrom<reqwest::Response>>(
    res: reqwest::Response,
) -> DropboxResult<T> {
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
    T::from(res).await.map(|i| *i)
}

#[cfg(feature = "non-blocking")]
#[async_trait]
trait AsyncFrom<T> {
    async fn from(t: T) -> DropboxResult<Box<Self>>;
}

#[cfg(feature = "non-blocking")]
#[async_trait]
impl AsyncFrom<reqwest::Response> for Vec<u8> {
    async fn from(res: reqwest::Response) -> DropboxResult<Box<Self>> {
        res.bytes()
            .await
            .map(|b| Box::new(b.to_vec()))
            .map_err(|e| DropboxError::NonBlockingRequestError(e))
    }
}

#[cfg(feature = "non-blocking")]
#[async_trait]
impl AsyncFrom<reqwest::Response> for () {
    async fn from(_res: reqwest::Response) -> DropboxResult<Box<Self>> {
        DropboxResult::Ok(Box::new(()))
    }
}

/////////////////////////////////////////////////////////////////////
#[cfg(feature = "blocking")]
//the blocking-io client
pub struct DBXClient {
    client: ureq::Agent,
    token: String,
}

#[cfg(feature = "blocking")]
impl DBXClient {
    pub fn new(token: &str) -> Self {
        let client = ureq::AgentBuilder::new()
            .timeout(time::Duration::from_secs(10))
            .build();
        let token = token.to_string();
        Self { client, token }
    }

    pub fn check_user(&self, ping_str: &str) -> DropboxResult<()> {
        let url = format!("{}{}", OPERATION_END_POINT, "/2/check/user");
        let res = self
            .client
            .post(&url)
            .set("Authorization", &format!("Bearer {}", self.token))
            .set("Content-Type", "application/json")
            .send_json(json!(
            {
                "query":ping_str,
            }
            ))?;
        match res.status() {
            400 => {
                let text = res.into_string()?;
                return Err(DropboxError::DbxInvalidTokenError(text));
            }
            _ => handle_dbx_request_response(res),
        }
    }
    ///binding /upload
    pub fn upload(&self, file: Vec<u8>, path: &str, mode: &UploadMode) -> DropboxResult<()> {
        let mode = match mode {
            UploadMode::Add => "add",
            UploadMode::Overwrite => "overwrite",
        };
        let url = format!("{}{}", CONTENT_END_POINT, "/2/files/upload");
        let res = self
            .client
            .post(&url)
            .set("Authorization", &format!("Bearer {}", self.token))
            .set("Content-Type", "application/octet-stream")
            .set(
                "Dropbox-API-Arg",
                json!({"path":path,"mode":mode}).to_string().as_str(),
            )
            .send_bytes(&file)?;

        handle_dbx_request_response(res)
    }

    ///binding /download
    pub fn download(&self, path: &str) -> DropboxResult<Vec<u8>> {
        let url = format!("{}{}", CONTENT_END_POINT, "/2/files/download");
        let res = self
            .client
            .post(&url)
            .set("Authorization", &format!("Bearer {}", self.token))
            .set(
                "Dropbox-API-Arg",
                json!({ "path": path }).to_string().as_str(),
            )
            .call()?;

        handle_dbx_request_response(res)
    }

    // binding /move_v2
    pub fn move_file(
        &self,
        from_path: &str,
        to_path: &str,
        option: &MoveOption,
    ) -> DropboxResult<()> {
        let url = format!("{}{}", OPERATION_END_POINT, "/2/files/move_v2");
        let res = self
            .client
            .post(&url)
            .set("Content-Type", "application/json")
            .set("Authorization", &format!("Bearer {}", self.token))
            .send_json(json!(
            {
                "from_path":from_path,
                "to_path":to_path,
                "allow_shared_folder": option.allow_shared_folder,
                "autorename": option.auto_rename,
                "allow_ownership_transfer": option.allow_ownership_transfer
            }
            ))?;
        handle_dbx_request_response(res)
    }
}

#[cfg(feature = "blocking")]
trait FromRes<T> {
    fn from_res(t: T) -> DropboxResult<Box<Self>>;
}

#[cfg(feature = "blocking")]
impl FromRes<ureq::Response> for Vec<u8> {
    fn from_res(res: ureq::Response) -> DropboxResult<Box<Self>> {
        let len = res
            .header("Content-Length")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap();
        let mut bytes: Vec<u8> = Vec::with_capacity(len);
        res.into_reader().read_to_end(&mut bytes)?;
        Ok(Box::new(bytes))
    }
}

#[cfg(feature = "blocking")]
impl FromRes<ureq::Response> for () {
    fn from_res(_res: ureq::Response) -> DropboxResult<Box<Self>> {
        DropboxResult::Ok(Box::new(()))
    }
}

#[inline]
#[cfg(feature = "blocking")]
fn handle_dbx_request_response<T: FromRes<ureq::Response>>(
    res: ureq::Response,
) -> DropboxResult<T> {
    if res.status() != 200 {
        match res.status() {
            400 => {
                let text = res.into_string()?;
                return Err(DropboxError::DbxPathError(text));
            }
            401 => {
                let error_summary = res.into_json::<DbxRequestErrorSummary>()?;
                return Err(DropboxError::DbxInvalidTokenError(
                    error_summary.error_summary,
                ));
            }
            403 => {
                let error_summary = res.into_json::<DbxRequestErrorSummary>()?;
                return Err(DropboxError::DbxAccessError(error_summary.error_summary));
            }
            409 => {
                let error_summary = res.into_json::<DbxRequestErrorSummary>()?;
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
            429 => {
                let text = res.into_string()?;
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
            500 | 503 => {
                let text = res.into_string()?;
                return Err(DropboxError::DbxServerError(text));
            }
            _ => {
                let text = res.into_string()?;
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
    T::from_res(res).map(|i| *i)
}
