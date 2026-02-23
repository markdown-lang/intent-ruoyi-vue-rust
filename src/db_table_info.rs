use heck::{ToLowerCamelCase, ToUpperCamelCase};
use serde::{Deserialize, Serialize};

/// 数据库表基本信息
#[derive(Clone, Deserialize)]
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
}

#[derive(Clone, PartialEq, Deserialize)]
pub enum DbDataType {
  #[serde(rename = "01")]
  Boolean,
  #[serde(rename = "02")]
  Char,
  #[serde(rename = "03")]
  Varchar,
  #[serde(rename = "04")]
  NChar,
  #[serde(rename = "05")]
  NVarchar,
  #[serde(rename = "06")]
  Clob,
  #[serde(rename = "07")]
  Number,
  #[serde(rename = "08")]
  TinyInt,
  #[serde(rename = "09")]
  SmallInt,
  #[serde(rename = "10")]
  MediumInt,
  #[serde(rename = "11")]
  Int,
  #[serde(rename = "12")]
  BigInt,
  #[serde(rename = "13")]
  Float,
  #[serde(rename = "14")]
  Double,
  #[serde(rename = "15")]
  Decimal,
  #[serde(rename = "16")]
  Currency,
  #[serde(rename = "17")]
  Date,
  #[serde(rename = "18")]
  DateTime,
  #[serde(rename = "19")]
  Time,
  #[serde(rename = "20")]
  Timestamp,
  #[serde(rename = "21")]
  Uuid,
  #[serde(rename = "22")]
  Blob,
  #[serde(rename = "23")]
  Xml,
  #[serde(rename = "24")]
  Json,
  #[serde(rename = "25")]
  TinyText,
  #[serde(rename = "26")]
  MediumText,
  #[serde(rename = "27")]
  LongText,
  #[serde(rename = "28")]
  TinyBlob,
  #[serde(rename = "29")]
  MediumBlob,
  #[serde(rename = "30")]
  LongBlob,
}

