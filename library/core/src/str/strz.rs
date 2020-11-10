#![allow(warnings)]
#![unstable(feature = "strz", issue = "none")]

impl crate::ops::Deref for strz {
    type Target = str;
    fn deref(&self) -> &str {
        // str and strz have the same representation, although not the same constraints.
        // a strz can always be converted to a str.
        unsafe { crate::mem::transmute::<&strz, &str>(self) }
    }
}

impl crate::convert::AsRef<str> for strz {
    fn as_ref(&self) -> &str {
        // str and strz have the same representation, although not the same constraints.
        // a strz can always be converted to a str.
        unsafe { crate::mem::transmute::<&strz, &str>(self) }
    }
}

#[lang = "strz"]
impl strz {
    pub fn split_at(&self, mid: usize) -> (&str, &strz) {
        let s: &str = self;
        let (lo, hi) = s.split_at(mid);
        // Transmuting 'hi' from str to strz is safe, because 'hi' is still contiguous
        // with the NUL terminator.
        (lo, unsafe { crate::mem::transmute::<&str, &strz>(hi) })
    }

    pub fn strz_split_at(&self, mid: usize) -> (&str, &strz) {
        let s: &str = self;
        let (lo, hi) = s.split_at(mid);
        // Transmuting 'hi' from str to strz is safe, because 'hi' is still contiguous
        // with the NUL terminator.
        (lo, unsafe { crate::mem::transmute::<&str, &strz>(hi) })
    }

    pub fn c_str<'a>(&'a self) -> &'a cstrz {
        unsafe {
            &*(self.as_ptr() as *const cstrz)
        }
    }
}

// #[repr(transparent)]
pub struct cstrz(());

extern "C" {
    fn strlen(s: *const u8) -> usize;
}

impl cstrz {
    #[inline]
    pub fn len(&self) -> usize {
        unsafe {
            strlen(self as *const cstrz as *const u8)
        }
    }

    pub fn as_str(&self) -> &strz {
        let len = self.len();
        unsafe {
            let bytes = crate::slice::from_raw_parts::<u8>(self as *const cstrz as *const u8, len);
            let s = crate::str::from_utf8_unchecked(bytes);
            crate::mem::transmute::<&str, &strz>(s)
        }
    }
}

impl Default for &strz {
    fn default() -> Self {
        ""z
    }
}

use crate::fmt;

impl fmt::Display for strz {
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        <str as fmt::Display>::fmt(self, fmt)
    }
}

impl fmt::Debug for strz {
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        <str as fmt::Debug>::fmt(self, fmt)
    }
}

impl fmt::Debug for cstrz {
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        <str as fmt::Debug>::fmt(self.as_str(), fmt)
    }
}
