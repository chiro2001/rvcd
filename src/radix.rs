use crate::wave::WireValue;
use num_bigint::BigUint;
use std::cmp::min;
use std::fmt::{Display, Formatter};
use tracing::trace;

#[derive(serde::Deserialize, serde::Serialize, Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
pub enum Radix {
    Bin,
    Oct,
    Dec,
    Hex,
}

impl Display for Radix {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Radix {
    pub fn to_number(&self) -> usize {
        match self {
            Radix::Bin => 2,
            Radix::Oct => 8,
            Radix::Dec => 10,
            Radix::Hex => 16,
        }
    }
}

/// Convert [Vec<WireValue>] to string in radix
pub fn radix_vector_to_string(radix: Radix, vec: &Vec<WireValue>) -> String {
    if radix == Radix::Dec {
        radix_vector_dec(vec)
    } else {
        let n: usize = match radix {
            Radix::Bin => 1,
            Radix::Oct => 3,
            Radix::Hex => 4,
            _ => panic!("internal err"),
        };
        radix_vector_to_string_n(vec, n)
    }
}

pub fn radix_vector_bin(vec: &[WireValue]) -> String {
    vec.iter()
        .rev()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join("")
}

fn value_map_val(v: &WireValue) -> u8 {
    match v {
        WireValue::V0 => 0,
        _ => 1,
    }
}

/// Convert [Vec<WireValue>] to BigUInt
///
/// **Warning**: all [WireValue::Z] and [WireValue::X] will be replaced by [WireValue::V1]**
///
/// - vec: lsb data
pub fn radix_value_big_uint(vec: &[WireValue]) -> BigUint {
    let bits = vec.iter().map(value_map_val);
    let mut bytes: Vec<u8> = vec![];
    let mut byte = 0u8;
    bits.enumerate().for_each(|(i, b)| {
        let offset = i & 0x7;
        byte |= b << offset;
        if offset == 7 {
            bytes.push(byte);
            byte = 0;
        }
    });
    if vec.len() & 0x7 != 0 {
        bytes.push(byte);
    }
    // assert!(bytes.len() % 8 < 2);
    BigUint::from_bytes_le(&bytes)
}

pub fn radix_vector_to_string_n(vec: &Vec<WireValue>, n: usize) -> String {
    trace!("n = {}", n);
    assert!(n > 0);
    let val = radix_value_big_uint(vec);
    let mut str = val.to_str_radix(1 << n);
    let bits_should_len = ((vec.len() / n) + usize::from(vec.len() % n != 0)) * n;
    let vec_extended = vec
        .iter()
        .chain((0..(bits_should_len - vec.len())).map(|_| &WireValue::V0))
        .copied()
        .collect::<Vec<_>>();
    trace!(
        "str len={}, vec len={}, str_len<<(n-1)={}, bits_should_len={}",
        str.len(),
        vec.len(),
        str.len() << (n - 1),
        bits_should_len
    );
    let prefix_len = (bits_should_len / n) - str.len();
    let prefix = (0..prefix_len).map(|_| "0").collect::<Vec<_>>().join("");
    trace!("prefix = {}", prefix);
    str = prefix + &str;
    // for every 'z' or 'x' bit,
    // 1. in this 2^n bit have only one 'x' or 'z', then change char as 'x' or 'z'
    // 2. in this 2^n bit have 'x' and 'z', use 'x'
    trace!("str={}", str);
    if !str.is_empty() {
        trace!(
            "vec_extended = {:?}\nrev: {:?}",
            vec_extended,
            vec_extended.iter().rev().collect::<Vec<_>>()
        );
        let indexes_target = |target: WireValue| {
            vec_extended
                .iter()
                .rev()
                .enumerate()
                .filter(|(_, v)| **v == target)
                .map(|i| i.0)
                .collect::<Vec<_>>()
        };
        let indexes_z = indexes_target(WireValue::Z);
        let indexes_x = indexes_target(WireValue::X);
        let mut do_replace = |indexes: Vec<usize>, with: &str| {
            trace!("indexes for {}: {:?}", with, indexes);
            indexes.into_iter().map(|i| i / n).for_each(|i| {
                str.replace_range(min(i, str.len() - 1)..min(i + 1, str.len()), with)
            });
        };
        do_replace(indexes_z, "z");
        do_replace(indexes_x, "x");
    }
    str
}

/// For radix dec, cannot display [WireValue::Z] and [WireValue::X] in especially position,
/// so will be all `xxx` or `zzz`
pub fn radix_vector_dec(vec: &Vec<WireValue>) -> String {
    let val = radix_value_big_uint(vec);
    let str = val.to_str_radix(10);
    let exists_x = vec.contains(&WireValue::X);
    let exists_z = vec.contains(&WireValue::Z);
    if exists_x || exists_z {
        // directly change all chars to x or z
        (0..str.len())
            .map(|_| if exists_x { "x" } else { "z" })
            .collect::<Vec<_>>()
            .join("")
    } else {
        str
    }
}

#[cfg(test)]
mod test {
    use crate::radix::{radix_value_big_uint, radix_vector_to_string, Radix};
    use crate::wave::WireValue::*;
    use crate::wave::{WaveDataItem, WaveDataValue, WireValue};
    use anyhow::Result;
    use num_bigint::BigUint;
    use tracing::debug;