impl DbDataType {
  pub fn as_code(&self) -> &'static str {
    match self {
      DbDataType::Boolean => "01",
      DbDataType::Char => "02",
      DbDataType::Varchar => "03",
      DbDataType::NChar => "04",
      DbDataType::NVarchar => "05",
      DbDataType::Clob => "06",
      DbDataType::Number => "07",
      DbDataType::TinyInt => "08",
      DbDataType::SmallInt => "09",
      DbDataType::MediumInt => "10",
      DbDataType::Int => "11",
      DbDataType::BigInt => "12",
      DbDataType::Float => "13",
      DbDataType::Double => "14",
      DbDataType::Decimal => "15",
      DbDataType::Currency => "16",
      DbDataType::Date => "17",
      DbDataType::DateTime => "18",
      DbDataType::Time => "19",
      DbDataType::Timestamp => "20",
      DbDataType::Uuid => "21",
      DbDataType::Blob => "22",
      DbDataType::Xml => "23",
      DbDataType::Json => "24",
      DbDataType::TinyText => "25",
      DbDataType::MediumText => "26",
      DbDataType::LongText => "27",
      DbDataType::TinyBlob => "28",
      DbDataType::MediumBlob => "29",
      DbDataType::LongBlob => "30",
    }
  }

  pub fn from_code(code: &str) -> Option<Self> {
    match code {
      "01" => Some(DbDataType::Boolean),
      "02" => Some(DbDataType::Char),
      "03" => Some(DbDataType::Varchar),
      "04" => Some(DbDataType::NChar),
      "05" => Some(DbDataType::NVarchar),
      "06" => Some(DbDataType::Clob),
      "07" => Some(DbDataType::Number),
      "08" => Some(DbDataType::TinyInt),
      "09" => Some(DbDataType::SmallInt),
      "10" => Some(DbDataType::MediumInt),
      "11" => Some(DbDataType::Int),
      "12" => Some(DbDataType::BigInt),
      "13" => Some(DbDataType::Float),
      "14" => Some(DbDataType::Double),
      "15" => Some(DbDataType::Decimal),
      "16" => Some(DbDataType::Currency),
      "17" => Some(DbDataType::Date),
      "18" => Some(DbDataType::DateTime),
      "19" => Some(DbDataType::Time),
      "20" => Some(DbDataType::Timestamp),
      "21" => Some(DbDataType::Uuid),
      "22" => Some(DbDataType::Blob),
      "23" => Some(DbDataType::Xml),
      "24" => Some(DbDataType::Json),
      "25" => Some(DbDataType::TinyText),
      "26" => Some(DbDataType::MediumText),
      "27" => Some(DbDataType::LongText),
      "28" => Some(DbDataType::TinyBlob),
      "29" => Some(DbDataType::MediumBlob),
      "30" => Some(DbDataType::LongBlob),
      _ => None,
    }
  }

  pub fn as_liquibase_type(&self) -> &'static str {
    match self {
      DbDataType::Boolean => "boolean",
      DbDataType::Char => "char",
      DbDataType::Varchar => "varchar",
      DbDataType::NChar => "nchar",
      DbDataType::NVarchar => "nvarchar",
      DbDataType::Clob => "clob",
      DbDataType::Number => "number",
      DbDataType::TinyInt => "tinyint",
      DbDataType::SmallInt => "smallint",
      DbDataType::MediumInt => "mediumint",
      DbDataType::Int => "int",
      DbDataType::BigInt => "bigint",
      DbDataType::Float => "float",
      DbDataType::Double => "double",
      DbDataType::Decimal => "decimal",
      DbDataType::Currency => "currency",
      DbDataType::Date => "date",
      DbDataType::DateTime => "datetime",
      DbDataType::Time => "time",
      DbDataType::Timestamp => "timestamp",
      DbDataType::Uuid => "uuid",
      DbDataType::Blob => "blob",
      DbDataType::Xml => "xml",
      DbDataType::Json => "json",
      DbDataType::TinyText => "tinytext",
      DbDataType::MediumText => "mediumtext",
      DbDataType::LongText => "longtext",
      DbDataType::TinyBlob => "tinyblob",
      DbDataType::MediumBlob => "mediumblob",
      DbDataType::LongBlob => "longblob",
    }
  }

  pub fn as_liquibase_default_value_attr_name(&self) -> &'static str {
    match self {
      DbDataType::Number
      | DbDataType::TinyInt
      | DbDataType::SmallInt
      | DbDataType::MediumInt
      | DbDataType::Int
      | DbDataType::BigInt
      | DbDataType::Float
      | DbDataType::Double
      | DbDataType::Decimal => "defaultValueNumeric",
      DbDataType::Boolean => "defaultValueBoolean",
      DbDataType::Date | DbDataType::DateTime | DbDataType::Time => "defaultValueDate",
      _ => "defaultValue",
    }
  }

  pub fn as_java_type(&self) -> &'static str {
    match self {
      DbDataType::Boolean => "Boolean",

      DbDataType::Char
      | DbDataType::Varchar
      | DbDataType::NChar
      | DbDataType::NVarchar
      | DbDataType::Clob
      | DbDataType::Uuid
      | DbDataType::Xml
      | DbDataType::TinyText
      | DbDataType::MediumText
      | DbDataType::LongText
      | DbDataType::Json => "String",

      DbDataType::TinyInt => "Byte",
      DbDataType::SmallInt => "Short",

      DbDataType::MediumInt | DbDataType::Int => "Integer",

      DbDataType::BigInt => "Long",
      DbDataType::Float => "Float",
      DbDataType::Double => "Double",

      DbDataType::Decimal | DbDataType::Currency | DbDataType::Number => "BigDecimal",

      DbDataType::Date | DbDataType::DateTime | DbDataType::Timestamp => "Date",

      DbDataType::Time => "Time",
      DbDataType::Blob => "Blob",

      DbDataType::TinyBlob | DbDataType::MediumBlob | DbDataType::LongBlob => "byte[]",
    }
  }
}

