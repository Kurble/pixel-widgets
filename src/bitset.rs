use std::iter::FromIterator;

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Clone)]
pub enum BitSet {
    Small(u64),
    Large(Vec<u64>),
}

impl BitSet {
    pub fn new() -> Self {
        BitSet::Small(0)
    }

    pub fn insert(&mut self, bit: usize) {
        *self = match std::mem::replace(self, BitSet::Small(0)) {
            BitSet::Small(bits) => {
                if bit < 64 {
                    BitSet::Small(bits | (1 << bit))
                } else {
                    let mut vec = Vec::from_iter(std::iter::repeat(0).take(1 + bit / 64));
                    vec[0] = bits;
                    vec[bit / 64] |= (1 << (bit & 0x3f));
                    BitSet::Large(vec)
                }
            }
            BitSet::Large(mut vec) => {
                while vec.len() < 1 + bit / 64 {
                    vec.push(0);
                }
                vec[bit / 64] |= (1 << (bit & 0x3f));
                BitSet::Large(vec)
            }
        };
    }

    /* todo: make sure that the bitset is never too long
    pub fn remove(&mut self, bit: usize) {
        match self {
            BitSet::Small(ref mut bits) => {
                *bits &= !(1 << bit);
            }
            BitSet::Large(ref mut vec) => {
                *vec.get_mut(bit / 64) &= !(1 << (bit & 0x3f));
            }
        }
    }*/
}

impl FromIterator<usize> for BitSet {
    fn from_iter<T: IntoIterator<Item=usize>>(iter: T) -> Self {
        let mut result = BitSet::new();
        for item in iter.into_iter() {
            result.insert(item);
        }
        result
    }
}
