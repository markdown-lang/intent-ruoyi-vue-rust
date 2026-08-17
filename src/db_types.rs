use heck::{ToLowerCamelCase, ToUpperCamelCase};
use napi_derive::napi;
use napi::Either;

/// 数据库表基本信息
#[napi(object)]
pub struct DbTable {
  /// 表名
  pub name: String,
  /// 注释
  pub comment: String,
}

impl DbTable {
  /// 将表名转换为实体类名
  pub fn to_entity_class_name(&self) -> String {
    self.name.to_upper_camel_case()
  }

  /// 将表名转换为字段名
  pub fn to_entity_field_name(&self) -> String {
    self.name.to_lower_camel_case()
  }
}

/// 列信息
#[napi(object)]
pub struct DbColumn {
  pub name: String,
  pub comment: String,
  /// 数据类型
  pub data_type: DbDataType,
  pub max_length: Option<u32>,
  pub scale: Option<u32>,
  /// 只有当数字类型时，此字段才需要赋值；否则值为 None
  pub unsigned: Option<bool>,
  /// 是否为主键列，所有列要么是，要么不是
  pub primary: bool,
  /// 是否可控，所有列要么可空，要么不可空
  pub nullable: bool,
  pub default_value: Option<String>,
  pub db_unit_name: Option<String>,
  pub ui_unit_name: Option<String>,
  pub after_column_name: Option<String>,
  /// 当列与表分开传入时，通过 table_name 与表结构定义建立关系；否则值为 None
  pub table_name: Option<String>,
}


/// 审计等常规字段
pub const AUDIT_FIELDS: &[&str] = &[
  "create_by",
  "create_time",
  "update_by",
  "update_time",
  "remark",
  "version",
];

impl DbColumn {
  pub fn to_entity_field_name(&self) -> String {
    self.name.to_lower_camel_case()
  }

  pub fn is_audit_field(&self) -> bool {
    AUDIT_FIELDS.contains(&self.name.as_str())
  }
}

/// 唯一约束
#[napi(object)]
pub struct UniqueConstraint {
  pub name: String,
  pub columns: Vec<DbColumn>,
}

///外键约束
#[napi(object)]
pub struct ForeignConstraint {
  pub name: String,
  pub base_table: DbTable,
  pub base_column: DbColumn,
  pub ref_table: DbTable,
  pub ref_column: DbColumn,
}

/// 定义一张数据库表结构。
/// 包括表基本信息，字段列表，唯一约束列表，外键约束列表
#[napi(object)]
pub struct DbTableStructure {
  pub table: DbTable,
  pub columns: Vec<DbColumn>,
  pub unique_constraints: Vec<UniqueConstraint>,
  pub foreign_constraints: Vec<ForeignConstraint>,
}

#[napi(string_enum)]
pub enum DbDataType {
  #[napi(value = "boolean")]
  Boolean,
  #[napi(value = "char")]
  Char,
  #[napi(value = "varchar")]
  Varchar,
  #[napi(value = "nchar")]
  NChar,
  #[napi(value = "nvarchar")]
  NVarchar,
  #[napi(value = "clob")]
  Clob,
  #[napi(value = "number")]
  Number,
  #[napi(value = "tinyint")]
  TinyInt,
  #[napi(value = "smallint")]
  SmallInt,
  #[napi(value = "mediumint")]
  MediumInt,
  #[napi(value = "int")]
  Int,
  #[napi(value = "bigint")]
  BigInt,
  #[napi(value = "float")]
  Float,
  #[napi(value = "double")]
  Double,
  #[napi(value = "decimal")]
  Decimal,
  #[napi(value = "currency")]
  Currency,
  #[napi(value = "date")]
  Date,
  #[napi(value = "datetime")]
  DateTime,
  #[napi(value = "time")]
  Time,
  #[napi(value = "timestamp")]
  Timestamp,
  #[napi(value = "uuid")]
  Uuid,
  #[napi(value = "blob")]
  Blob,
  #[napi(value = "xml")]
  Xml,
  #[napi(value = "json")]
  Json,
  #[napi(value = "tinytext")]
  TinyText,
  #[napi(value = "mediumtext")]
  MediumText,
  #[napi(value = "longtext")]
  LongText,
  #[napi(value = "tinyblob")]
  TinyBlob,
  #[napi(value = "mediumblob")]
  MediumBlob,
  #[napi(value = "longblob")]
  LongBlob,
}

impl DbDataType {
  pub fn is_ts_string_type(&self) -> bool {
    matches!(
      self,
      DbDataType::Char |
      DbDataType::Varchar |
      DbDataType::NChar |
      DbDataType::NVarchar |
      DbDataType::Clob |
      DbDataType::DateTime |
      DbDataType::Date |
      DbDataType::Time |
      DbDataType::Timestamp |
      DbDataType::Uuid |
      DbDataType::Xml |
      DbDataType::TinyText |
      DbDataType::MediumText |
      DbDataType::LongText
    )
  }

