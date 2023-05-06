use std::error::Error;

/// Represents a masked byte.
/// 
/// This can be checked for equality with a normal `u8` byte.
/// The other `u8` is first masked with `mask`, then compared against `byte`.
/// 
/// If two `MaskedByte`s are compared, the union both `MaskedByte`'s `mask`s is
/// used.
#[derive(Debug, Eq)]
pub struct MaskedByte {
    /// The byte value to use for equality comparison.
    pub byte: u8,
    /// The mask to apply to a read byte before equality comparison.
    pub mask: u8,
}
impl PartialEq<MaskedByte> for MaskedByte {
    fn eq(&self, other: &MaskedByte) -> bool {
        let mask = self.mask & other.mask;
        (self.byte & mask) == (other.byte & mask)
    }
}
impl PartialEq<u8> for MaskedByte {
    fn eq(&self, other: &u8) -> bool {
        (self.byte & self.mask) == (other & self.mask)
    }
}

/// Represents a memory search query.
#[derive(Debug, PartialEq)]
pub struct Query {
    /// A slice of masked bytes to compare against.
    pub bytes: Box<[MaskedByte]>,
    /// The anchor position within the search query.
    /// If a match is found, the anchor position is added to the start address
    /// of the match.
    pub anchor: usize,
}

/// The canonical anchor character.
const ANCHOR_CHAR: char = '|';
/// The canonical masked character.
const MASKED_CHAR: char = 'x';

/// Returns whether the character is an anchor character.
/// 
/// # Arguments
/// 
/// * `char` - The character to check.
fn is_anchor_char(c: char) -> bool {
    c == ANCHOR_CHAR
}
/// Returns whether the character is a masked character.
/// 
/// # Arguments
/// 
/// * `char` - The character to check.
fn is_masked_char(c: char) -> bool {
    c.eq_ignore_ascii_case(&MASKED_CHAR)
}
/// Returns whether the character is a nibble character.
/// 
/// # Arguments
/// 
/// * `char` - The character to check.
fn is_nibble_char(c: char) -> bool {
    c.is_ascii_hexdigit() || is_masked_char(c)
}

/// Returns computed maximum query size in bytes from a query string.
/// 
/// # Arguments
/// 
/// * `what` - A string slice that holds the query string.
fn calc_query_size(what: &str) -> Result<usize, &str> {
    let nibble_count = what.chars().filter(|c| is_nibble_char(*c)).count();

    if nibble_count % 2 == 0 {
        Ok(nibble_count / 2)
    } else {
        Err("query string should not contain unterminated bytes")
    }
}

impl Query {
    /// Returns the length of this query.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Returns a query built from a query string.
    /// 
    /// # Arguments
    /// 
    /// * `what` - A string slice that holds that query string.
    pub fn build(what: &str) -> Result<Self, Box<dyn Error>> {
        let mut query = Vec::with_capacity(calc_query_size(what)?);
        let mut anchor: Option<usize> = None;

        let mut byte: u8 = 0;
        let mut mask: u8 = 0;
        let mut nibble_idx = 0;

        for c in what.chars() {
            if c.is_whitespace() {
                continue;
            }
            if is_anchor_char(c) {
                if anchor.is_some() {
                    return Err("anchor should not appear more than once in query string".into());
                }
                else if nibble_idx % 2 != 0 {
                    return Err("anchor should not appear mid-byte".into())
                }
                else {
                    anchor = Some(nibble_idx / 2);
                }
                continue;
            }
            
            // Char may be a nibble
            byte <<= 4;
            mask <<= 4;
            
            if let Some(digit) = c.to_digit(16) {
                byte |= digit as u8;
                mask |= 0xF;
            }
            else if !is_masked_char(c) {
                return Err(format!("query string should not contain character {c}").into());
            }

            // Move to next nibble
            nibble_idx += 1;
            if nibble_idx % 2 == 0 {
                query.push(MaskedByte { byte, mask });
                byte = 0;
                mask = 0;
            }
        }

        Ok(Self {
            bytes: query.into_boxed_slice(),
            anchor: anchor.unwrap_or(0)
        })
    }

    /// Executes query on memory range starting at address `start` and having
    /// length `len`, and returns a boxed slice of matched memory addresses.
    pub fn execute(&self, start: usize, len: usize) -> Box<[usize]> {
        let mut matches = Vec::new();
        let match_len = self.len();

        let mut addr = start;
        let end = addr + len;
        // end - match_len calculates the address of the last possible byte
        // that can be part of a match.
        while addr <= end - match_len {
            let mut match_idx = 0;
            let mut match_start = addr;

            while addr < end {
                let ptr = addr as *const u8;
                addr += 1;

                // Do we match this byte?
                if self.bytes[match_idx] == unsafe { *ptr } {
                    if match_idx == 0 {
                        // First byte matched
                        match_start = ptr as usize;
                    }

                    // Consume byte in match pattern
                    match_idx += 1;

                    // End of pattern reached?
                    if match_idx < match_len {
                        // Not reached, so continue to next byte
                        continue;
                    }

                    // At this point we matched the whole pattern
                    matches.push(match_start + self.anchor);
                }
                // If we get here then we finished a match
                // (either matched or discarded)
                
                // Rewind to next byte after start of match
                addr = match_start + 1;
                break;
            }
        }

        matches.into_boxed_slice()
    }
}

