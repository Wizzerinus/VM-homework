use std::{ffi::CString, fmt::Display};

pub(crate) struct CStringFormattable<'a>(pub &'a CString);

impl<'a> Display for CStringFormattable<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.to_str() {
            Ok(s) => f.write_fmt(format_args!("{}", s)),
            Err(_) => f.write_fmt(format_args!("{:x?}", self.0.to_bytes())),
        }
    }
}

pub(crate) fn decode_cstring(pool: &[u8], mut idx: usize) -> Option<CString> {
    let mut xs: Vec<u8> = Vec::new();
    while let Some(k) = pool.get(idx)
        && *k != 0
    {
        xs.push(*k);
        idx += 1;
    }
    if idx >= pool.len() {
        None
    } else {
        // SAFETY: we have explicitly checked for null terminators in the string above.
        Some(unsafe { CString::from_vec_unchecked(xs) })
    }
}

pub(crate) fn decode_cstring_pool(pool: &[u8]) -> Option<Vec<(usize, CString)>> {
    let mut idx = 0;
    let mut out = Vec::new();

    while idx < pool.len() {
        let cstr = decode_cstring(pool, idx)?;
        let len_d = cstr.as_bytes().len() + 1;
        out.push((idx, cstr));
        idx += len_d;
    }

    Some(out)
}
