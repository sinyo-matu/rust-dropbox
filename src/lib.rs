pub mod client;
mod test;

use serde::Deserialize;
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

pub type DropboxResult<T> = std::result::Result<T, DropboxError>;
#[derive(Debug)]
pub enum DropboxError {
    #[cfg(feature = "non-blocking")]
    NonBlockingRequestError(reqwest::Error),
    #[cfg(feature = "blocking")]
    BlockingRequestError(ureq::Error),
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
    #[cfg(feature = "blocking")]
    BodyParseError(std::io::Error),
}

#[cfg(feature = "blocking")]
impl From<std::io::Error> for DropboxError {
    fn from(e: std::io::Error) -> Self {
        Self::BodyParseError(e)
    }
}

#[cfg(feature = "non-blocking")]
impl From<reqwest::Error> for DropboxError {
    fn from(e: reqwest::Error) -> Self {
        Self::NonBlockingRequestError(e)
    }
}
#[cfg(feature = "blocking")]
impl From<ureq::Error> for DropboxError {
    fn from(e: ureq::Error) -> Self {
        Self::BlockingRequestError(e)
    }
}

#[derive(Clone)]
pub struct UploadOption {
    mode: UploadMode,
    allow_auto_rename: bool,
    mute_notification: bool,
    allow_strict_conflict: bool,
}
pub struct UploadOptionBuilder {
    mode: UploadMode,
    allow_auto_rename: bool,
    mute_notification: bool,
    allow_strict_conflict: bool,
}

impl UploadOptionBuilder {
    ///new will return an option with follow value
    ///mode:"add", autorename:"true", mute:"false", strict_conflict: "false"
    pub fn new() -> Self {
        Self {
            mode: UploadMode::Add,
            allow_auto_rename: true,
            mute_notification: false,
            allow_strict_conflict: false,
        }
    }

    pub fn disallow_auto_rename(&mut self) -> &mut UploadOptionBuilder {
        self.allow_auto_rename = false;
        self
    }

    pub fn mute_notification(&mut self) -> &mut UploadOptionBuilder {
        self.mute_notification = true;
        self
    }

    pub fn allow_strict_conflict(&mut self) -> &mut UploadOptionBuilder {
        self.allow_strict_conflict = true;
        self
    }

    pub fn set_upload_mode(&mut self, mode: UploadMode) -> &mut UploadOptionBuilder {
        self.mode = mode;
        self
    }

    pub fn build(&self) -> UploadOption {
        UploadOption {
            mode: self.mode.clone(),
            allow_auto_rename: self.allow_auto_rename,
            mute_notification: self.mute_notification,
            allow_strict_conflict: self.allow_strict_conflict,
        }
    }
}

#[derive(Clone)]
pub struct MoveCopyOption {
    allow_shared_folder: bool,
    auto_rename: bool,
    allow_ownership_transfer: bool,
}
pub struct MoveCopyOptionBuilder {
    allow_shared_folder: bool,
    auto_rename: bool,
    allow_ownership_transfer: bool,
}

impl MoveCopyOptionBuilder {
    ///new will return an option with follow value
    ///sheared_folder:"true", autorename:"false", ownership_transfer:"false"
    pub fn new() -> Self {
        Self {
            allow_shared_folder: false,
            auto_rename: false,
            allow_ownership_transfer: false,
        }
    }

    pub fn allow_shared_folder(&mut self) -> &mut MoveCopyOptionBuilder {
        self.allow_shared_folder = true;
        self
    }

    pub fn allow_auto_rename(&mut self) -> &mut MoveCopyOptionBuilder {
        self.auto_rename = true;
        self
    }

    pub fn allow_ownership_transfer(&mut self) -> &mut MoveCopyOptionBuilder {
        self.allow_ownership_transfer = true;
        self
    }

    pub fn build(&mut self) -> MoveCopyOption {
        MoveCopyOption {
            allow_ownership_transfer: self.allow_ownership_transfer,
            allow_shared_folder: self.allow_shared_folder,
            auto_rename: self.auto_rename,
        }
    }
}

#[derive(Clone)]
///Update will receive rev for the Update.0
pub enum UploadMode {
    Add,
    Overwrite,
    Update(String),
}
