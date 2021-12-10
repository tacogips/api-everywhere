use once_cell::sync::OnceCell;
use regex::Regex;
use serde::Serialize;
use thiserror::Error;

static VALID_SHEET_ID_RE: OnceCell<Regex> = OnceCell::new();
static VALID_SHEET_URL_RE: OnceCell<Regex> = OnceCell::new();
static VALID_SHEET_URL_WITH_TAB_ID_RE: OnceCell<Regex> = OnceCell::new();

type Result<T> = std::result::Result<T, SheetMetaError>;

#[derive(Error, Debug, PartialEq)]
pub enum SheetMetaError {
    #[error("invalid spread sheet id:{0}")]
    InvalidSheetId(String),

    #[error("invalid spread sheet tab id:{0}")]
    InvalidTabId(String),

    #[error("invalid spread sheet url:{0}")]
    InvalidSheetUrl(String),
}

fn valid_sheet_id_regex() -> &'static Regex {
    VALID_SHEET_ID_RE.get_or_init(|| {
        let r = Regex::new(r"^[A-Za-z0-9]+$").unwrap();
        r
    })
}

fn valid_sheet_url_regex() -> &'static Regex {
    VALID_SHEET_URL_RE.get_or_init(|| {
        let r =
            Regex::new(r"^https://docs.google.com/spreadsheets/d/(?P<SHEET_ID>[A-Za-z0-9]+)/?.*$")
                .unwrap();
        r
    })
}

fn valid_sheet_url_with_tab_id_regex() -> &'static Regex {
    VALID_SHEET_URL_WITH_TAB_ID_RE.get_or_init(|| {
        let r = Regex::new(
            r"^https://docs.google.com/spreadsheets/d/(?P<SHEET_ID>[A-Za-z0-9]+)/?.*gid=(?P<TAB_ID>[0-9]+)$",
        )
        .unwrap();
        r
    })
}

#[derive(Debug, PartialEq, Serialize)]
pub struct SheetIdOrName {
    pub tab_sheet_id: Option<u32>,
    pub tab_sheet_name: Option<String>,
}

impl SheetIdOrName {
    pub fn is_need_get_sheet_name_by_id(&self) -> Option<u32> {
        if self.tab_sheet_name.is_none() {
            if let Some(sheet_id) = self.tab_sheet_id {
                if sheet_id != 0 {
                    return self.tab_sheet_id;
                }
            }
        }
        None
    }

