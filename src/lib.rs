#![deny(clippy::all)]

mod db_table_info;
mod db_types;
mod java_project;
mod json_loader;
mod menu_group;
mod menu_item;
mod quick_xml_util;
mod source_file;
mod ts_type;
mod types;
mod ui_types;
mod vue_script;

use crate::types::{CodeGenerateResult, into_napi};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::path::Path;

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

/// 判断当前目录是不是多模块的 maven 项目
#[napi]
pub fn is_maven_multiple_module_project(project_root_path: String) -> bool {
  java_project::maven::is_multiple_module_project(project_root_path.as_str())
}

#[napi]
pub fn new_maven_module(
  project_root_dir: String,
  module_name: String,
  base_package: String,
  module_description: Option<String>,
) -> Result<CodeGenerateResult> {
  java_project::module::new_module(
    project_root_dir.as_ref(),
    module_name.as_str(),
    module_description,
    base_package.as_str(),
  )
  .map_err(into_napi)
}

#[napi]
pub fn install_liquibase_addon(
  project_root_dir: String,
  module_name: String,
  base_package: String,
  author: String,
) -> Result<CodeGenerateResult> {
  java_project::liquibase_addon::install(
    project_root_dir.as_ref(),
    module_name.as_str(),
    base_package.as_str(),
    author.as_str(),
  )
  .map_err(into_napi)
}
