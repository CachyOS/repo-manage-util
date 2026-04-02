#[inline]
#[must_use]
pub const fn string_substr(src_str: &str, pos: usize, n: usize) -> &str {
    let rlen = if n < src_str.len() - pos { n } else { src_str.len() - pos };
    let bytes = src_str.as_bytes();
    let slice = bytes.split_at(pos).1.split_at(rlen).0;
    // const-compatible: from_utf8 validates but the input is always valid (sliced from a &str)
    match std::str::from_utf8(slice) {
        Ok(s) => s,
        Err(_) => unreachable!(),
    }
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
