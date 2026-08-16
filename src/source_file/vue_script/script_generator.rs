use heck::ToUpperCamelCase;
use napi::Either;
use oxc_allocator::Allocator;
use crate::db_types::{DbTableStructure, Form, FormField, MatchOperation, TableParamSlot};
use crate::source_file::script_ast::ScriptAst;

pub fn get_sfc_script_code(
  parent_keys: &[&str],
  page_key: String,
  dict_names: &[&str],
  db_table_structure: DbTableStructure,
  between_date_fields: &[&str],
  table_param_slot: Option<TableParamSlot>,
  form: Option<Form>,
) -> String {
  let allocator = Allocator::new();
  let mut script_ast = ScriptAst::new(&allocator);

  let upper_page_key = page_key.to_upper_camel_case();

  let type_import_source = format!("@/types/api/{}/{}", parent_keys.join("/"), page_key);
  let params_type_name = format!("{}QueryParams", upper_page_key);
  let table_type_name = db_table_structure.table.to_entity_class_name();
  script_ast.add_import_named_type(
    type_import_source.as_str(),
    &[
      table_type_name.as_str(),
      params_type_name.as_str()
    ]
  );

  let api_import_source = format!("@/api/{}/{}", parent_keys.join("/"), page_key);
  let fetch_data_list_method_name = format!("fetch{}List", upper_page_key);
  let fetch_one_data_by_id_method_name = format!("fetch{}ById", upper_page_key);
  let add_one_data_method_name = format!("add{}", upper_page_key);
  let update_one_data_by_id_method_name = format!("update{}", upper_page_key);
  let delete_one_data_by_id_method_name = format!("delete{}ById", upper_page_key);
  script_ast.add_import_named_value(
    api_import_source.as_str(),
    &[
      fetch_data_list_method_name.as_str(),
      fetch_one_data_by_id_method_name.as_str(),
      add_one_data_method_name.as_str(),
      update_one_data_by_id_method_name.as_str(),
      delete_one_data_by_id_method_name.as_str()
    ]
  );

  script_ast.add_call_use_dict(dict_names);
  let data_list_var_name = format!("{}List", page_key);
  script_ast.add_const_ref_object_array(data_list_var_name.as_str(), table_type_name.as_str());

  script_ast.add_const_ref_boolean("open", false);
  script_ast.add_const_ref_boolean("loading", true);
  script_ast.add_const_ref_boolean("showSearch", true);
  script_ast.add_const_ref_number_array("ids");
  script_ast.add_const_ref_boolean("single", true);
  script_ast.add_const_ref_boolean("multiple", true);
  script_ast.add_const_ref_number("total", 0.0);
  script_ast.add_const_ref_string("title", "");

  for date_field in between_date_fields {
    let date_range_name = allocator.alloc_str(format!("dateRange{}", date_field.to_upper_camel_case()).as_str());
    script_ast.add_const_ref_string_array(date_range_name);
  }

  // queryParams
  if let Some(table_param_slot) = &table_param_slot {
    let mut query_params_properties = vec![
      script_ast.new_decimal_object_property("pageNum", 1.0),
      script_ast.new_decimal_object_property("pageSize", 10.0),
    ];

    for item in &table_param_slot.children {
      match item {
        Either::A(param_item) => {
          if param_item.operation != MatchOperation::Between {
            let property_name = allocator.alloc_str(param_item.property.as_str());
            query_params_properties.push(script_ast.new_undefined_object_property(property_name));
          }
        }
        Either::B(param_action_slot) => {
          // do nothing
        }
      }
    }

    let table_param_name = allocator.alloc_str(table_param_slot.name.as_str());
    // FIXME: 此处使用 table 的表名，是不是属于少了一层推导?
    script_ast.add_const_reactive_object(table_param_name, &[params_type_name.as_str()], query_params_properties);
  }

  // form
  if let Some(form) = form {
    let form_name = allocator.alloc_str(form.name.as_str());
    script_ast.add_const_reactive_object(form_name, &[table_type_name.as_str()], []);

    let mut property_rules = vec![];
    for field in form.fields {
      if let Some(rule_info) = field.get_rule_info() {
        if rule_info.required {
          let property = allocator.alloc_str(rule_info.property);
          let message = allocator.alloc_str(format!("{}不能为空", rule_info.label).as_str());
          let property_rule = script_ast.new_array_object_property(property, [
            script_ast.new_array_object_element([
              script_ast.new_boolean_object_property("required", true),
              script_ast.new_string_object_property("message", message),
              script_ast.new_string_object_property("trigger", "blur"),
            ])
          ]);
          property_rules.push(property_rule);
        }
      }
    }

    script_ast.add_const_reactive_object("rules", &["FormRules", table_type_name.as_str()], property_rules);
  }

  //region 函数
  // fetch data list 只有存在查询条件，才出现该方法。后续支持不需要任何查询条件的情况。
  if let Some(table_param_slot) = &table_param_slot {
    let fetch_data_list_api_name = allocator.alloc_str(format!("fetch{}List", upper_page_key).as_str());
    script_ast.add_arrow_async_function("getList", [], [
      script_ast.new_set_ref_boolean_value("loading", true),
      script_ast.new_try_catch_finally_statement([
        script_ast.new_call_fetch_data_list_api(fetch_data_list_api_name, table_param_slot.name.as_str()),
        script_ast.new_set_ref_identifier_value(data_list_var_name.as_str(), "rows"),
        script_ast.new_set_ref_identifier_value("total", "total"),
      ], [
        script_ast.new_call_console_error([
          script_ast.new_argument_identifier("e")
        ])
      ], [
        script_ast.new_set_ref_boolean_value("loading", false)
      ]),
    ]);
  }

  //endregion

  script_ast.get_code()
}

