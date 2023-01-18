use num_bigint::{BigInt, BigUint};
use vcd::Value;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum Radix {
    Bin,
    Oct,
    Dec,
    Hex,
}

pub fn vcd_vector_to_string(radix: Radix, vec: &Vec<Value>) -> String {
    if radix == Radix::Dec { return vcd_vector_dec(vec); } else {
        let n: usize = match radix {
            Radix::Bin => 1,
            Radix::Oct => 3,
            Radix::Hex => 4,
            Radix::Dec => 0,
        };
        vcd_vector_to_string_n(vec, n)
    }
}

pub fn vcd_vector_bin(vec: &Vec<Value>) -> String {
    vec.iter().rev().map(|v| v.to_string()).collect::<Vec<_>>().join("")
}

fn value_map_val(v: &Value) -> u8 {
    match v {
        Value::V1 => 1,
        _ => 0,
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
    assert!(bytes.len() % 8 < 2);
    BigUint::from_bytes_le(&bytes)
}

pub fn vcd_vector_to_string_n(vec: &Vec<Value>, n: usize) -> String {
    let val = value_big_int(vec);
    let mut str = val.to_str_radix(8);
    // for every 'z' or 'x' bit,
    // 1. in this 2^n bit have only one 'x' or 'z', then change char as 'x' or 'z'
    // 2. in this 2^n bit have 'x' and 'z', use 'x'
    let indexes_target = |target: Value|
        vec.iter().enumerate()
            .filter(|(i, v)| **v == target)
            .map(|i| i.0)
            .collect::<Vec<_>>();
    let indexes_z = indexes_target(Value::Z);
    let indexes_x = indexes_target(Value::X);
    indexes_z.into_iter().map(|i| i >> n).for_each(|i| str.replace_range(i..(i + 1), "z"));
    indexes_x.into_iter().map(|i| i >> n).for_each(|i| str.replace_range(i..(i + 1), "x"));
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

