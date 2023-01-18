use std::cmp::min;
use num_bigint::{BigUint};
use vcd::Value;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum Radix {
    Bin,
    Oct,
    Dec,
    Hex,
}

pub fn vcd_vector_to_string(radix: Radix, vec: &Vec<Value>) -> String {
    if radix == Radix::Dec {
        vcd_vector_dec(vec)
    } else {
        let n: usize = match radix {
            Radix::Bin => 1,
            Radix::Oct => 3,
            Radix::Hex => 4,
            _ => panic!("internal err"),
        };
        vcd_vector_to_string_n(vec, n)
    }
}

pub fn vcd_vector_bin(vec: &Vec<Value>) -> String {
    vec.iter().rev().map(|v| v.to_string()).collect::<Vec<_>>().join("")
}

fn value_map_val(v: &Value) -> u8 {
    // match v {
    //     Value::V1 => 1,
    //     _ => 0,
    // }
    match v {
        Value::V0 => 0,
        _ => 1,
    }
}

fn value_big_int(vec: &Vec<Value>) -> BigUint {
    let bits = vec.iter().map(value_map_val);
    let mut bytes: Vec<u8> = vec![];
    let mut byte = 0_u8;
    bits.enumerate().for_each(|i| {
        let offset = i.0 & 0x7;
        if offset == 0 {
            byte = i.1;
        } else {
            byte |= i.1 << offset;
            if offset == 7 {
                bytes.push(byte);
            }
        }
    });
    if vec.len() & 0x7 != 0 { bytes.push(byte); }
    // assert!(bytes.len() % 8 < 2);
    BigUint::from_bytes_le(&bytes)
}

pub fn vcd_vector_to_string_n(vec: &Vec<Value>, n: usize) -> String {
    println!("n = {}", n);
    assert!(n > 0);
    let val = value_big_int(vec);
    let mut str = val.to_str_radix(1 << n);
    let bits_should_len = ((vec.len() / n) + (if vec.len() % n == 0 { 0 } else { 1 })) * n;
    let vec_extended = vec.iter().chain((0..(bits_should_len - vec.len())).map(|_| &Value::V0))
        .map(|i| *i).collect::<Vec<_>>();
    println!("str len={}, vec len={}, str_len<<(n-1)={}, bits_should_len={}", str.len(), vec.len(), str.len() << (n - 1), bits_should_len);
    let prefix_len = ((bits_should_len / n) - str.len());
    let prefix = (0..prefix_len).map(|_| "0").collect::<Vec<_>>().join("");
    println!("prefix = {}", prefix);
    str = prefix + &str;
    // for every 'z' or 'x' bit,
    // 1. in this 2^n bit have only one 'x' or 'z', then change char as 'x' or 'z'
    // 2. in this 2^n bit have 'x' and 'z', use 'x'
    println!("str={}", str);
    if !str.is_empty() {
        println!("vec_extended = {:?}\nrev: {:?}", vec_extended, vec_extended.iter().rev().collect::<Vec<_>>());
        let indexes_target = |target: Value|
            vec_extended.iter().rev().enumerate()
                .filter(|(_, v)| **v == target)
                .map(|i| i.0)
                .collect::<Vec<_>>();
        let indexes_z = indexes_target(Value::Z);
        let indexes_x = indexes_target(Value::X);
        let mut do_replace = |indexes: Vec<usize>, with: &str| {
            println!("indexes for {}: {:?}", with, indexes);
            indexes.into_iter().map(|i| i / n)
                .for_each(|i| str.replace_range(min(i, str.len() - 1)..min(i + 1, str.len()), with));
        };
        do_replace(indexes_z, "z");
        do_replace(indexes_x, "x");
    }
    str
}

pub fn vcd_vector_dec(vec: &Vec<Value>) -> String {
    let val = value_big_int(vec);
    let str = val.to_str_radix(10);
    let exists_x = vec.contains(&Value::X);
    let exists_z = vec.contains(&Value::Z);
    if exists_x || exists_z {
        // directly change all chars to x or z
        (0..str.len()).map(|_| if exists_x { "x" } else { "z" }).collect::<Vec<_>>().join("")
    } else { str }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use vcd::Value;
    use vcd::Value::*;
    use crate::radix::{Radix, vcd_vector_to_string};

    #[test]
    fn test_vector_string() -> Result<()> {
        let vec: Vec<Value> = vec![V1, V1, V1, V0, V0, V1, V1, V0];
        let bin = vcd_vector_to_string(Radix::Bin, &vec);
        let oct = vcd_vector_to_string(Radix::Oct, &vec);
        let dec = vcd_vector_to_string(Radix::Dec, &vec);
        let hex = vcd_vector_to_string(Radix::Hex, &vec);
        println!("vec rev: {:?}, bin={}, oct={}, dec={}, hex={}", vec.iter().rev().collect::<Vec<_>>(), bin, oct, dec, hex);

        let vec: Vec<Value> = vec![V1, V1, V1, X, V0, V1, V1, Z];
        let bin = vcd_vector_to_string(Radix::Bin, &vec);
        let oct = vcd_vector_to_string(Radix::Oct, &vec);
        let dec = vcd_vector_to_string(Radix::Dec, &vec);
        let hex = vcd_vector_to_string(Radix::Hex, &vec);
        println!("vec rev: {:?}, bin={}, oct={}, dec={}, hex={}", vec.iter().rev().collect::<Vec<_>>(), bin, oct, dec, hex);

        let vec: Vec<Value> = vec![V1, V1, V1, X, V0, V1, X, Z, V0, V0, V0];
        let bin = vcd_vector_to_string(Radix::Bin, &vec);
        let oct = vcd_vector_to_string(Radix::Oct, &vec);
        let dec = vcd_vector_to_string(Radix::Dec, &vec);
        let hex = vcd_vector_to_string(Radix::Hex, &vec);
        println!("vec rev: {:?}, bin={}, oct={}, dec={}, hex={}", vec.iter().rev().collect::<Vec<_>>(), bin, oct, dec, hex);
        Ok(())
    }
}