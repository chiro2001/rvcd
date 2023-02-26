#![warn(clippy::all, rust_2018_idioms)]

pub mod app;
pub mod files;
pub mod frame_history;
pub mod message;
pub mod radix;
pub mod run_mode;
pub mod rvcd;
pub mod manager;
pub mod service;
pub mod size;
pub mod tree_view;
pub mod utils;
pub mod view;
pub mod wave;
pub mod verilog;
pub mod client;
pub mod rpc;
pub mod code;

#[macro_use]
extern crate rust_i18n;
i18n!("locales");

pub use crate::rvcd::Rvcd;
