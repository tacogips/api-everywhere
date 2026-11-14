use super::super::TokenManager;
use super::*;
use reqwest::{header, Client as ReqClient, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Sheet {
    pub spreadsheet_id: String,
    pub sheets: Vec<SheetProperty>,
}

impl Sheet {
    pub fn find_property_by_id(&self, sheet_id: u32) -> Option<&SheetProperty> {
        self.sheets
            .iter()
            .find(|e| e.properties.sheet_id == sheet_id)
    }

    pub fn find_property_by_name(&self, name: Option<&str>) -> Option<&SheetProperty> {
        match name {
            None => self.sheets.get(0),
            Some(name) => self.sheets.iter().find(|e| e.properties.title == name),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SheetProperty {
    pub properties: SheetPropertyData,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SheetPropertyData {
    pub sheet_id: u32,
    pub title: String,
    pub index: usize,
    pub sheet_type: String,
    pub grid_properties: GridProperties,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GridProperties {
    pub row_count: usize,
    pub column_count: usize,
    pub frozen_row_count: Option<usize>,
    pub frozen_column_count: Option<usize>,
    pub hide_gridlines: Option<bool>,
    pub row_group_control_after: Option<bool>,
    pub column_group_control_after: Option<bool>,
}

///https://developers.google.com/sheets/api/reference/rest/v4/spreadsheets#Spreadsheet
pub async fn get_sheet<HttpConnector>(
    client: &ReqClient,
    token_manager: Arc<TokenManager<HttpConnector>>,
    spread_sheet_id: &SpreadSheetId,
) -> Result<Sheet> {
    let url = SheetOperation::Get.endpoint(spread_sheet_id);

    let req_header = {
        let auth_token = token_manager.current_token().load();
        request_header(auth_token.as_str()).await
    };

    let response = client.get(&url).headers(req_header).send().await?;
    let result = if response.status() == StatusCode::NOT_FOUND {
        return Err(SheetApiError::SpreadSheetNotFoundError(format!(
            "{}",
            spread_sheet_id
        )));
    } else if response.status() == StatusCode::BAD_REQUEST {
        let json_value: JsonValue = response.json().await?;
        log::error!("sheet apid error :{}", json_value);

        return Err(SheetApiError::BadReqestError(format!("{}", json_value)));
    } else {
        response.json().await?
    };

    Ok(result)
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SheetValues {
    pub spreadsheet_id: String,
    pub value_ranges: Option<Vec<ValueRange>>,
}

impl SheetValues {
    pub fn merge(&mut self, other: SheetValues) {
        if other.value_ranges.is_none() {
            return;
        }

        match self.value_ranges.as_mut() {
            None => self.value_ranges = other.value_ranges,
            Some(value_ranges) => value_ranges.extend(other.value_ranges.unwrap()),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ValueRange {
    pub range: String,
    pub major_dimension: String,
    pub values: Option<Vec<Vec<JsonValue>>>,
}

/// https://developers.google.com/sheets/api/reference/rest/v4/spreadsheets.values/batchGet
pub async fn get_sheet_value<HttpConnector>(
    client: &ReqClient,
    token_manager: Arc<TokenManager<HttpConnector>>,
    spread_sheet_id: &SpreadSheetId,
    ranges: &str,
    _major_dimension: Option<MajorDimension>,
    _value_render_option: Option<ValueRenderOption>,
    _date_time_render_option: Option<DateTimeRenderOption>,
) -> Result<SheetValues> {
    let url = SheetOperation::BatchGet.endpoint(spread_sheet_id);

    let req_header = {
        let auth_token = token_manager.current_token().load();
        request_header(auth_token.as_str()).await
    };
    let query_param = vec![("ranges", ranges)];

    let response = client
        .get(&url)
        .headers(req_header)
        .query(&query_param)
        .send()
        .await?;

    let result = if response.status() == StatusCode::NOT_FOUND {
        return Err(SheetApiError::SpreadSheetNotFoundError(format!(
            "{}",
            spread_sheet_id
        )));
    } else if response.status() == StatusCode::BAD_REQUEST {
        let json_value: JsonValue = response.json().await?;
        log::error!("sheet apid error :{}", json_value);

        return Err(SheetApiError::BadReqestError(format!("{}", json_value)));
    } else {
        response.json().await?
    };

    Ok(result)
}

async fn request_header(token: &str) -> header::HeaderMap {
    let mut result = header::HeaderMap::new();
    result.insert(
        header::AUTHORIZATION,
        format!("Bearer {}", token).parse().unwrap(),
    );
    result
}

#[cfg(all(test, feature = "test-using-sa"))]
mod test {
    use super::super::super::scopes;
    use super::super::super::test::{load_test_sa_file_path, TEST_SHEET1_ID};
    use super::super::super::token_manager_from_service_account_file;
    use super::*;
    use reqwest::Client;
    use serde_json::Value as JsonValue;
    use std::sync::Arc;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn api_get_sheet_test() {
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

        let result = get_sheet(
            &client,
            token_manager,
            &SpreadSheetId::new(sheet_id.to_string()),
        )
        .await;

        let sheets = vec![
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
        ];

        let expected = Sheet {
            spreadsheet_id: "1HA4munsvl5UUlb9DKmJvhrwfGlSQ97hSQZf13M3ZO4Y".to_string(),
            sheets,
        };

        assert_eq!(expected, result.unwrap());
    }

    fn jstr(s: &str) -> JsonValue {
        JsonValue::String(s.to_string())
    }

    #[tokio::test]
    async fn api_get_sheet_value_test() {
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
        let ranges = "A1:J4";

        let result = get_sheet_value(
            &client,
            token_manager,
            &SpreadSheetId::new(sheet_id.to_string()),
            ranges,
            None,
            None,
            None,
        )
        .await;

        let expected = SheetValues {
            spreadsheet_id: "1HA4munsvl5UUlb9DKmJvhrwfGlSQ97hSQZf13M3ZO4Y".to_string(),
            value_ranges: Some(vec![ValueRange {
                range: "grouping!A1:J4".to_string(),
                major_dimension: "ROWS".to_string(),
                values: Some(vec![
                    vec![
                        jstr("name"),
                        jstr("age"),
                        jstr("sex"),
                        jstr("favorite"),
                        jstr("favorite"),
                        jstr("favorite"),
                        jstr("address.city.name"),
                        jstr("address.zipcode"),
                        jstr("address.city.code"),
                        jstr("link title note"),
                    ],
                    vec![
                        jstr("Alice"),
                        jstr("21"),
                        jstr("female"),
                        jstr("diving"),
                        jstr("programming"),
                        jstr("politics"),
                        jstr("kyoto"),
                        jstr("111222"),
                        jstr("KY"),
                    ],
                    vec![
                        jstr("Bob"),
                        jstr("34"),
                        jstr("male"),
                        jstr("shopping"),
                        jstr(""),
                        jstr("fishing"),
                        jstr("tokyo"),
                        jstr("111222"),
                        jstr("TK"),
                    ],
                    vec![
                        jstr("Charlie"),
                        jstr("18"),
                        jstr("male"),
                        jstr(""),
                        jstr("boxing"),
                        jstr(""),
                        jstr("yokohama"),
                        jstr("2223333"),
                        jstr("YK"),
                    ],
                ]),
            }]),
        };

        assert_eq!(expected, result.unwrap());
    }

    #[tokio::test]
    async fn api_get_not_exist_sheet() {
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
        let sheet_id = "not_exists_xxxxx123123";

        let result = get_sheet(
            &client,
            token_manager,
            &SpreadSheetId::new(sheet_id.to_string()),
        )
        .await;

        assert!(result.is_err());
    }
}