    pub fn sheet_name(self) -> Option<String> {
        self.tab_sheet_name
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct SheetMeta {
    pub spread_sheet_id: String,
    pub sheet_id_or_name: SheetIdOrName,
}

impl SheetMeta {
    pub fn new(
        spread_sheet_id: String,
        tab_sheet_id: Option<u32>,
        tab_sheet_name: Option<String>,
    ) -> Self {
        let sheet_id_or_name = SheetIdOrName {
            tab_sheet_id,
            tab_sheet_name,
        };

        Self {
            spread_sheet_id,
            sheet_id_or_name,
        }
    }

    pub fn from_url(url: &str) -> Result<SheetMeta> {
        let re = valid_sheet_url_with_tab_id_regex();
        let sheet_meta = re.captures(url).map_or_else(
            || Err(SheetMetaError::InvalidSheetUrl(url.to_string())),
            |capture| {
                let sheet_id = &capture["SHEET_ID"];
                let tab_id = &capture["TAB_ID"];

                let tab_id = tab_id
                    .parse::<u32>()
                    .map_err(|_e| SheetMetaError::InvalidTabId(tab_id.to_string()))?;
                Ok(SheetMeta::new(sheet_id.to_string(), Some(tab_id), None))
            },
        );
        let sheet_meta = match sheet_meta {
            Ok(meta) => Ok(meta),
            Err(_) => {
                let re = valid_sheet_url_regex();
                re.captures(url).map_or_else(
                    || Err(SheetMetaError::InvalidSheetUrl(url.to_string())),
                    |capture| {
                        let sheet_id = &capture["SHEET_ID"];
                        Ok(SheetMeta::new(sheet_id.to_string(), None, None))
                    },
                )
            }
        };
        sheet_meta.and_then(|sheet_meta| sheet_meta.validate())
    }

    pub fn validate(self) -> Result<Self> {
        let re = valid_sheet_id_regex();
        if !re.is_match(&self.spread_sheet_id) {
            return Err(SheetMetaError::InvalidSheetId(self.spread_sheet_id));
        };

        Ok(self)
    }
}

#[cfg(test)]
mod test {

    use super::*;

    use super::super::test::{TEST_SHEET1, TEST_SHEET1_WITH_TAG_ID};

    #[test]
    fn sheet_meta_parse_url_valid_1() {
        let sheet_meta = SheetMeta::from_url(TEST_SHEET1);
        assert_eq!(
            sheet_meta,
            Ok(SheetMeta::new(
                "1HA4munsvl5UUlb9DKmJvhrwfGlSQ97hSQZf13M3ZO4Y".to_string(),
                Some(0),
                None,
            ))
        );
    }

    #[test]
    fn sheet_meta_parse_url_valid_2() {
        let sheet_meta = SheetMeta::from_url(
            "https://docs.google.com/spreadsheets/d/1HA4munsvl5UUlb9DKmJvhrwfGlSQ97hSQZf13M3ZO4Y",
        );
        assert_eq!(
            sheet_meta,
            Ok(SheetMeta::new(
                "1HA4munsvl5UUlb9DKmJvhrwfGlSQ97hSQZf13M3ZO4Y".to_string(),
                None,
                None,
            ))
        );
    }

    #[test]
    fn sheet_meta_parse_url_valid_3() {
        // not alphanumeric character
        let sheet_meta = SheetMeta::from_url(
            "https://docs.google.com/spreadsheets/d/1HA4munsvl5UUlb9DKmJvhrwfGlSQ97hSQZf13M3ZO4Y;?",
        );
        assert_eq!(
            sheet_meta,
            Ok(SheetMeta::new(
                "1HA4munsvl5UUlb9DKmJvhrwfGlSQ97hSQZf13M3ZO4Y".to_string(),
                None,
                None,
            ))
        );
    }

    #[test]
    fn sheet_meta_parse_url_valid_4() {
        let sheet_meta = SheetMeta::from_url(TEST_SHEET1_WITH_TAG_ID);
        assert_eq!(
            sheet_meta,
            Ok(SheetMeta::new(
                "1HA4munsvl5UUlb9DKmJvhrwfGlSQ97hSQZf13M3ZO4Y".to_string(),
                Some(2089556915),
                None,
            ))
        );
    }

    #[test]
    fn sheet_meta_parse_url_invalid_1() {
        let sheet_meta = SheetMeta::from_url(
            "https://docs.google.com/spreadsheets/1HA4munsvl5UUlb9DKmJvhrwfGlSQ97hSQZf13M3ZO4Y",
        );
        assert!(sheet_meta.is_err());
    }

    #[test]
    fn sheet_meta_parse_id_valid_1() {
        let sheet_meta = SheetMeta::new(
            "1HA4munsvl5UUlb9DKmJvhrwfGlSQ97hSQZf13M3ZO4Y".to_string(),
            None,
            None,
        );

        let sheet_meta = sheet_meta.validate();
        assert_eq!(
            sheet_meta,
            Ok(SheetMeta::new(
                "1HA4munsvl5UUlb9DKmJvhrwfGlSQ97hSQZf13M3ZO4Y".to_string(),
                None,
                None
            ))
        );
    }

    #[test]
    fn sheet_meta_parse_id_invalid_1() {
        let sheet_meta = SheetMeta::new(
            "1HA4;?munsvl5UUlb9DKmJvhrwfGlSQ97hSQZf13M3ZO4Y".to_string(),
            None,
            None,
        );
        let sheet_meta = sheet_meta.validate();
        assert!(sheet_meta.is_err());
    }
}
