use crate::quick_xml_util::{
  write_new_line_ident, write_new_line_then_n_ident, write_one_new_line, write_two_new_lines,
};
use crate::sql_updater::{drop_columns_for_select_sql, insert_columns_for_select_sql};
use crate::db_table_info::{
  BusinessOperator, DBChange, DbColumn, DbCompare, DbDataType, DbTable, GroupLocation,
  QueryCondition, DbTableStructure, to_entity_field_name
};
use anyhow::Result;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use std::io::Cursor;

const INSERT_IGNORE_FIELDS: [&str; 3] = ["dbid", "update_by", "update_time"];
const UPDATE_IGNORE_FIELDS: [&str; 3] = ["dbid", "create_by", "create_time"];

/// QueryList 当前只能基于当前表生成，不支持多表关联。
/// TODO: 第一个参数是当前表，第二个参数是相关表列表，第三个参数是操作（其中的列信息中有tableId,便于与第一个参数和第二个参数建立关联）
pub fn generate(
  table_structure: DbTableStructure,
  business_operators: Vec<BusinessOperator>,
  group_location: GroupLocation,
) -> Result<String> {
  let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 4);

  // 写入 XML 声明
  let decl = BytesDecl::new("1.0", Some("UTF-8"), None);
  writer.write_event(Event::Decl(decl))?;
  writer.write_event(Event::DocType(BytesText::from_escaped("mapper PUBLIC \"-//mybatis.org//DTD Mapper 3.0//EN\" \"http://mybatis.org/dtd/mybatis-3-mapper.dtd\"")))?;

  write_two_new_lines(&mut writer)?;

  let entity_class_name = table_structure.table.to_entity_class_name();
  let table_name = table_structure.table.name;

  // mapper
  let mut mapper_tag = BytesStart::new("mapper");
  let namespace_value = format!(
    "{}.{}.{}.{}Mapper",
    group_location.java_base_package, group_location.group, "mapper", entity_class_name
  );
  mapper_tag.push_attribute(("namespace", namespace_value.as_str()));
  writer.write_event(Event::Start(mapper_tag))?;

  // resultMap
  let result_map_id = format!("{}Result", entity_class_name);
  let mut result_map_tag = BytesStart::new("resultMap");
  result_map_tag.push_attribute(("id", result_map_id.as_str()));
  result_map_tag.push_attribute(("type", entity_class_name.as_str()));
  writer.write_event(Event::Start(result_map_tag))?;

  for column in &table_structure.columns {
    write_result_tag(&mut writer, column)?;
  }

  writer.write_event(Event::End(BytesEnd::new("resultMap")))?;

  write_new_line_ident(&mut writer)?;

  // sql 默认的公用的 sql 片段
  let sql_id = format!("select{}Vo", entity_class_name);
  let mut sql_tag = BytesStart::new("sql");
  sql_tag.push_attribute(("id", sql_id.as_str()));
  writer.write_event(Event::Start(sql_tag))?;
  if !table_structure.columns.is_empty() {
    let formatted_column_names = table_structure
      .columns
      .iter()
      .map(|col| col.name.as_str())
      .collect::<Vec<_>>()
      .join(",\n            ");
    writer.write_event(Event::Text(BytesText::new(
      format!(
        r#"
        SELECT
            {}
        FROM
            {}
    "#,
        formatted_column_names, table_name
      )
      .as_str(),
    )))?;
  }
  writer.write_event(Event::End(BytesEnd::new("sql")))?;

  // if !business_operators.is_empty() {
  //     write_new_line_ident(&mut writer)?;
  // }

  for operator in business_operators {
    match operator {
      BusinessOperator::QueryList {
        conditions: query_conditions,
      } => {
        write_new_line_ident(&mut writer)?;

        //region 条件查询数据列表
        let mut query_list_select_tag = BytesStart::new("select");
        query_list_select_tag
          .push_attribute(("id", format!("select{}List", entity_class_name).as_str()));
        query_list_select_tag.push_attribute(("parameterType", entity_class_name.as_str()));
        query_list_select_tag.push_attribute(("resultMap", result_map_id.as_str()));
        writer.write_event(Event::Start(query_list_select_tag))?;
        // include
        let mut query_list_include_tag = BytesStart::new("include");
        query_list_include_tag.push_attribute(("refid", sql_id.as_str()));
        writer.write_event(Event::Empty(query_list_include_tag))?;

        // where
        if !query_conditions.is_empty() {
          let query_list_where_tag = BytesStart::new("where");
          writer.write_event(Event::Start(query_list_where_tag))?;

          for condition in query_conditions {
            match condition {
              QueryCondition { column, compare } => {
                let mut if_tag = BytesStart::new("if");
                let entity_field_name = column.to_entity_field_name();

                let test_condition = if compare == DbCompare::InThenItemLike
                  || compare == DbCompare::InThenItemEqual
                {
                  format!(
                    "{}List != null and {}List.length > 0",
                    entity_field_name, entity_field_name
                  )
                } else {
                  if column.data_type == DbDataType::Char || column.data_type == DbDataType::Varchar
                  {
                    format!(
                      "{} != null and {} != ''",
                      entity_field_name, entity_field_name
                    )
                  } else if column.data_type == DbDataType::Date
                    || column.data_type == DbDataType::DateTime
                  {
                    let field_name = column.name_to_upper_camel_case();
                    format!(
                      "params.begin{} != null and params.end{} != null",
                      field_name, field_name
                    )
                  } else {
                    format!("{} != null", entity_field_name)
                  }
                };

                let comparator = match compare {
                  DbCompare::Equal => "=",
                  DbCompare::NotEqual => "!=",
                  DbCompare::Greater => ">",
                  DbCompare::GreaterOrEqual => ">=",
                  DbCompare::Less => "<",
                  DbCompare::LessOrEqual => "<=",
                  _ => "",
                };
                // 直接传入 [u8] 不 escape
                if_tag.push_attribute(("test".as_bytes(), test_condition.as_bytes()));
                writer.write_event(Event::Start(if_tag))?;

                let add_clause = match compare {
                  DbCompare::Equal
                  | DbCompare::NotEqual
                  | DbCompare::Greater
                  | DbCompare::GreaterOrEqual
                  | DbCompare::Less
                  | DbCompare::LessOrEqual => {
                    format!(
                      "AND {} {} #{{{}}}",
                      column.name, comparator, entity_field_name
                    )
                  }
                  DbCompare::Like => {
                    format!(
                      "AND {} LIKE concat('%', #{{{}}}, '%')",
                      column.name, entity_field_name
                    )
                  }
                  DbCompare::Between => {
                    if column.data_type == DbDataType::Date
                      || column.data_type == DbDataType::DateTime
                    {
                      let column_name = &column.name;
                      let field_name = column.name_to_upper_camel_case();
                      format!(
                        "AND ({} >= #{{params.begin{}}} AND {} < #{{params.end{}}} + INTERVAL 1 DAY)",
                        column_name, field_name, column_name, field_name
                      )
                    } else {
                      "".to_string()
                    }
                  }
                  DbCompare::InThenItemLike => {
                    writer.write_indent()?;
                    "AND".to_string()
                  }
                  DbCompare::InThenItemEqual => {
                    writer.write_indent()?;
                    format!("AND {} IN", column.name)
                  }
                };
                writer.write_event(Event::Text(BytesText::from_escaped(add_clause.as_str())))?;
                match compare {
                  DbCompare::InThenItemLike => {
                    writer.write_indent()?;

                    let mut foreach_tag = BytesStart::new("foreach");
                    foreach_tag.push_attribute((
                      "collection",
                      format!("{}List", entity_field_name).as_str(),
                    ));
                    foreach_tag.push_attribute(("item", entity_field_name.as_str()));
                    foreach_tag.push_attribute(("open", "("));
                    foreach_tag.push_attribute(("separator", "OR"));
                    foreach_tag.push_attribute(("close", ")"));
                    writer.write_event(Event::Start(foreach_tag))?;

                    writer.write_indent()?;
                    let like_clause = format!(
                      "{} LIKE concat('%', #{{{}}}, '%')",
                      column.name, entity_field_name
                    );
                    writer.write_event(Event::Text(BytesText::from_escaped(like_clause)))?;
                    write_new_line_then_n_ident(&mut writer, 4)?;
                    writer.write_event(Event::End(BytesEnd::new("foreach")))?;
                  }
                  DbCompare::InThenItemEqual => {
                    writer.write_indent()?;

                    let mut foreach_tag = BytesStart::new("foreach");
                    foreach_tag.push_attribute((
                      "collection",
                      format!("{}List", entity_field_name).as_str(),
                    ));
                    foreach_tag.push_attribute(("item", entity_field_name.as_str()));
                    foreach_tag.push_attribute(("open", "("));
                    foreach_tag.push_attribute(("separator", ","));
                    foreach_tag.push_attribute(("close", ")"));
                    writer.write_event(Event::Start(foreach_tag))?;

                    writer.write_indent()?;
                    let clause = format!("#{{{}}}", entity_field_name);
                    writer.write_event(Event::Text(BytesText::from_escaped(clause)))?;
                    write_new_line_then_n_ident(&mut writer, 4)?;
                    writer.write_event(Event::End(BytesEnd::new("foreach")))?;
                  }
                  _ => {}
                }
                writer.write_event(Event::End(BytesEnd::new("if")))?
              }
            }
          }

          writer.write_event(Event::End(BytesEnd::new("where")))?;
        }
        writer.write_indent()?;
        let order_by_clause = "ORDER BY dbid DESC\n    ";
        writer.write_event(Event::Text(BytesText::new(order_by_clause)))?;
        writer.write_event(Event::End(BytesEnd::new("select")))?;
        //endregion

        //region 根据外键(或主表标识)查询子表中的数据列表
        if !table_structure.foreign_constraints.is_empty() {
          write_new_line_ident(&mut writer)?;

          for constraint in &table_structure.foreign_constraints {
            let mut select_tag = BytesStart::new("select");
            let method_name = format!(
              "selectSimple{}ListBy{}",
              entity_class_name,
              constraint.base_column.name_to_upper_camel_case()
            );
            select_tag.push_attribute(("id", method_name.as_str()));
            select_tag.push_attribute(("parameterType", "Long"));
            select_tag.push_attribute(("resultMap", result_map_id.as_str()));
            writer.write_event(Event::Start(select_tag))?;
            // include
            let mut query_list_include_tag = BytesStart::new("include");
            query_list_include_tag.push_attribute(("refid", sql_id.as_str()));
            writer.write_event(Event::Empty(query_list_include_tag))?;
            // where
            writer.write_indent()?;
            let where_part = format!(
              "WHERE {} = #{{{}}}",
              constraint.base_column.name,
              constraint.base_column.to_entity_field_name()
            );
            writer.write_event(Event::Text(BytesText::new(where_part.as_str())))?;
            write_new_line_then_n_ident(&mut writer, 1)?;
            writer.write_event(Event::End(BytesEnd::new("select")))?;
          }
        }
        //endregion
      }
      BusinessOperator::QueryOne => {
        write_new_line_ident(&mut writer)?;

        //region selectSimpleOneById 根据唯一约束查询一条记录，必须是单表查询
        let mut query_one_select_tag = BytesStart::new("select");
        query_one_select_tag.push_attribute((
          "id",
          format!("selectSimple{}ById", entity_class_name).as_str(),
        ));
        query_one_select_tag.push_attribute(("parameterType", "Long"));
        query_one_select_tag.push_attribute(("resultMap", result_map_id.as_str()));
        writer.write_event(Event::Start(query_one_select_tag))?;
        // include
        let mut query_one_include_tag = BytesStart::new("include");
        query_one_include_tag.push_attribute(("refid", sql_id.as_str()));
        writer.write_event(Event::Empty(query_one_include_tag))?;
        // where
        writer.write_indent()?;
        let where_clause = "WHERE dbid = #{dbid}";
        writer.write_event(Event::Text(BytesText::new(where_clause)))?;
        write_new_line_then_n_ident(&mut writer, 1)?;

        writer.write_event(Event::End(BytesEnd::new("select")))?;
        //endregion

        //region 根据外键(或主表标识)查询子表中的数据列表
        for unique_constraint in &table_structure.unique_constraints {
          write_new_line_ident(&mut writer)?;

          let mut select_tag = BytesStart::new("select");
          let joined_by_condition_parts = unique_constraint
            .columns
            .iter()
            .map(|col| col.name_to_upper_camel_case())
            .collect::<Vec<_>>()
            .join("And");
          select_tag.push_attribute((
            "id",
            format!(
              "selectSimple{}By{}",
              entity_class_name, joined_by_condition_parts
            )
            .as_str(),
          ));
          // 如果多个字段联合唯一，则不设置 parameterType 参数，而是通过 MyBatis 注解设置参数的对照关系
          if unique_constraint.columns.len() > 1 {
            // do nothing
            // 不设置 parameterType 参数
          } else if unique_constraint.columns.len() == 1 {
            select_tag.push_attribute((
              "parameterType",
              unique_constraint
                .columns
                .first()
                .unwrap()
                .data_type
                .as_java_type(),
            ));
          }
          select_tag.push_attribute(("resultMap", result_map_id.as_str()));
          writer.write_event(Event::Start(select_tag))?;

          // include
          let mut query_one_include_tag = BytesStart::new("include");
          query_one_include_tag.push_attribute(("refid", sql_id.as_str()));
          writer.write_event(Event::Empty(query_one_include_tag))?;

          let where_parts = unique_constraint
            .columns
            .iter()
            .map(|col| format!("{} = #{{{}}}", col.name, col.to_entity_field_name()))
            .collect::<Vec<_>>()
            .join(" AND ");
          let where_sql = format!("WHERE {}", where_parts);
          writer.write_indent()?;
          writer.write_event(Event::Text(BytesText::new(where_sql.as_str())))?;
          write_new_line_then_n_ident(&mut writer, 1)?;
          writer.write_event(Event::End(BytesEnd::new("select")))?;
        }
        //endregion
      }
      BusinessOperator::CreateOne => {
        write_new_line_ident(&mut writer)?;

        let mut create_one_insert_tag = BytesStart::new("insert");
        create_one_insert_tag
          .push_attribute(("id", format!("insert{}", entity_class_name).as_str()));
        create_one_insert_tag.push_attribute(("parameterType", entity_class_name.as_str()));
        if let Some(primary_column) = table_structure.columns.iter().find(|item| item.primary) {
          create_one_insert_tag.push_attribute(("useGeneratedKeys", "true"));
          create_one_insert_tag.push_attribute(("keyProperty", primary_column.name.as_str()));
        }
        writer.write_event(Event::Start(create_one_insert_tag))?;

        writer.write_indent()?;
        writer.write_event(Event::Text(BytesText::new(
          format!("INSERT INTO {}", table_name).as_str(),
        )))?;
        writer.write_indent()?;

        let valid_columns: Vec<&DbColumn> = table_structure
          .columns
          .iter()
          .filter(|item| !INSERT_IGNORE_FIELDS.contains(&item.name.as_str()))
          .collect();
        //region fields trim tag
        let mut fields_trim_tag = BytesStart::new("trim");
        fields_trim_tag.push_attribute(("prefix", "("));
        fields_trim_tag.push_attribute(("suffix", ")"));
        fields_trim_tag.push_attribute(("suffixOverrides", ","));
        writer.write_event(Event::Start(fields_trim_tag))?;

        for column in &valid_columns {
          write_insert_field_trim_if_tag(&mut writer, column)?;
        }

        writer.write_event(Event::End(BytesEnd::new("trim")))?;
        //endregion

        //region values trim tag
        let mut values_trim_tag = BytesStart::new("trim");
        values_trim_tag.push_attribute(("prefix", "VALUES ("));
        values_trim_tag.push_attribute(("suffix", ")"));
        values_trim_tag.push_attribute(("suffixOverrides", ","));
        writer.write_event(Event::Start(values_trim_tag))?;

        // if
        for column in &valid_columns {
          write_insert_value_trim_if_tag(&mut writer, column)?;
        }

        writer.write_event(Event::End(BytesEnd::new("trim")))?;
        //endregion

        writer.write_event(Event::End(BytesEnd::new("insert")))?;
      }
      BusinessOperator::UpdateOne => {
        write_new_line_ident(&mut writer)?;

        let mut update_tag = BytesStart::new("update");
        update_tag.push_attribute(("id", format!("update{}", entity_class_name).as_str()));
        update_tag.push_attribute(("parameterType", entity_class_name.as_str()));
        writer.write_event(Event::Start(update_tag))?;

        writer.write_indent()?;
        writer.write_event(Event::Text(BytesText::new(
          format!("UPDATE {}", table_name).as_str(),
        )))?;
        writer.write_indent()?;

        // trim 节点
        let mut trim_tag = BytesStart::new("trim");
        trim_tag.push_attribute(("prefix", "SET"));
        trim_tag.push_attribute(("suffixOverrides", ","));
        writer.write_event(Event::Start(trim_tag))?;

        let valid_columns: Vec<&DbColumn> = table_structure
          .columns
          .iter()
          .filter(|item| !UPDATE_IGNORE_FIELDS.contains(&item.name.as_str()))
          .collect();

        for (index, column) in valid_columns.iter().enumerate() {
          if column.nullable && !INSERT_IGNORE_FIELDS.contains(&column.name.as_str()) {
            writer.write_indent()?;

            let update_expression =
              format!("{} = #{{{}}},", column.name, column.to_entity_field_name());
            writer.write_event(Event::Text(BytesText::new(update_expression.as_str())))?;
          } else {
            // if tag
            write_update_trim_if_tag(&mut writer, column)?;
          }
        }
        write_new_line_then_n_ident(&mut writer, 2)?;
        writer.write_event(Event::End(BytesEnd::new("trim")))?;
        writer.write_indent()?;

        writer.write_event(Event::Text(BytesText::new("WHERE dbid = #{dbid}")))?;
        write_new_line_then_n_ident(&mut writer, 1)?;

        writer.write_event(Event::End(BytesEnd::new("update")))?;
      }
      BusinessOperator::DeleteOne => {
        write_new_line_ident(&mut writer)?;

        //region delete one by id
        let mut delete_one_by_id = BytesStart::new("delete");
        delete_one_by_id
          .push_attribute(("id", format!("delete{}ById", entity_class_name).as_str()));
        delete_one_by_id.push_attribute(("parameterType", "Long"));
        writer.write_event(Event::Start(delete_one_by_id))?;
        writer.write_indent()?;
        let delete_one_by_id_expression =
          format!("DELETE FROM {} WHERE dbid = #{{dbid}}", table_name);
        writer.write_event(Event::Text(BytesText::new(
          delete_one_by_id_expression.as_str(),
        )))?;
        write_new_line_then_n_ident(&mut writer, 1)?;
        writer.write_event(Event::End(BytesEnd::new("delete")))?;
        //endregion

        //region delete one by unique constraint
        for unique_constraint in &table_structure.unique_constraints {
          write_new_line_ident(&mut writer)?;

          let mut delete_tag = BytesStart::new("delete");
          let joined_by_condition_parts = unique_constraint
            .columns
            .iter()
            .map(|col| col.name_to_upper_camel_case())
            .collect::<Vec<_>>()
            .join("And");
          delete_tag.push_attribute((
            "id",
            format!("delete{}By{}", entity_class_name, joined_by_condition_parts).as_str(),
          ));
          // 如果多个字段联合唯一，则不设置 parameterType 参数，而是通过 MyBatis 注解设置参数的对照关系
          if unique_constraint.columns.len() > 1 {
            // do nothing
            // 不设置 parameterType 参数
          } else if unique_constraint.columns.len() == 1 {
            delete_tag.push_attribute((
              "parameterType",
              unique_constraint
                .columns
                .first()
                .unwrap()
                .data_type
                .as_java_type(),
            ));
          }
          writer.write_event(Event::Start(delete_tag))?;

          let where_parts = unique_constraint
            .columns
            .iter()
            .map(|col| format!("{} = #{{{}}}", col.name, col.to_entity_field_name()))
            .collect::<Vec<_>>()
            .join(" AND ");
          let delete_sql = format!("DELETE FROM {} WHERE {}", table_name, where_parts);
          writer.write_indent()?;
          writer.write_event(Event::Text(BytesText::new(delete_sql.as_str())))?;
          write_new_line_then_n_ident(&mut writer, 1)?;
          writer.write_event(Event::End(BytesEnd::new("delete")))?;
        }
        //endregion

        //region delete list by foreign constraint
        for foreign_constraint in &table_structure.foreign_constraints {
          write_new_line_ident(&mut writer)?;
          let mut delete_tag = BytesStart::new("delete");
          let method_name = format!(
            "delete{}ListBy{}",
            entity_class_name,
            foreign_constraint.base_column.name_to_upper_camel_case()
          );
          delete_tag.push_attribute(("id", method_name.as_str()));
          delete_tag.push_attribute(("parameterType", "Long"));
          writer.write_event(Event::Start(delete_tag))?;
          writer.write_indent()?;
          let delete_sql = format!(
            "DELETE FROM {} WHERE {} = #{{{}}}",
            table_name,
            foreign_constraint.base_column.name,
            foreign_constraint.base_column.to_entity_field_name()
          );
          writer.write_event(Event::Text(BytesText::new(delete_sql.as_str())))?;
          write_new_line_then_n_ident(&mut writer, 1)?;
          writer.write_event(Event::End(BytesEnd::new("delete")))?;
        }
        //endregion
      }
      BusinessOperator::None => {

      }
    }
  }

  // mapper 结束符
  writer.write_event(Event::End(BytesEnd::new("mapper")))?;

  let result = writer.into_inner().into_inner();
  let xml_string = String::from_utf8(result)?;
  Ok(xml_string)
}

