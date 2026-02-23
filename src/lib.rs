#![deny(clippy::all)]

pub mod db_table_info;
pub mod json_loader;
pub mod liquibase_util;
pub mod my_batis_xml_creator;
pub mod quick_xml_util;
pub mod sql_updater;

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::{
  db_table_info::{BusinessOperator, GroupLocation},
  json_loader::load_table_structure,
};

#[napi]
pub fn plus_100(input: u32) -> u32 {
  input + 100
}

#[napi]
pub fn sum(a: i32, b: i32) -> i32 {
  a + b
}

/// 创建 MyBatis 实现类代码时使用的参数
#[napi(object)]
pub struct CreateMyBatisXmlParams {
  /// JSON格式的表结构定义文件路径
  pub table_define_file_path: String,
  /// 业务操作名称，多个时用英文逗号分割，
  /// 当前支持的业务操作符有 createOne、updateOne、deleteOne、queryOne、queryList
  pub business_operators: String,
  /// java 的根目录，项目级变量，是具体业务代码的存放包，如 com.ruoyi.biz
  pub java_base_package: String,
  /// 当前程序模块归属的功能模块名，作为 package 的一段
  pub group: String,
}

#[napi]
pub fn create_my_batis_xml(params: CreateMyBatisXmlParams) -> Result<String> {
  let table_structure = load_table_structure(&params.table_define_file_path)
    .map_err(|e| Error::new(Status::InvalidArg, e.to_string()))?;

  let business_operators: Vec<BusinessOperator> = params
    .business_operators
    .split(',')
    .map(|s| s.trim())
    .filter(|s| !s.is_empty())
    .map(|s| match s {
      "QueryOne" => BusinessOperator::QueryOne,
      "QueryList" => BusinessOperator::QueryList {
        conditions: Vec::new(),
      },
      "CreateOne" => BusinessOperator::CreateOne,
      "UpdateOne" => BusinessOperator::UpdateOne,
      "DeleteOne" => BusinessOperator::DeleteOne,
      _ => BusinessOperator::None,
    })
    .collect();

  let group_location = GroupLocation {
    java_base_package: params.java_base_package,
    group: params.group,
  };

  my_batis_xml_creator::generate(table_structure, business_operators, group_location)
    .map_err(|e| Error::new(Status::InvalidArg, e.to_string()))
}