/// Searches the given memory range for any addresses matching the given query
/// string, and returns the matches.
pub fn search(what: &str, start: usize, length: usize) -> Result<Box<[usize]>, Box<dyn Error>> {
    Ok(Query::build(what)?.execute(start, length))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests comparing a MaskedByte to a byte.
    #[test]
    fn maskedbyte_partialeq_u8() {
        let masked_byte = MaskedByte { byte: 0x10, mask: 0xF0 };
        assert_eq!(masked_byte, 0x10);
        assert_eq!(masked_byte, 0x12);
        assert_ne!(masked_byte, 0x00);
        assert_ne!(masked_byte, 0x02);
    }

    /// Tests building query with valid query string.
    #[test]
    fn query_build_ok_simple() {
        let query = Query::build("12 34 56").unwrap();
        assert_eq!(query, Query {
            bytes: Box::new([
                MaskedByte { byte: 0x12, mask: 0xFF },
                MaskedByte { byte: 0x34, mask: 0xFF },
                MaskedByte { byte: 0x56, mask: 0xFF },
            ]),
            anchor: 0,
        });
        assert_eq!(query.len(), 3);

        // Coverage for Debug trait
        println!("{:?}", query);
    }

    /// Tests building query with anchor in query string.
    #[test]
    fn query_build_ok_anchor() {
        let query = Query::build("00 11|22").unwrap();
        assert_eq!(query, Query {
            bytes: Box::new([
                MaskedByte { byte: 0x00, mask: 0xFF },
                MaskedByte { byte: 0x11, mask: 0xFF },
                MaskedByte { byte: 0x22, mask: 0xFF },
            ]),
            anchor: 2,
        });
        assert_eq!(query.len(), 3);
    }

    /// Tests building query with masked bytes in query string.
    #[test]
    fn query_build_ok_masked() {
        let query = Query::build("0x xx x2").unwrap();
        assert_eq!(query, Query {
            bytes: Box::new([
                MaskedByte { byte: 0x00, mask: 0xF0 },
                MaskedByte { byte: 0x00, mask: 0x00 },
                MaskedByte { byte: 0x02, mask: 0x0F },
            ]),
            anchor: 0,
        });
        assert_eq!(query.len(), 3);
    }

    /// Tests building query with query string containing an invalid character.
    #[test]
    fn query_build_err_invalid_char() {
        Query::build("00 11+22").unwrap_err();
    }

    /// Tests building query with query string containing an unterminated byte.
    #[test]
    fn query_build_err_unterminated_byte() {
        Query::build("00 11 2").unwrap_err();
    }

    /// Tests building query with query string containing more than one anchor.
    #[test]
    fn query_build_err_multiple_anchor() {
        Query::build("00|11|22").unwrap_err();
    }

    /// Tests building query with query string containing an anchor in the
    /// middle of a byte.
    #[test]
    fn query_build_err_anchor_mid_byte() {
        Query::build("00 1|1 22").unwrap_err();
    }

    /// Helper function for executing a query on data.
    fn query_execute_helper(what: &str, data: &[u8], expected: &[usize]) {
        let query = Query::build(what).unwrap();

        let start = data.as_ptr() as usize;
        let length = data.len();

        let mut matches = query.execute(start, length);
        for addr in matches.iter_mut() {
            *addr -= start;
        }

        assert_eq!(matches, expected.into());
    }

    /// Helper function for searching data.
    fn search_execute_helper(what: &str, data: &[u8], expected: Result<&[usize], ()>) {
        let start = data.as_ptr() as usize;
        let length = data.len();

        let matches = search(what, start, length);
        if let Ok(mut matches) = matches {
            for addr in matches.iter_mut() {
                *addr -= start;
            }
            assert_eq!(matches, expected.unwrap().into());
        } else {
            assert!(expected.is_err());
        }
    }

    /// Tests executing query on block of memory, yielding 1 match.
    #[test]
    fn query_execute_ok_simple() {
        query_execute_helper(
            "12 34",
            &[0x12, 0x34],
            &[0]
        );
    }

    /// Tests executing query on too small block of memory, yielding 0 matches.
    #[test]
    fn query_execute_ok_too_small() {
        query_execute_helper(
            "34 56",
            &[0x12],
            &[]
        );
    }

    /// Tests executing query on block of memory, yielding more than 1 match.
    #[test]
    fn query_execute_ok_multiple() {
        query_execute_helper(
            "34 56",
            &[0x12, 0x34, 0x56, 0x78, 0x12, 0x34, 0x56, 0x78],
            &[1, 5]
        );
    }

    /// Tests executing query on block of memory, yielding overlapping matches.
    #[test]
    fn query_execute_ok_overlapping() {
        query_execute_helper(
            "xx 34",
            &[0x12, 0x34, 0x34, 0x56],
            &[0, 1]
        );
    }

    /// Tests executing query on block of memory, yielding all addresses.
    #[test]
    fn query_execute_ok_all() {
        query_execute_helper(
            "xx",
            &[0x12, 0x34, 0x56, 0x78],
            &[0, 1, 2, 3]
        );
    }

    /// Test executing query with on block of memory with masking.
    #[test]
    fn query_execute_ok_masked() {
        query_execute_helper(
            "34 xx 78",
            &[0x12, 0x34, 0x56, 0x78],
            &[1]
        );
    }

    /// Test executing query with on block of memory with masking.
    #[test]
    fn query_execute_ok_anchor() {
        query_execute_helper(
            "34|56",
            &[0x12, 0x34, 0x56, 0x78],
            &[2]
        );
    }

    /// Test searching without building a query first.
    #[test]
    fn search_ok() {
        search_execute_helper(
            "34 56",
            &[0x12, 0x34, 0x56, 0x78],
            Ok(&[1])
        );
    }

    /// Test searching with an invalid query string.
    #[test]
    fn search_err() {
        search_execute_helper(
            "34 5",
            &[0x12, 0x34, 0x56, 0x78],
            Err(())
        );
    }
}
