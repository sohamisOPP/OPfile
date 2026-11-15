use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Serialize)]
pub enum ErrorCode {
    IoError,
    PermissionDenied,
    NotFound,
    ArchiveError,
    ConversionError,
    UnsupportedFormat,
    CryptoError,
    InvalidArgument,
    Cancelled,
    PanicRecovered,
}

#[derive(Debug)]
pub struct OpfileError {
    pub code: ErrorCode,
    pub message: String,
}

impl OpfileError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn io(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::IoError, msg)
    }

    pub fn archive(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::ArchiveError, msg)
    }

    pub fn convert(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::ConversionError, msg)
    }

    pub fn unsupported(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::UnsupportedFormat, msg)
    }

    pub fn crypto(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::CryptoError, msg)
    }

    pub fn invalid(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidArgument, msg)
    }

    pub fn cancelled() -> Self {
        Self::new(ErrorCode::Cancelled, "Task cancelled by user")
    }
}

impl fmt::Display for OpfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)
    }
}

impl std::error::Error for OpfileError {}

impl From<std::io::Error> for OpfileError {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::PermissionDenied => {
                Self::new(ErrorCode::PermissionDenied, e.to_string())
            }
            std::io::ErrorKind::NotFound => Self::new(ErrorCode::NotFound, e.to_string()),
            _ => Self::new(ErrorCode::IoError, e.to_string()),
        }
    }
}

impl From<serde_json::Error> for OpfileError {
    fn from(e: serde_json::Error) -> Self {
        Self::invalid(e.to_string())
    }
}

pub type OpResult<T> = Result<T, OpfileError>;

#[derive(Serialize)]
pub struct ApiEnvelope<T: Serialize> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiErrorPayload>,
}

#[derive(Serialize)]
pub struct ApiErrorPayload {
    pub code: ErrorCode,
    pub message: String,
}

impl<T: Serialize> ApiEnvelope<T> {
    pub fn ok(data: T) -> String {
        serde_json::to_string(&Self {
            ok: true,
            data: Some(data),
            error: None,
        })
        .unwrap_or_else(|_| r#"{"ok":false,"error":{"code":"InvalidArgument","message":"Serialization failed"}}"#.to_string())
    }
}

impl ApiEnvelope<()> {
    pub fn err(err: OpfileError) -> String {
        serde_json::to_string(&ApiEnvelope::<()> {
            ok: false,
            data: None,
            error: Some(ApiErrorPayload {
                code: err.code,
                message: err.message,
            }),
        })
        .unwrap_or_else(|_| r#"{"ok":false,"error":{"code":"InvalidArgument","message":"Serialization failed"}}"#.to_string())
    }
}
