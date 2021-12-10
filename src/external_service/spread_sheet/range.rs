use super::*;
use once_cell::sync::OnceCell;
use regex::Regex;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::ptr;
use std::str::FromStr;

const ALPHABET: [char; 26] = [
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];

// 'A' -> to index
pub(crate) fn alpha_to_num(c: char) -> Result<usize> {
    if (c as u8) < 65 || (c as u8) > 90 {
        return Err(RangeError::ColumnAlphabetOutOfRange(c));
    }
    Ok(c as usize - 65)
}

type Result<T> = std::result::Result<T, RangeError>;

#[derive(Error, Debug, PartialEq)]
pub enum RangeError {
    #[error("column alphabet out of range:{0}")]
    ColumnAlphabetOutOfRange(char),

    #[error("invalid range:{0}")]
    InvalidRangeString(String),

    #[error(
        "invalid range direction:start cell must be at upper-left or same position as end cell:{0}"
    )]
    InvalidRangeDirection(String),

    #[error("invalid sheet name:{0}")]
    InvalidSheetName(String),

    #[error("invalid cell ref:{0}")]
    InvalidCellRefString(String),

    #[error("invalid range ref row index:{0}")]
    InvalidRangeRefRow(String),
}

/// 0 -> "A"
/// 25 -> "Z"
/// 26 -> "AA"
pub fn num_to_alphabet_base_number(mut n: usize) -> String {
    let mut result: Vec<char> = Vec::new();

    loop {
        let m = n % 26;
        let d = n / 26;
        let c = ALPHABET[m];
        prepend_vec(c, &mut result);
        if d == 0 {
            return result.iter().collect();
        }
        n = d - 1
    }
}

///  "A" -> 0
///  "Z" -> 25
///  "AA"-> 26
pub fn col_alphabet_to_num(n: &str) -> Result<usize> {
    let chars: Vec<char> = n.chars().into_iter().collect();
    let mut digit = chars.len();
    let mut result = 0usize;
    for each in chars {
        let num_at_digit = alpha_to_num(each)?;

        if digit == 1 {
            result += num_at_digit;
        } else {
            result += (num_at_digit + 1) * 26;
        }

        digit -= 1;
    }

    Ok(result)
}

pub(crate) fn prepend_vec<T>(v: T, vs: &mut Vec<T>) {
    if vs.len() == vs.capacity() {
        vs.reserve(1);
    }
    unsafe {
        let head = vs.as_mut_ptr();

        ptr::copy(head, head.offset(1), vs.len());
        ptr::write(head, v);
        vs.set_len(vs.len() + 1);
    }
}

static VALID_RANGE_RE: OnceCell<Regex> = OnceCell::new();
static VALID_CELL_REF_RE: OnceCell<Regex> = OnceCell::new();
static VALID_SHEET_NAME_RE: OnceCell<Regex> = OnceCell::new();

pub(crate) fn valid_range_regex() -> &'static Regex {
    VALID_RANGE_RE.get_or_init(|| {
        let r = Regex::new(r"(?P<SHEET_NAME>.*?)!?(?P<START_RANGE_COL>[A-Z]+)(?P<START_RANGE_ROW>[0-9]+):(?P<END_RANGE_COL>[A-Z]+)(?P<END_RANGE_ROW>[0-9]+)").unwrap();
        r
    })
}

fn valid_cell_ref_regex() -> &'static Regex {
    VALID_CELL_REF_RE.get_or_init(|| {
        let r = Regex::new(r"(?P<RANGE_COL>[A-Z]+)(?P<RANGE_ROW>[0-9]+)").unwrap();
        r
    })
}

fn quoted_sheet_name() -> &'static Regex {
    VALID_SHEET_NAME_RE.get_or_init(|| {
        let r = Regex::new(r"'(?P<SHEET_NAME>.*)'").unwrap();
        r
    })
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct CellRef {
    pub col_index: usize, //zero_base
    pub row_index: usize, //zero_base
}

#[derive(Debug)]
struct ColAlphabet<'a>(&'a str);

impl Display for CellRef {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "{}{}",
            num_to_alphabet_base_number(self.col_index),
            self.row_index + 1
        )
    }
}

impl CellRef {
    pub fn new(col_index: usize, row_index: usize) -> Self {
        Self {
            col_index,
            row_index,
        }
    }

