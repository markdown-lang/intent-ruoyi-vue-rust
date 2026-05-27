use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use std::io::Cursor;

use crate::types::ChangeSetInfo;

pub(crate) fn write_xml_declaration(writer: &mut Writer<Cursor<Vec<u8>>>) -> anyhow::Result<()> {
  let decl = BytesDecl::new("1.0", Some("UTF-8"), None);
  writer.write_event(Event::Decl(decl))?;
  writer.write_event(Event::Text(BytesText::new("\n\n")))?;
  Ok(())
}

pub(crate) fn start_change_set_tag(
  writer: &mut Writer<Cursor<Vec<u8>>>,
  change_set_info: &ChangeSetInfo,
  run_on_change: bool,
) -> anyhow::Result<()> {
  let mut change_set = BytesStart::new("changeSet");
  change_set.push_attribute(("author", change_set_info.author.as_str()));

  change_set.push_attribute(("id", change_set_info.id.as_ref()));
  if run_on_change {
    change_set.push_attribute(("runOnChange", "true"));
  }
  writer.write_event(Event::Start(change_set))?;
  Ok(())
}

pub(crate) fn end_change_set_tag(writer: &mut Writer<Cursor<Vec<u8>>>) -> anyhow::Result<()> {
  writer.write_event(Event::End(BytesEnd::new("changeSet")))?;
  Ok(())
}

pub(crate) fn start_database_change_log_tag(
  writer: &mut Writer<Cursor<Vec<u8>>>,
) -> anyhow::Result<()> {
  let mut database_change_log = BytesStart::new("databaseChangeLog");
  database_change_log.push_attribute(("xmlns", "http://www.liquibase.org/xml/ns/dbchangelog"));
  database_change_log.push_attribute(("xmlns:xsi", "http://www.w3.org/2001/XMLSchema-instance"));
  database_change_log.push_attribute(("xsi:schemaLocation", "http://www.liquibase.org/xml/ns/dbchangelog http://www.liquibase.org/xml/ns/dbchangelog/dbchangelog-latest.xsd"));
  writer.write_event(Event::Start(database_change_log))?;
  Ok(())
}

pub(crate) fn end_database_change_log_tag(
  writer: &mut Writer<Cursor<Vec<u8>>>,
) -> anyhow::Result<()> {
  writer.write_event(Event::End(BytesEnd::new("databaseChangeLog")))?;
  Ok(())
}

//#region 添加 column 元素的工具函数
pub(crate) fn add_column_with_string_value(
  writer: &mut Writer<Cursor<Vec<u8>>>,
  name: &str,
  value: &str,
) -> anyhow::Result<()> {
  let mut column = BytesStart::new("column");
  column.push_attribute(("name", name));
  column.push_attribute(("value", value));
  writer.write_event(Event::Empty(column))?;
  Ok(())
}

pub(crate) fn add_column_with_number_value(
  writer: &mut Writer<Cursor<Vec<u8>>>,
  name: &str,
  value: i64,
) -> anyhow::Result<()> {
  let mut column = BytesStart::new("column");
  column.push_attribute(("name", name));
  column.push_attribute(("valueNumeric", value.to_string().as_str()));
  writer.write_event(Event::Empty(column))?;
  Ok(())
}

pub(crate) fn add_column_with_computed_value(
  writer: &mut Writer<Cursor<Vec<u8>>>,
  name: &str,
  value: &str,
) -> anyhow::Result<()> {
  let mut column = BytesStart::new("column");
  column.push_attribute(("name", name));
  column.push_attribute(("valueComputed", value.to_string().as_str()));
  writer.write_event(Event::Empty(column))?;
  Ok(())
}
//#endregion
