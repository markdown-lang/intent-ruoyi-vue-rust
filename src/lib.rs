#![deny(clippy::all)]

pub mod db_table_info;
pub mod json_loader;
pub mod quick_xml_util;
mod menu_group;
mod source_file;
mod git2_client;

use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi]
pub fn plus_100(input: u32) -> u32 {
  input + 100
}

#[napi]
pub fn sum(a: i32, b: i32) -> i32 {
  a + b
}

#[napi]
pub fn create_my_batis_xml() -> Result<String> {
  Ok("".to_string())
}
