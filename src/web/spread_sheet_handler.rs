use crate::external_service::spread_sheet::*;
use crate::json_structure;
use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value as JsonValue;
use std::sync::Arc;

// The query parameters for todos index
#[derive(Debug, Deserialize, Default)]
pub struct GetSpreadSheetQuery {
    pub sheet_id: Option<u32>,
    pub sheet_name: Option<String>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
    pub row: Option<usize>,
}

impl GetSpreadSheetQuery {
    fn return_as_single_obj(&self) -> bool {
        self.row.is_some()
    }
    fn as_header_sheet_meta(&self, spread_sheet_id: SpreadSheetId) -> SheetMeta {
        SheetMeta::new(
            spread_sheet_id.into_inner(),
            self.sheet_id.clone(),
            self.sheet_name.clone(),
        )
    }

    fn as_row_search_condition(&self) -> FetchRowCondition {
        if let Some(row) = self.row {
            FetchRowCondition::with_specific_row_idx(row)
        } else {
            FetchRowCondition::with_pagination(self.offset, self.limit)
        }
    }
}

pub async fn get_spread_sheet_value<HttpConnector>(
    Path(spread_sheet_id): Path<SpreadSheetId>,
    query: Query<GetSpreadSheetQuery>,
    Extension(token_manager): Extension<Arc<TokenManager<HttpConnector>>>,
) -> impl IntoResponse
where
    HttpConnector: Clone + Send + Sync + 'static,
{
    let sheet_meta = query.as_header_sheet_meta(spread_sheet_id);
    inner_get_spread_sheet_value(
        sheet_meta,
        query.as_row_search_condition(),
        query.return_as_single_obj(),
        token_manager.clone(),
    )
    .await
}

pub async fn inner_get_spread_sheet_value<HttpConnector>(
    sheet_meta: SheetMeta,
    row_search_condition: FetchRowCondition,
    return_as_single_obj: bool,
    token_manager: Arc<TokenManager<HttpConnector>>,
) -> impl IntoResponse
where
    HttpConnector: Clone + Send + Sync + 'static,
{
    let header_search_condition =
        create_header_condition_from_sheet_meta(token_manager.clone(), sheet_meta, None).await;

    let header_search_condition = match header_search_condition {
        Err(e) => {
            if e.is_not_found() {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(json!({"error_message":e.to_string()})),
                ));
            } else {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error_message":e.to_string()})),
                ));
            }
        }
        Ok(v) => v,
    };

    let sheet_response = fetch_sheet_value(
        token_manager.clone(),
        &header_search_condition,
        &row_search_condition,
    )
    .await;

    let mut sheet_response = match sheet_response {
        Err(e) => {
            if e.is_not_found() {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(json!({"error_message":e.to_string()})),
                ));
            } else {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error_message":e.to_string()})),
                ));
            }
        }
        Ok(v) => {
            if v.is_empty() {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(json!({"error_message":"no records"})),
                ));
            } else {
                v
            }
        }
    };

    let json_response = build_json(&mut sheet_response, return_as_single_obj);

    let json_response = match json_response {
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error_message":e.to_string()})),
            ))
        }
        Ok(v) => v,
    };

    let response = GetSpreadSheetValueResponse {
        data: json_response,
        pagination: sheet_response.pagination,
    };

    Ok(Json(response))
}

fn build_json<'a>(
    sheet_response: &'a mut SheetValueResponse,
    as_single_obj: bool,
) -> Result<JsonValue, json_structure::JsonStructureError> {
    let headers: Vec<&str> = sheet_response
        .headers
        .values
        .iter()
        .map(|header_value| header_value.as_str())
        .collect();
    let strcuture_obj = json_structure::Object::from_strs(headers.as_slice())?;
    let structure_obj = json_structure::Structure::new_obj(strcuture_obj);

    if as_single_obj {
        // its confirmed that sheet_response is not empty
        let first_row = sheet_response.row_values.values.get(0).unwrap();
        let first_row: Vec<&JsonValue> = first_row.iter().map(|v| v.as_inner()).collect();
        let response_json = structure_obj.build_json(first_row.as_slice())?;
        Ok(response_json.into_json_value())
    } else {
        let mut result = Vec::with_capacity(sheet_response.row_values.values.len());
        for each_row in &sheet_response.row_values.values {
            let each_row: Vec<&JsonValue> = each_row.iter().map(|v| v.as_inner()).collect();
            let response_json = structure_obj.build_json(each_row.as_slice())?;
            result.push(response_json.into_json_value())
        }

        Ok(JsonValue::Array(result))
    }
}

#[derive(Serialize, Deserialize)]
pub struct GetSpreadSheetValueResponse {
    pub data: JsonValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<Pagination>,
}
