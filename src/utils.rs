#[inline]
#[must_use]
pub const fn string_substr(src_str: &str, pos: usize, n: usize) -> &str {
    let rlen = if n < src_str.len() - pos { n } else { src_str.len() - pos };

    let bytes = src_str.as_bytes();
    // SAFETY: pos..pos+rlen is within bounds (checked above) and we slice
    // from a valid &str, so the byte range is guaranteed valid UTF-8.
    unsafe { std::str::from_utf8_unchecked(bytes.split_at(pos).1.split_at(rlen).0) }
}

#[cfg(test)]
mod tests {
    #[test]
    fn getting_string_substr() {
        assert_eq!(crate::utils::string_substr("ABCDEF", 4, 42), "EF");
        assert_eq!(crate::utils::string_substr("ABCDEF", 1, 10), "BCDEF");
        assert_eq!(crate::utils::string_substr("ABCDEF", 2, 3), "CDE");
    }
}
