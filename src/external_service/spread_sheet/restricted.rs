const MAX_COL: usize = 130;
const MAX_ROW: usize = 1000;

pub fn is_col_range_overflow(col_num: usize) -> Result<(), usize> {
    // 130:A to DZ
    if col_num > MAX_COL {
        Err(MAX_COL)
    } else {
        Ok(())
    }
}

pub fn is_row_range_overflow(row_num: usize) -> Result<(), usize> {
    if row_num > MAX_ROW {
        Err(MAX_ROW)
    } else {
        Ok(())
    }
}
