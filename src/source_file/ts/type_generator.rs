use heck::{ToLowerCamelCase, ToUpperCamelCase};
use oxc_allocator::{Allocator, ArenaVec};
use oxc_ast::ast::TSSignature;
use crate::db_types::{DbTableStructure, QueryParam};
use crate::source_file::script_ast::ScriptAst;
use crate::ui_types::MatchOperation;

/// 获取 types 源码
/// ## Parameters
/// * `main_table`: 主表信息
/// * `sub_tables`: 从表列表
/// * `page_key`: 业务模块码
/// * `query_columns`: 查询条件列列表
pub fn get_type_code(
  main_table: DbTableStructure,
  sub_tables: Vec<DbTableStructure>,
  page_key: Option<String>,
  query_params: Vec<QueryParam>,
) -> String {
  let allocator = Allocator::new();
  let mut script_ast = ScriptAst::new(&allocator);

  script_ast.add_import_named_type("../common", &["PageDomain", "BaseEntity"]);

  // 分页查询参数 interface 源码
  if let Some(page_key) = page_key {
    let interface_name = allocator.alloc_str(format!("{}QueryParams", page_key.to_upper_camel_case()).as_str());

    let mut direct_properties = Vec::new();
    let mut params_properties = Vec::new();

    for query_param in query_params {
      match query_param.operation {
        MatchOperation::Between => {
          let begin_field_name = allocator.alloc_str(format!("begin{}", query_param.column_name.to_upper_camel_case()).as_str());
          let end_field_name = allocator.alloc_str(format!("end{}", query_param.column_name.to_upper_camel_case()).as_str());
          if query_param.data_type.is_ts_string_type() {
            params_properties.push(script_ast.new_interface_property_string(begin_field_name, true));
            params_properties.push(script_ast.new_interface_property_string(end_field_name, true));
          } else if query_param.data_type.is_ts_number_type() {
            params_properties.push(script_ast.new_interface_property_number(begin_field_name, true));
            params_properties.push(script_ast.new_interface_property_number(end_field_name, true));
          }else {
            params_properties.push(script_ast.new_interface_property_string(begin_field_name, true));
            params_properties.push(script_ast.new_interface_property_string(end_field_name, true));
          };
        }
        _ => {
          let field_name = allocator.alloc_str(query_param.column_name.to_lower_camel_case().as_str());
          let property = if query_param.data_type.is_ts_string_type() {
            script_ast.new_interface_property_string(field_name, true)
          } else if query_param.data_type.is_ts_number_type() {
            script_ast.new_interface_property_number(field_name, true)
          } else if query_param.data_type.is_ts_boolean_type() {
            script_ast.new_interface_property_boolean(field_name, true)
          } else {
            script_ast.new_interface_property_string(field_name, true)
          };
          direct_properties.push(property);
        }
      }
    }

    direct_properties.push(script_ast.new_interface_property_type_literal("params", params_properties, true));

    script_ast.add_interface(
      interface_name,
      direct_properties,
      &["PageDomain"]
    );
  }

  // 实体 interface 源码
  // 主表
  let entity_class_name = main_table.table.to_entity_class_name();
  let properties: Vec<TSSignature> = main_table.columns
    .iter()
    .filter(|column| !column.is_audit_field())
    .map(|column| {
      let field_name = allocator.alloc_str(column.to_entity_field_name().as_str());
      if column.data_type.is_ts_string_type() {
        script_ast.new_interface_property_string(field_name, true)
      } else if column.data_type.is_ts_number_type() {
        script_ast.new_interface_property_number(field_name, true)
      } else if column.data_type.is_ts_boolean_type() {
        script_ast.new_interface_property_boolean(field_name, true)
      } else {
        script_ast.new_interface_property_any(field_name, true)
      }
    })
    .chain(sub_tables.iter().map(|sub_table| {
      // 此处没有根据子表中的外键做进一步判断，而是假定传入的子表都跟主表是一对多关系。此处是一个深化点。
      let class_name = allocator.alloc_str(sub_table.table.to_entity_class_name().as_str());
      let field_name = allocator.alloc_str(format!("{}List", sub_table.table.to_entity_field_name()).as_str());
      script_ast.new_interface_property_array_type(field_name, class_name, true)
    })).collect();

  script_ast.add_interface(
    entity_class_name.as_str(),
    properties,
    &["BaseEntity"]
  );

  // 从表
  for sub_table in sub_tables {
    let entity_class_name = allocator.alloc_str(sub_table.table.to_entity_class_name().as_str());
    let properties: Vec<TSSignature> = sub_table.columns
      .iter()
      .filter(|column| !column.is_audit_field())
      .map(|column| {
        let field_name = allocator.alloc_str(column.to_entity_field_name().as_str());
        if column.data_type.is_ts_string_type() {
          script_ast.new_interface_property_string(field_name, true)
        } else if column.data_type.is_ts_number_type() {
          script_ast.new_interface_property_number(field_name, true)
        } else if column.data_type.is_ts_boolean_type() {
          script_ast.new_interface_property_boolean(field_name, true)
        } else {
          script_ast.new_interface_property_any(field_name, true)
        }
      })
      .collect();
    script_ast.add_interface(
      entity_class_name,
      properties,
      &["BaseEntity"]
    );
  }

  script_ast.get_code()
}

