use core::slice;
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

/// Iterators through matches for a search query on an address range.
pub struct QueryIter<'a> {
    query: &'a Query,
    addr_iter: Box<dyn Iterator<Item = usize>>,
}
impl Iterator for QueryIter<'_> {
    type Item = usize;

    /// Returns the next match in the search query.
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(addr) = self.addr_iter.next() {
            if self.query.does_match_at(addr) {
                return Some(addr + self.query.anchor);
            }
        }
        None
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

    /// Returns whether the query matches at the 
    pub fn does_match_at(&self, addr: usize) -> bool {
        let memory = unsafe { slice::from_raw_parts(addr as *const u8, self.len()) };

        *self.bytes == *memory
    }

    /// Returns an iterator that iterates over query matches in memory range
    /// starting at address `start` and having length `len`.
    pub fn iter_matches_in(&self, start: usize, len: usize) -> QueryIter {
        // Calculate address of last possible byte where a match can begin.
        let end = start + len - self.len();

        QueryIter {
            query: &self,
            addr_iter: Box::from(start..=end),
        }
    }

    /// Executes query on memory range starting at address `start` and having
    /// length `len`, and returns a boxed slice of matched memory addresses.
    pub fn find_matches_in(&self, start: usize, len: usize) -> Box<[usize]> {
        self.iter_matches_in(start, len)
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }
}

/// Searches the given memory range for any addresses matching the given query
/// string, and returns all matches.
pub fn find_in(what: &str, start: usize, len: usize) -> Result<Box<[usize]>, Box<dyn Error>> {
    Ok(
        Query::build(what)?
        .find_matches_in(start, len)
    )
}

/// Searches the given memory range for any addresses matching the given query
/// string, and returns the first `n` matches.
pub fn find_n_in(what: &str, start: usize, len: usize, n: usize) -> Result<Box<[usize]>, Box<dyn Error>> {
    Ok(
        Query::build(what)?
        .iter_matches_in(start, len)
        .take(n)
        .collect()
    )
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

    /// Helper function for executing tests.
    fn execute_helper<F>(what: Option<&str>, data: &[u8], expected: Result<&[usize], ()>, f: F)
    where
        F: FnOnce(Option<Query>, usize, usize) -> Result<Box<[usize]>, Box<dyn Error>>,
    {
        // If we got a what string, build a query
        let query = if let Some(what) = what {
            if let Ok(query) = Query::build(what) {
                Some(query)
            } else {
                // Any error result is fine for now
                assert!(expected.is_err());
                return;
            }
        } else {
            None
        };

        // Obtain start pointer and length from data
        let start = data.as_ptr() as usize;
        let len = data.len();

        // Call closure to obtain matches
        let matches = f(query, start, len);

        // Check if we expected matches or an error result
        if let Ok(mut matches) = matches {
            // Translate the addresses in matches to offsets in data
            for addr in matches.iter_mut() {
                *addr -= start;
            }
            // Now let's see if we got the expected matches
            assert_eq!(matches, expected.unwrap().into());
        }
        else {
            // Any error result is fine for now
            assert!(expected.is_err());
        }
    }

    /// Helper function for executing a query on data.
    fn query_execute_helper(what: &str, data: &[u8], expected: &[usize]) {
        execute_helper(Some(what), data, Ok(expected),
            |query, start, len| {
                Ok(query.unwrap().find_matches_in(start, len))
            }
        )
    }

    /// Helper function for searching data.
    fn find_in_execute_helper(what: &str, data: &[u8], expected: Result<&[usize], ()>) {
        execute_helper(Some(what), data, expected,
            |_, start, len| {
                find_in(what, start, len)
            }
        )
    }

    /// Helper function for searching data and returning the first n results.
    fn find_n_in_execute_helper(what: &str, data: &[u8], n: usize, expected: Result<&[usize], ()>) {
        execute_helper(None, data, expected,
            |_, start, len| {
                find_n_in(what, start, len, n)
            }
        )
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

    /// Test finding without building a query first.
    #[test]
    fn find_in_ok() {
        find_in_execute_helper(
            "34 56",
            &[0x12, 0x34, 0x56, 0x78],
            Ok(&[1])
        );
    }

    /// Test finding with an invalid query string.
    #[test]
    fn find_in_err() {
        find_in_execute_helper(
            "34 5",
            &[0x12, 0x34, 0x56, 0x78],
            Err(())
        );
    }

    /// Test finding the first n results.
    #[test]
    fn find_n_in_ok() {
        find_n_in_execute_helper(
            "34",
            &[0x12, 0x34, 0x34, 0x34],
            2,
            Ok(&[1, 2])
        );
    }

    /// Test finding the first n results with an invalid query string
    #[test]
    fn find_n_in_err() {
        find_n_in_execute_helper(
            "34 5",
            &[0x12, 0x34, 0x34, 0x34],
            2,
            Err(())
        );
    }
}
