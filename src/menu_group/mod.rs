use std::fmt::format;
use std::fs;
use std::path::Path;
use chrono::Local;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use crate::git2_client::commit_files;
use crate::source_file::liquibase::changelog_main::append_include_tag_to_changelog_file;
use crate::source_file::liquibase::menu_group_creator::generate_menu_group_liquibase;

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
  pub commit_hash: String,
  pub commit_time: String,
  pub files: Vec<FileInfo>,
}

#[napi(object)]
pub struct ModifyInfo {
  pub author: String,
  pub time: String,
}

#[napi(object)]
pub struct FilePathConfig {
  pub liquibase_root_file_full_path: String,
  pub liquibase_root_file_include_path: String,
  pub liquibase_menu_group_insert_file_path: String,
  pub project_root_dir: String,
}

pub fn into_napi(err: anyhow::Error) -> napi::Error {
  napi::Error::from_reason(format!("{:#}", err))
}

/// 在分组下新增一个分组
#[napi]
pub fn add_menu_group(
  new_group: MenuGroup,
  modify_info: ModifyInfo,
  file_path_config: FilePathConfig,
) -> Result<CodeGenerateResult>  {
  // 1 新增创建菜单分组的 liquibase 脚本
  //    * 生成liquibase脚本代码
  //    * 创建liquibase脚本文件
  // 2 修改 liquibase 根文件，引入新创建的 liquibase 文件
  // 3 git commit 这两个文件
  // 4 返回文件列表及提交信息

  let menu_group_path = Path::new(&file_path_config.liquibase_menu_group_insert_file_path);

  let mut files: Vec<FileInfo> = Vec::new();
  if let Ok(true) = menu_group_path.try_exists() {
    files.push(FileInfo {
      path: file_path_config.liquibase_menu_group_insert_file_path.clone(),
      operation: FileOperation::Ignore,
      message: "文件已存在，不覆盖".to_string(),
    });
  } else {
    let codes = generate_menu_group_liquibase(&new_group, modify_info).map_err(into_napi)?;
    fs::write(menu_group_path, codes)?;

    files.push(FileInfo {
      path: file_path_config.liquibase_menu_group_insert_file_path.clone(),
      operation: FileOperation::Add,
      message: "创建成功".to_string(),
    });
  }

  append_include_tag_to_changelog_file(&file_path_config.liquibase_menu_group_insert_file_path, &file_path_config.liquibase_root_file_include_path).map_err(into_napi)?;
  files.push(FileInfo {
    path: file_path_config.liquibase_root_file_include_path,
    operation: FileOperation::Modify,
    message: "修改成功".to_string(),
  });

  let committed_files = vec![&file_path_config.liquibase_root_file_full_path, &file_path_config.liquibase_menu_group_insert_file_path];
  let commit_info = commit_files(&file_path_config.project_root_dir, &format!("feat: 新增菜单分组 {}", &new_group.title), committed_files).map_err(into_napi)?;

  Ok(CodeGenerateResult {
    commit_hash: commit_info,
    commit_time: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    files
  })
}
