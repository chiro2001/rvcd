use num_bigint::{BigInt, BigUint};
use vcd::Value;

pub enum Radix {
    Bin,
    Oct,
    Dec,
    Hex,
}

pub fn vcd_vector_to_string(radix: Radix, vec: &Vec<Value>) -> String {
    match radix {
        Radix::Bin => vcd_vector_bin(vec),
        Radix::Oct => vcd_vector_oct(vec),
        Radix::Dec => vcd_vector_dec(vec),
        Radix::Hex => vcd_vector_hex(vec),
    }
}

pub fn vcd_vector_bin(vec: &Vec<Value>) -> String {
    vec.iter().map(|v| v.to_string()).collect::<Vec<_>>().join("")
}

fn value_map(v: &Value) -> Value {
    match v {
        Value::V1 => Value::V1,
        _ => Value::V0,
    }
}

fn value_map_val(v: &Value) -> u8 {
    match v {
        Value::V1 => 1,
        _ => 0,
    }
}

fn value_big_int(vec: &Vec<Value>) -> BigUint {
    let bits = vec.iter().map(value_map_val).collect::<Vec<_>>();
    let mut bytes: Vec<u8> = vec![];
    let mut byte = 0_u8;
    bits.iter().enumerate().for_each(|i| {
        let offset = i.0 & 0x7;
        if offset == 0 {
            byte = *i.1;
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

pub fn vcd_vector_oct(vec: &Vec<Value>) -> String {
    let val = value_big_int(vec);
    let str = val.to_str_radix(8);
    // for every 'z' or 'x' bit,
    // 1. in this 2^n bit have only one 'x' or 'z', then change char as 'x' or 'z'
    // 2. in this 2^n bit have 'x' and 'z', use 'x'
    str
}

pub fn vcd_vector_dec(vec: &Vec<Value>) -> String {
    "".to_string()
}

pub fn vcd_vector_hex(vec: &Vec<Value>) -> String {
    "".to_string()
}
