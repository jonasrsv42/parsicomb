//! Parser combinator that matches exactly N occurrences

use super::parser::Parser;

/// Parser combinator that matches exactly N occurrences of the given parser
pub struct NTimes<P> {
    parser: P,
    count: usize,
}

impl<P> NTimes<P> {
    pub fn new(parser: P, count: usize) -> Self {
        NTimes { parser, count }
    }
}

impl<'code, P> Parser<'code> for NTimes<P>
where
    P: Parser<'code>,
{
    type Cursor = P::Cursor;
    type Output = Vec<P::Output>;
    type Error = P::Error;

    fn parse(&self, mut cursor: Self::Cursor) -> Result<(Self::Output, Self::Cursor), Self::Error> {
        let mut results = Vec::with_capacity(self.count);

        for _ in 0..self.count {
            let (value, next_cursor) = self.parser.parse(cursor)?;
            results.push(value);
            cursor = next_cursor;
        }

        Ok((results, cursor))
    }
}

/// Convenience function to create an NTimes parser
pub fn ntimes<'code, P>(count: usize, parser: P) -> NTimes<P>
where
    P: Parser<'code>,
{
    NTimes::new(parser, count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ByteCursor;
    use crate::Cursor;
    use crate::byte::is_byte;

    #[test]
    fn test_ntimes_zero() {
        let data = b"abc";
        let cursor = ByteCursor::new(data);
        let parser = ntimes(0, is_byte(b'a'));

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, Vec::<u8>::new());
        assert_eq!(cursor.value().unwrap(), b'a');
    }

    #[test]
    fn test_ntimes_one() {
        let data = b"abc";
        let cursor = ByteCursor::new(data);
        let parser = ntimes(1, is_byte(b'a'));

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![b'a']);
        assert_eq!(cursor.value().unwrap(), b'b');
    }

    #[test]
    fn test_ntimes_multiple() {
        let data = b"aaabcd";
        let cursor = ByteCursor::new(data);
        let parser = ntimes(3, is_byte(b'a'));

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![b'a', b'a', b'a']);
        assert_eq!(cursor.value().unwrap(), b'b');
    }

    #[test]
    fn test_ntimes_exact_count() {
        let data = b"aaa";
        let cursor = ByteCursor::new(data);
        let parser = ntimes(3, is_byte(b'a'));

        let (results, cursor) = parser.parse(cursor).unwrap();
        assert_eq!(results, vec![b'a', b'a', b'a']);
        assert!(cursor.eos());
    }

    #[test]
    fn test_ntimes_not_enough() {
        let data = b"aa";
        let cursor = ByteCursor::new(data);
        let parser = ntimes(3, is_byte(b'a'));

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_ntimes_wrong_byte() {
        let data = b"aab";
        let cursor = ByteCursor::new(data);
        let parser = ntimes(3, is_byte(b'a'));

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_ntimes_empty_input() {
        let data = b"";
        let cursor = ByteCursor::new(data);
        let parser = ntimes(1, is_byte(b'a'));

        let result = parser.parse(cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_ntimes_empty_input_zero_count() {
        let data = b"";
        let cursor = ByteCursor::new(data);
        let parser = ntimes(0, is_byte(b'a'));

        let (results, _) = parser.parse(cursor).unwrap();
        assert_eq!(results, Vec::<u8>::new());
    }
}
