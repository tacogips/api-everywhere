use super::*;
use reqwest::Client as ReqClient;
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error;

type Result<T> = std::result::Result<T, HeaderError>;

#[derive(Error, Debug, PartialEq)]
pub enum HeaderError {
    #[error("failed to fetch sheet info:{0}")]
    FetchSheetInfoError(String),

    #[error("failed to find sheet name:{0}")]
    FetchSheetNameError(String),

    #[error("spread sheet not found:{0}")]
    SpreadSheetNotFound(String),

    #[error("multiple header not supported:{0}")]
    UnsupportedMultipleHeader(String),

    #[error("failed to fetch header values from api:{0}")]
    FetchHeaderApiError(String),

    #[error("failed to fetch header values ranges from api:{0}")]
    EmptyHeaderValueRanges(String),

    #[error("invalid range ref in returned value:{0}")]
    InvalidRangeRefInReturnedValue(String),

    #[error("empty header values. sheet [{0}]")]
    EmptyHeaderValues(String),

    #[error("cell error :{0}")]
    CellError(#[from] CellError),

    #[error("unknown error occured:{0}")]
    UnknwonError(String),

    #[cfg(feature = "restricted")]
    #[error("col index out of restriction:{0}")]
    ColIndexOutOfRescription(usize),
}

impl HeaderError {
    pub fn is_not_found(&self) -> bool {
        if let HeaderError::SpreadSheetNotFound(_) = self {
            true
        } else {
            false
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct HeaderSearchCondition {
    pub spread_sheet_id: SpreadSheetId,
    pub sheet_name: Option<SheetName>,
    pub specified_cell_range: Option<(CellRef, CellRef)>,
    pub sheet_info: Sheet,
}

impl HeaderSearchCondition {
    pub fn new(
        spread_sheet_id: SpreadSheetId,
        sheet_name: Option<SheetName>,
        specified_cell_range: Option<(CellRef, CellRef)>,
        sheet_info: Sheet,
    ) -> Self {
        Self {
            spread_sheet_id,
            sheet_name,
            specified_cell_range,
            sheet_info,
        }
    }

    pub fn as_range(&self) -> Option<RangeRef> {
        match &self.specified_cell_range {
            Some((start, end)) => Some(RangeRef {
                sheet_name: self
                    .sheet_name
                    .as_ref()
                    .map(|name| name.as_str().to_string()),
                start: start.clone(),
                end: end.clone(),
            }),

            None => None,
        }
    }

    pub async fn create<HttpConnector>(
        client: &ReqClient,
        token_manager: Arc<TokenManager<HttpConnector>>,
        meta: SheetMeta,
        specified_cell_range: Option<(CellRef, CellRef)>,
    ) -> Result<HeaderSearchCondition> {
        let spread_sheet_id = SpreadSheetId::new(meta.spread_sheet_id);

        let sheet_info = api::get_sheet(client, token_manager, &spread_sheet_id)
            .await
            .map_err(|e| {
                if e.is_not_found() {
                    HeaderError::SpreadSheetNotFound(format!(
                        "spread sheet:{} not found",
                        spread_sheet_id
                    ))
                } else {
                    log::error!(
                        "error on fetching spread sheet: {},  error:{}",
                        spread_sheet_id,
                        e
                    );
                    HeaderError::FetchSheetInfoError(format!("spread sheet {}:", spread_sheet_id,))
                }
            })?;

        let sheet_name = match meta.sheet_id_or_name.is_need_get_sheet_name_by_id() {
            Some(sheet_id) => {
                match sheet_info
                    .find_property_by_id(sheet_id)
                    .map(|prop| SheetName::new(prop.properties.title.to_string()))
                {
                    v @ Some(_) => v,
                    None => {
                        return Err(HeaderError::FetchSheetNameError(format!(
                            "spread sheet :{} {}",
                            spread_sheet_id, sheet_id
                        )))
                    }
                }
            }
            None => meta
                .sheet_id_or_name
                .sheet_name()
                .map(|name| SheetName::new(name)),
        };

        Ok(Self::new(
            spread_sheet_id,
            sheet_name,
            specified_cell_range,
            sheet_info,
        ))
    }
}

fn default_header_range(sheet_name: Option<&SheetName>) -> RangeRef {
    RangeRef::new(
        sheet_name.map(|e| e.clone().into_inner()),
        CellRef::new(0, 0),  //A1
        CellRef::new(25, 0), //Z1
    )
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct RecordHeader(String);

impl RecordHeader {
    fn new(s: &str) -> Result<Self> {
        let s = Cell::sanitize_str_value(s)?;
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn empty_values(len: usize) -> Vec<RecordHeader> {
        (0..len)
            .into_iter()
            .map(|_| RecordHeader("".to_string()))
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct RawHeaders {
    pub range: RangeRef,
    pub values: Vec<RecordHeader>,
}

impl RawHeaders {
    pub(crate) fn convert_from(
        value_ranges: Vec<ValueRange>,
        specified_range: bool,
    ) -> Result<RawHeaders> {
        if value_ranges.is_empty() {
            return Err(HeaderError::InvalidRangeRefInReturnedValue("".to_string()));
        }

        let mut merged_range: Option<RangeRef> = None;
        let mut all_headers: Vec<RecordHeader> = Vec::new();

        for each_value_range in value_ranges {
            let mut each_response_header_range = RangeRef::from_str(&each_value_range.range)
                .map_err(|e| {
                    log::error!("invalid range ref in returned value of header {}", e);
                    HeaderError::InvalidRangeRefInReturnedValue(each_value_range.range.clone())
                })?;

            if each_value_range.values.is_none() {
                break;
            }

            let mut each_header_values = each_value_range.values.unwrap();
            if each_header_values.is_empty() {
                break;
            }

            // not multiple header lines
            let each_header_values = each_header_values.swap_remove(0);

            let (headers, range) = if specified_range {
                let headers_values = if each_header_values.is_empty() {
                    RecordHeader::empty_values(each_response_header_range.col_range_size())
                } else {
                    let header_value_len = each_header_values.len();
                    let mut headers = Vec::<RecordHeader>::new();
                    for each in each_header_values.into_iter() {
                        headers.push(RecordHeader::new(each.to_string().as_ref())?);
                    }

                    let padding_size: i64 = each_response_header_range.col_range_size() as i64
                        - header_value_len as i64;
                    if padding_size > 0 {
                        headers.extend(RecordHeader::empty_values(padding_size as usize));
                    }
                    headers
                };

                (headers_values, each_response_header_range)
            } else {
                // split by empty cell
                let empty_splited_headers = each_header_values
                    .as_slice()
                    .split(|each_header_value| {
                        each_header_value.to_string().as_str() == ""
                            || each_header_value.to_string().as_str() == "\"\""
                    })
                    .next()
                    .unwrap();

                let mut headers = Vec::<RecordHeader>::new();
                for each in empty_splited_headers.into_iter() {
                    headers.push(RecordHeader::new(each.to_string().as_ref())?);
                }

                if headers.is_empty() {
                    if merged_range.is_none() {
                        return Err(HeaderError::EmptyHeaderValues(format!(
                            "range:{}",
                            each_value_range.range
                        )));
                    } else {
                        (vec![], each_response_header_range)
                    }
                } else {
                    each_response_header_range
                        .set_end_col_index(
                            each_response_header_range.start.col_index + headers.len() - 1,
                        )
                        .map_err(|e| {
                            HeaderError::UnknwonError(format!(
                                "invalid response header range : {} {}",
                                each_response_header_range, e
                            ))
                        })?;
                    (headers, each_response_header_range)
                }
            };

            match merged_range.as_mut() {
                None => merged_range = Some(range),
                Some(prev_merged_range) => prev_merged_range.expand(&range),
            };
            all_headers.extend(headers);
        }

        match merged_range {
            None => Err(HeaderError::EmptyHeaderValues("value range ".to_string())),
            Some(merged_range) => Ok(RawHeaders {
                range: merged_range,
                values: all_headers,
            }),
        }
    }

    pub async fn read_raw_headers<HttpConnector>(
        client: &ReqClient,
        token_manager: Arc<TokenManager<HttpConnector>>,
        condition: &HeaderSearchCondition,
    ) -> Result<RawHeaders> {
        let specified_range = condition.specified_cell_range.is_some();
        let mut header_range = match condition.as_range() {
            Some(range) => range,
            None => default_header_range(condition.sheet_name.as_ref()),
        };

        if !header_range.is_one_line_row() {
            log::warn!("header range is multiple line :{}", header_range);
            return Err(HeaderError::UnsupportedMultipleHeader(format!(
                "{}",
                header_range
            )));
        }

        log::debug!("fetching header range :{}", header_range);

        let mut all_sheet_values: Option<SheetValues> = None;

        let max_col_count_of_grid = {
            let sheet_name = condition.sheet_name.as_ref().map(|s| s.as_str());
            match condition.sheet_info.find_property_by_name(sheet_name) {
                None => {
                    return Err(HeaderError::UnknwonError(format!(
                        "sheet info not found:{:?}",
                        sheet_name
                    )))
                }
                Some(property) => property.properties.grid_properties.column_count,
            }
        };

        let mut range_str: String = "".to_string();
        loop {
            #[cfg(feature = "restricted")]
            if let Err(e) = is_col_range_overflow(header_range.end_col_index()) {
                return Err(HeaderError::ColIndexOutOfRescription(e));
            }

            if max_col_count_of_grid <= header_range.start_col_index() {
                break;
            }
            range_str = header_range.as_string();

            let sheet_values = get_sheet_value(
                &client,
                token_manager.clone(),
                &condition.spread_sheet_id,
                &range_str,
                None,
                None,
                None,
            )
            .await
            .map_err(|e| {
                if e.is_not_found() {
                    match header_range.sheet_name.clone() {
                        Some(sheet_name) => HeaderError::SpreadSheetNotFound(format!(
                            "sheet name :{} is not found in spread sheet {}",
                            sheet_name, &condition.spread_sheet_id,
                        )),
                        None => HeaderError::SpreadSheetNotFound(format!(
                            "spread sheet {} is not found",
                            &condition.spread_sheet_id,
                        )),
                    }
                } else {
                    HeaderError::FetchHeaderApiError(format!("{}", e))
                }
            });
            let sheet_values = match sheet_values {
                Err(e) => {
                    log::error!("fetch header error :{:?}", e);
                    return Err(e);
                }
                Ok(sheet_values) => sheet_values,
            };

            if specified_range {
                all_sheet_values = Some(sheet_values);
                break;
            } else {
                // if returned value contains empty data or blank data, finish the loop
                let is_break = match sheet_values.value_ranges.as_ref() {
                    Some(value_ranges) => match value_ranges.first() {
                        None => true,
                        Some(first_value_line) => match first_value_line.values.as_ref() {
                            Some(values) => match values.first() {
                                None => true,
                                Some(line_values) => line_values
                                    .iter()
                                    .find(|each| each.to_string().as_str() == "")
                                    .is_some(),
                            },

                            None => true,
                        },
                    },

                    None => true,
                };

                match all_sheet_values.as_mut() {
                    None => all_sheet_values = Some(sheet_values),
                    Some(prev_all_values) => prev_all_values.merge(sheet_values),
                }

                if is_break {
                    break;
                }
                header_range.shift_in_col(26).unwrap();
            }
        }

        if all_sheet_values.is_none() {
            return Err(HeaderError::EmptyHeaderValues(format!(
                "{} {:?}",
                condition.spread_sheet_id.to_string(),
                header_range.sheet_name
            )));
        }

        let value_ranges = match all_sheet_values.unwrap().value_ranges {
            // several ranges  not suppoorted so we take only first element
            Some(value_ranges) => {
                if value_ranges.is_empty() {
                    return Err(HeaderError::EmptyHeaderValueRanges(format!(
                        "latest range {}",
                        &range_str
                    )));
                } else {
                    value_ranges
                }
            }
            None => {
                return Err(HeaderError::EmptyHeaderValueRanges(format!(
                    "latest range {}",
                    &range_str
                )))
            }
        };

        let result = Self::convert_from(value_ranges, specified_range)?;
        Ok(result)
    }
}

#[cfg(test)]
mod test {}

#[cfg(all(test, feature = "test-using-sa"))]
mod cloud_test {

    use super::super::api::*;
    use super::super::scopes;
    use super::super::test::*;
    use super::super::token_manager_from_service_account_file;
    use super::*;
    use reqwest::Client;
    use tokio::sync::broadcast;

    fn get_expected_sheet_info() -> Sheet {
        Sheet {
            spreadsheet_id: TEST_SHEET1_ID.to_string(),
            sheets: vec![
                SheetProperty {
                    properties: SheetPropertyData {
                        sheet_id: 0,
                        title: "grouping".to_string(),
                        index: 0,
                        sheet_type: "GRID".to_string(),
                        grid_properties: GridProperties {
                            row_count: 1000,
                            column_count: 28,
                            frozen_row_count: None,
                            frozen_column_count: None,
                            hide_gridlines: None,
                            row_group_control_after: None,
                            column_group_control_after: None,
                        },
                    },
                },
                SheetProperty {
                    properties: SheetPropertyData {
                        sheet_id: 2089556915,
                        title: "empty_sheet".to_string(),
                        index: 1,
                        sheet_type: "GRID".to_string(),
                        grid_properties: GridProperties {
                            row_count: 1000,
                            column_count: 26,
                            frozen_row_count: None,
                            frozen_column_count: None,
                            hide_gridlines: None,
                            row_group_control_after: None,
                            column_group_control_after: None,
                        },
                    },
                },
                SheetProperty {
                    properties: SheetPropertyData {
                        sheet_id: 1351342444,
                        title: "many headers".to_string(),
                        index: 2,
                        sheet_type: "GRID".to_string(),
                        grid_properties: GridProperties {
                            row_count: 1000,
                            column_count: 81,
                            frozen_row_count: None,
                            frozen_column_count: None,
                            hide_gridlines: None,
                            row_group_control_after: None,
                            column_group_control_after: None,
                        },
                    },
                },
                SheetProperty {
                    properties: SheetPropertyData {
                        sheet_id: 921136108,
                        title: "header escape".to_string(),
                        index: 3,
                        sheet_type: "GRID".to_string(),
                        grid_properties: GridProperties {
                            row_count: 1000,
                            column_count: 26,
                            frozen_row_count: None,
                            frozen_column_count: None,
                            hide_gridlines: None,
                            row_group_control_after: None,
                            column_group_control_after: None,
                        },
                    },
                },
            ],
        }
    }

    #[tokio::test]
    async fn get_header_test_valid_1() {
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

        let sheet_mata = SheetMeta::new(sheet_id.to_string(), Some(TEST_SHEET1_EMPTY_TAG_ID), None);

        let result = HeaderSearchCondition::create(&client, token_manager, sheet_mata, None).await;
        assert!(result.is_ok());

        let expected = HeaderSearchCondition::new(
            SpreadSheetId::new(TEST_SHEET1_ID.to_string()),
            Some(SheetName::new("empty_sheet".to_string())),
            None,
            get_expected_sheet_info(),
        );

        assert_eq!(expected, result.unwrap());
    }

    #[tokio::test]
    async fn get_header_test_valid_2() {
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

        let sheet_mata = SheetMeta::new(sheet_id.to_string(), None, None);
        let result = HeaderSearchCondition::create(&client, token_manager, sheet_mata, None).await;
        assert!(result.is_ok());

        let expected = HeaderSearchCondition::new(
            SpreadSheetId::new(TEST_SHEET1_ID.to_string()),
            None,
            None,
            get_expected_sheet_info(),
        );

        assert_eq!(expected, result.unwrap());
    }

    #[tokio::test]
    async fn get_header_test_valid_3() {
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

        let sheet_mata = SheetMeta::new(
            sheet_id.to_string(),
            None,
            Some("name_specified_sheet".to_string()),
        );
        let result = HeaderSearchCondition::create(&client, token_manager, sheet_mata, None).await;
        assert!(result.is_ok());

        let expected = HeaderSearchCondition::new(
            SpreadSheetId::new(TEST_SHEET1_ID.to_string()),
            Some(SheetName::new("name_specified_sheet".to_string())),
            None,
            get_expected_sheet_info(),
        );

        assert_eq!(expected, result.unwrap());
    }

    #[tokio::test]
    async fn get_header_test_valid_4() {
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

        let sheet_mata = SheetMeta::new(sheet_id.to_string(), Some(0), None);
        let result = HeaderSearchCondition::create(&client, token_manager, sheet_mata, None).await;
        assert!(result.is_ok());

        let expected = HeaderSearchCondition::new(
            SpreadSheetId::new(TEST_SHEET1_ID.to_string()),
            None,
            None,
            get_expected_sheet_info(),
        );

        assert_eq!(expected, result.unwrap());
    }

    #[tokio::test]
    async fn get_header_test_invalid_1() {
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
        let sheet_not_exist_tag_id = 12345;

        let sheet_mata = SheetMeta::new(sheet_id.to_string(), Some(sheet_not_exist_tag_id), None);
        let result = HeaderSearchCondition::create(&client, token_manager, sheet_mata, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn get_headers_from_sheet_1() {
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

        let sheet_mata = SheetMeta::new(sheet_id.to_string(), None, None);
        let condition =
            HeaderSearchCondition::create(&client, token_manager.clone(), sheet_mata, None)
                .await
                .unwrap();

        let result = RawHeaders::read_raw_headers(&client, token_manager, &condition).await;

        assert!(result.is_ok());
        let result = result.unwrap();

        let values: Vec<RecordHeader> = vec![
            RecordHeader("name".to_string()),
            RecordHeader("age".to_string()),
            RecordHeader("sex".to_string()),
            RecordHeader("favorite".to_string()),
            RecordHeader("favorite".to_string()),
            RecordHeader("favorite".to_string()),
            RecordHeader("address.city.name".to_string()),
            RecordHeader("address.zipcode".to_string()),
            RecordHeader("address.city.code".to_string()),
            RecordHeader("link title note".to_string()),
        ];
        let expected = RawHeaders {
            range: RangeRef::from_str("grouping!A1:J1").unwrap(),
            values,
        };

        assert_eq!(result, expected)
    }

    #[tokio::test]
    async fn get_headers_from_sheet_2() {
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

        let sheet_mata = SheetMeta::new(
            sheet_id.to_string(),
            None,
            Some("header escape".to_string()),
        );
        let condition = HeaderSearchCondition::create(
            &client,
            token_manager.clone(),
            sheet_mata,
            Some((
                CellRef::from_str("B2").unwrap(),
                CellRef::from_str("D2").unwrap(),
            )),
        )
        .await
        .unwrap();

        let result = RawHeaders::read_raw_headers(&client, token_manager, &condition).await;

        assert!(result.is_ok());
        let result = result.unwrap();

        let values: Vec<RecordHeader> = vec![
            RecordHeader("".to_string()),
            RecordHeader(r#"""#.to_string()),
            RecordHeader("".to_string()),
        ];
        let expected = RawHeaders {
            range: RangeRef::from_str("header escape!B2:D2").unwrap(),
            values,
        };

        assert_eq!(result, expected)
    }

    #[tokio::test]
    async fn get_headers_from_sheet_3() {
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

        let sheet_mata =
            SheetMeta::new(sheet_id.to_string(), None, Some("many headers".to_string()));

        let condition =
            HeaderSearchCondition::create(&client, token_manager.clone(), sheet_mata, None)
                .await
                .unwrap();

        let result = RawHeaders::read_raw_headers(&client, token_manager, &condition).await;

        assert!(result.is_ok());
        let result = result.unwrap();

        let values: Vec<RecordHeader> = (1..=81)
            .into_iter()
            .map(|n| RecordHeader(format!("h{}", n)))
            .collect();

        let expected = RawHeaders {
            range: RangeRef::from_str("many headers!A1:CC1").unwrap(),
            values,
        };

        assert_eq!(result, expected)
    }

    #[tokio::test]
    async fn get_headers_from_sheet_4_invalid() {
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

        let sheet_mata = SheetMeta::new(
            sheet_id.to_string(),
            None,
            Some("not existing shet".to_string()),
        );

        let condition =
            HeaderSearchCondition::create(&client, token_manager.clone(), sheet_mata, None)
                .await
                .unwrap();

        let result = RawHeaders::read_raw_headers(&client, token_manager, &condition).await;

        assert!(result.is_err());
    }
}
