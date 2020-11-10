#![allow(warnings)]
#![unstable(feature = "strz", issue = "none")]

#[cfg(not(bootstrap))]
impl crate::ops::Deref for strz {
    type Target = str;
    fn deref(&self) -> &str {
        // str and strz have the same representation, although not the same constraints.
        // a strz can always be converted to a str.
        unsafe {
            crate::mem::transmute::<&strz, &str>(self)
        }
    }
}

#[cfg(not(bootstrap))]
impl crate::convert::AsRef<str> for strz {
    fn as_ref(&self) -> &str {
        // str and strz have the same representation, although not the same constraints.
        // a strz can always be converted to a str.
        unsafe {
            crate::mem::transmute::<&strz, &str>(self)
        }
    }
}
