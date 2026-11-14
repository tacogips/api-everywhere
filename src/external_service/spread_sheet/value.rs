use super::TokenManager;
use super::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

type Result<T> = std::result::Result<T, ValueError>;

#[cfg(feature = "restricted")]
pub const MAX_ROW_NUMBER_TO_READ_AT_ONCE: usize = 100;

pub const DEFAULT_ROW_NUMBER_TO_READ_AT_ONCE: usize = 100;

#[cfg(not(feature = "restricted"))]
pub const MAX_ROW_NUMBER_TO_READ_AT_ONCE: usize = 10000;

#[derive(Error, Debug, PartialEq)]
pub enum ValueError {
    #[error("failed to fetch values from api:{0}")]
    FetchValueApiError(String),

    #[error("spread sheet not found:{0}")]
    SpreadSheetNotFound(String),

    #[error("invalid row number start:{0} end:{1}")]
    InvalidRowNumber(usize, usize),

    #[error("invalid col range start:{0} end:{1}")]
    InvalidColRange(usize, usize),

    #[error("too many row number to read. max is {0}, passed {1} ")]
    TooManyRowNumber(usize, usize),

    #[cfg(feature = "restricted")]
    #[error("row index out of restriction:{0}")]
    RowIndexOutOfRescription(usize),
}

