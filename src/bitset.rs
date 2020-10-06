use std::fmt::*;
use std::hash::{Hash, Hasher};
use std::iter::{once, repeat, FromIterator};
use std::ops::{BitAndAssign, BitOrAssign};

#[derive(Clone)]
pub enum BitSet {
    Small(u64),
    Large(Vec<u64>),
}

#[allow(unused)]
impl BitSet {
    pub fn new() -> Self {
        BitSet::Small(0)
    }

    fn grow(&mut self, size: usize) {
        *self = match std::mem::replace(self, BitSet::Small(0)) {
            BitSet::Small(bits) => {
                if size > 1 {
                    BitSet::Large(Vec::from_iter(
                        std::iter::once(bits).chain(std::iter::repeat(0)).take(size),
                    ))
                } else {
                    BitSet::Small(bits)
                }
            }
            BitSet::Large(mut vec) => {
                while vec.len() < size {
                    vec.push(0);
                }
                BitSet::Large(vec)
            }
        };
    }

    fn shrink(&mut self) {
        if let BitSet::Large(ref mut vec) = self {
            while let Some(&0) = vec.last() {
                vec.pop();
            }
        }
    }

    pub fn insert(&mut self, bit: usize) {
        self.grow(bit / 64);
        match self {
            BitSet::Small(ref mut bits) => {
                bits.bitor_assign((1 << bit) as u64);
            }
            BitSet::Large(ref mut vec) => {
                vec[bit / 64].bitor_assign((1 << bit) as u64);
            }
        }
    }

    pub fn remove(&mut self, bit: usize) {
        match self {
            BitSet::Small(ref mut bits) => {
                bits.bitand_assign(!(1 << (bit & 0x03f)));
            }
            BitSet::Large(ref mut vec) => {
                let block = bit / 64;
                if vec.len() > block {
                    vec[block].bitand_assign(!(1 << (bit & 0x3f)));
                }
            }
        }
        self.shrink();
    }

    pub fn contains(&self, bit: usize) -> bool {
        match self {
            &BitSet::Small(bits) => {
                if bit < 64 {
                    bits & (1 << bit) > 0
                } else {
                    false
                }
            }
            &BitSet::Large(ref vec) => {
                let block = bit / 64;
                if block < vec.len() {
                    vec[block] & (1 << (bit & 0x3f)) > 0
                } else {
                    false
                }
            }
        }
    }

    pub fn iter(&self) -> BitIter {
        match self {
            &BitSet::Small(bits) => BitIter {
                current: bits,
                offset: 0,
                remaining: &[],
            },
            &BitSet::Large(ref blocks) => {
                let slice = blocks.as_slice();
                let (&current, remaining) = slice.split_first().unwrap_or((&0, &[]));
                BitIter {
                    current,
                    remaining,
                    offset: 0,
                }
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            &BitSet::Small(bits) => bits == 0,
            &BitSet::Large(ref vec) => vec.last().map(|&last| last == 0).unwrap_or(true),
        }
    }

    pub fn apply<F: Fn(u64, u64) -> u64>(&self, other: &Self, f: F) -> Self {
        let mut result = match (self, other) {
            (&BitSet::Small(bits), &BitSet::Small(other_bits)) => BitSet::Small(f(bits, other_bits)),
            (&BitSet::Small(bits), &BitSet::Large(ref vec)) | (&BitSet::Large(ref vec), &BitSet::Small(bits)) => {
                BitSet::Large(
                    vec.iter()
                        .cloned()
                        .zip(once(bits).chain(repeat(0)))
                        .map(|(a, b)| f(a, b))
                        .collect(),
                )
            }
            (&BitSet::Large(ref vec), &BitSet::Large(ref other_vec)) => {
                if vec.len() > other_vec.len() {
                    BitSet::Large(
                        vec.iter()
                            .cloned()
                            .zip(other_vec.iter().cloned().chain(repeat(0)))
                            .map(|(a, b)| f(a, b))
                            .collect(),
                    )
                } else {
                    BitSet::Large(
                        other_vec
                            .iter()
                            .cloned()
                            .zip(vec.iter().cloned().chain(repeat(0)))
                            .map(|(a, b)| f(a, b))
                            .collect(),
                    )
                }
            }
        };
        result.shrink();
        result
    }

    pub fn difference(&self, other: &Self) -> Self {
        self.apply(other, |a, b| a ^ b)
    }

    pub fn union(&self, other: &Self) -> Self {
        self.apply(other, |a, b| a | b)
    }

    pub fn intersection(&self, other: &Self) -> Self {
        match (self, other) {
            (&BitSet::Small(bits), &BitSet::Small(other_bits)) => BitSet::Small(bits & other_bits),
            (&BitSet::Small(bits), &BitSet::Large(ref vec)) | (&BitSet::Large(ref vec), &BitSet::Small(bits)) => {
                match vec.len() {
                    0 => BitSet::Small(0),
                    _ => BitSet::Small(bits & vec[0]),
                }
            }
            (&BitSet::Large(ref vec), &BitSet::Large(ref other_vec)) => {
                let mut result = BitSet::Large(vec.iter().zip(other_vec.iter()).map(|(&a, &b)| a & b).collect());
                result.shrink();
                result
            }
        }
    }
}

pub struct BitIter<'a> {
    remaining: &'a [u64],
    offset: usize,
    current: u64,
}

impl<'a> Iterator for BitIter<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        while self.current == 0 {
            self.current = self.remaining.get(0).cloned()?;
            self.offset += 64;
            self.remaining = &self.remaining[1..];
        }

        // find the least significant bit
        let bit = self.current & !(self.current - 1);
        // clear the least significant bit
        self.current = self.current & (self.current - 1);

        // convert the bit to number of zeros on the right
        let mut c = 64;
        if bit > 0 {
            c -= 1
        };
        if bit & 0x00000000ffffffff > 0 {
            c -= 32
        };
        if bit & 0x0000ffff0000ffff > 0 {
            c -= 16
        };
        if bit & 0x00ff00ff00ff00ff > 0 {
            c -= 8
        };
        if bit & 0x0f0f0f0f0f0f0f0f > 0 {
            c -= 4
        };
        if bit & 0x3333333333333333 > 0 {
            c -= 2
        };
        if bit & 0x5555555555555555 > 0 {
            c -= 1
        };

        Some(self.offset + c)
    }
}

impl FromIterator<usize> for BitSet {
    fn from_iter<T: IntoIterator<Item = usize>>(iter: T) -> Self {
        let mut result = BitSet::new();
        for item in iter.into_iter() {
            result.insert(item);
        }
        result
    }
}

impl Hash for BitSet {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            &BitSet::Small(bits) => bits.hash(state),
            &BitSet::Large(ref vec) => {
                for block in vec.iter() {
                    block.hash(state);
                }
            }
        }
    }
}

impl PartialEq for BitSet {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (&BitSet::Small(bits), &BitSet::Small(other_bits)) => bits == other_bits,
            (&BitSet::Small(bits), &BitSet::Large(ref vec)) | (&BitSet::Large(ref vec), &BitSet::Small(bits)) => {
                match vec.len() {
                    0 => bits == 0,
                    1 => bits == vec[0],
                    _ => false,
                }
            }
            (&BitSet::Large(ref vec), &BitSet::Large(ref other_vec)) => vec.iter().eq(other_vec.iter()),
        }
    }
}

impl Eq for BitSet {}

impl Debug for BitSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_list().entries(self.iter()).finish()
    }
}
