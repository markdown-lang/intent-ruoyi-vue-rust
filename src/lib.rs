#![deny(clippy::all)]

mod db_table_info;
mod json_loader;
mod menu_group;
mod menu_item;
mod quick_xml_util;
mod source_file;
mod types;
mod ui_types;
mod vue_script;
mod ts_type;
mod db_types;

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
