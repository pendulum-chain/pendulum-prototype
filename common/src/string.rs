use sp_std::ops::Add;
use sp_std::prelude::*;
use sp_std::str::from_utf8;

#[derive(PartialEq, PartialOrd, Eq, Ord)]
pub struct String {
    vec: Vec<u8>,
}

impl String {
    pub fn as_str(&self) -> &str {
        from_utf8(self.vec.as_slice()).expect("Cannot decode utf-8")
    }
}

impl Add<Self> for String {
    type Output = String;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        let vec = [self.vec.as_slice(), rhs.vec.as_slice()].concat();

        Self { vec }
    }
}

impl Add<&str> for String {
    type Output = String;

    #[inline]
    fn add(self, rhs: &str) -> Self::Output {
        let vec = [self.vec.as_slice(), rhs.as_bytes()].concat();

        Self { vec }
    }
}

impl From<&str> for String {
    #[inline]
    fn from(s: &str) -> Self {
        Self {
            vec: Vec::from(s.as_bytes()),
        }
    }
}
