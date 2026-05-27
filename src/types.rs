use napi_derive::napi;

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

#[napi(object)]
pub struct MenuItem {
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
  /// 组件地址
  pub component: Option<String>,
  /// 权限字符串
  pub perms: Option<String>,
  /// 是否为外链
  pub is_frame: bool,
  /// 是否缓存
  pub is_cache: bool,
  /// 显示状态
  pub visible: bool,
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
#[derive(Clone)]
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
