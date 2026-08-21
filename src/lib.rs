#![deny(clippy::all)]

mod db_table_info;
mod db_types;
mod json_loader;
mod menu_group;
mod menu_item;
mod quick_xml_util;
mod source_file;
mod ts_type;
mod types;
mod ui_types;
mod vue_script;

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
