mod operation;

pub use operation::*;
use reqwest::Error as ReqError;
use serde::Deserialize;
use std::sync::Arc;
use thiserror::Error;

use std::fmt::{Display, Formatter, Result as FmtResult};

pub type Result<T> = std::result::Result<T, SheetApiError>;

#[derive(Error, Debug)]
pub enum SheetApiError {
    #[error("HTTP error {0}")]
    ReqwestError(#[from] ReqError),

    #[error("sheet api error:bad request [{0}]")]
    BadReqestError(String),

    #[error("Spread sheet not found")]
    SpreadSheetNotFoundError(String),
}

impl SheetApiError {
    pub fn is_not_found(&self) -> bool {
        if let SheetApiError::SpreadSheetNotFoundError(_) = self {
            true
        } else {
            false
        }
    }
}

#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct SpreadSheetId(String);
impl SpreadSheetId {
    pub fn new(spread_sheet_id: String) -> Self {
        Self(spread_sheet_id)
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl From<String> for SpreadSheetId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl Display for SpreadSheetId {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct SheetName(String);
impl SheetName {
    pub fn new(sheet_name: String) -> Self {
        Self(sheet_name)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for SheetName {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

const BASE_ENDPOINT_V4: &str = "https://sheets.googleapis.com/v4/spreadsheets";
///https://developers.google.com/sheets/api/reference/rest/v4/Dimension
#[allow(dead_code)]
pub enum MajorDimension {
    Unspecified,
    Rows,
    Columns,
}

impl std::fmt::Display for MajorDimension {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let v = match self {
            Self::Unspecified => "DIMENSION_UNSPECIFIED",
            Self::Rows => "ROWS",
            Self::Columns => "COLUMNS",
        };

        write!(f, "{}", v)
    }
}

/// https://developers.google.com/sheets/api/reference/rest/v4/ValueRenderOption
#[allow(dead_code)]
pub enum ValueRenderOption {
    FormattedValue,
    UnfromattedValue,
    Formula,
}

/// https://developers.google.com/sheets/api/reference/rest/v4/DateTimeRenderOption
#[allow(dead_code)]
pub enum DateTimeRenderOption {
    SerialNumber,
    FormattedString,
}

pub enum SheetOperation {
    Get,
    BatchGet,
}

impl SheetOperation {
    pub fn endpoint(&self, spread_sheet_id: &SpreadSheetId) -> String {
        match self {
            Self::Get => {
                format!("{}/{}", BASE_ENDPOINT_V4, spread_sheet_id)
            }
            Self::BatchGet => {
                format!("{}/{}/values:batchGet", BASE_ENDPOINT_V4, spread_sheet_id)
            }
        }
    }
}
