use heck::{ToLowerCamelCase, ToUpperCamelCase};
use napi::Either;
use oxc_allocator::{Allocator};
use oxc_ast::ast::ObjectPropertyKind;
use crate::db_types::{DbTableStructure, Form, MatchOperation, TableParamSlot};
use crate::source_file::script_ast::ScriptAst;

pub fn get_sfc_script_code(
  parent_keys: &[&str],
  page_key: String,
  page_name: String,
  dict_names: &[&str],
  main_table: DbTableStructure,
  sub_tables: Vec<DbTableStructure>,
  table_param_slot: Option<TableParamSlot>,
  form: Option<Form>,
) -> String {
  let allocator = Allocator::new();
  let mut script_ast = ScriptAst::new(&allocator);

  let upper_page_key = page_key.to_upper_camel_case();

  let type_import_source = format!("@/types/api/{}/{}", parent_keys.join("/"), page_key);
  let params_type_name = format!("{}QueryParams", upper_page_key);
  let table_type_name = main_table.table.to_entity_class_name();
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
  
  // queryParams
  if let Some(table_param_slot) = &table_param_slot {
    let date_range_fields = table_param_slot.get_date_range_fields();
    for date_field in date_range_fields {
      let date_range_name = allocator.alloc_str(format!("dateRange{}", date_field.to_upper_camel_case()).as_str());
      script_ast.add_const_ref_string_array(date_range_name);
    }
    
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
    script_ast.add_const_ref_object(table_param_name, params_type_name.as_str(), query_params_properties);
  }

  // form
  if let Some(form) = &form {
    let form_name = allocator.alloc_str(form.name.as_str());
    script_ast.add_const_ref_object(form_name, table_type_name.as_str(), []);

    let mut property_rules = vec![];
    for field in &form.fields {
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

  script_ast.add_const_use_template_ref("queryRef");
  let form_ref_key = format!("{}Ref", page_key);
  script_ast.add_const_use_template_ref(form_ref_key.as_str());

  //region 函数, TODO: 根据 action 判断是否生成
  // fetch data list 只有存在查询条件，才出现该方法。后续支持不需要任何查询条件的情况。
  if let Some(table_param_slot) = &table_param_slot {
    let table_param_name = table_param_slot.name.as_str();
    let mut try_statements = vec![];
    if table_param_slot.has_date_range_param() {
      try_statements.push(script_ast.new_clear_ref_object_property(table_param_name, "params"));
    }
    let date_range_fields = table_param_slot.get_date_range_fields();
    for date_field in date_range_fields {
      let date_range_name = allocator.alloc_str(format!("dateRange{}", date_field.to_upper_camel_case()).as_str());
      let begin_params = allocator.alloc_str(format!("[begin{}]", date_field.to_upper_camel_case()).as_str());
      let end_params = allocator.alloc_str(format!("[end{}]", date_field.to_upper_camel_case()).as_str());
      try_statements.push(script_ast.new_if_statement(
        script_ast.new_check_ref_value_is_blank(date_range_name),
        [
          script_ast.new_set_member_identifier_value(&[table_param_name, "value", "params", begin_params], &[date_range_name, "value", "[0]"]),
          script_ast.new_set_member_identifier_value(&[table_param_name, "value", "params", end_params], &[date_range_name, "value", "[1]"]),
        ]
      ));
    }
    let fetch_data_list_api_name = allocator.alloc_str(format!("fetch{}List", upper_page_key).as_str());
    try_statements.push(script_ast.new_call_fetch_data_list_api(fetch_data_list_api_name, table_param_name));
    try_statements.push(script_ast.new_set_ref_identifier_value(data_list_var_name.as_str(), "rows"));
    try_statements.push(script_ast.new_set_ref_identifier_value("total", "total"));

    script_ast.add_arrow_async_function("getList", [], [
      script_ast.new_set_ref_boolean_value("loading", true),
      script_ast.new_try_catch_finally_statement(try_statements, [
        script_ast.new_call_console_error([
          script_ast.new_argument_identifier("e")
        ])
      ], [
        script_ast.new_set_ref_boolean_value("loading", false),
      ]),
    ]);

    // cancel
    script_ast.add_arrow_function("cancel", [], [
      script_ast.new_set_ref_boolean_value("open", false),
      script_ast.new_call_function("reset", []),
    ]);

    //reset
    let reset_form_properties: Vec<ObjectPropertyKind> = main_table.columns
      .iter()
      .filter(|column| !column.is_audit_field())
      .map(|column| {
        let field_name = allocator.alloc_str(column.to_entity_field_name().as_str());
        script_ast.new_undefined_object_property(field_name)
      })
      .collect();
    script_ast.add_arrow_function("reset", [], [
      script_ast.new_set_ref_object_value("form", reset_form_properties),
      script_ast.new_call_member_method(&[form_ref_key.as_str(), "value?"], "resetFields", []),
    ]);

    // handleQuery
    script_ast.add_arrow_function("handleQuery", [], [
      script_ast.new_set_member_number_value(&[table_param_name, "value", "pageNum"], 1.0),
      script_ast.new_call_function("getList", []),
    ]);

    // resetQuery
    let mut reset_query_body_statements = vec![];
    let date_range_fields = table_param_slot.get_date_range_fields();
    for date_field in date_range_fields {
      let date_range_name = allocator.alloc_str(format!("dateRange{}", date_field.to_upper_camel_case()).as_str());
      reset_query_body_statements.push(script_ast.new_set_ref_array_empty_value(date_range_name));
    }
    reset_query_body_statements.push(script_ast.new_call_member_method(&["queryRef", "value?"], "resetFields", []));
    reset_query_body_statements.push(script_ast.new_call_function("handleQuery", []));
    script_ast.add_arrow_function("resetQuery", [], reset_query_body_statements);

  }

  // handleSelectionChange
  let primary_column_name = main_table.get_primary_key_column_name().to_lower_camel_case();
  let non_null_primary_column_name = format!("{}!", primary_column_name);
  script_ast.add_arrow_function("handleSelectionChange", [
    script_ast.new_formal_array_parameter("selection", table_type_name.as_str()),
  ], [
    script_ast.new_set_ref_expression_value(
      "ids",
      script_ast.new_call_object_method_expression(
        "selection",
        "map",
        [script_ast.new_argument_arrow_function_member_expression(
          [script_ast.new_formal_parameter("item")],
          "item",
          non_null_primary_column_name.as_str()
        )]
      )
    ),
    script_ast.new_set_ref_expression_value("single", script_ast.new_member_not_equal_number(&["selection", "length"], 1.0)),
    script_ast.new_set_ref_expression_value("multiple", script_ast.new_member_not(&["selection", "length"])),
  ]);

  // handleAdd
  let add_title = format!("添加{}", page_name);
  script_ast.add_arrow_function("handleAdd", [], [
    script_ast.new_call_function("reset", []),
    script_ast.new_set_ref_boolean_value("open", true),
    script_ast.new_set_ref_string_value("title", add_title.as_str())
  ]);

  // handleUpdate
  let update_title = format!("修改{}", page_name);
  script_ast.add_arrow_async_function("handleUpdate", [
    script_ast.new_formal_type_parameter("row", table_type_name.as_str()),
  ], [
    script_ast.new_call_function("reset", []),
    script_ast.new_const_expression(primary_column_name.as_str(), script_ast.new_or_expression([
      script_ast.new_right_member_expression(&["row", primary_column_name.as_str()]),
      script_ast.new_right_member_expression(&["ids", "[0]"]),
    ])),
    script_ast.new_call_fetch_one_data_by_id_api(fetch_one_data_by_id_method_name.as_str(), primary_column_name.as_str()),
    script_ast.new_set_ref_identifier_value("form", "data"),
    script_ast.new_set_ref_boolean_value("open", true),
    script_ast.new_set_ref_string_value("title", update_title.as_str()),
  ]);

  // submitForm
  if let Some(form) = &form {
    let form_name = allocator.alloc_str(form.name.as_str());
    script_ast.add_arrow_async_function("submitForm", [], [
      script_ast.new_try_catch_statement(
        [
          script_ast.new_call_form_validate(form_ref_key.as_str()),
          script_ast.new_if_else_statement(
            script_ast.new_check_member_is_not_undefined(&[form_name, "value", primary_column_name.as_str()]),
            [
              script_ast.new_call_save_one_data_api(update_one_data_by_id_method_name.as_str(), form_name),
              script_ast.new_call_msg_success("修改成功"),
            ],
            [
              script_ast.new_call_save_one_data_api(add_one_data_method_name.as_str(), form_name),
              script_ast.new_call_msg_success("新增成功"),
            ]
          ),
          script_ast.new_set_ref_boolean_value("open", false),
          script_ast.new_call_function("getList", []),
        ], [
          script_ast.new_call_console_error([
            script_ast.new_argument_string("提交失败"),
            script_ast.new_argument_identifier("e"),
          ]),
        ]),
    ]);
  }

  // handleDelete
  let ids_name = format!("{}Ids", page_key);
  script_ast.add_arrow_async_function(
    "handleDelete",
    [script_ast.new_formal_type_parameter("row", table_type_name.as_str()),],
    [
      script_ast.new_const_expression(
        ids_name.as_str(),
        script_ast.new_or_expression([
          script_ast.new_right_member_expression(&["row", primary_column_name.as_str()]),
          script_ast.new_right_member_expression(&["ids", "value"]),
        ]),
      ),
      script_ast.new_try_catch_statement(
        [
          script_ast.new_call_confirm("确定要删除吗？"),
          script_ast.new_call_delete_one_data_by_id(delete_one_data_by_id_method_name.as_str(), ids_name.as_str()),
          script_ast.new_call_function("getList", []),
          script_ast.new_call_msg_success("删除成功"),
        ],
        [
          script_ast.new_call_console_error([
            script_ast.new_argument_string("删除取消或删除失败"),
            script_ast.new_argument_identifier("e"),
          ]),
        ]
      ),
    ]
  );

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
        },
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
            db_data_type: DbDataType::Varchar,
          }
        ),
        Either::A(
          TableParamItem {
            property: "column2".to_string(),
            operation: MatchOperation::Contains,
            db_data_type: DbDataType::Varchar,
          }
        ),
        Either::A(
          TableParamItem {
            property: "theDate1".to_string(),
            operation: MatchOperation::Between,
            db_data_type: DbDataType::Date,
          }
        ),
        Either::A(
          TableParamItem {
            property: "theDate2".to_string(),
            operation: MatchOperation::Between,
            db_data_type: DbDataType::Date,
          }
        ),
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
      "示例".to_string(),
      &["dict_1", "dict_2"],
      db_table_structure,
      vec![DbTableStructure {
        table: DbTable {
          name: "sub_table1".to_string(),
          comment: "子表1".to_string(),
        },
        columns: vec![],
        unique_constraints: vec![],
        foreign_constraints: vec![],
      }],
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
      "const queryParams = ref<DemoQueryParams>({\n  pageNum: 1,\n  pageSize: 10,\n  column1: undefined,\n  column2: undefined\n});\n",
      "const form = ref<DemoTable>({});\n",
      "const rules: FormRules<DemoTable> = reactive({\n",
      "  field1: [{\n    required: true,\n    message: \"字段1不能为空\",\n    trigger: \"blur\"\n  }],\n",
      "  field3: [{\n    required: true,\n    message: \"字段3不能为空\",\n    trigger: \"blur\"\n  }]\n",
      "});\n",
      "const queryRef = useTemplateRef(\"queryRef\");\n",
      "const demoRef = useTemplateRef(\"demoRef\");\n",
      "const getList = async () => {\n",
      "  loading.value = true;\n",
      "  try {\n",
      "    queryParams.value.params = {};\n",
      "    if (null != dateRangeTheDate1.value && \"\" != dateRangeTheDate1.value) {\n",
      "      queryParams.value.params[\"beginTheDate1\"] = dateRangeTheDate1.value[0];\n",
      "      queryParams.value.params[\"endTheDate1\"] = dateRangeTheDate1.value[1];\n",
      "    }\n",
      "    if (null != dateRangeTheDate2.value && \"\" != dateRangeTheDate2.value) {\n",
      "      queryParams.value.params[\"beginTheDate2\"] = dateRangeTheDate2.value[0];\n",
      "      queryParams.value.params[\"endTheDate2\"] = dateRangeTheDate2.value[1];\n",
      "    }\n",
      "    const { rows, total } = await fetchDemoList(queryParams.value);\n",
      "    demoList.value = rows;\n",
      "    total.value = total;\n",
      "  } catch (e) {\n",
      "    console.error(e);\n",
      "  } finally {\n",
      "    loading.value = false;\n",
      "  }\n",
      "};\n",
      "const cancel = () => {\n",
      "  open.value = false;\n",
      "  reset();\n",
      "};\n",
      "const reset = () => {\n",
      "  form.value = {\n",
      "    id: undefined,\n",
      "    column1: undefined,\n",
      "    column2: undefined\n", // TODO: 支持将数组类型的属性设置为[]
      "  };\n",
      "  demoRef.value?.resetFields();\n",
      "};\n",
      "const handleQuery = () => {\n",
      "  queryParams.value.pageNum = 1;\n",
      "  getList();\n",
      "};\n",
      "const resetQuery = () => {\n",
      "  dateRangeTheDate1.value = [];\n",
      "  dateRangeTheDate2.value = [];\n",
      "  queryRef.value?.resetFields();\n",
      "  handleQuery();\n",
      "};\n",
      "const handleSelectionChange = (selection: DemoTable[]) => {\n",
      "  ids.value = selection.map((item) => item.id!);\n",
      "  single.value = selection.length != 1;\n",
      "  multiple.value = !selection.length;\n",
      "};\n",
      "const handleAdd = () => {\n",
      "  reset();\n",
      "  open.value = true;\n",
      "  title.value = \"添加示例\";\n",
      "};\n",
      "const handleUpdate = async (row: DemoTable) => {\n",
      "  reset();\n",
      "  const id = row.id || ids[0];\n",
      "  const { data } = await fetchDemoById(id);\n",
      "  form.value = data;\n",
      "  open.value = true;\n",
      "  title.value = \"修改示例\";\n",
      "};\n",
      "const submitForm = async () => {\n",
      "  try {\n",
      "    await demoRef.value?.validate();\n",
      "    if (form.value.id != undefined) {\n",
      "      await updateDemo(form.value);\n",
      "      modal.msgSuccess(\"修改成功\");\n",
      "    } else {\n",
      "      await addDemo(form.value);\n",
      "      modal.msgSuccess(\"新增成功\");\n",
      "    }\n",
      "    open.value = false;\n",
      "    getList();\n", // TODO: 如果是修改数据，则精准更新，不查询整个列表
      "  } catch (e) {\n",
      "    console.error(\"提交失败\", e);\n",
      "  }\n",
      "};\n",
      "const handleDelete = async (row: DemoTable) => {\n",
      "  const demoIds = row.id || ids.value;\n",
      "  try {\n",
      "    await modal.confirm(\"确定要删除吗？\");\n",
      "    await deleteDemoById(demoIds);\n",
      "    getList();\n",
      "    modal.msgSuccess(\"删除成功\");\n",
      "  } catch (e) {\n",
      "    console.error(\"删除取消或删除失败\", e);\n",
      "  }\n",
      "};\n",
    );
    assert_eq!(actual_code, expect_code);
  }
}