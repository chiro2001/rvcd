#![cfg(not(target_arch = "wasm32"))]

tonic::include_proto!("rvcd");

pub const RVCD_FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("rvcd_descriptor");

tonic::include_proto!("scaleda");