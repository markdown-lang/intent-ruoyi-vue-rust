use heck::{ToLowerCamelCase, ToUpperCamelCase};
use oxc_allocator::{Allocator, GetAllocator};
use crate::db_types::DbTableStructure;
use crate::source_file::script_ast::ScriptAst;

pub struct TsApiInfo<'a> {
  // 分组key，支持多层
  parent_keys: &'a [&'a str],
  page_key: String,
  db_table_structure: DbTableStructure,
  script_ast: ScriptAst<'a>
}

impl<'a> TsApiInfo<'a> {

  pub fn new(
    parent_keys: &'a [&'a str],
    page_key: String,
    db_table_structure: DbTableStructure,
    allocator: &'a Allocator
  ) -> TsApiInfo<'a> {
    let script_ast = ScriptAst::new(&allocator);

    Self {
      parent_keys,
      page_key,
      db_table_structure,
      script_ast,
    }
  }

  pub fn get_code(mut self) -> String {
    self.add_import();
    self.add_fetch_data_list_api();
    self.add_fetch_one_data_api();
    self.add_add_one_data_api();
    self.add_update_one_data_api();
    self.add_delete_data_list_api();

    self.script_ast.get_code()
  }

  fn add_import(&mut self) {
    self.script_ast.add_import_default("@/utils/request", "request");

    let params_type_name = self.alloc_str(format!("{}QueryParams", self.page_key.to_upper_camel_case()).as_str());
    let table_type_name = self.alloc_str(self.db_table_structure.table.to_entity_class_name().as_str());
    self.script_ast.add_import_named_type("@/types", &[
      "AjaxResult",
      "TableDataInfo",
      params_type_name,
      table_type_name,
    ]);
  }

  fn alloc_str(&mut self, src: &str) -> &'a str {
    self.script_ast.allocator().alloc_str(src)
  }

  fn add_fetch_data_list_api(&mut self) {
    let function_name = self.alloc_str(format!("fetch{}List", self.page_key).as_str());

    let formal_parameter_type_name = self.alloc_str(format!("{}QueryParams", self.page_key.to_upper_camel_case()).as_str());
    let url = self.alloc_str(format!("/{}/{}/list", self.parent_keys.join("/"), self.page_key.to_lower_camel_case()).as_str());

    self.script_ast.add_api_function(
      function_name,
      [
        self.script_ast.new_formal_type_parameter("params", formal_parameter_type_name)
      ],
      &["TableDataInfo", "DemoTable[]"],
      [
        self.script_ast.new_return_request_get_statement(
          url,
          &["params"]
        )
      ]
    );
  }

  fn add_fetch_one_data_api(&mut self) {
    let function_name = self.alloc_str(format!("fetch{}ById", self.page_key).as_str());

    let url = self.alloc_str(format!("`/{}/{}/${{id}}`", self.parent_keys.join("/"), self.page_key.to_lower_camel_case()).as_str());

    self.script_ast.add_api_function(
      function_name,
      [
        self.script_ast.new_formal_number_parameter("id")
      ],
      &["AjaxResult", "DemoTable"],
      [
        self.script_ast.new_return_request_get_statement(
          url,
          &[]
        )
      ]
    );
  }

  fn add_add_one_data_api(&mut self) {
    let function_name = self.alloc_str(format!("add{}", self.page_key).as_str());

    let type_name = self.alloc_str(self.db_table_structure.table.to_entity_class_name().as_str());
    let url = self.alloc_str(format!("/{}/{}", self.parent_keys.join("/"), self.page_key.to_lower_camel_case()).as_str());

    self.script_ast.add_api_function(
      function_name,
      [
        self.script_ast.new_formal_type_parameter("data", type_name)
      ],
      &["AjaxResult"],
      [
        self.script_ast.new_return_request_post_statement(
          url,
          "data"
        )
      ]
    );
  }

  fn add_update_one_data_api(&mut self) {
    let function_name = self.alloc_str(format!("update{}", self.page_key).as_str());

    let type_name = self.alloc_str(self.db_table_structure.table.to_entity_class_name().as_str());
    let url = self.alloc_str(format!("/{}/{}", self.parent_keys.join("/"), self.page_key.to_lower_camel_case()).as_str());

    self.script_ast.add_api_function(
      function_name,
      [
        self.script_ast.new_formal_type_parameter("data", type_name)
      ],
      &["AjaxResult"],
      [
        self.script_ast.new_return_request_put_statement(
          url,
          "data"
        )
      ]
    );
  }

  fn add_delete_data_list_api(&mut self) {
    let function_name = self.alloc_str(format!("delete{}ById", self.page_key).as_str());

    let url = self.alloc_str(format!("`/{}/{}/${{id}}`", self.parent_keys.join("/"), self.page_key.to_lower_camel_case()).as_str());

    self.script_ast.add_api_function(
      function_name,
      [
        self.script_ast.new_formal_union_types_parameter("id", [
          self.script_ast.new_ts_number_type(),
          self.script_ast.new_ts_array_number_type()
        ])
      ],
      &["AjaxResult"],
      [
        self.script_ast.new_return_request_delete_statement(
          url,
        )
      ]
    );
  }
}

#[cfg(test)]
mod tests {
  use crate::db_types::{DbTableStructure, DbColumn, DbDataType, DbTable};
  use super::*;

  #[test]
  fn test_get_api_code() {
    let parent_keys = &["group1"];
    let table_structure = DbTableStructure {
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

    let allocator = Allocator::new();
    let ts_api_info = TsApiInfo::new(parent_keys, "Demo".to_string(), table_structure, &allocator);

    let actual_code = ts_api_info.get_code();
    let expect_code = concat!(
      "import request from \"@/utils/request\";\n",
      "import type { AjaxResult, TableDataInfo, DemoQueryParams, DemoTable } from \"@/types\";\n",
      "export function fetchDemoList(params: DemoQueryParams): Promise<TableDataInfo<DemoTable[]>> {\n",
      "  return request.get(\"/group1/demo/list\", { params });\n",
      "}\n",
      "export function fetchDemoById(id: number): Promise<AjaxResult<DemoTable>> {\n",
      "  return request.get(`/group1/demo/${id}`);\n",
      "}\n",
      "export function addDemo(data: DemoTable): Promise<AjaxResult> {\n",
      "  return request.post(\"/group1/demo\", data);\n",
      "}\n",
      "export function updateDemo(data: DemoTable): Promise<AjaxResult> {\n",
      "  return request.put(\"/group1/demo\", data);\n",
      "}\n",
      "export function deleteDemoById(id: number | number[]): Promise<AjaxResult> {\n",
      "  return request.delete(`/group1/demo/${id}`);\n",
      "}\n",
    );
    assert_eq!(actual_code, expect_code);
  }
}