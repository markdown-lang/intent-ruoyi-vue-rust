use crate::menu_group::{MenuGroup, ModifyInfo};
use crate::source_file::liquibase_util::{
  add_column_with_computed_value, add_column_with_number_value, add_column_with_string_value,
  end_change_set_tag, end_database_change_log_tag, start_change_set_tag,
  start_database_change_log_tag, write_xml_declaration,
};
use anyhow::Result;
use quick_xml::Writer;
use quick_xml::events::{BytesEnd, BytesStart, Event};
use std::io::Cursor;

pub fn generate_menu_group_liquibase(
  menu_group: &MenuGroup,
  modify_info: ModifyInfo,
) -> Result<String> {
  let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 4);

  // 写入 XML 声明
  write_xml_declaration(&mut writer)?;
  // databaseChangeLog
  start_database_change_log_tag(&mut writer)?;
  // changeSet
  start_change_set_tag(&mut writer, &modify_info, false)?;
  // insert sys_menu
  let mut insert_sys_menu = BytesStart::new("insert");
  insert_sys_menu.push_attribute(("table_name", "sys_menu"));
  writer.write_event(Event::Start(insert_sys_menu))?;

  add_column_with_number_value(&mut writer, "menu_id", menu_group.id)?;
  add_column_with_string_value(&mut writer, "menu_name", menu_group.title.as_str())?;
  add_column_with_number_value(&mut writer, "parent_id", menu_group.parent_id)?;
  add_column_with_number_value(&mut writer, "order_num", menu_group.seq)?;
  add_column_with_string_value(&mut writer, "path", menu_group.route.as_str())?;
  add_column_with_number_value(&mut writer, "is_frame", 1)?;
  add_column_with_string_value(&mut writer, "menu_type", "M")?;
  add_column_with_string_value(&mut writer, "visible", "0")?;
  add_column_with_string_value(&mut writer, "status", "0")?;
  add_column_with_string_value(&mut writer, "icon", menu_group.icon.as_str())?;
  add_column_with_string_value(&mut writer, "remark", menu_group.title.as_str())?;
  add_column_with_string_value(&mut writer, "create_by", modify_info.author.as_str())?;
  add_column_with_computed_value(&mut writer, "create_time", "now()")?;

  writer.write_event(Event::End(BytesEnd::new("insert")))?;

  // insert sys_menu_client_type
  if let Some(ref client_type) = menu_group.client_type {
    let mut insert_client_type = BytesStart::new("insert");
    insert_client_type.push_attribute(("table_name", "sys_menu_client_type"));
    writer.write_event(Event::Start(insert_client_type))?;

    add_column_with_number_value(&mut writer, "menu_id", menu_group.id)?;
    add_column_with_string_value(&mut writer, "client_type", client_type.as_str())?;
    add_column_with_string_value(&mut writer, "create_by", modify_info.author.as_str())?;
    add_column_with_computed_value(&mut writer, "create_time", "now()")?;

    writer.write_event(Event::End(BytesEnd::new("insert")))?;
  }

  end_change_set_tag(&mut writer)?;
  end_database_change_log_tag(&mut writer)?;

  let result = writer.into_inner().into_inner();
  let xml_string = String::from_utf8(result)?;
  Ok(xml_string)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn generate_success() {
    let menu_group = MenuGroup {
      id: 1,
      key: "key1".to_string(),
      title: "分组1".to_string(),
      route: "path1".to_string(),
      seq: 100,
      icon: "icon1".to_string(),
      client_type: Some("01".to_string()),
      parent_id: 0,
    };
    let modify_info = ModifyInfo {
      author: "wzy".to_string(),
      time: "202507311439".to_string(),
    };

    let actual_code = generate_menu_group_liquibase(&menu_group, modify_info).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>

<databaseChangeLog xmlns="http://www.liquibase.org/xml/ns/dbchangelog" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://www.liquibase.org/xml/ns/dbchangelog http://www.liquibase.org/xml/ns/dbchangelog/dbchangelog-latest.xsd">
    <changeSet author="wzy" id="202507311439">
        <insert table_name="sys_menu">
            <column name="menu_id" valueNumeric="1"/>
            <column name="menu_name" value="分组1"/>
            <column name="parent_id" valueNumeric="0"/>
            <column name="order_num" valueNumeric="100"/>
            <column name="path" value="path1"/>
            <column name="is_frame" valueNumeric="1"/>
            <column name="menu_type" value="M"/>
            <column name="visible" value="0"/>
            <column name="status" value="0"/>
            <column name="icon" value="icon1"/>
            <column name="remark" value="分组1"/>
            <column name="create_by" value="wzy"/>
            <column name="create_time" valueComputed="now()"/>
        </insert>
        <insert table_name="sys_menu_client_type">
            <column name="menu_id" valueNumeric="1"/>
            <column name="client_type" value="01"/>
            <column name="create_by" value="wzy"/>
            <column name="create_time" valueComputed="now()"/>
        </insert>
    </changeSet>
</databaseChangeLog>"#;
    assert_eq!(actual_code, expect_code);
  }

  #[test]
  fn generate_when_no_client_type() {
    let menu_group = MenuGroup {
      id: 1,
      key: "key1".to_string(),
      title: "分组1".to_string(),
      route: "path1".to_string(),
      seq: 100,
      icon: "icon1".to_string(),
      client_type: None,
      parent_id: 0,
    };
    let modify_info = ModifyInfo {
      author: "wzy".to_string(),
      time: "202507311439".to_string(),
    };

    let actual_code = generate_menu_group_liquibase(&menu_group, modify_info).unwrap();
    let expect_code = r#"<?xml version="1.0" encoding="UTF-8"?>

<databaseChangeLog xmlns="http://www.liquibase.org/xml/ns/dbchangelog" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://www.liquibase.org/xml/ns/dbchangelog http://www.liquibase.org/xml/ns/dbchangelog/dbchangelog-latest.xsd">
    <changeSet author="wzy" id="202507311439">
        <insert table_name="sys_menu">
            <column name="menu_id" valueNumeric="1"/>
            <column name="menu_name" value="分组1"/>
            <column name="parent_id" valueNumeric="0"/>
            <column name="order_num" valueNumeric="100"/>
            <column name="path" value="path1"/>
            <column name="is_frame" valueNumeric="1"/>
            <column name="menu_type" value="M"/>
            <column name="visible" value="0"/>
            <column name="status" value="0"/>
            <column name="icon" value="icon1"/>
            <column name="remark" value="分组1"/>
            <column name="create_by" value="wzy"/>
            <column name="create_time" valueComputed="now()"/>
        </insert>
    </changeSet>
</databaseChangeLog>"#;
    assert_eq!(actual_code, expect_code);
  }
}