#[derive(Clone, PartialEq, Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct DbColumn {
  pub name: String,
  pub comment: String,
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

impl DbColumn {
  pub fn get_data_type_expression(&self) -> String {
    let mut result = String::new();
    let db_type_name = self.data_type.as_liquibase_type();
    result.push_str(db_type_name);

    if let Some(max_length) = self.max_length {
      if max_length > 0 {
        result.push('(');
        result.push_str(max_length.to_string().as_str());

        if let Some(scale) = self.scale {
          if scale > 0 {
            result.push(',');
            result.push_str(scale.to_string().as_str());
          }
        }
        result.push(')');
      }
    }

    result
  }

  pub fn to_entity_field_name(&self) -> String {
    self.name.to_lower_camel_case()
  }

  pub fn name_to_upper_camel_case(&self) -> String {
    self.name.to_upper_camel_case()
  }
}

#[derive(Deserialize)]
pub struct UniqueConstraint {
  pub name: String,
  pub columns: Vec<DbColumn>,
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct ForeignConstraint {
  pub name: String,
  pub base_table: DbTable,
  pub base_column: DbColumn,
  pub ref_table: DbTable,
  pub ref_column: DbColumn,
}

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct DbTableStructure {
  pub table: DbTable,
  pub columns: Vec<DbColumn>,
  pub unique_constraints: Vec<UniqueConstraint>,
  pub foreign_constraints: Vec<ForeignConstraint>,
}

/// 用户编辑相关的信息
#[derive(Deserialize)]
pub struct ModifyInfo {
  pub author: String,
  pub time: String,
}

/// 功能模块(分组)的位置信息
#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct GroupLocation {
  /// 包的根路径，使用英文.分割
  pub java_base_package: String,
  /// 功能模块的英文名
  pub group: String,
}

pub struct RenameColumnInfo {
  pub before: DbColumn,
  pub after: DbColumn,
}

// 注意，变化的排列顺序，dropUniqueConstraint 放在最前，addUniqueConstraint 放在最后
pub enum DBChange {
  DropUniqueConstraint(Vec<UniqueConstraint>),
  AddColumn(Vec<DbColumn>),
  DropColumn(Vec<DbColumn>),
  RenameColumn(Vec<RenameColumnInfo>),
  AddUniqueConstraint(Vec<UniqueConstraint>),
}

/// 业务操作
#[derive(Deserialize)]
#[serde(rename_all(deserialize = "camelCase"), tag = "type")]
pub enum BusinessOperator {
  /// 查询列表
  QueryList { conditions: Vec<QueryCondition> },
  /// 查询一条记录，即查询详情
  QueryOne,
  /// 新增一条记录
  CreateOne,
  /// 修改一条记录
  UpdateOne,
  /// 删除一条记录
  DeleteOne,
  /// 一个空操作，不做任何操作
  None,
}

/// 描述一个查询条件
#[derive(PartialEq, Deserialize)]
pub struct QueryCondition {
  /// 应用在哪一个列上
  pub column: DbColumn,
  /// 比较方式
  pub compare: DbCompare,
}

/// 描述如何跟列比较
#[derive(PartialEq, Deserialize)]
pub enum DbCompare {
  /// 相等
  Equal,
  /// 不相等
  NotEqual,
  /// 大于
  Greater,
  /// 大于等于
  GreaterOrEqual,
  /// 小于
  Less,
  /// 小于等于
  LessOrEqual,
  /// 左右匹配
  Like,
  /// 区间之内，包含左右边界
  Between,
  /// 在指定元素之内，每个元素做相等比较
  InThenItemEqual,
  /// 在指定元素之内，每个元素做 LIKE 匹配
  InThenItemLike,
}

pub fn to_entity_field_name(column_name: &str) -> String {
  column_name.to_lower_camel_case()
}