fn write_update_trim_if_tag(writer: &mut Writer<Cursor<Vec<u8>>>, column: &DbColumn) -> Result<()> {
  let mut if_tag = BytesStart::new("if");

  let test_expression = get_if_test_value(column);
  if_tag.push_attribute(("test".as_bytes(), test_expression.as_bytes()));
  writer.write_event(Event::Start(if_tag))?;
  let update_expression = format!("{} = #{{{}}},", column.name, column.to_entity_field_name());
  writer.write_event(Event::Text(BytesText::new(update_expression.as_str())))?;
  writer.write_event(Event::End(BytesEnd::new("if")))?;
  Ok(())
}

fn write_insert_field_trim_if_tag(
  writer: &mut Writer<Cursor<Vec<u8>>>,
  column: &DbColumn,
) -> Result<()> {
  let mut if_tag = BytesStart::new("if");
  let test_expression = get_if_test_value(column);
  if_tag.push_attribute(("test".as_bytes(), test_expression.as_bytes()));
  writer.write_event(Event::Start(if_tag))?;
  writer.write_event(Event::Text(BytesText::new(
    format!("{},", column.name).as_str(),
  )))?;
  writer.write_event(Event::End(BytesEnd::new("if")))?;
  Ok(())
}

fn write_insert_value_trim_if_tag(
  writer: &mut Writer<Cursor<Vec<u8>>>,
  column: &DbColumn,
) -> Result<()> {
  let mut if_tag = BytesStart::new("if");
  let test_expression = get_if_test_value(column);
  if_tag.push_attribute(("test".as_bytes(), test_expression.as_bytes()));
  writer.write_event(Event::Start(if_tag))?;
  let value_expression = format!("#{{{}}},", column.to_entity_field_name());
  writer.write_event(Event::Text(BytesText::new(value_expression.as_str())))?;
  writer.write_event(Event::End(BytesEnd::new("if")))?;
  Ok(())
}

