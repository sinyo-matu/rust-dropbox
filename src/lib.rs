use reqwest::{Error as reqwest_error, StatusCode};
use serde::Deserialize;
use serde_json::json;

const CONTENT_END_POINT: &str = "https://content.dropboxapi.com";
const OPERATION_END_POINT: &str = "https://api.dropboxapi.com";
#[derive(Debug,Clone)]
pub struct Client {
    token: String,
}
#[derive(Debug, Deserialize)]
struct DbxRequestLimitsErrorSummary {
    error_summary: String,
    error: DbxRequestErrorReason,
}
#[derive(Debug, Deserialize)]
struct DbxRequestErrorSummary{
    error_summary:String,
    error: DbxRequestErrorTag,
}

#[derive(Debug, Deserialize)]
struct DbxRequestErrorReason { 
    reason: DbxRequestErrorTag,
    retry_after:u32,
}
#[derive(Debug, Deserialize)]
struct DbxRequestErrorTag {
    #[serde(alias = ".tag")]
    tag: String,

}


impl Client {
    pub fn new(token: &str) -> Self {
        Self {
            token: token.to_string(),
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
        let client = reqwest::Client::new();
        let res = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/octet-stream")
            .header(
                "Dropbox-API-Arg",
                json!({"path":path,"mode":mode}).to_string(),
            )
            .body(file)
            .send()
            .await?;

        handle_dbx_request_response(res).await
    }

    // binding /move_v2
    pub async fn move_file(&self,from_path:&str,to_path:&str,option:MoveOption) -> DropboxResult<()> {
        println!("moving {} to {}", from_path, to_path);
    let url = format!("{}{}", OPERATION_END_POINT, "/2/files/move_v2");
    let client = reqwest::Client::new();
    let res = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", self.token))
        .header("Content-Type", "application/json")
        .body(json!(
            {
                "from_path":from_path,
                "to_path":to_path,
                "allow_shared_folder": option.allow_shared_folder,
                "autorename": option.auto_rename,
                "allow_ownership_transfer": option.allow_ownership_transfer
            }
            ).to_string())
        .send()
        .await?;
        handle_dbx_request_response(res).await
    }
}


async fn handle_dbx_request_response(res:reqwest::Response) -> DropboxResult<()>{
    if res.status() != StatusCode::OK {
        match res.status() {
            StatusCode::BAD_REQUEST => {
                let text = res.text().await?;
                return Err(DropboxError::DbxPathError(text));
            }
            StatusCode::UNAUTHORIZED => {
                let text = res.text().await?;
                let error_summary: DbxRequestErrorSummary =
                    serde_json::from_str(&text).unwrap();
                return Err(DropboxError::DbxInvalidTokenError(
                    error_summary.error_summary,
                ));
            }
            StatusCode::FORBIDDEN => {
                let text = res.text().await?;
                let error_summary: DbxRequestErrorSummary =
                    serde_json::from_str(&text).unwrap();
                return Err(DropboxError::DbxAccessError(error_summary.error_summary));
            }
            StatusCode::CONFLICT => {
                let text = res.text().await?;
                let error_summary: DbxRequestErrorSummary =
                    serde_json::from_str(&text).unwrap();
                return Err(DropboxError::DbxConflictError(error_summary.error_summary));
            }
            StatusCode::TOO_MANY_REQUESTS => {
                let text = res.text().await?;
                match serde_json::from_str::<DbxRequestLimitsErrorSummary>(&text) {
                    Ok(error_summary) => {
                        return Err(DropboxError::DbxRequestLimitsError(
                            format!("{} , retry after {}",error_summary.error_summary,error_summary.error.retry_after)
                        ));
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
    Ok(())
}

pub struct MoveOption {
    allow_shared_folder:bool,
    auto_rename:bool,
    allow_ownership_transfer:bool,
}

impl MoveOption {
    pub fn new() -> Self {
        Self {
            allow_shared_folder:false,
            auto_rename:false,
            allow_ownership_transfer:false,
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

    pub fn allow_ownership_transfer(mut self) -> Self{
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
    DbxPathError(String),
    DbxInvalidTokenError(String),
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
#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::Read};
    use std::env;
    #[test]
    fn test_upload() {
        let token = env::var("DROPBOX_TOKEN").unwrap();
        let mut file = File::open("./profile.jpg").unwrap();
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).unwrap();
        let client = Client::new(&token);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let res =
            rt.block_on(async move { client.upload(buf, "/test/profile.jpg", UploadMode::Overwrite).await });
        assert_eq!((), res.unwrap())
    }

    #[test]
    fn test_move() {
        let token = env::var("DROPBOX_TOKEN").unwrap();
        let client = Client::new(&token);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let res = rt.block_on(
            async move {
                let move_option = MoveOption::new().allow_ownership_transfer().allow_shared_folder().auto_rename();
                client.move_file("/test/profile.jpg", "/profile.jpg", move_option).await
            }
        );
        assert_eq!((),res.unwrap())
    }
}
