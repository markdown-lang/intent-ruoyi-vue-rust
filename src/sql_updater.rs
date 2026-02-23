use crate::db_table_info::DbColumn;
use anyhow::Result;
use sqlformat::{Dialect, FormatOptions, QueryParams};
use sqlparser::ast::{Expr, Ident, SelectItem, SetExpr, Statement};
use sqlparser::dialect::MySqlDialect;
use sqlparser::parser::Parser;

/// 修改 select sql 语句，在其中增加列
pub fn insert_columns_for_select_sql(
  select_sql: &str,
  added_columns: &[DbColumn],
  indent_level: usize,
) -> Result<String> {
  let dialect = MySqlDialect {};
  let mut stmts = Parser::parse_sql(&dialect, select_sql)?;
  let result = if let Some(stmt) = stmts.first_mut() {
    if let Statement::Query(query) = stmt {
      if let SetExpr::Select(select) = query.body.as_mut() {
        let mut current_after_column_name = "";
        let mut insert_index = 0;
        for column in added_columns {
          // 判断列是否已存在，如果已存在，则不再新增
          let exists = select.projection.iter().any(|item| match item {
            SelectItem::UnnamedExpr(Expr::Identifier(ident)) => ident.value == column.name,
            _ => false,
          });
          // 不重复添加列
          if exists {
            continue;
          }

          if let Some(after_column_name) = &column.after_column_name {
            if current_after_column_name != after_column_name {
              insert_index = select
                .projection
                .iter()
                .position(|item| {
                  if let SelectItem::UnnamedExpr(Expr::Identifier(ident)) = item {
                    ident.value == after_column_name.to_string()
                  } else {
                    false
                  }
                })
                .map(|pos| pos + 1)
                .unwrap_or(0);
              current_after_column_name = after_column_name;
            } else {
              insert_index += 1;
            }
          }

          let new_column = SelectItem::UnnamedExpr(Expr::Identifier(Ident::new(&column.name)));
          select.projection.insert(insert_index, new_column);
        }
      }
    }
    stmt.to_string()
  } else {
    "".to_string()
  };

  Ok(format_sql(&result, indent_level))
}

pub fn drop_columns_for_select_sql(
  select_sql: &str,
  dropped_columns: &[DbColumn],
  indent_level: usize,
) -> Result<String> {
  let dialect = MySqlDialect {};
  let mut stmts = Parser::parse_sql(&dialect, select_sql)?;
  let result = if let Some(stmt) = stmts.first_mut() {
    if let Statement::Query(query) = stmt {
      if let SetExpr::Select(select) = query.body.as_mut() {
        for column in dropped_columns {
          // 判断列是否已存在，如果已存在，则不再新增
          let index_option = select.projection.iter().position(|item| match item {
            SelectItem::UnnamedExpr(Expr::Identifier(ident)) => ident.value == column.name,
            _ => false,
          });
          if let Some(index) = index_option {
            select.projection.remove(index);
          }
        }
      }
    }
    stmt.to_string()
  } else {
    "".to_string()
  };

  Ok(format_sql(&result, indent_level))
}

fn format_sql(sql: &String, all_line_indent_level: usize) -> String {
  let formated_sql = sqlformat::format(
    &sql,
    &QueryParams::None,
    &FormatOptions {
      indent: sqlformat::Indent::Spaces(4),
      uppercase: Some(true),
      lines_between_queries: 1,
      ignore_case_convert: None,
      inline: false,
      max_inline_block: 0,
      max_inline_arguments: None,
      max_inline_top_level: None,
      joins_as_top_level: false,
      dialect: Dialect::Generic,
    },
  );

  // 进一步格式化，所有的行一起缩进
  let indented_sql = indent_all_lines(formated_sql, all_line_indent_level);
  indented_sql
}