fn write_result_tag(writer: &mut Writer<Cursor<Vec<u8>>>, column: &DbColumn) -> Result<()> {
  let tag_name = if column.primary { "id" } else { "result" };
  let mut result_tag = BytesStart::new(tag_name);
  result_tag.push_attribute(("property", column.to_entity_field_name().as_str()));
  result_tag.push_attribute(("column", column.name.as_str()));
  writer.write_event(Event::Empty(result_tag))?;
  Ok(())
}

fn get_if_test_value(column: &DbColumn) -> String {
  let entity_field_name = column.to_entity_field_name();
  let test_expression =
    if column.data_type == DbDataType::Char || column.data_type == DbDataType::Varchar {
      format!(
        "{} != null and {} != ''",
        entity_field_name, entity_field_name
      )
    } else {
      format!("{} != null", entity_field_name)
    };
  test_expression
}

/// 调整 mybatis xml 文件
/// 传入修改前的 `source`，返回修改后的源码
pub fn apply_changes(
  source: &str,
  table: DbTable,
  changes: Vec<DBChange>,
) -> anyhow::Result<String> {
  let mut reader = Reader::from_str(source);
  reader.config_mut().trim_text(false);

  let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 4);

  let mut in_result_map_tag = false;
  let mut in_sql_tag = false;

  //region insert one data
  let mut in_insert_table_tag = false;

  let mut in_insert_fields_trim_tag = false;
  // insert/fields trim/if 标签中 test 属性的值
  let mut in_insert_fields_trim_if_test_value = String::new();

  let mut in_insert_values_trim_tag = false;
  // insert/values trim/if 标签中 test 属性的值
  let mut in_insert_values_trim_if_test_value = String::new();
  //endregion

  //region update one data
  let mut in_update_table_tag = false;
  let mut in_update_trim_tag = false;
  let mut in_update_trim_if_test_value = String::new();
  //endregion

  let mut is_dropping_insert_fields_trim_tag = false;
  let mut is_dropping_insert_values_trim_tag = false;
  let mut is_dropping_update_values_trim_if_tag = false;

  loop {
    match reader.read_event() {
      Ok(Event::Start(e)) => {
        // 设置条件
        if e.name().as_ref() == b"resultMap" {
          in_result_map_tag = true;
        } else if e.name().as_ref() == b"sql" {
          in_sql_tag = true;
        } else if e.name().as_ref() == b"insert" {
          if let Some(attr) = e.try_get_attribute("id")? {
            let attr_value = attr.unescape_value()?;
            let expected_value = format!("insert{}", table.to_entity_class_name());
            if attr_value == expected_value {
              in_insert_table_tag = true;
            }
          }
        } else if e.name().as_ref() == b"update" {
          if let Some(attr) = e.try_get_attribute("id")? {
            let attr_value = attr.unescape_value()?;
            let expected_value = format!("update{}", table.to_entity_class_name());
            if attr_value == expected_value {
              in_update_table_tag = true;
            }
          }
        } else if e.name().as_ref() == b"trim" {
          if in_insert_table_tag {
            if let Some(attr) = e.try_get_attribute("prefix")? {
              let attr_value = attr.unescape_value()?;
              if attr_value == "(" {
                in_insert_fields_trim_tag = true;
              } else if attr_value == "VALUES (" {
                in_insert_values_trim_tag = true;
              }
            }
          } else if in_update_table_tag {
            if let Some(attr) = e.try_get_attribute("prefix")? {
              let attr_value = attr.unescape_value()?;
              if attr_value == "SET" {
                in_update_trim_tag = true;
              }
            }
          }
        } else if e.name().as_ref() == b"if" {
          if let Some(attr) = e.try_get_attribute("test")? {
            let attr_value = attr.unescape_value()?;
            if in_insert_fields_trim_tag {
              in_insert_fields_trim_if_test_value = attr_value.to_string();
            } else if in_insert_values_trim_tag {
              in_insert_values_trim_if_test_value = attr_value.to_string();
            } else if in_update_trim_tag {
              in_update_trim_if_test_value = attr_value.to_string();
            }
          }
        }

        if in_insert_fields_trim_tag && !in_insert_fields_trim_if_test_value.is_empty() {
          is_dropping_insert_fields_trim_tag =
            if let Some(first_part) = in_insert_fields_trim_if_test_value.trim().split(' ').next() {
              changes.iter().any(|change| {
                if let DBChange::DropColumn(columns) = change {
                  columns
                    .iter()
                    .any(|column| column.to_entity_field_name() == first_part.trim())
                } else {
                  false
                }
              })
            } else {
              false
            }
        }

        if in_insert_values_trim_tag && !in_insert_values_trim_if_test_value.is_empty() {
          is_dropping_insert_values_trim_tag =
            if let Some(first_part) = in_insert_values_trim_if_test_value.trim().split(' ').next() {
              changes.iter().any(|change| {
                if let DBChange::DropColumn(columns) = change {
                  columns
                    .iter()
                    .any(|column| column.to_entity_field_name() == first_part.trim())
                } else {
                  false
                }
              })
            } else {
              false
            }
        }

        if in_update_trim_tag && !in_update_trim_if_test_value.is_empty() {
          is_dropping_update_values_trim_if_tag =
            if let Some(first_part) = in_update_trim_if_test_value.trim().split(' ').next() {
              changes.iter().any(|change| {
                if let DBChange::DropColumn(columns) = change {
                  columns
                    .iter()
                    .any(|column| column.to_entity_field_name() == first_part.trim())
                } else {
                  false
                }
              })
            } else {
              false
            }
        }

        if is_dropping_insert_fields_trim_tag
          || is_dropping_insert_values_trim_tag
          || is_dropping_update_values_trim_if_tag
        {
          // 不做任何操作，就相当于删除
        } else {
          writer.write_event(Event::Start(e.borrow()))?;
        }
      }
      Ok(Event::Empty(e)) => {
        // 应用变更
        if in_result_map_tag {
          if e.name().as_ref() == b"id" || e.name().as_ref() == b"result" {
            if let Some(column_attr) = e.try_get_attribute("column")? {
              let column_name = column_attr.unescape_value()?;
              for change in &changes {
                match change {
                  DBChange::DropUniqueConstraint(_) => {}
                  DBChange::AddColumn(columns) => {
                    writer.write_event(Event::Empty(e.borrow()))?;
                    for column in columns {
                      if let Some(after_column_name) = &column.after_column_name {
                        if after_column_name.as_str() == column_name {
                          // 新增 result 节点
                          write_result_tag(&mut writer, column)?;
                        }
                      }
                    }
                  }
                  DBChange::DropColumn(columns) => {
                    for column in columns {
                      if column.name == column_name {
                        // 忽略此节点
                      } else {
                        writer.write_event(Event::Empty(e.borrow()))?;
                      }
                    }
                  }
                  DBChange::RenameColumn(_) => {}
                  DBChange::AddUniqueConstraint(_) => {}
                }
              }
            }
          }
        }
      }
      Ok(Event::End(e)) => {
        if is_dropping_insert_fields_trim_tag
          || is_dropping_insert_values_trim_tag
          || is_dropping_update_values_trim_if_tag
        {
          // 相当于删除代码
        } else {
          writer.write_event(Event::End(e.borrow()))?;
        }
        // 应用变更
        if in_insert_fields_trim_tag {
          if e.name().as_ref() == b"if" {
            for change in &changes {
              match change {
                DBChange::DropUniqueConstraint(_) => {}
                DBChange::AddColumn(columns) => {
                  for column in columns {
                    if let Some(after_column_name) = &column.after_column_name {
                      if in_insert_fields_trim_if_test_value
                        .starts_with(to_entity_field_name(after_column_name).as_str())
                      {
                        write_insert_field_trim_if_tag(&mut writer, column)?;
                      }
                    }
                  }
                }
                DBChange::DropColumn(_) => {}
                DBChange::RenameColumn(_) => {}
                DBChange::AddUniqueConstraint(_) => {}
              }
            }
          }
        } else if in_insert_values_trim_tag {
          if e.name().as_ref() == b"if" {
            for change in &changes {
              match change {
                DBChange::DropUniqueConstraint(_) => {}
                DBChange::AddColumn(columns) => {
                  for column in columns {
                    if let Some(after_column_name) = &column.after_column_name {
                      if in_insert_values_trim_if_test_value
                        .starts_with(to_entity_field_name(after_column_name).as_str())
                      {
                        write_insert_value_trim_if_tag(&mut writer, column)?;
                      }
                    }
                  }
                }
                DBChange::DropColumn(_) => {}
                DBChange::RenameColumn(_) => {}
                DBChange::AddUniqueConstraint(_) => {}
              }
            }
          }
        }

        // 重置条件
        if in_result_map_tag && e.name().as_ref() == b"resultMap" {
          in_result_map_tag = false;
        } else if in_sql_tag && e.name().as_ref() == b"sql" {
          in_sql_tag = false;
        } else if e.name().as_ref() == b"trim" {
          if in_insert_table_tag {
            if in_insert_fields_trim_tag {
              in_insert_fields_trim_tag = false;
              in_insert_fields_trim_if_test_value = String::new();
            } else if in_insert_values_trim_tag {
              in_insert_values_trim_tag = false;
              in_insert_values_trim_if_test_value = String::new();
            }
          } else if in_update_table_tag {
            if in_update_trim_tag {
              in_update_trim_tag = false;
              in_update_trim_if_test_value = String::new();
            }
          }
        } else if in_insert_table_tag && e.name().as_ref() == b"insert" {
          in_insert_table_tag = false;
        } else if in_update_table_tag && e.name().as_ref() == b"update" {
          in_update_table_tag = false;
        } else if e.name().as_ref() == b"if" {
          if is_dropping_insert_fields_trim_tag {
            is_dropping_insert_fields_trim_tag = false;
          } else if is_dropping_insert_values_trim_tag {
            is_dropping_insert_values_trim_tag = false;
          } else if is_dropping_update_values_trim_if_tag {
            is_dropping_update_values_trim_if_tag = false;
          }
        }
      }
      Ok(Event::Text(e)) => {
        if in_result_map_tag {
          for change in &changes {
            match change {
              DBChange::DropUniqueConstraint(_) => {}
              DBChange::AddColumn(columns) => {}
              DBChange::DropColumn(columns) => {
                // 不执行任何写入，就相当于删除了空白字符
              }
              DBChange::RenameColumn(_) => {}
              DBChange::AddUniqueConstraint(_) => {}
            }
          }
        } else if in_sql_tag {
          let old_sql = e.decode()?;

          for change in &changes {
            match change {
              DBChange::DropUniqueConstraint(_) => {}
              DBChange::AddColumn(columns) => {
                let new_sql =
                  insert_columns_for_select_sql(old_sql.to_string().as_str(), columns, 2)?;
                let formated_sql = format!(
                  "{}{}{}",
                  get_start_whitespace(old_sql.to_string().as_str()),
                  new_sql.trim_start(),
                  get_end_whitespace(old_sql.to_string().as_str())
                );
                writer.write_event(Event::Text(BytesText::new(formated_sql.as_str())))?;
              }
              DBChange::DropColumn(columns) => {
                let new_sql =
                  drop_columns_for_select_sql(old_sql.to_string().as_str(), columns, 2)?;
                let formated_sql = format!(
                  "{}{}{}",
                  get_start_whitespace(old_sql.to_string().as_str()),
                  new_sql.trim_start(),
                  get_end_whitespace(old_sql.to_string().as_str())
                );
                writer.write_event(Event::Text(BytesText::new(formated_sql.as_str())))?;
              }
              DBChange::RenameColumn(_) => {}
              DBChange::AddUniqueConstraint(_) => {}
            }
          }
        } else if in_insert_table_tag {
          if !in_insert_fields_trim_tag && !in_insert_values_trim_tag {
            writer.write_event(Event::Text(e))?;
          } else if in_insert_fields_trim_tag {
            if is_dropping_insert_fields_trim_tag {
              // do nothing
            } else {
              let is_blank = e.decode()?.trim().is_empty();
              if !is_blank {
                writer.write_event(Event::Text(e))?;
              }
            }
          } else if in_insert_values_trim_tag {
            if is_dropping_insert_values_trim_tag {
              // do nothing
            } else {
              let is_blank = e.decode()?.trim().is_empty();
              if !is_blank {
                writer.write_event(Event::Text(e))?;
              }
            }
          }
        } else if in_update_trim_tag {
          let old_text = e.decode()?;

          println!("aa{}aa", old_text);

          // if is_dropping_update_values_trim_if_tag {
          //     // do nothing
          // } else {
          //     let is_blank = old_text.trim().is_empty();
          //     if !is_blank {
          //         writer.write_event(Event::Text(e))?;
          //     }
          // }

          for change in &changes {
            match change {
              DBChange::DropUniqueConstraint(_) => {}
              DBChange::AddColumn(columns) => {
                for column in columns {
                  let insert_index_option = old_text.lines().position(|line| {
                    if let Some(first_part) = line.trim().split('=').next() {
                      if let Some(after_column_name) = &column.after_column_name {
                        first_part.trim() == after_column_name
                      } else {
                        false
                      }
                    } else {
                      false
                    }
                  });

                  if let Some(insert_index) = insert_index_option {
                    // 注意，lines 中的第一行为0个空字符串，最后一个行是缩进的空字符串
                    let mut lines = old_text.lines().collect::<Vec<&str>>();
                    if column.nullable {
                      let insert_content = format!(
                        "{}{} = #{{{}}},",
                        get_start_whitespace(lines.get(insert_index).unwrap()),
                        column.name,
                        column.to_entity_field_name()
                      );
                      lines.insert(insert_index + 1, insert_content.as_str());
                      let new_text = lines.join("\n");
                      writer.write_event(Event::Text(BytesText::new(new_text.as_str())))?;
                    } else {
                      // 将文本拆为两段，在中间插入节点
                      let split_index = insert_index + 1;
                      let before_lines = &lines[..split_index];
                      let after_lines = &lines[split_index..];

                      if !before_lines.is_empty() {
                        writer.write_event(Event::Text(BytesText::new(
                          before_lines.join("\n").as_str(),
                        )))?;
                      }
                      writer.write_indent()?;
                      write_update_trim_if_tag(&mut writer, column)?;
                      write_one_new_line(&mut writer)?;
                      if !after_lines.is_empty() {
                        writer.write_event(Event::Text(BytesText::new(
                          after_lines.join("\n").as_str(),
                        )))?;
                      }
                    }
                  }
                }
              }
              DBChange::DropColumn(columns) => {
                // 循环要删除的列，如果列名相同，则删除（即不再写入）；如果列名不同，则保持原状
                // 文本是一整块解析的，需要覆盖原来的内容
                // 只有存在时才删除
                let cloned_old_text = old_text.clone();
                if is_dropping_update_values_trim_if_tag {
                  let dropped_column_names: Vec<&str> =
                    columns.iter().map(|item| &item.name[..]).collect();

                  let new_text = old_text
                    .trim_end()
                    .lines()
                    .filter(|item| {
                      if let Some(first_part) = item.split('=').next() {
                        !dropped_column_names.contains(&first_part.trim())
                      } else {
                        true
                      }
                    })
                    .collect::<Vec<&str>>()
                    .join("\n");
                  println!("new'{}'new", new_text);
                  if !new_text.is_empty() {
                    writer.write_event(Event::Text(BytesText::new(new_text.as_str())))?;
                  }
                  println!("猜我打印了几次");
                  write_new_line_then_n_ident(&mut writer, 2)?;
                } else {
                  // 只处理被删除行的上一行
                  writer.write_event(Event::Text(BytesText::new(
                    cloned_old_text.to_string().trim_end(),
                  )))?;
                }
              }
              DBChange::RenameColumn(_) => {}
              DBChange::AddUniqueConstraint(_) => {}
            }
          }
        } else {
          writer.write_event(Event::Text(e))?;
        }
      }
      Ok(Event::Eof) => break,
      Ok(event) => writer.write_event(event)?,
      Err(e) => return Err(e.into()),
    }
  }

  let result = writer.into_inner().into_inner();
  let xml_string = String::from_utf8(result)?;
  Ok(xml_string)
}

