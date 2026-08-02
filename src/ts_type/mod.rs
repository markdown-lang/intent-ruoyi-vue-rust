use napi::bindgen_prelude::*;
use napi_derive::napi;
use crate::db_types::{DbTableStructure, DbColumn };
use crate::types::CodeGenerateResult;

/// 新增一个 typescript d.ts 文件
/// ## Parameters
/// * `types_root_dir`: types 文件的根目录，绝对路径，如 ``
/// * `name`: The identifier name being bound.
#[napi]
pub fn add_ts_types(
  types_root_dir: String,
  path_parts: Vec<String>,
  main_table: DbTableStructure,
  sub_tables: Vec<DbTableStructure>,
  page_name: Option<String>,
  query_columns: Vec<DbColumn>,
) -> Result<CodeGenerateResult> {

  Ok(CodeGenerateResult {files: vec![]})
}