    #[test]
    fn test_vector_string() -> Result<()> {
        let vec: Vec<WireValue> = vec![V1, V1, V1, V0, V0, V1, V1, V0];
        let bin = radix_vector_to_string(Radix::Bin, &vec);
        let oct = radix_vector_to_string(Radix::Oct, &vec);
        let dec = radix_vector_to_string(Radix::Dec, &vec);
        let hex = radix_vector_to_string(Radix::Hex, &vec);
        debug!(
            "vec rev: {:?}, bin={}, oct={}, dec={}, hex={}",
            vec.iter().rev().collect::<Vec<_>>(),
            bin,
            oct,
            dec,
            hex
        );

        let vec: Vec<WireValue> = vec![V1, V1, V1, X, V0, V1, V1, Z];
        let bin = radix_vector_to_string(Radix::Bin, &vec);
        let oct = radix_vector_to_string(Radix::Oct, &vec);
        let dec = radix_vector_to_string(Radix::Dec, &vec);
        let hex = radix_vector_to_string(Radix::Hex, &vec);
        debug!(
            "vec rev: {:?}, bin={}, oct={}, dec={}, hex={}",
            vec.iter().rev().collect::<Vec<_>>(),
            bin,
            oct,
            dec,
            hex
        );

        let vec: Vec<WireValue> = vec![V1, V1, V1, X, V0, V1, X, Z, V0, V0, V0];
        let bin = radix_vector_to_string(Radix::Bin, &vec);
        let oct = radix_vector_to_string(Radix::Oct, &vec);
        let dec = radix_vector_to_string(Radix::Dec, &vec);
        let hex = radix_vector_to_string(Radix::Hex, &vec);
        debug!(
            "vec rev: {:?}, bin={}, oct={}, dec={}, hex={}",
            vec.iter().rev().collect::<Vec<_>>(),
            bin,
            oct,
            dec,
            hex
        );
        Ok(())
    }

    #[test]
    fn test_radix_value_big_uint() {
        use WireValue::*;
        for test in 0usize..=32 {
            let test_uint = BigUint::from(test);
            let test_binary_str = test_uint.to_str_radix(2);
            let test_binary = test_binary_str
                .chars()
                .rev()
                .map(|x| match x {
                    '0' => V0,
                    _ => V1,
                })
                .collect::<Vec<_>>();
            // let v = vec![V0, V1, V1, V0, V0, V1];
            let v = test_binary;
            let d = radix_value_big_uint(&v);
            let le = d.to_bytes_le();
            let d2 = BigUint::from_bytes_le(&le);
            println!(
                "v: {:?}, d: {:?} {}, le: {:?}, d2: {:?} {}",
                v,
                d,
                d.to_str_radix(2),
                le,
                d2,
                d2.to_str_radix(2)
            );
        }
    }

    #[test]
    fn test_compress() {
        for test in 0usize..=20 {
            let mut t = test.clone();
            let mut bits = vec![];
            for _i in 0..32 {
                bits.push((t & 0x1usize) as u8);
                t = t >> 1;
                if t == 0 {
                    break;
                }
            }
            let bits = bits
                .into_iter()
                .map(|x| match x {
                    0 => V0,
                    _ => V1,
                })
                .collect::<Vec<_>>();
            let item = WaveDataItem {
                value: WaveDataValue::Raw(bits.clone()),
                timestamp: 0,
            }
            .compress()
            .unwrap();
            let item_value_v = match &item.value {
                WaveDataValue::Comp(v) => v.as_slice(),
                WaveDataValue::Raw(_) => &[],
            };
            let item_value = BigUint::from_bytes_le(item_value_v);
            println!(
                "[test {}], item_value: {}, bits: {:?}, item_value_v: {:?}",
                test, item_value, bits, item_value_v
            );
        }
    }
}
