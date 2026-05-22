use crate::source_file::liquibase::changelog_main::append_include_tag_to_changelog_file;
use crate::source_file::liquibase::menu_group_creator::generate_menu_group_liquibase;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::fs;
use std::path::Path;

#[napi(object)]
pub struct MenuGroup {
  pub id: i64,
  pub key: String,
  pub title: String,
  pub icon: String,
  pub route: String,
  pub seq: i64,
  /// 默认为 0
  pub parent_id: i64,
  /// 客户端类型
  pub client_type: Option<String>,
}

#[napi]
pub enum FileOperation {
  Add,
  Modify,
  Delete,
  Ignore,
}

#[napi(object)]
pub struct FileInfo {
  pub path: String,
  pub operation: FileOperation,
  pub message: String,
}

#[napi(object)]
pub struct CodeGenerateResult {
  pub files: Vec<FileInfo>,
}

#[napi(object)]
pub struct ChangeSetInfo {
  pub author: String,
  pub id: String,
}

#[napi(object)]
pub struct FilePathConfig {
  /// liquibase changelog 根文件的完整路径
  pub liquibase_root_file_full_path: String,
  /// liquibase 新增文件的完整路径
  pub liquibase_new_file_full_path: String,
  /// liquibase 新增文件往根文件中引入的相对路径
  pub liquibase_new_file_include_path: String,
}

pub fn into_napi(err: anyhow::Error) -> napi::Error {
  napi::Error::from_reason(format!("{:#}", err))
}

/// 在分组下新增一个分组
#[napi]
pub fn add_menu_group(
  new_group: MenuGroup,
  change_set_info: ChangeSetInfo,
  file_path_config: FilePathConfig,
) -> Result<CodeGenerateResult> {
  // 1 新增创建菜单分组的 liquibase 脚本
  //    * 生成liquibase脚本代码
  //    * 创建liquibase脚本文件
  // 2 修改 liquibase 根文件，引入新创建的 liquibase 文件
  // 3 返回文件列表

  let menu_group_path = Path::new(&file_path_config.liquibase_new_file_full_path);

  let mut files: Vec<FileInfo> = Vec::new();
  if let Ok(true) = menu_group_path.try_exists() {
    files.push(FileInfo {
      path: file_path_config.liquibase_new_file_full_path.clone(),
      operation: FileOperation::Ignore,
      message: "文件已存在，不覆盖".to_string(),
    });
  } else {
    let codes = generate_menu_group_liquibase(&new_group, change_set_info).map_err(into_napi)?;

    // 如果文件夹不存在，则递归创建
    if let Some(dir) = menu_group_path.parent() {
      fs::create_dir_all(dir)?;
    }
    fs::write(menu_group_path, codes)?;

    files.push(FileInfo {
      path: file_path_config.liquibase_new_file_full_path.clone(),
      operation: FileOperation::Add,
      message: "创建成功".to_string(),
    });
  }

  append_include_tag_to_changelog_file(
    &file_path_config.liquibase_root_file_full_path,
    &file_path_config.liquibase_new_file_include_path,
  )
  .map_err(into_napi)?;
  files.push(FileInfo {
    path: file_path_config.liquibase_new_file_include_path,
    operation: FileOperation::Modify,
    message: "修改成功".to_string(),
  });

  Ok(CodeGenerateResult { files })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  pub fn test_add_menu_group() {
    let new_group = MenuGroup {
      id: 1,
      key: "menu_group_1".to_string(),
      title: "菜单组1".to_string(),
      icon: "#".to_string(),
      route: "menu-group-1".to_string(),
      seq: 1,
      parent_id: 0,
      client_type: None,
    };
    let change_set = ChangeSetInfo {
      author: "cx".to_string(),
      id: "202605192050".to_string(),
    };
    let file_path_config = FilePathConfig {
        liquibase_root_file_full_path: "D:\\sources\\markdown-lang\\ide-plugins\\vscode\\generated-code\\src\\main\\resources\\db\\changelog\\db.changelog-master.xml".to_string(),
        liquibase_new_file_include_path: "db/changelog/system/sys_menu/202605192050_insert_menu_group_1.xml".to_string(),
        liquibase_new_file_full_path: "D:\\sources\\markdown-lang\\ide-plugins\\vscode\\generated-code\\src\\main\\resources\\db\\changelog\\system\\sys_menu\\202605192050_insert_menu_group_1.xml".to_string(),
    };
    let result = add_menu_group(new_group, change_set, file_path_config).unwrap();
    assert_eq!(2, result.files.len());
  }
}
