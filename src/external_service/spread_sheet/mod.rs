mod api;
mod cell;
mod header;
mod range;
mod restricted;
mod sheet;
mod token_manager;
mod value;

pub use api::*;
pub use cell::*;
pub use header::*;
use once_cell::sync::OnceCell;
pub use range::*;
use reqwest::Client as ReqClient;
#[cfg(feature = "restricted")]
use restricted::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
pub use sheet::*;
use std::sync::Arc;
use thiserror::Error;
pub use token_manager::*;
pub use value::*;

static REQWEST_CLIENT: OnceCell<ReqClient> = OnceCell::new();

fn reqwest_client() -> &'static ReqClient {
    REQWEST_CLIENT.get_or_init(|| ReqClient::new())
}

type Result<T> = std::result::Result<T, SpreadSheetError>;

#[derive(Error, Debug, PartialEq)]
pub enum SpreadSheetError {
    #[error("header error :{0}")]
    HeaderError(#[from] HeaderError),

    #[error("value error :{0}")]
    ValueError(#[from] ValueError),
}

impl SpreadSheetError {
    pub fn is_not_found(&self) -> bool {
        if let SpreadSheetError::HeaderError(e) = self {
            e.is_not_found()
        } else if let SpreadSheetError::ValueError(e) = self {
            e.is_not_found()
        } else {
            false
        }
    }
}

pub mod scopes {
    pub const SHEET_READ_ONLY: &[&'static str] =
        &["https://www.googleapis.com/auth/spreadsheets.readonly"];
}

pub struct FetchRowCondition {
    specific_row_idx: Option<usize>,
    pagination: Option<Pagination>,
}

impl FetchRowCondition {
    pub fn with_specific_row_idx(row_idx: usize) -> Self {
        Self {
            specific_row_idx: Some(row_idx),
            pagination: None,
        }
    }

    pub fn with_pagination(offset: Option<usize>, limit: Option<usize>) -> Self {
        Self {
            specific_row_idx: None,
            pagination: Some(Pagination::new(offset, limit)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    offset: Option<usize>,
    limit: Option<usize>,
}

impl Pagination {
    pub fn new(offset: Option<usize>, limit: Option<usize>) -> Self {
        Self { offset, limit }
    }
}

pub async fn create_header_condition_from_sheet_meta<HttpConnector>(
    token_manager: Arc<TokenManager<HttpConnector>>,
    sheet_meta: SheetMeta,
    specified_cell_range: Option<(CellRef, CellRef)>,
) -> Result<HeaderSearchCondition> {
    //TODO(tacogips)  restriction
    let client = reqwest_client();
    let header_condition =
        HeaderSearchCondition::create(client, token_manager, sheet_meta, specified_cell_range)
            .await?;
    Ok(header_condition)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SheetValueResponse {
    pub headers: RawHeaders,
    pub row_values: RowValues,
    pub pagination: Option<Pagination>,
}

impl SheetValueResponse {
    pub fn is_empty(&self) -> bool {
        self.row_values.values.is_empty()
    }
}

pub async fn fetch_sheet_value<HttpConnector>(
    token_manager: Arc<TokenManager<HttpConnector>>,
    header_search_condition: &HeaderSearchCondition,
    row_serach_condition: &FetchRowCondition,
) -> Result<SheetValueResponse> {
    //TODO(tacogips)  restriction
    let client = reqwest_client();

    let headers =
        RawHeaders::read_raw_headers(&client, token_manager.clone(), header_search_condition)
            .await?;

    let value_col_range = headers.range.col_range_indices();

    let (start_row_idx, finish_row_idx, pagination_in_response) =
        if let Some(specific_row_idx) = row_serach_condition.specific_row_idx {
            let row_idx = headers.range.next_row_index() + specific_row_idx;
            (row_idx, row_idx, None)
        } else {
            let (offset, limit) = match &row_serach_condition.pagination {
                None => (0, DEFAULT_ROW_NUMBER_TO_READ_AT_ONCE),
                Some(pagination) => {
                    let offset = pagination.offset.unwrap_or(0);
                    let limit = pagination
                        .limit
                        .unwrap_or(DEFAULT_ROW_NUMBER_TO_READ_AT_ONCE);
                    (offset, limit)
                }
            };

            let start_row_idx = headers.range.next_row_index() + offset;
            let finish_row_idx = start_row_idx + limit;
            (
                start_row_idx,
                finish_row_idx,
                Some(Pagination::new(Some(offset), Some(limit))),
            )
        };

    let max_row_count_of_grid = {
        let sheet_name = header_search_condition
            .sheet_name
            .as_ref()
            .map(|s| s.as_str());
        match header_search_condition
            .sheet_info
            .find_property_by_name(sheet_name)
        {
            None => {
                return Err(HeaderError::UnknwonError(format!(
                    "sheet info not found:{:?}",
                    sheet_name
                )))?
            }
            Some(property) => property.properties.grid_properties.row_count,
        }
    };

    if max_row_count_of_grid <= start_row_idx {
        return Ok(SheetValueResponse {
            headers,
            row_values: RowValues::empty(),
            pagination: pagination_in_response,
        });
    } else {
        let value_option = ReadValueOption::new(
            header_search_condition.spread_sheet_id.clone(),
            header_search_condition.sheet_name.clone(),
            value_col_range,
            start_row_idx,
            finish_row_idx,
        );

        let row_values =
            RowValues::read_values(&client, token_manager.clone(), &value_option).await?;

        return Ok(SheetValueResponse {
            headers,
            row_values,
            pagination: pagination_in_response,
        });
    }
}

#[cfg(test)]
mod test {
    #[cfg(feature = "test-using-sa")]
    use std::path::PathBuf;

    pub const TEST_SHEET1 :&str= "https://docs.google.com/spreadsheets/d/1HA4munsvl5UUlb9DKmJvhrwfGlSQ97hSQZf13M3ZO4Y/edit#gid=0";
    pub const TEST_SHEET1_ID: &str = "1HA4munsvl5UUlb9DKmJvhrwfGlSQ97hSQZf13M3ZO4Y";

    pub const TEST_SHEET1_WITH_TAG_ID :&str= "https://docs.google.com/spreadsheets/d/1HA4munsvl5UUlb9DKmJvhrwfGlSQ97hSQZf13M3ZO4Y/edit#gid=2089556915";
    pub const TEST_SHEET1_EMPTY_TAG_ID: u32 = 2089556915;

    #[cfg(feature = "test-using-sa")]
    pub fn load_test_sa_file_path() -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("dev-secret/test-sa-key.json");
        p
    }
}