#[cfg(test)]
mod tests {
  use crate::db_types::{DbColumn, DbDataType, DbTable, QueryParam};
  use crate::ui_types::MatchOperation;
  use super::*;

  #[test]
  fn test_get_type_code_number() {
    let main_table_structure = DbTableStructure {
      table: DbTable {
        name: "demo_table".to_string(),
        comment: "示例表".to_string(),
      },
      columns: vec![DbColumn {
        name: "id".to_string(),
        comment: "主键".to_string(),
        data_type: DbDataType::BigInt,
        max_length: None,
        scale: None,
        unsigned: None,
        primary: true,
        nullable: false,
        default_value: None,
        db_unit_name: None,
        ui_unit_name: None,
        after_column_name: None,
        table_name: None,
      }],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let actual_code = get_type_code(main_table_structure, vec![], None, vec![]);
    let expect_code = concat!(
      "import type { PageDomain, BaseEntity } from \"../common\";\n",
      "export interface DemoTable extends BaseEntity {\n  id?: number;\n}\n"
    );
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn test_get_type_code_string() {
    let main_table_structure = DbTableStructure {
      table: DbTable {
        name: "demo_table".to_string(),
        comment: "示例表".to_string(),
      },
      columns: vec![DbColumn {
        name: "nick_name".to_string(),
        comment: "昵称".to_string(),
        data_type: DbDataType::Varchar,
        max_length: None,
        scale: None,
        unsigned: None,
        primary: true,
        nullable: false,
        default_value: None,
        db_unit_name: None,
        ui_unit_name: None,
        after_column_name: None,
        table_name: None,
      }],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let actual_code = get_type_code(main_table_structure, vec![], None, vec![]);
    let expect_code = concat!(
    "import type { PageDomain, BaseEntity } from \"../common\";\n",
    "export interface DemoTable extends BaseEntity {\n  nickName?: string;\n}\n"
    );
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn test_get_type_code_ignore_audit_fields() {
    let main_table_structure = DbTableStructure {
      table: DbTable {
        name: "demo_table".to_string(),
        comment: "示例表".to_string(),
      },
      columns: vec![DbColumn {
        name: "create_by".to_string(),
        comment: "创建人标识".to_string(),
        data_type: DbDataType::BigInt,
        max_length: None,
        scale: None,
        unsigned: None,
        primary: false,
        nullable: false,
        default_value: None,
        db_unit_name: None,
        ui_unit_name: None,
        after_column_name: None,
        table_name: None,
      }, DbColumn {
        name: "create_time".to_string(),
        comment: "创建时间".to_string(),
        data_type: DbDataType::DateTime,
        max_length: None,
        scale: None,
        unsigned: None,
        primary: false,
        nullable: false,
        default_value: None,
        db_unit_name: None,
        ui_unit_name: None,
        after_column_name: None,
        table_name: None,
      }],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let actual_code = get_type_code(main_table_structure, vec![], None, vec![]);
    let expect_code = concat!(
    "import type { PageDomain, BaseEntity } from \"../common\";\n",
    "export interface DemoTable extends BaseEntity {}\n"
    );
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn test_get_type_code_sub_tables() {
    let main_table_structure = DbTableStructure {
      table: DbTable {
        name: "main_table".to_string(),
        comment: "主表".to_string(),
      },
      columns: vec![DbColumn {
        name: "id".to_string(),
        comment: "主键".to_string(),
        data_type: DbDataType::BigInt,
        max_length: None,
        scale: None,
        unsigned: None,
        primary: true,
        nullable: false,
        default_value: None,
        db_unit_name: None,
        ui_unit_name: None,
        after_column_name: None,
        table_name: None,
      }],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let sub_table_structure_1 = DbTableStructure {
      table: DbTable {
        name: "sub_table_1".to_string(),
        comment: "从表1".to_string(),
      },
      columns: vec![DbColumn {
        name: "id".to_string(),
        comment: "主键".to_string(),
        data_type: DbDataType::BigInt,
        max_length: None,
        scale: None,
        unsigned: None,
        primary: true,
        nullable: false,
        default_value: None,
        db_unit_name: None,
        ui_unit_name: None,
        after_column_name: None,
        table_name: None,
      }],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let sub_table_structure_2 = DbTableStructure {
      table: DbTable {
        name: "sub_table_2".to_string(),
        comment: "从表2".to_string(),
      },
      columns: vec![DbColumn {
        name: "id".to_string(),
        comment: "主键".to_string(),
        data_type: DbDataType::BigInt,
        max_length: None,
        scale: None,
        unsigned: None,
        primary: true,
        nullable: false,
        default_value: None,
        db_unit_name: None,
        ui_unit_name: None,
        after_column_name: None,
        table_name: None,
      }],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let actual_code = get_type_code(main_table_structure, vec![sub_table_structure_1, sub_table_structure_2], None, vec![]);
    let expect_code = concat!(
    "import type { PageDomain, BaseEntity } from \"../common\";\n",
    "export interface MainTable extends BaseEntity {\n  id?: number;\n  subTable1List?: SubTable1[];\n  subTable2List?: SubTable2[];\n}\n",
    "export interface SubTable1 extends BaseEntity {\n  id?: number;\n}\n",
    "export interface SubTable2 extends BaseEntity {\n  id?: number;\n}\n",
    );
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn test_get_type_code_query_columns() {
    let main_table_structure = DbTableStructure {
      table: DbTable {
        name: "main_table".to_string(),
        comment: "主表".to_string(),
      },
      columns: vec![DbColumn {
        name: "id".to_string(),
        comment: "主键".to_string(),
        data_type: DbDataType::BigInt,
        max_length: None,
        scale: None,
        unsigned: None,
        primary: true,
        nullable: false,
        default_value: None,
        db_unit_name: None,
        ui_unit_name: None,
        after_column_name: None,
        table_name: None,
      }],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let query_columns = vec![QueryParam {
      table_name: "table1".to_string(),
      column_name: "str_field1".to_string(),
      data_type: DbDataType::Varchar,
      operation: MatchOperation::Contains,
    }, QueryParam {
      table_name: "table1".to_string(),
      column_name: "str_field2".to_string(),
      data_type: DbDataType::Varchar,
      operation: MatchOperation::Contains,
    }, QueryParam {
      table_name: "table1".to_string(),
      column_name: "the_date1".to_string(),
      data_type: DbDataType::Date,
      operation: MatchOperation::Between,
    }];

    let actual_code = get_type_code(main_table_structure, vec![], Some("page1".to_string()), query_columns);
    let expect_code = concat!(
    "import type { PageDomain, BaseEntity } from \"../common\";\n",
    "export interface Page1QueryParams extends PageDomain {\n  strField1?: string;\n  strField2?: string;\n  params?: {\n    beginTheDate1?: string;\n    endTheDate1?: string;\n  };\n}\n",
    "export interface MainTable extends BaseEntity {\n  id?: number;\n}\n",
    );
    assert_eq!(actual_code, expect_code);
  }
}