impl ValueError {
    pub fn is_not_found(&self) -> bool {
        if let ValueError::SpreadSheetNotFound(_) = self {
            true
        } else {
            false
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct CellValue(JsonValue);
impl CellValue {
    pub fn as_inner(&self) -> &JsonValue {
        &self.0
    }
}

pub struct ReadValueOption {
    spread_sheet_id: SpreadSheetId,
    sheet_name: Option<SheetName>,
    col_range: (usize, usize),
    start_row_idx: usize,
    end_row_idx: usize,
}

impl ReadValueOption {
    pub fn new(
        spread_sheet_id: SpreadSheetId,
        sheet_name: Option<SheetName>,
        col_range: (usize, usize),
        start_row_idx: usize,
        end_row_idx: usize,
    ) -> Self {
        Self {
            spread_sheet_id,
            sheet_name,
            col_range,
            start_row_idx,
            end_row_idx,
        }
    }
    pub fn validate(&self) -> Result<()> {
        let row_num = self.end_row_idx as i64 - self.start_row_idx as i64;
        if row_num < 0 {
            return Err(ValueError::InvalidRowNumber(
                self.start_row_idx,
                self.end_row_idx,
            ));
        }

        if row_num as usize > MAX_ROW_NUMBER_TO_READ_AT_ONCE {
            return Err(ValueError::TooManyRowNumber(
                MAX_ROW_NUMBER_TO_READ_AT_ONCE,
                row_num as usize,
            ));
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct RowValues {
    pub values: Vec<Vec<CellValue>>,
}

impl RowValues {
    pub fn new(values: Vec<Vec<CellValue>>) -> Self {
        Self { values }
    }

    pub fn empty() -> Self {
        Self { values: vec![] }
    }

    pub fn push(&mut self, row: Vec<CellValue>) {
        self.values.push(row)
    }

    pub async fn read_values<HttpConnector>(
        client: &ReqClient,
        token_manager: Arc<TokenManager<HttpConnector>>,
        option: &ReadValueOption,
    ) -> Result<RowValues> {
        option.validate()?;

        let (start_col, end_col) = option.col_range;
        if start_col > end_col {
            return Err(ValueError::InvalidColRange(start_col, end_col));
        }
        #[cfg(feature = "restricted")]
        if let Err(e) = is_row_range_overflow(end_col) {
            return Err(ValueError::RowIndexOutOfRescription(e));
        }

        let col_size = end_col - start_col + 1;

        let sheet_name = option.sheet_name.clone().map(|v| v.into_inner());

        let start = CellRef::new(start_col, option.start_row_idx);

        let end = CellRef::new(end_col, option.end_row_idx);

        let value_range = RangeRef::new(sheet_name.clone(), start, end);

        let sheet_values = get_sheet_value(
            &client,
            token_manager.clone(),
            &option.spread_sheet_id,
            &value_range.as_string(),
            None,
            None,
            None,
        )
        .await
        .map_err(|e| {
            if e.is_not_found() {
                ValueError::SpreadSheetNotFound(format!(
                    "sheet name: [{}] not found on spread sheet: {}",
                    sheet_name.unwrap_or_default(),
                    &option.spread_sheet_id,
                ))
            } else {
                ValueError::FetchValueApiError(format!("{}", e))
            }
        })?;

        let mut result = RowValues::default();
        if let Some(mut values) = sheet_values.value_ranges {
            if !values.is_empty() {
                let first_values = values.remove(0);
                if let Some(rows) = first_values.values {
                    for mut each_row in rows {
                        // fill tailing
                        if each_row.len() < col_size {
                            let mut padding = std::iter::repeat("".into())
                                .take(col_size - each_row.len())
                                .collect();
                            each_row.append(&mut padding);
                        }
                        result.push(each_row.into_iter().map(CellValue).collect());
                    }
                }
            }
        };
        Ok(result)
    }
}

impl Default for RowValues {
    fn default() -> Self {
        Self::new(vec![])
    }
}

#[cfg(test)]
mod test {}

#[cfg(all(test, feature = "test-using-sa"))]
mod cloud_test {

    use super::super::scopes;
    use super::super::test::*;
    use super::super::token_manager_from_service_account_file;
    use super::*;
    use tokio::sync::broadcast;

    use reqwest::Client;

    #[tokio::test]
    async fn get_values_test_valid_1() {
        let (_, rx) = broadcast::channel(1);
        let token_manager = token_manager_from_service_account_file(
            scopes::SHEET_READ_ONLY,
            load_test_sa_file_path(),
            rx,
            None,
        )
        .await
        .unwrap();
        let token_manager = Arc::new(token_manager);

        let client = Client::new();
        let sheet_id = TEST_SHEET1_ID;

        let option =
            ReadValueOption::new(SpreadSheetId::new(sheet_id.to_string()), None, (0, 9), 1, 2);

        let row_values = RowValues::read_values(&client, token_manager, &option).await;
        assert!(row_values.is_ok());

        let row_values = row_values.unwrap();

        let expected = RowValues::new(vec![
            vec![
                CellValue("Alice".into()),
                CellValue("21".into()),
                CellValue("female".into()),
                CellValue("diving".into()),
                CellValue("programming".into()),
                CellValue("politics".into()),
                CellValue("kyoto".into()),
                CellValue("111222".into()),
                CellValue("KY".into()),
                CellValue("".into()),
            ],
            vec![
                CellValue("Bob".into()),
                CellValue("34".into()),
                CellValue("male".into()),
                CellValue("shopping".into()),
                CellValue("".into()),
                CellValue("fishing".into()),
                CellValue("tokyo".into()),
                CellValue("111222".into()),
                CellValue("TK".into()),
                CellValue("".into()),
            ],
        ]);

        assert_eq!(row_values, expected);
    }

    #[tokio::test]
    async fn get_values_test_valid_2() {
        let (_, rx) = broadcast::channel(1);
        let token_manager = token_manager_from_service_account_file(
            scopes::SHEET_READ_ONLY,
            load_test_sa_file_path(),
            rx,
            None,
        )
        .await
        .unwrap();
        let token_manager = Arc::new(token_manager);

        let client = Client::new();
        let sheet_id = TEST_SHEET1_ID;

        let option =
            ReadValueOption::new(SpreadSheetId::new(sheet_id.to_string()), None, (0, 9), 4, 4);

        let row_values = RowValues::read_values(&client, token_manager, &option).await;
        assert!(row_values.is_ok());

        let row_values = row_values.unwrap();

        let expected = RowValues::new(vec![vec![
            CellValue("David".into()),
            CellValue("48".into()),
            CellValue("male".into()),
            CellValue("".into()),
            CellValue("".into()),
            CellValue("".into()),
            CellValue("".into()),
            CellValue("".into()),
            CellValue("".into()),
            CellValue("*note*".into()),
        ]]);

        assert_eq!(row_values, expected);
    }
}