fn get_start_whitespace(input: &str) -> &str {
  let trimmed = input.trim_start();
  &input[0..(input.len() - trimmed.len())]
}

fn get_end_whitespace(input: &str) -> &str {
  &input[input.trim_end().len()..]
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db_table_info::{
    DbColumn, DbCompare, DbDataType, DbTable, ForeignConstraint, QueryCondition, UniqueConstraint,
  };

  //region generate
  #[test]
  fn generate_no_columns() {
    let table_structure = DbTableStructure {
      table: DbTable {
        name: "table_1".to_string(),
        comment: "表1".to_string(),
      },
      columns: vec![],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };
    let business_operators = vec![];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
    </resultMap>

    <sql id="selectTable1Vo">
    </sql>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn generate_two_columns() {
    let column1 = DbColumn {
      name: "string_column1".to_string(),
      comment: "字符串1".to_string(),
      data_type: DbDataType::Varchar,
      max_length: Some(32),
      scale: None,
      unsigned: None,
      primary: false,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };
    let column2 = DbColumn {
      name: "int_column1".to_string(),
      comment: "数字".to_string(),
      data_type: DbDataType::Int,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: false,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };

    let table_structure = DbTableStructure {
      table: DbTable {
        name: "table_1".to_string(),
        comment: "表1".to_string(),
      },
      columns: vec![column1, column2],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };
    let business_operators = vec![];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <result property="stringColumn1" column="string_column1"/>
        <result property="intColumn1" column="int_column1"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            string_column1,
            int_column1
        FROM
            table_1
    </sql>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn generate_one_dbid_column() {
    let column1 = DbColumn {
      name: "dbid".to_string(),
      comment: "主键".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: true,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };

    let table_structure = DbTableStructure {
      table: DbTable {
        name: "table_1".to_string(),
        comment: "表1".to_string(),
      },
      columns: vec![column1],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };
    let business_operators = vec![];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <id property="dbid" column="dbid"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            dbid
        FROM
            table_1
    </sql>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn generate_one_string_column_query_list_equal() {
    let column1 = DbColumn {
      name: "string_column1".to_string(),
      comment: "字符串1".to_string(),
      data_type: DbDataType::Varchar,
      max_length: Some(32),
      scale: None,
      unsigned: None,
      primary: false,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };

    let table_structure = DbTableStructure {
      table: DbTable {
        name: "table_1".to_string(),
        comment: "表1".to_string(),
      },
      columns: vec![column1.clone()],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let query_list = BusinessOperator::QueryList {
      conditions: vec![QueryCondition {
        column: column1,
        compare: DbCompare::Equal,
      }],
    };
    let business_operators = vec![query_list];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <result property="stringColumn1" column="string_column1"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            string_column1
        FROM
            table_1
    </sql>

    <select id="selectTable1List" parameterType="Table1" resultMap="Table1Result">
        <include refid="selectTable1Vo"/>
        <where>
            <if test="stringColumn1 != null and stringColumn1 != ''">AND string_column1 = #{stringColumn1}</if>
        </where>
        ORDER BY dbid DESC
    </select>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn generate_one_number_column_query_list_equal() {
    let column1 = DbColumn {
      name: "number_column1".to_string(),
      comment: "数值1".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: false,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };

    let table_structure = DbTableStructure {
      table: DbTable {
        name: "table_1".to_string(),
        comment: "表1".to_string(),
      },
      columns: vec![column1.clone()],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let query_list = BusinessOperator::QueryList {
      conditions: vec![QueryCondition {
        column: column1,
        compare: DbCompare::Equal,
      }],
    };
    let business_operators = vec![query_list];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <result property="numberColumn1" column="number_column1"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            number_column1
        FROM
            table_1
    </sql>

    <select id="selectTable1List" parameterType="Table1" resultMap="Table1Result">
        <include refid="selectTable1Vo"/>
        <where>
            <if test="numberColumn1 != null">AND number_column1 = #{numberColumn1}</if>
        </where>
        ORDER BY dbid DESC
    </select>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn generate_one_string_column_query_list_not_equal() {
    let column1 = DbColumn {
      name: "string_column1".to_string(),
      comment: "字符串1".to_string(),
      data_type: DbDataType::Varchar,
      max_length: Some(32),
      scale: None,
      unsigned: None,
      primary: false,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };

    let table_structure = DbTableStructure {
      table: DbTable {
        name: "table_1".to_string(),
        comment: "表1".to_string(),
      },
      columns: vec![column1.clone()],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let query_list = BusinessOperator::QueryList {
      conditions: vec![QueryCondition {
        column: column1,
        compare: DbCompare::NotEqual,
      }],
    };
    let business_operators = vec![query_list];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <result property="stringColumn1" column="string_column1"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            string_column1
        FROM
            table_1
    </sql>

    <select id="selectTable1List" parameterType="Table1" resultMap="Table1Result">
        <include refid="selectTable1Vo"/>
        <where>
            <if test="stringColumn1 != null and stringColumn1 != ''">AND string_column1 != #{stringColumn1}</if>
        </where>
        ORDER BY dbid DESC
    </select>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn generate_one_string_column_query_list_like() {
    let column1 = DbColumn {
      name: "string_column1".to_string(),
      comment: "字符串1".to_string(),
      data_type: DbDataType::Varchar,
      max_length: Some(32),
      scale: None,
      unsigned: None,
      primary: false,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };

    let table_structure = DbTableStructure {
      table: DbTable {
        name: "table_1".to_string(),
        comment: "表1".to_string(),
      },
      columns: vec![column1.clone()],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let query_list = BusinessOperator::QueryList {
      conditions: vec![QueryCondition {
        column: column1,
        compare: DbCompare::Like,
      }],
    };
    let business_operators = vec![query_list];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <result property="stringColumn1" column="string_column1"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            string_column1
        FROM
            table_1
    </sql>

    <select id="selectTable1List" parameterType="Table1" resultMap="Table1Result">
        <include refid="selectTable1Vo"/>
        <where>
            <if test="stringColumn1 != null and stringColumn1 != ''">AND string_column1 LIKE concat('%', #{stringColumn1}, '%')</if>
        </where>
        ORDER BY dbid DESC
    </select>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn generate_one_date_column_query_list_between() {
    let column1 = DbColumn {
      name: "date_column1".to_string(),
      comment: "日期1".to_string(),
      data_type: DbDataType::Date,
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
    };

    let table_structure = DbTableStructure {
      table: DbTable {
        name: "table_1".to_string(),
        comment: "表1".to_string(),
      },
      columns: vec![column1.clone()],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let query_list = BusinessOperator::QueryList {
      conditions: vec![QueryCondition {
        column: column1,
        compare: DbCompare::Between,
      }],
    };
    let business_operators = vec![query_list];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <result property="dateColumn1" column="date_column1"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            date_column1
        FROM
            table_1
    </sql>

    <select id="selectTable1List" parameterType="Table1" resultMap="Table1Result">
        <include refid="selectTable1Vo"/>
        <where>
            <if test="params.beginDateColumn1 != null and params.endDateColumn1 != null">AND (date_column1 >= #{params.beginDateColumn1} AND date_column1 < #{params.endDateColumn1} + INTERVAL 1 DAY)</if>
        </where>
        ORDER BY dbid DESC
    </select>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn generate_one_string_column_query_list_in_like() {
    let column1 = DbColumn {
      name: "string_column1".to_string(),
      comment: "字符串1".to_string(),
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
    };

    let table_structure = DbTableStructure {
      table: DbTable {
        name: "table_1".to_string(),
        comment: "表1".to_string(),
      },
      columns: vec![column1.clone()],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let query_list = BusinessOperator::QueryList {
      conditions: vec![QueryCondition {
        column: column1,
        compare: DbCompare::InThenItemLike,
      }],
    };
    let business_operators = vec![query_list];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <result property="stringColumn1" column="string_column1"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            string_column1
        FROM
            table_1
    </sql>

    <select id="selectTable1List" parameterType="Table1" resultMap="Table1Result">
        <include refid="selectTable1Vo"/>
        <where>
            <if test="stringColumn1List != null and stringColumn1List.length > 0">
                AND
                <foreach collection="stringColumn1List" item="stringColumn1" open="(" separator="OR" close=")">
                    string_column1 LIKE concat('%', #{stringColumn1}, '%')
                </foreach>
            </if>
        </where>
        ORDER BY dbid DESC
    </select>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn generate_one_string_column_query_list_in_equal() {
    let column1 = DbColumn {
      name: "number_column1".to_string(),
      comment: "数值1".to_string(),
      data_type: DbDataType::Varchar,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: false,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };

    let table_structure = DbTableStructure {
      table: DbTable {
        name: "table_1".to_string(),
        comment: "表1".to_string(),
      },
      columns: vec![column1.clone()],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let query_list = BusinessOperator::QueryList {
      conditions: vec![QueryCondition {
        column: column1,
        compare: DbCompare::InThenItemEqual,
      }],
    };
    let business_operators = vec![query_list];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <result property="numberColumn1" column="number_column1"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            number_column1
        FROM
            table_1
    </sql>

    <select id="selectTable1List" parameterType="Table1" resultMap="Table1Result">
        <include refid="selectTable1Vo"/>
        <where>
            <if test="numberColumn1List != null and numberColumn1List.length > 0">
                AND number_column1 IN
                <foreach collection="numberColumn1List" item="numberColumn1" open="(" separator="," close=")">
                    #{numberColumn1}
                </foreach>
            </if>
        </where>
        ORDER BY dbid DESC
    </select>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn generate_one_number_column_query_list_by_foreign_constraint() {
    let table1 = DbTable {
      name: "table_1".to_string(),
      comment: "表1".to_string(),
    };
    let column1 = DbColumn {
      name: "dbid".to_string(),
      comment: "主键".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: true,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };

    let table2 = DbTable {
      name: "table_2".to_string(),
      comment: "表2".to_string(),
    };
    let column2 = DbColumn {
      name: "dbid".to_string(),
      comment: "主键".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: true,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };
    let column3 = DbColumn {
      name: "number_column2".to_string(),
      comment: "数值2".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: false,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };

    let foreign_constraint = ForeignConstraint {
      name: "fk_table_2_number_column1".to_string(),
      base_table: table2.clone(),
      base_column: column3.clone(),
      ref_table: table1.clone(),
      ref_column: column1,
    };

    let table_structure = DbTableStructure {
      table: table2,
      columns: vec![column2, column3],
      unique_constraints: vec![],
      foreign_constraints: vec![foreign_constraint],
    };

    let query_list = BusinessOperator::QueryList { conditions: vec![] };
    let business_operators = vec![query_list];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table2Mapper">
    <resultMap id="Table2Result" type="Table2">
        <id property="dbid" column="dbid"/>
        <result property="numberColumn2" column="number_column2"/>
    </resultMap>

    <sql id="selectTable2Vo">
        SELECT
            dbid,
            number_column2
        FROM
            table_2
    </sql>

    <select id="selectTable2List" parameterType="Table2" resultMap="Table2Result">
        <include refid="selectTable2Vo"/>
        ORDER BY dbid DESC
    </select>

    <select id="selectSimpleTable2ListByNumberColumn2" parameterType="Long" resultMap="Table2Result">
        <include refid="selectTable2Vo"/>
        WHERE number_column2 = #{numberColumn2}
    </select>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn generate_one_string_column_query_one_by_id() {
    let column1 = DbColumn {
      name: "dbid".to_string(),
      comment: "主键".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: true,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };

    let table_structure = DbTableStructure {
      table: DbTable {
        name: "table_1".to_string(),
        comment: "表1".to_string(),
      },
      columns: vec![column1.clone()],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let query_one = BusinessOperator::QueryOne;
    let business_operators = vec![query_one];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <id property="dbid" column="dbid"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            dbid
        FROM
            table_1
    </sql>

    <select id="selectSimpleTable1ById" parameterType="Long" resultMap="Table1Result">
        <include refid="selectTable1Vo"/>
        WHERE dbid = #{dbid}
    </select>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  /// 表上有两个唯一约束，一个关联一个字段，一个关联两个字段
  #[test]
  fn generate_two_string_column_query_one_by_unique_constraint() {
    let column1 = DbColumn {
      name: "dbid".to_string(),
      comment: "主键".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: true,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };
    let column2 = DbColumn {
      name: "string_column1".to_string(),
      comment: "字符串1".to_string(),
      data_type: DbDataType::Varchar,
      max_length: Some(32),
      scale: None,
      unsigned: None,
      primary: false,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };
    let column3 = DbColumn {
      name: "string_column2".to_string(),
      comment: "字符串2".to_string(),
      data_type: DbDataType::Varchar,
      max_length: Some(32),
      scale: None,
      unsigned: None,
      primary: false,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };
    let column4 = DbColumn {
      name: "string_column3".to_string(),
      comment: "字符串3".to_string(),
      data_type: DbDataType::Varchar,
      max_length: Some(32),
      scale: None,
      unsigned: None,
      primary: false,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };

    let unique_constraint1 = UniqueConstraint {
      name: "uk_table_1_string_column1".to_string(),
      columns: vec![column2.clone()],
    };
    let unique_constraint2 = UniqueConstraint {
      name: "uk_table_1_string_column3_string_column4".to_string(),
      columns: vec![column3.clone(), column4.clone()],
    };

    let table_structure = DbTableStructure {
      table: DbTable {
        name: "table_1".to_string(),
        comment: "表1".to_string(),
      },
      columns: vec![
        column1.clone(),
        column2.clone(),
        column3.clone(),
        column4.clone(),
      ],
      unique_constraints: vec![unique_constraint1, unique_constraint2],
      foreign_constraints: vec![],
    };

    let query_one = BusinessOperator::QueryOne;
    let business_operators = vec![query_one];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <id property="dbid" column="dbid"/>
        <result property="stringColumn1" column="string_column1"/>
        <result property="stringColumn2" column="string_column2"/>
        <result property="stringColumn3" column="string_column3"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            dbid,
            string_column1,
            string_column2,
            string_column3
        FROM
            table_1
    </sql>

    <select id="selectSimpleTable1ById" parameterType="Long" resultMap="Table1Result">
        <include refid="selectTable1Vo"/>
        WHERE dbid = #{dbid}
    </select>

    <select id="selectSimpleTable1ByStringColumn1" parameterType="String" resultMap="Table1Result">
        <include refid="selectTable1Vo"/>
        WHERE string_column1 = #{stringColumn1}
    </select>

    <select id="selectSimpleTable1ByStringColumn2AndStringColumn3" resultMap="Table1Result">
        <include refid="selectTable1Vo"/>
        WHERE string_column2 = #{stringColumn2} AND string_column3 = #{stringColumn3}
    </select>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn generate_one_string_column_create_one_by_id() {
    let column1 = DbColumn {
      name: "dbid".to_string(),
      comment: "主键".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: true,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };
    let column2 = DbColumn {
      name: "string_column1".to_string(),
      comment: "字符串1".to_string(),
      data_type: DbDataType::Varchar,
      max_length: Some(32),
      scale: None,
      unsigned: None,
      primary: false,
      nullable: true,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };

    let table_structure = DbTableStructure {
      table: DbTable {
        name: "table_1".to_string(),
        comment: "表1".to_string(),
      },
      columns: vec![column1.clone(), column2.clone()],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let create_one = BusinessOperator::CreateOne;
    let business_operators = vec![create_one];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <id property="dbid" column="dbid"/>
        <result property="stringColumn1" column="string_column1"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            dbid,
            string_column1
        FROM
            table_1
    </sql>

    <insert id="insertTable1" parameterType="Table1" useGeneratedKeys="true" keyProperty="dbid">
        INSERT INTO table_1
        <trim prefix="(" suffix=")" suffixOverrides=",">
            <if test="stringColumn1 != null and stringColumn1 != ''">string_column1,</if>
        </trim>
        <trim prefix="VALUES (" suffix=")" suffixOverrides=",">
            <if test="stringColumn1 != null and stringColumn1 != ''">#{stringColumn1},</if>
        </trim>
    </insert>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn generate_one_string_column_nullable_update_one_by_id() {
    let column1 = DbColumn {
      name: "dbid".to_string(),
      comment: "主键".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: true,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };
    let column2 = DbColumn {
      name: "string_column1".to_string(),
      comment: "字符串1".to_string(),
      data_type: DbDataType::Varchar,
      max_length: Some(32),
      scale: None,
      unsigned: None,
      primary: false,
      nullable: true,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };

    let table_structure = DbTableStructure {
      table: DbTable {
        name: "table_1".to_string(),
        comment: "表1".to_string(),
      },
      columns: vec![column1.clone(), column2.clone()],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let update_one = BusinessOperator::UpdateOne;
    let business_operators = vec![update_one];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <id property="dbid" column="dbid"/>
        <result property="stringColumn1" column="string_column1"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            dbid,
            string_column1
        FROM
            table_1
    </sql>

    <update id="updateTable1" parameterType="Table1">
        UPDATE table_1
        <trim prefix="SET" suffixOverrides=",">
            string_column1 = #{stringColumn1},
        </trim>
        WHERE dbid = #{dbid}
    </update>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn generate_one_string_column_not_null_update_one_by_id() {
    let column1 = DbColumn {
      name: "dbid".to_string(),
      comment: "主键".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: true,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };
    let column2 = DbColumn {
      name: "string_column1".to_string(),
      comment: "字符串1".to_string(),
      data_type: DbDataType::Varchar,
      max_length: Some(32),
      scale: None,
      unsigned: None,
      primary: false,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };

    let table_structure = DbTableStructure {
      table: DbTable {
        name: "table_1".to_string(),
        comment: "表1".to_string(),
      },
      columns: vec![column1.clone(), column2.clone()],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let update_one = BusinessOperator::UpdateOne;
    let business_operators = vec![update_one];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <id property="dbid" column="dbid"/>
        <result property="stringColumn1" column="string_column1"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            dbid,
            string_column1
        FROM
            table_1
    </sql>

    <update id="updateTable1" parameterType="Table1">
        UPDATE table_1
        <trim prefix="SET" suffixOverrides=",">
            <if test="stringColumn1 != null and stringColumn1 != ''">string_column1 = #{stringColumn1},</if>
        </trim>
        WHERE dbid = #{dbid}
    </update>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn generate_one_number_column_nullable_update_one_by_id() {
    let column1 = DbColumn {
      name: "dbid".to_string(),
      comment: "主键".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: true,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };
    let column2 = DbColumn {
      name: "number_column1".to_string(),
      comment: "数值1".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: false,
      nullable: true,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };

    let table_structure = DbTableStructure {
      table: DbTable {
        name: "table_1".to_string(),
        comment: "表1".to_string(),
      },
      columns: vec![column1.clone(), column2.clone()],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let update_one = BusinessOperator::UpdateOne;
    let business_operators = vec![update_one];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <id property="dbid" column="dbid"/>
        <result property="numberColumn1" column="number_column1"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            dbid,
            number_column1
        FROM
            table_1
    </sql>

    <update id="updateTable1" parameterType="Table1">
        UPDATE table_1
        <trim prefix="SET" suffixOverrides=",">
            number_column1 = #{numberColumn1},
        </trim>
        WHERE dbid = #{dbid}
    </update>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn generate_one_number_column_not_null_update_one_by_id() {
    let column1 = DbColumn {
      name: "dbid".to_string(),
      comment: "主键".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: true,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };
    let column2 = DbColumn {
      name: "number_column1".to_string(),
      comment: "数值1".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: false,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };

    let table_structure = DbTableStructure {
      table: DbTable {
        name: "table_1".to_string(),
        comment: "表1".to_string(),
      },
      columns: vec![column1.clone(), column2.clone()],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let update_one = BusinessOperator::UpdateOne;
    let business_operators = vec![update_one];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <id property="dbid" column="dbid"/>
        <result property="numberColumn1" column="number_column1"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            dbid,
            number_column1
        FROM
            table_1
    </sql>

    <update id="updateTable1" parameterType="Table1">
        UPDATE table_1
        <trim prefix="SET" suffixOverrides=",">
            <if test="numberColumn1 != null">number_column1 = #{numberColumn1},</if>
        </trim>
        WHERE dbid = #{dbid}
    </update>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  /// 虽然 update_by 和 update_time 两个字段可空，但是也要加上 if 判断，以防清空已存的值
  #[test]
  fn generate_update_by_update_time_column_update_one_by_id() {
    let column1 = DbColumn {
      name: "dbid".to_string(),
      comment: "主键".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: true,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };
    let column2 = DbColumn {
      name: "update_by".to_string(),
      comment: "修改人".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: false,
      nullable: true,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };
    let column3 = DbColumn {
      name: "update_time".to_string(),
      comment: "修改时间".to_string(),
      data_type: DbDataType::DateTime,
      max_length: None,
      scale: None,
      unsigned: None,
      primary: false,
      nullable: true,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };

    let table_structure = DbTableStructure {
      table: DbTable {
        name: "table_1".to_string(),
        comment: "表1".to_string(),
      },
      columns: vec![column1, column2, column3],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let update_one = BusinessOperator::UpdateOne;
    let business_operators = vec![update_one];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <id property="dbid" column="dbid"/>
        <result property="updateBy" column="update_by"/>
        <result property="updateTime" column="update_time"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            dbid,
            update_by,
            update_time
        FROM
            table_1
    </sql>

    <update id="updateTable1" parameterType="Table1">
        UPDATE table_1
        <trim prefix="SET" suffixOverrides=",">
            <if test="updateBy != null">update_by = #{updateBy},</if>
            <if test="updateTime != null">update_time = #{updateTime},</if>
        </trim>
        WHERE dbid = #{dbid}
    </update>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn generate_two_column_nullable_update_one_by_id() {
    let column1 = DbColumn {
      name: "dbid".to_string(),
      comment: "主键".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: true,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };
    let column2 = DbColumn {
      name: "number_column1".to_string(),
      comment: "数值列1".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: false,
      nullable: true,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };
    let column3 = DbColumn {
      name: "number_column2".to_string(),
      comment: "数值列".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: false,
      nullable: true,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };

    let table_structure = DbTableStructure {
      table: DbTable {
        name: "table_1".to_string(),
        comment: "表1".to_string(),
      },
      columns: vec![column1, column2, column3],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let update_one = BusinessOperator::UpdateOne;
    let business_operators = vec![update_one];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <id property="dbid" column="dbid"/>
        <result property="numberColumn1" column="number_column1"/>
        <result property="numberColumn2" column="number_column2"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            dbid,
            number_column1,
            number_column2
        FROM
            table_1
    </sql>

    <update id="updateTable1" parameterType="Table1">
        UPDATE table_1
        <trim prefix="SET" suffixOverrides=",">
            number_column1 = #{numberColumn1},
            number_column2 = #{numberColumn2},
        </trim>
        WHERE dbid = #{dbid}
    </update>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  /// 有两列，第一列为 not null，第二列为 nullable
  #[test]
  fn generate_two_column_not_null_nullable_update_one_by_id() {
    let column1 = DbColumn {
      name: "dbid".to_string(),
      comment: "主键".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: true,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };
    let column2 = DbColumn {
      name: "number_column1".to_string(),
      comment: "数值列1".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: false,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };
    let column3 = DbColumn {
      name: "number_column2".to_string(),
      comment: "数值列".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: false,
      nullable: true,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };

    let table_structure = DbTableStructure {
      table: DbTable {
        name: "table_1".to_string(),
        comment: "表1".to_string(),
      },
      columns: vec![column1, column2, column3],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let update_one = BusinessOperator::UpdateOne;
    let business_operators = vec![update_one];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <id property="dbid" column="dbid"/>
        <result property="numberColumn1" column="number_column1"/>
        <result property="numberColumn2" column="number_column2"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            dbid,
            number_column1,
            number_column2
        FROM
            table_1
    </sql>

    <update id="updateTable1" parameterType="Table1">
        UPDATE table_1
        <trim prefix="SET" suffixOverrides=",">
            <if test="numberColumn1 != null">number_column1 = #{numberColumn1},</if>
            number_column2 = #{numberColumn2},
        </trim>
        WHERE dbid = #{dbid}
    </update>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn generate_one_number_column_delete_one_by_id() {
    let column1 = DbColumn {
      name: "dbid".to_string(),
      comment: "主键".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: true,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };
    let column2 = DbColumn {
      name: "number_column1".to_string(),
      comment: "数值1".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: false,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };

    let table_structure = DbTableStructure {
      table: DbTable {
        name: "table_1".to_string(),
        comment: "表1".to_string(),
      },
      columns: vec![column1.clone(), column2.clone()],
      unique_constraints: vec![],
      foreign_constraints: vec![],
    };

    let delete_one = BusinessOperator::DeleteOne;
    let business_operators = vec![delete_one];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <id property="dbid" column="dbid"/>
        <result property="numberColumn1" column="number_column1"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            dbid,
            number_column1
        FROM
            table_1
    </sql>

    <delete id="deleteTable1ById" parameterType="Long">
        DELETE FROM table_1 WHERE dbid = #{dbid}
    </delete>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  /// 归属于 DeleteOne
  /// 表上有两个唯一约束，一个关联一个字段，一个关联两个字段
  #[test]
  fn generate_two_string_column_delete_one_by_unique_constraint() {
    let column1 = DbColumn {
      name: "dbid".to_string(),
      comment: "主键".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: true,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };
    let column2 = DbColumn {
      name: "string_column1".to_string(),
      comment: "字符串1".to_string(),
      data_type: DbDataType::Varchar,
      max_length: Some(32),
      scale: None,
      unsigned: None,
      primary: false,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };
    let column3 = DbColumn {
      name: "string_column2".to_string(),
      comment: "字符串2".to_string(),
      data_type: DbDataType::Varchar,
      max_length: Some(32),
      scale: None,
      unsigned: None,
      primary: false,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };
    let column4 = DbColumn {
      name: "string_column3".to_string(),
      comment: "字符串3".to_string(),
      data_type: DbDataType::Varchar,
      max_length: Some(32),
      scale: None,
      unsigned: None,
      primary: false,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };

    let unique_constraint1 = UniqueConstraint {
      name: "uk_table_1_string_column1".to_string(),
      columns: vec![column2.clone()],
    };
    let unique_constraint2 = UniqueConstraint {
      name: "uk_table_1_string_column3_string_column4".to_string(),
      columns: vec![column3.clone(), column4.clone()],
    };

    let table_structure = DbTableStructure {
      table: DbTable {
        name: "table_1".to_string(),
        comment: "表1".to_string(),
      },
      columns: vec![
        column1.clone(),
        column2.clone(),
        column3.clone(),
        column4.clone(),
      ],
      unique_constraints: vec![unique_constraint1, unique_constraint2],
      foreign_constraints: vec![],
    };

    let delete_one = BusinessOperator::DeleteOne;
    let business_operators = vec![delete_one];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <id property="dbid" column="dbid"/>
        <result property="stringColumn1" column="string_column1"/>
        <result property="stringColumn2" column="string_column2"/>
        <result property="stringColumn3" column="string_column3"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            dbid,
            string_column1,
            string_column2,
            string_column3
        FROM
            table_1
    </sql>

    <delete id="deleteTable1ById" parameterType="Long">
        DELETE FROM table_1 WHERE dbid = #{dbid}
    </delete>

    <delete id="deleteTable1ByStringColumn1" parameterType="String">
        DELETE FROM table_1 WHERE string_column1 = #{stringColumn1}
    </delete>

    <delete id="deleteTable1ByStringColumn2AndStringColumn3">
        DELETE FROM table_1 WHERE string_column2 = #{stringColumn2} AND string_column3 = #{stringColumn3}
    </delete>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn generate_one_number_column_delete_list_by_foreign_constraint() {
    let table1 = DbTable {
      name: "table_1".to_string(),
      comment: "表1".to_string(),
    };
    let column1 = DbColumn {
      name: "dbid".to_string(),
      comment: "主键".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: true,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };

    let table2 = DbTable {
      name: "table_2".to_string(),
      comment: "表2".to_string(),
    };
    let column2 = DbColumn {
      name: "dbid".to_string(),
      comment: "主键".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: true,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };
    let column3 = DbColumn {
      name: "number_column2".to_string(),
      comment: "数值2".to_string(),
      data_type: DbDataType::BigInt,
      max_length: None,
      scale: None,
      unsigned: Some(false),
      primary: false,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };

    let foreign_constraint = ForeignConstraint {
      name: "fk_table_2_number_column1".to_string(),
      base_table: table2.clone(),
      base_column: column3.clone(),
      ref_table: table1.clone(),
      ref_column: column1,
    };

    let table_structure = DbTableStructure {
      table: table2,
      columns: vec![column2, column3],
      unique_constraints: vec![],
      foreign_constraints: vec![foreign_constraint],
    };

    let delete_one = BusinessOperator::DeleteOne;
    let business_operators = vec![delete_one];
    let group_location = GroupLocation {
      java_base_package: "org.corp.project1".to_string(),
      group: "group1".to_string(),
    };
    let actual_code = generate(table_structure, business_operators, group_location).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table2Mapper">
    <resultMap id="Table2Result" type="Table2">
        <id property="dbid" column="dbid"/>
        <result property="numberColumn2" column="number_column2"/>
    </resultMap>

    <sql id="selectTable2Vo">
        SELECT
            dbid,
            number_column2
        FROM
            table_2
    </sql>

    <delete id="deleteTable2ById" parameterType="Long">
        DELETE FROM table_2 WHERE dbid = #{dbid}
    </delete>

    <delete id="deleteTable2ListByNumberColumn2" parameterType="Long">
        DELETE FROM table_2 WHERE number_column2 = #{numberColumn2}
    </delete>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }
  //endregion

  //region apply_changes
  #[test]
  fn apply_changes_add_one_column_nullable_is_true() {
    let source = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <id property="dbid" column="dbid"/>
        <result property="stringColumn1" column="string_column1"/>
        <result property="stringColumn3" column="string_column3"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            dbid,
            string_column1,
            string_column3
        FROM
            table_1
    </sql>

    <insert id="insertTable1" parameterType="Table1" useGeneratedKeys="true" keyProperty="dbid">
        INSERT INTO table_1
        <trim prefix="(" suffix=")" suffixOverrides=",">
            <if test="stringColumn1 != null and stringColumn1 != ''">string_column1,</if>
            <if test="stringColumn3 != null and stringColumn3 != ''">string_column3,</if>
        </trim>
        <trim prefix="VALUES (" suffix=")" suffixOverrides=",">
            <if test="stringColumn1 != null and stringColumn1 != ''">#{stringColumn1},</if>
            <if test="stringColumn3 != null and stringColumn3 != ''">#{stringColumn3},</if>
        </trim>
    </insert>

    <update id="updateTable1" parameterType="Table1">
        UPDATE table_1
        <trim prefix="SET" suffixOverrides=",">
            string_column1 = #{stringColumn1},
            string_column3 = #{stringColumn3},
        </trim>
        WHERE dbid = #{dbid}
    </update>
</mapper>"#;

    let table = DbTable {
      name: "table_1".to_string(),
      comment: "表1".to_string(),
    };
    let column2 = DbColumn {
      name: "string_column2".to_string(),
      comment: "字符串2".to_string(),
      data_type: DbDataType::Varchar,
      max_length: Some(32),
      scale: None,
      unsigned: None,
      primary: false,
      nullable: true,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: Some("string_column1".to_string()),
      table_name: None,
    };
    let change1 = DBChange::AddColumn(vec![column2]);
    let changes = vec![change1];
    let actual_code = apply_changes(source, table, changes).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <id property="dbid" column="dbid"/>
        <result property="stringColumn1" column="string_column1"/>
        <result property="stringColumn2" column="string_column2"/>
        <result property="stringColumn3" column="string_column3"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            dbid,
            string_column1,
            string_column2,
            string_column3
        FROM
            table_1
    </sql>

    <insert id="insertTable1" parameterType="Table1" useGeneratedKeys="true" keyProperty="dbid">
        INSERT INTO table_1
        <trim prefix="(" suffix=")" suffixOverrides=",">
            <if test="stringColumn1 != null and stringColumn1 != ''">string_column1,</if>
            <if test="stringColumn2 != null and stringColumn2 != ''">string_column2,</if>
            <if test="stringColumn3 != null and stringColumn3 != ''">string_column3,</if>
        </trim>
        <trim prefix="VALUES (" suffix=")" suffixOverrides=",">
            <if test="stringColumn1 != null and stringColumn1 != ''">#{stringColumn1},</if>
            <if test="stringColumn2 != null and stringColumn2 != ''">#{stringColumn2},</if>
            <if test="stringColumn3 != null and stringColumn3 != ''">#{stringColumn3},</if>
        </trim>
    </insert>

    <update id="updateTable1" parameterType="Table1">
        UPDATE table_1
        <trim prefix="SET" suffixOverrides=",">
            string_column1 = #{stringColumn1},
            string_column2 = #{stringColumn2},
            string_column3 = #{stringColumn3},
        </trim>
        WHERE dbid = #{dbid}
    </update>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn apply_changes_add_one_column_nullable_is_false() {
    let source = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <id property="dbid" column="dbid"/>
        <result property="stringColumn1" column="string_column1"/>
        <result property="stringColumn3" column="string_column3"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            dbid,
            string_column1,
            string_column3
        FROM
            table_1
    </sql>

    <insert id="insertTable1" parameterType="Table1" useGeneratedKeys="true" keyProperty="dbid">
        INSERT INTO table_1
        <trim prefix="(" suffix=")" suffixOverrides=",">
            <if test="stringColumn1 != null and stringColumn1 != ''">string_column1,</if>
            <if test="stringColumn3 != null and stringColumn3 != ''">string_column3,</if>
        </trim>
        <trim prefix="VALUES (" suffix=")" suffixOverrides=",">
            <if test="stringColumn1 != null and stringColumn1 != ''">#{stringColumn1},</if>
            <if test="stringColumn3 != null and stringColumn3 != ''">#{stringColumn3},</if>
        </trim>
    </insert>

    <update id="updateTable1" parameterType="Table1">
        UPDATE table_1
        <trim prefix="SET" suffixOverrides=",">
            string_column1 = #{stringColumn1},
            string_column3 = #{stringColumn3},
        </trim>
        WHERE dbid = #{dbid}
    </update>
</mapper>"#;

    let table = DbTable {
      name: "table_1".to_string(),
      comment: "表1".to_string(),
    };
    let column2 = DbColumn {
      name: "string_column2".to_string(),
      comment: "字符串2".to_string(),
      data_type: DbDataType::Varchar,
      max_length: Some(32),
      scale: None,
      unsigned: None,
      primary: false,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: Some("string_column1".to_string()),
      table_name: None,
    };
    let change1 = DBChange::AddColumn(vec![column2]);
    let changes = vec![change1];
    let actual_code = apply_changes(source, table, changes).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <id property="dbid" column="dbid"/>
        <result property="stringColumn1" column="string_column1"/>
        <result property="stringColumn2" column="string_column2"/>
        <result property="stringColumn3" column="string_column3"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            dbid,
            string_column1,
            string_column2,
            string_column3
        FROM
            table_1
    </sql>

    <insert id="insertTable1" parameterType="Table1" useGeneratedKeys="true" keyProperty="dbid">
        INSERT INTO table_1
        <trim prefix="(" suffix=")" suffixOverrides=",">
            <if test="stringColumn1 != null and stringColumn1 != ''">string_column1,</if>
            <if test="stringColumn2 != null and stringColumn2 != ''">string_column2,</if>
            <if test="stringColumn3 != null and stringColumn3 != ''">string_column3,</if>
        </trim>
        <trim prefix="VALUES (" suffix=")" suffixOverrides=",">
            <if test="stringColumn1 != null and stringColumn1 != ''">#{stringColumn1},</if>
            <if test="stringColumn2 != null and stringColumn2 != ''">#{stringColumn2},</if>
            <if test="stringColumn3 != null and stringColumn3 != ''">#{stringColumn3},</if>
        </trim>
    </insert>

    <update id="updateTable1" parameterType="Table1">
        UPDATE table_1
        <trim prefix="SET" suffixOverrides=",">
            string_column1 = #{stringColumn1},
            <if test="stringColumn2 != null and stringColumn2 != ''">string_column2 = #{stringColumn2},</if>
            string_column3 = #{stringColumn3},
        </trim>
        WHERE dbid = #{dbid}
    </update>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn apply_changes_drop_one_column_nullable_is_false() {
    let source = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <id property="dbid" column="dbid"/>
        <result property="stringColumn1" column="string_column1"/>
        <result property="stringColumn2" column="string_column2"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            dbid,
            string_column1,
            string_column2
        FROM
            table_1
    </sql>

    <insert id="insertTable1" parameterType="Table1" useGeneratedKeys="true" keyProperty="dbid">
        INSERT INTO table_1
        <trim prefix="(" suffix=")" suffixOverrides=",">
            <if test="stringColumn1 != null and stringColumn1 != ''">string_column1,</if>
            <if test="stringColumn2 != null and stringColumn2 != ''">string_column2,</if>
        </trim>
        <trim prefix="VALUES (" suffix=")" suffixOverrides=",">
            <if test="stringColumn1 != null and stringColumn1 != ''">#{stringColumn1},</if>
            <if test="stringColumn2 != null and stringColumn2 != ''">#{stringColumn2},</if>
        </trim>
    </insert>

    <update id="updateTable1" parameterType="Table1">
        UPDATE table_1
        <trim prefix="SET" suffixOverrides=",">
            string_column1 = #{stringColumn1},
            <if test="stringColumn2 != null and stringColumn2 != ''">string_column2 = #{stringColumn2},</if>
        </trim>
        WHERE dbid = #{dbid}
    </update>
</mapper>"#;

    let table = DbTable {
      name: "table_1".to_string(),
      comment: "表1".to_string(),
    };
    let column2 = DbColumn {
      name: "string_column2".to_string(),
      comment: "字符串2".to_string(),
      data_type: DbDataType::Varchar,
      max_length: Some(32),
      scale: None,
      unsigned: None,
      primary: false,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: None,
      table_name: None,
    };
    let change1 = DBChange::DropColumn(vec![column2]);
    let changes = vec![change1];
    let actual_code = apply_changes(source, table, changes).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE mapper PUBLIC "-//mybatis.org//DTD Mapper 3.0//EN" "http://mybatis.org/dtd/mybatis-3-mapper.dtd">

<mapper namespace="org.corp.project1.group1.mapper.Table1Mapper">
    <resultMap id="Table1Result" type="Table1">
        <id property="dbid" column="dbid"/>
        <result property="stringColumn1" column="string_column1"/>
    </resultMap>

    <sql id="selectTable1Vo">
        SELECT
            dbid,
            string_column1
        FROM
            table_1
    </sql>

    <insert id="insertTable1" parameterType="Table1" useGeneratedKeys="true" keyProperty="dbid">
        INSERT INTO table_1
        <trim prefix="(" suffix=")" suffixOverrides=",">
            <if test="stringColumn1 != null and stringColumn1 != ''">string_column1,</if>
        </trim>
        <trim prefix="VALUES (" suffix=")" suffixOverrides=",">
            <if test="stringColumn1 != null and stringColumn1 != ''">#{stringColumn1},</if>
        </trim>
    </insert>

    <update id="updateTable1" parameterType="Table1">
        UPDATE table_1
        <trim prefix="SET" suffixOverrides=",">
            string_column1 = #{stringColumn1},
        </trim>
        WHERE dbid = #{dbid}
    </update>
</mapper>"#;
    assert_eq!(actual_code, expect_code);
  }
  //endregion
}
