use anyhow::Result;
use std::{fs::File, io::Read, path::Path};

use crate::db_table_info::DbTableStructure;

pub(crate) fn load_table_structure(table_define_file_path: &str) -> Result<DbTableStructure> {
  let path = Path::new(table_define_file_path);
  let mut file = File::open(path)?;

  let mut json_content = String::new();
  file.read_to_string(&mut json_content)?;
  let data: DbTableStructure = serde_json::from_str(&json_content)?;
  Ok(data)
}