  pub fn is_ts_number_type(&self) -> bool {
    matches!(
      self,
      DbDataType::Number |
      DbDataType::TinyInt |
      DbDataType::SmallInt |
      DbDataType::MediumInt |
      DbDataType::Int |
      DbDataType::BigInt |
      DbDataType::Float |
      DbDataType::Double |
      DbDataType::Decimal |
      DbDataType::Currency
    )
  }

  pub fn is_ts_boolean_type(&self) -> bool {
    matches!(
      self,
      DbDataType::Boolean
    )
  }
}

/// 查询条件
#[napi(object)]
pub struct QueryParam {
  pub table_name: String,
  pub column_name: String,
  pub data_type: DbDataType,
  pub operation: MatchOperation
}


//region table 一级的查询条件

#[napi(string_enum)]
#[derive(PartialEq)]
pub enum MatchOperation {
  Contains,
  Equal,
  Between,
  ItemEqual,
  ItemContains,
}

#[napi(object)]
pub struct TableParamItem {
  /// 属性名，对应实体类的字段名
  pub property: String,
  /// 底层用一套最全面详尽的类型系统，不一层一层推导，而是基于这一套类型系统推导出不同语言的类型系统？
  pub db_data_type: DbDataType,
  /// 比较操作
  pub operation: MatchOperation,
}

#[napi(string_enum="snake_case")]
pub enum UIParamActionKey {
  ListQuery,
  FormReset,
}
#[napi(object)]
pub struct TableParamActionSlot {
  pub actions: Vec<UIParamActionKey>,
}

/// 表格一级的参数容器对象
#[napi(object)]
pub struct TableParamSlot {
  /// 参数对象的名称
  pub name: String,
  /// 参数列表
  pub children: Vec<Either<TableParamItem, TableParamActionSlot>>,
}

impl TableParamSlot {
  pub fn has_date_range_param(&self) -> bool {
    self.children.iter().any(|child| {
      if let Either::A(table_param) = child {
        matches!(table_param.db_data_type, DbDataType::Date | DbDataType::DateTime) &&
          table_param.operation == MatchOperation::Between
      } else {
        false
      }
    })
  }

  pub fn get_date_range_fields(&self) -> Vec<&str> {
    self.children
      .iter()
      .filter_map(|child| {
        if let Either::A(table_param) = child {
          if matches!(table_param.db_data_type, DbDataType::Date | DbDataType::DateTime) &&
            table_param.operation == MatchOperation::Between {
            Some(table_param.property.as_str())
          } else {
            None
          }
        } else {
          None
        }
      })
      .collect()
  }

}
//endregion

//region Form 表单
#[napi(object)]
pub struct Form {
  /// form 表单的名称
  pub name: String,
  /// 字段列表
  pub fields: Vec<FormField>,
}

#[napi]
pub enum FormField {
  TextInput(FormTextInput),
  NumberInput(FormNumberInput),
  Textarea(FormTextarea),
  DatePicker(FormDatePicker),
  Select(FormSelect),
}

pub struct FormRuleInfo<'a> {
  pub property: &'a str,
  pub label: &'a str,
  pub required: bool,
}

impl FormField {
  pub fn get_rule_info(&self) -> Option<FormRuleInfo<'_>> {
    match self {
      FormField::TextInput(f) => {Some(FormRuleInfo {property: f.property.as_str(), label: f.label.as_str(), required: f.required})}
      FormField::NumberInput(f) => {Some(FormRuleInfo {property: f.property.as_str(), label: f.label.as_str(), required: f.required})}
      FormField::Textarea(f) => {Some(FormRuleInfo {property: f.property.as_str(), label: f.label.as_str(), required: f.required})}
      FormField::DatePicker(f) => {Some(FormRuleInfo {property: f.property.as_str(), label: f.label.as_str(), required: f.required})}
      FormField::Select(f) => {Some(FormRuleInfo {property: f.property.as_str(), label: f.label.as_str(), required: f.required})}
    }
  }
}

#[napi(object)]
pub struct FormTextInput {
  pub property: String,
  pub label: String,
  pub required: bool,
}

#[napi(object)]
pub struct FormNumberInput {
  pub property: String,
  pub label: String,
  pub required: bool,
}

#[napi(object)]
pub struct FormTextarea {
  pub property: String,
  pub label: String,
  pub required: bool,
}

#[napi(object)]
pub struct FormDatePicker {
  pub property: String,
  pub label: String,
  pub required: bool,
}

#[napi(object)]
pub struct FormSelect {
  pub property: String,
  pub label: String,
  pub required: bool,
}
//endregion
