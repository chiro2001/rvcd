#![warn(clippy::all, rust_2018_idioms)]

pub mod app;
pub mod message;
pub mod radix;
pub mod rvcd;
pub mod service;
pub mod tree_view;
pub mod utils;
pub mod wave;
pub mod server;
pub mod view;
pub mod frame_history;
pub mod run_mode;
pub mod files;
pub mod size;

pub use crate::rvcd::Rvcd;
