use crate::external_service::spread_sheet::*;
use axum::{extract::Query, http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;
use serde_json::json;

// The query parameters for todos index
#[derive(Debug, Deserialize, Default)]
pub struct GetSpreadSheetMetaQuery {
    pub sheet_url: Option<String>,
}

pub async fn get_spread_sheet_meta(query: Query<GetSpreadSheetMetaQuery>) -> impl IntoResponse {
    if let Some(sheet_url) = &query.sheet_url {
        let sheet_url = urlencoding::decode(&sheet_url);
        match sheet_url {
            Err(e) => Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error_message": format!("query parameter sheet_url is invalid {}", e)
                })),
            )),
            Ok(sheet_url) => match SheetMeta::from_url(&sheet_url) {
                Err(e) => Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error_message": format!("query parameter sheet_url is invalid {}", e)
                    })),
                )),
                Ok(meta) => Ok(Json(json!({ "data": meta }))),
            },
        }
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error_message":"query parameter sheet_url is required"})),
        ))
    }
}