fn indent_all_lines(sql: String, indent_level: usize) -> String {
  if indent_level > 0 {
    let indent_chars = "    ".repeat(indent_level);
    sql
      .lines()
      .map(|s| format!("{}{}", indent_chars, s))
      .collect::<Vec<_>>()
      .join("\n")
  } else {
    sql
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db_table_info::DbDataType;

  #[test]
  fn insert_columns_one_column_insert_index_0() {
    let sql = r#"SELECT
            dbid,
            string_column1,
            string_column3
        FROM
            table_1"#;

    let column1 = DbColumn {
      name: "string_column2".to_string(),
      comment: "字符串2".to_string(),
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
    let actual_code = insert_columns_for_select_sql(sql, &vec![column1], 0).unwrap();
    let expect_code = r#"SELECT
    string_column2,
    dbid,
    string_column1,
    string_column3
FROM
    table_1"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn insert_columns_one_column_indent_1() {
    let sql = r#"SELECT
            dbid,
            string_column1,
            string_column3
        FROM
            table_1"#;

    let column1 = DbColumn {
      name: "string_column2".to_string(),
      comment: "字符串2".to_string(),
      data_type: DbDataType::Varchar,
      max_length: None,
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
    let actual_code = insert_columns_for_select_sql(sql, &vec![column1], 1).unwrap();
    let expect_code = r#"    SELECT
        dbid,
        string_column1,
        string_column2,
        string_column3
    FROM
        table_1"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn insert_columns_one_column() {
    let sql = r#"SELECT
            dbid,
            string_column1,
            string_column3
        FROM
            table_1"#;

    let column1 = DbColumn {
      name: "string_column2".to_string(),
      comment: "字符串2".to_string(),
      data_type: DbDataType::Varchar,
      max_length: None,
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
    let actual_code = insert_columns_for_select_sql(sql, &vec![column1], 0).unwrap();
    let expect_code = r#"SELECT
    dbid,
    string_column1,
    string_column2,
    string_column3
FROM
    table_1"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn insert_columns_one_column_duplicate() {
    let sql = r#"SELECT
            dbid,
            string_column1,
            string_column2
        FROM
            table_1"#;

    let column1 = DbColumn {
      name: "string_column2".to_string(),
      comment: "字符串2".to_string(),
      data_type: DbDataType::Varchar,
      max_length: None,
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
    let actual_code = insert_columns_for_select_sql(sql, &vec![column1], 0).unwrap();
    let expect_code = r#"SELECT
    dbid,
    string_column1,
    string_column2
FROM
    table_1"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn insert_columns_two_column_one_by_one() {
    let sql = r#"SELECT
            dbid,
            string_column1,
            string_column4
        FROM
            table_1"#;

    let column1 = DbColumn {
      name: "string_column2".to_string(),
      comment: "字符串2".to_string(),
      data_type: DbDataType::Varchar,
      max_length: None,
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
    let column2 = DbColumn {
      name: "string_column3".to_string(),
      comment: "字符串3".to_string(),
      data_type: DbDataType::Varchar,
      max_length: None,
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
    let actual_code = insert_columns_for_select_sql(sql, &vec![column1, column2], 0).unwrap();
    let expect_code = r#"SELECT
    dbid,
    string_column1,
    string_column2,
    string_column3,
    string_column4
FROM
    table_1"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn insert_columns_two_column() {
    let sql = r#"SELECT
            dbid,
            string_column1,
            string_column3
        FROM
            table_1"#;

    let column1 = DbColumn {
      name: "string_column2".to_string(),
      comment: "字符串2".to_string(),
      data_type: DbDataType::Varchar,
      max_length: None,
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
    let column2 = DbColumn {
      name: "string_column4".to_string(),
      comment: "字符串4".to_string(),
      data_type: DbDataType::Varchar,
      max_length: None,
      scale: None,
      unsigned: None,
      primary: false,
      nullable: false,
      default_value: None,
      db_unit_name: None,
      ui_unit_name: None,
      after_column_name: Some("string_column3".to_string()),
      table_name: None,
    };
    let actual_code = insert_columns_for_select_sql(sql, &vec![column1, column2], 0).unwrap();
    let expect_code = r#"SELECT
    dbid,
    string_column1,
    string_column2,
    string_column3,
    string_column4
FROM
    table_1"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn drop_columns_one_column() {
    let sql = r#"SELECT
            dbid,
            string_column1,
            string_column2
        FROM
            table_1"#;

    let column1 = DbColumn {
      name: "string_column2".to_string(),
      comment: "字符串2".to_string(),
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
    let actual_code = drop_columns_for_select_sql(sql, &vec![column1], 0).unwrap();
    let expect_code = r#"SELECT
    dbid,
    string_column1
FROM
    table_1"#;
    assert_eq!(actual_code, expect_code);
  }
}
