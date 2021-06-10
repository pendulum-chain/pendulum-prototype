#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;
use sp_std::ops::Add;
use sp_std::str::from_utf8;

#[derive(PartialOrd, Eq, Ord)]
pub struct String {
    vec: Vec<u8>,
}

impl String {
  pub fn as_str(&self) -> &str {
    from_utf8(self.vec.as_slice()).expect("Cannot decode utf-8")
  }
}

impl PartialEq for String {
    fn eq(&self, other: &Self) -> bool {
        if self.vec.len() != other.vec.len() {
          return false;
        }

        let self_bytes = self.vec.as_slice();
        let other_bytes = other.vec.as_slice();

        for i in 0..self.vec.len() {
            if self_bytes[i] != other_bytes[i] {
              return false;
            }
        }

        true
    }
}

impl Add<Self> for String {
    type Output = String;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        let vec = [self.vec.as_slice(), rhs.vec.as_slice()].concat();

        Self {
          vec,
        }
    }
}

impl Add<&str> for String {
    type Output = String;

    #[inline]
    fn add(self, rhs: &str) -> Self::Output {
        let vec = [self.vec.as_slice(), rhs.as_bytes()].concat();

        Self {
          vec,
        }
    }
}

impl From<&str> for String {
    #[inline]
    fn from(s: &str) -> Self {
        Self {
          vec: Vec::from(s.as_bytes())
        }
    }
}
