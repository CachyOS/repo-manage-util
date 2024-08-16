use std::{env, fs, slice, str};

use rand::Rng;

#[inline]
pub const fn const_min(v1: usize, v2: usize) -> usize {
    if v1 <= v2 {
        v1
    } else {
        v2
    }
}

#[inline]
pub const fn string_substr(src_str: &str, pos: usize, n: usize) -> Result<&str, str::Utf8Error> {
    let rlen = const_min(n, src_str.len() - pos);
    let s = unsafe {
        // First, we build a &[u8]...
        let slice = slice::from_raw_parts(src_str.as_ptr().add(pos), rlen);

        // ... and then convert that slice into a string slice
        str::from_utf8(slice)
    };
    s
}

pub fn create_temporary_directory(max_tries: Option<u32>) -> Option<String> {
    let tmp_dir = env::temp_dir();
    let max_tries = max_tries.unwrap_or(1000);

    let mut i: u32 = 0;
    let mut rng = rand::thread_rng();
    loop {
        let res_path = format!("{}/{}", tmp_dir.to_string_lossy(), rng.gen::<u64>());
        if fs::create_dir_all(res_path.as_str()).is_ok() {
            return Some(res_path);
        }
        if i == max_tries {
            return None;
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn getting_min_val() {
        assert_eq!(crate::utils::const_min(4, 4), 4);
        assert_eq!(crate::utils::const_min(4, 1), 1);
        assert_eq!(crate::utils::const_min(1, 4), 1);
    }
    #[test]
    fn getting_string_substr() {
        assert_eq!(crate::utils::string_substr("ABCDEF", 4, 42), Ok("EF"));
        assert_eq!(crate::utils::string_substr("ABCDEF", 1, 10), Ok("BCDEF"));
        assert_eq!(crate::utils::string_substr("ABCDEF", 2, 3), Ok("CDE"));
    }
}