    fn from_row_and_col<'a>(col_alpha: &ColAlphabet<'a>, row_num_str: &str) -> Result<Self> {
        let col_index = col_alphabet_to_num(&col_alpha.0)?;
        let row_num = row_num_str.parse::<usize>().map_err(|_e| {
            RangeError::InvalidCellRefString(Self::invalid_cell_error_msg(col_alpha, row_num_str))
        })?;

        let row_index = if row_num == 0 {
            return Err(RangeError::InvalidCellRefString(
                Self::invalid_cell_error_msg(col_alpha, row_num_str),
            ));
        } else {
            row_num - 1
        };

        Ok(Self {
            col_index,
            row_index,
        })
    }

    fn invalid_cell_error_msg(col_alpha: &ColAlphabet, row_num: &str) -> String {
        format!("{}{}", col_alpha.0, row_num)
    }

    fn is_at_left_upper_or_same_as(&self, other: &CellRef) -> bool {
        if self.col_index > other.col_index {
            return false;
        }

        if self.row_index > other.row_index {
            return false;
        }

        true
    }
}

impl FromStr for CellRef {
    type Err = RangeError;
    fn from_str(cell_ref_str: &str) -> std::result::Result<CellRef, Self::Err> {
        let re = valid_cell_ref_regex();
        re.captures(cell_ref_str).map_or_else(
            || Err(RangeError::InvalidCellRefString(cell_ref_str.to_string())),
            |capture| {
                let start_range_col = &capture["RANGE_COL"];
                let start_range_row = &capture["RANGE_ROW"];
                let col_alpha = ColAlphabet(start_range_col);
                Ok(CellRef::from_row_and_col(&col_alpha, start_range_row)?)
            },
        )
    }
}

