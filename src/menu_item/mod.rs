use crate::source_file::liquibase::changelog_master::append_include_tag_to_changelog_file;
use crate::source_file::liquibase::menu_item_creator::generate_menu_item_liquibase;
use crate::types::{
  ChangeSetInfo, CodeGenerateResult, FileInfo, FileOperation, FilePathConfig, MenuItem, into_napi,
};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::fs;
use std::path::Path;

/// 在分组下新增一个菜单
#[napi]
pub fn add_menu_item(
  new_menu_item: MenuItem,
  change_set_info: ChangeSetInfo,
  file_path_config: FilePathConfig,
) -> Result<CodeGenerateResult> {
  let menu_item_path = Path::new(&file_path_config.liquibase_new_file_full_path);

  let mut files: Vec<FileInfo> = Vec::new();
  if let Ok(true) = menu_item_path.try_exists() {
    files.push(FileInfo {
      path: file_path_config.liquibase_new_file_full_path.clone(),
      operation: FileOperation::Ignore,
      message: "文件已存在，不覆盖".to_string(),
    });
  } else {
    let codes = generate_menu_item_liquibase(&new_menu_item, change_set_info).map_err(into_napi)?;

    // 如果文件夹不存在，则递归创建
    if let Some(dir) = menu_item_path.parent() {
      fs::create_dir_all(dir)?;
    }
    fs::write(menu_item_path, codes)?;

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
    path: file_path_config.liquibase_root_file_full_path,
    operation: FileOperation::Modify,
    message: "修改成功".to_string(),
  });

  Ok(CodeGenerateResult { files })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  pub fn test_add_menu_item() {
    let new_item = MenuItem {
      id: 1,
      key: "menu_item_1".to_string(),
      title: "菜单1".to_string(),
      icon: "#".to_string(),
      route: "menu-item-1".to_string(),
      seq: 1,
      parent_id: 0,
      client_type: None,
      component: Some("a/b/index".to_string()),
      perms: Some("a:b:list".to_string()),
      is_frame: false,
      is_cache: true,
      visible: true,
    };
    let change_set = ChangeSetInfo {
      author: "cx".to_string(),
      id: "202605192050".to_string(),
    };
    let file_path_config = FilePathConfig {
        liquibase_root_file_full_path: "D:/sources/markdown-lang/ide-plugins/vscode/generated-code/server/src/main/resources/db/changelog/db.changelog-master.xml".to_string(),
        liquibase_new_file_include_path: "db/changelog/system/sys_menu/202605192050_insert_menu_menu_item_1.xml".to_string(),
        liquibase_new_file_full_path: "D:/sources/markdown-lang/ide-plugins/vscode/generated-code/server/src/main/resources/db/changelog/system/sys_menu/202605192050_insert_menu_menu_item_1.xml".to_string(),
    };
    let result = add_menu_item(new_item, change_set, file_path_config.clone()).unwrap();
    assert_eq!(2, result.files.len());
    assert_eq!(
      file_path_config.liquibase_new_file_full_path,
      result.files[0].path
    );
    assert_eq!(
      file_path_config.liquibase_root_file_full_path,
      result.files[1].path
    );
  }
}
