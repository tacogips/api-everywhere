use once_cell::sync::OnceCell;
use regex::Regex;

static VALID_HEADER_STR_VALUE: OnceCell<Regex> = OnceCell::new();

use thiserror::Error;

type Result<T> = std::result::Result<T, CellError>;

#[derive(Error, Debug, PartialEq)]
pub enum CellError {
    #[error("invalid cell value:{0}")]
    InvalidCellValue(String),
}

pub(crate) fn valid_quote_cell_value_regex() -> &'static Regex {
    VALID_HEADER_STR_VALUE.get_or_init(|| {
        let r = Regex::new(r#""(?P<VALUE>.*)""#).unwrap();
        r
    })
}

pub struct Cell;
impl Cell {
    pub fn sanitize_str_value(s: &str) -> Result<String> {
        let re = valid_quote_cell_value_regex();

        re.captures(s).map_or_else(
            || {
                if s.contains(r#"""#) {
                    Err(CellError::InvalidCellValue(s.to_string()))
                } else {
                    Ok(s.to_string())
                }
            },
            |capture| {
                let s = &capture["VALUE"];
                Ok(s.replace(r#"\""#, r#"""#))
            },
        )
    }
}

#[cfg(test)]
mod test {

    use super::*;
    #[test]
    fn cell_sanitize_test() {
        {
            assert_eq!(Ok("".to_string()), Cell::sanitize_str_value(r#""""#));
        }

        {
            assert_eq!(Ok(r#"a"#.to_string()), Cell::sanitize_str_value(r#"a"#));
        }

        {
            assert_eq!(Ok(r#"""#.to_string()), Cell::sanitize_str_value(r#""\"""#));
        }

        {
            assert_eq!(Ok("".to_string()), Cell::sanitize_str_value("\"\""));
        }

        {
            assert_eq!(
                Ok("aaa ".to_string()),
                Cell::sanitize_str_value(r#""aaa ""#)
            );
        }
    }
}