#[cfg(test)]
mod tests {
  use napi::Either;
  use crate::db_types::{DbColumn, DbDataType, DbTable, FormField, FormNumberInput, FormTextInput, MatchOperation, TableParamItem};
  use super::*;

  #[test]
  fn test_get_sfc_script_code() {
    let db_table_structure = DbTableStructure {
      table: DbTable {
        name: "demo_table".to_string(),
        comment: "示例表".to_string(),
      },
      columns: vec![
        DbColumn {
          name: "column1".to_string(),
          comment: "列1".to_string(),
          data_type: DbDataType::Varchar,
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
        },
        DbColumn {
          name: "column2".to_string(),
          comment: "列2".to_string(),
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
        }
      ],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };
    let table_param_slot = TableParamSlot {
      name: "queryParams".to_string(),
      children: vec![
        Either::A(
          TableParamItem {
            property: "column1".to_string(),
            operation: MatchOperation::Contains,
          }
        ),
        Either::A(
          TableParamItem {
            property: "column2".to_string(),
            operation: MatchOperation::Contains,
          }
        )
      ],
    };
    let form = Form {
      name: "form".to_string(),
      fields: vec![FormField::TextInput(FormTextInput {
        property: "field1".to_string(),
        label: "字段1".to_string(),
        required: true,
      }), FormField::TextInput(FormTextInput {
        property: "field2".to_string(),
        label: "字段2".to_string(),
        required: false,
      }), FormField::NumberInput(FormNumberInput {
        property: "field3".to_string(),
        label: "字段3".to_string(),
        required: true,
      })],
    };

    let actual_code = get_sfc_script_code(
      &["group1"],
      "demo".to_string(),
      &["dict_1", "dict_2"],
      db_table_structure,
      &["theDate1", "theDate2"],
      Some(table_param_slot),
      Some(form),
    );
    let expect_code = concat!(
      "import type { DemoTable, DemoQueryParams } from \"@/types/api/group1/demo\";\n",
      "import { fetchDemoList, fetchDemoById, addDemo, updateDemo, deleteDemoById } from \"@/api/group1/demo\";\n",
      "const { dict_1, dict_2 } = useDict(\"dict_1\", \"dict_2\");\n",
      "const demoList = ref<DemoTable[]>([]);\n",
      "const open = ref<boolean>(false);\n",
      "const loading = ref<boolean>(true);\n",
      "const showSearch = ref<boolean>(true);\n",
      "const ids = ref<number[]>([]);\n",
      "const single = ref<boolean>(true);\n",
      "const multiple = ref<boolean>(true);\n",
      "const total = ref<number>(0);\n",
      "const title = ref<string>(\"\");\n",
      "const dateRangeTheDate1 = ref<string[]>([]);\n",
      "const dateRangeTheDate2 = ref<string[]>([]);\n",
      "const queryParams: DemoQueryParams = reactive({\n  pageNum: 1,\n  pageSize: 10,\n  column1: undefined,\n  column2: undefined\n});\n",
      "const form: DemoTable = reactive({});\n",
      "const rules: FormRules<DemoTable> = reactive({\n",
      "  field1: [{\n    required: true,\n    message: \"字段1不能为空\",\n    trigger: \"blur\"\n  }],\n",
      "  field3: [{\n    required: true,\n    message: \"字段3不能为空\",\n    trigger: \"blur\"\n  }]\n",
      "});\n",
      "const getList = async () => {\n",
      "  loading.value = true;\n",
      "  try {\n",
      "    const { rows, total } = await fetchDemoList(queryParams.value);\n",
      "    demoList.value = rows;\n",
      "    total.value = total;\n",
      "  } catch (e) {\n",
      "    console.error(e);\n",
      "  } finally {\n",
      "    loading.value = false;\n",
      "  }\n",
      "};\n",
    );
    assert_eq!(actual_code, expect_code);
  }
}