fn sanitize_sheet_name(sheet_name: &str) -> Result<Option<String>> {
    if sheet_name.is_empty() {
        Ok(None)
    } else {
        let re = quoted_sheet_name();
        re.captures(sheet_name).map_or_else(
            || {
                if sheet_name.contains("'") {
                    Err(RangeError::InvalidSheetName(sheet_name.to_string()))
                } else {
                    Ok(Some(sheet_name.to_string()))
                }
            },
            |capture| {
                let sheet_name = &capture["SHEET_NAME"];
                Ok(Some(sheet_name.replace("''", "'")))
            },
        )
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RangeRef {
    pub sheet_name: Option<String>,
    pub start: CellRef,
    pub end: CellRef,
}

impl RangeRef {
    pub fn new(sheet_name: Option<String>, start: CellRef, end: CellRef) -> Self {
        Self {
            sheet_name,
            start,
            end,
        }
    }

    pub fn next_row_index(&self) -> usize {
        self.end.row_index + 1
    }

    pub fn col_range_indices(&self) -> (usize, usize) {
        (self.start.col_index, self.end.col_index)
    }

    pub fn contains(&mut self, other: &RangeRef) -> bool {
        self.start.col_index <= other.start.col_index
            && self.start.row_index <= other.start.row_index
            && self.end.col_index >= other.end.col_index
            && self.end.row_index >= other.end.row_index
    }

    pub fn set_end_col_index(&mut self, col_index: usize) -> Result<()> {
        self.end.col_index = col_index;
        self.validate()?;
        Ok(())
    }

    pub fn start_col_index(&mut self) -> usize {
        self.start.col_index
    }

    pub fn end_col_index(&mut self) -> usize {
        self.end.col_index
    }

    pub fn expand(&mut self, other: &RangeRef) {
        if self.contains(other) {
            return;
        }

        if other.start.col_index < self.start.col_index {
            self.start.col_index = other.start.col_index
        }

        if other.start.row_index < self.start.row_index {
            self.start.row_index = other.start.row_index
        }

        if other.end.col_index > self.end.col_index {
            self.end.col_index = other.end.col_index
        }

        if other.end.row_index > self.end.row_index {
            self.end.row_index = other.end.row_index
        }
    }

    pub fn col_range_size(&self) -> usize {
        self.end.col_index - self.start.col_index + 1
    }

    pub fn as_string(&self) -> String {
        format!("{}", self)
    }

    pub fn shift_in_col(&mut self, shift: i32) -> Result<()> {
        let start_col_index = (self.start.col_index as i32) + shift;
        if start_col_index < 0 {
            return Err(RangeError::InvalidRangeRefRow(format!(
                "invalid start cell row :{}",
                start_col_index
            )));
        }
        let end_col_index = (self.end.col_index as i32) + shift;

        if end_col_index < 0 {
            return Err(RangeError::InvalidRangeRefRow(format!(
                "invalid end cell row :{}",
                end_col_index
            )));
        }

        self.start.col_index = start_col_index as usize;
        self.end.col_index = end_col_index as usize;
        Ok(())
    }

    pub fn is_one_line_row(&self) -> bool {
        self.start.row_index == self.end.row_index
    }

    pub fn validate(&self) -> Result<()> {
        if !self.start.is_at_left_upper_or_same_as(&self.end) {
            return Err(RangeError::InvalidRangeDirection(format!(
                "start cell is not at left right of end cell {}:{}",
                self.start, self.end,
            )));
        }
        Ok(())
    }
}
impl Display for RangeRef {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self.sheet_name.as_ref() {
            Some(sheet_name) => write!(f, "'{}'!{}:{}", sheet_name, self.start, self.end),
            None => write!(f, "{}:{}", self.start, self.end),
        }
    }
}

impl FromStr for RangeRef {
    type Err = RangeError;
    fn from_str(range_str: &str) -> std::result::Result<RangeRef, Self::Err> {
        let re = valid_range_regex();
        re.captures(range_str).map_or_else(
            || Err(RangeError::InvalidRangeString(range_str.to_string())),
            |capture| {
                let sheet_name = &capture.name("SHEET_NAME");
                let sheet_name = match sheet_name.map(|name| name.as_str()) {
                    Some("") => None,
                    Some(s @ _) => sanitize_sheet_name(s)?,
                    None => None,
                };

                let start = {
                    let start_range_col = &capture["START_RANGE_COL"];
                    let start_range_row = &capture["START_RANGE_ROW"];
                    let col_alpha = ColAlphabet(start_range_col);
                    CellRef::from_row_and_col(&col_alpha, start_range_row)?
                };

                let end = {
                    let end_range_col = &capture["END_RANGE_COL"];
                    let end_range_row = &capture["END_RANGE_ROW"];
                    let col_alpha = ColAlphabet(end_range_col);
                    CellRef::from_row_and_col(&col_alpha, end_range_row)?
                };

                let range_ref = RangeRef {
                    sheet_name,
                    start,
                    end,
                };
                range_ref.validate()?;
                Ok(range_ref)
            },
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_alpha_base_number() {
        assert_eq!("A".to_string(), num_to_alphabet_base_number(0));
        assert_eq!("B".to_string(), num_to_alphabet_base_number(1));
        assert_eq!("Z".to_string(), num_to_alphabet_base_number(25));
        assert_eq!("AA".to_string(), num_to_alphabet_base_number(26));
        assert_eq!("AB".to_string(), num_to_alphabet_base_number(27));

        assert_eq!("AZ".to_string(), num_to_alphabet_base_number(51));
        assert_eq!("BA".to_string(), num_to_alphabet_base_number(52));
    }

    #[test]
    fn test_prepend() {
        {
            let mut v = Vec::<u8>::new();
            prepend_vec(10, &mut v);
            assert_eq!(v, vec![10]);
            assert_eq!(v.len(), 1);
        }

        {
            let mut v = Vec::<u8>::new();
            prepend_vec(10, &mut v);
            prepend_vec(11, &mut v);
            prepend_vec(12, &mut v);
            assert_eq!(v, vec![12, 11, 10]);
            assert_eq!(v.len(), 3);
        }
    }

    #[test]
    fn test_alpha_to_num() {
        assert_eq!(alpha_to_num('A'), Ok(0));
        assert_eq!(alpha_to_num('B'), Ok(1));
        assert_eq!(alpha_to_num('Z'), Ok(25));
        assert!(alpha_to_num('a').is_err());
    }

    #[test]
    fn test_col_alpha_to_num() {
        assert_eq!(col_alphabet_to_num("A"), Ok(0));
        assert_eq!(col_alphabet_to_num("B"), Ok(1));
        assert_eq!(col_alphabet_to_num("Z"), Ok(25));
        assert_eq!(col_alphabet_to_num("AA"), Ok(26));

        assert_eq!(col_alphabet_to_num("AZ"), Ok(51));
        assert_eq!(col_alphabet_to_num("BA"), Ok(52));
        assert_eq!(col_alphabet_to_num("DZ"), Ok(129));

        assert!(col_alphabet_to_num("a").is_err());
    }

    #[test]
    fn test_cell_ref_from_str() {
        {
            let result = CellRef::from_str("A1");
            assert!(result.is_ok());
            assert_eq!(CellRef::new(0, 0), result.unwrap());
        }

        {
            let result = CellRef::from_str("B3");
            assert!(result.is_ok());
            assert_eq!(CellRef::new(1, 2), result.unwrap());
        }

        {
            let result = CellRef::from_str("BA2");
            assert!(result.is_ok());
            assert_eq!(CellRef::new(52, 1), result.unwrap());
        }
        {
            let result = CellRef::from_str("B0");
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_range_ref_from_str() {
        {
            let result = RangeRef::from_str("A1:A2");

            assert!(result.is_ok());
            let result = result.unwrap();

            assert_eq!(
                RangeRef {
                    sheet_name: None,
                    start: CellRef::new(0, 0),
                    end: CellRef::new(0, 1),
                },
                result
            );
        }

        {
            let result = RangeRef::from_str("'sheet name 1'!A1:A2");

            assert!(result.is_ok());
            let result = result.unwrap();

            assert_eq!(
                RangeRef {
                    sheet_name: Some("sheet name 1".to_string()),
                    start: CellRef::new(0, 0),
                    end: CellRef::new(0, 1),
                },
                result
            );
        }

        {
            let result = RangeRef::from_str("'sheet name 1'!A1:B2");

            assert!(result.is_ok());
            let result = result.unwrap();

            assert_eq!(
                RangeRef {
                    sheet_name: Some("sheet name 1".to_string()),
                    start: CellRef::new(0, 0),
                    end: CellRef::new(1, 1),
                },
                result
            );
        }

        {
            let result = RangeRef::from_str("sheet name 1!A1:B2");

            assert!(result.is_ok());
            let result = result.unwrap();

            assert_eq!(
                RangeRef {
                    sheet_name: Some("sheet name 1".to_string()),
                    start: CellRef::new(0, 0),
                    end: CellRef::new(1, 1),
                },
                result
            );
        }

        {
            let result = RangeRef::from_str("'sheet name 1'!B2:B2");

            assert!(result.is_ok());
            let result = result.unwrap();

            assert_eq!(
                RangeRef {
                    sheet_name: Some("sheet name 1".to_string()),
                    start: CellRef::new(1, 1),
                    end: CellRef::new(1, 1),
                },
                result
            );
        }

        {
            let result = RangeRef::from_str("'sheet ''name 1'!B2:B2");

            assert!(result.is_ok());
            let result = result.unwrap();

            assert_eq!(
                RangeRef {
                    sheet_name: Some("sheet 'name 1".to_string()),
                    start: CellRef::new(1, 1),
                    end: CellRef::new(1, 1),
                },
                result
            );
        }

        {
            let result = RangeRef::from_str("'sheet name 1!B2:B2");

            assert!(result.is_err());
        }

        {
            let result = RangeRef::from_str("'sheet name 1'!B2:A2");

            assert!(result.is_err());
        }

        {
            let result = RangeRef::from_str("A3:A2");

            assert!(result.is_err());
        }
    }

    #[test]
    fn test_fmt_range() {
        {
            let input = "'sheet name 1'!B2:BA2".to_string();
            let result = RangeRef::from_str(&input);
            assert!(result.is_ok());
            assert_eq!(input, result.unwrap().to_string())
        }

        {
            let input = "B2:BA3".to_string();
            let result = RangeRef::from_str(&input);
            assert!(result.is_ok());
            assert_eq!(input, result.unwrap().to_string())
        }
    }

    #[test]
    fn test_expand() {
        {
            let input = "B2:D4".to_string();
            let mut input = RangeRef::from_str(&input).unwrap();

            let other = "B3:D3".to_string();
            let other = RangeRef::from_str(&other).unwrap();

            input.expand(&other);

            assert_eq!(input.as_string().as_str(), "B2:D4");
        }

        {
            let input = "B2:D4".to_string();
            let mut input = RangeRef::from_str(&input).unwrap();

            let other = "A2:D3".to_string();
            let other = RangeRef::from_str(&other).unwrap();

            input.expand(&other);

            assert_eq!(input.as_string().as_str(), "A2:D4");
        }

        {
            let input = "B2:D4".to_string();
            let mut input = RangeRef::from_str(&input).unwrap();

            let other = "A1:D3".to_string();
            let other = RangeRef::from_str(&other).unwrap();

            input.expand(&other);

            assert_eq!(input.as_string().as_str(), "A1:D4");
        }

        {
            let input = "B2:D4".to_string();
            let mut input = RangeRef::from_str(&input).unwrap();

            let other = "A2:D3".to_string();
            let other = RangeRef::from_str(&other).unwrap();

            input.expand(&other);

            assert_eq!(input.as_string().as_str(), "A2:D4");
        }

        {
            let input = "B2:D4".to_string();
            let mut input = RangeRef::from_str(&input).unwrap();

            let other = "B3:G3".to_string();
            let other = RangeRef::from_str(&other).unwrap();

            input.expand(&other);

            assert_eq!(input.as_string().as_str(), "B2:G4");
        }

        {
            let input = "B2:D4".to_string();
            let mut input = RangeRef::from_str(&input).unwrap();

            let other = "B3:G5".to_string();
            let other = RangeRef::from_str(&other).unwrap();

            input.expand(&other);

            assert_eq!(input.as_string().as_str(), "B2:G5");
        }

        {
            let input = "B2:D4".to_string();
            let mut input = RangeRef::from_str(&input).unwrap();

            let other = "B3:D5".to_string();
            let other = RangeRef::from_str(&other).unwrap();

            input.expand(&other);

            assert_eq!(input.as_string().as_str(), "B2:D5");
        }
    }
}
