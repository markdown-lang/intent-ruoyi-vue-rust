use crate::types::{CodeGenerateResult, FileInfo, FileOperation};
use anyhow::Result;
use chrono::Local;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use std::fmt::format;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

/// 让项目支持 liquibase
///
/// 1. 在根目录的 pom.xml 中添加 liquibase 依赖，版本号通过 properties 管理
/// 2. 在选中的 module 中添加 liquibase 依赖
/// 3. 在 src/main/java/com/ruoyi/xx/core/config 下添加 LiquibaseConfig.java
/// 4. 在 src/main/resources/db/changelog 下 创建 db.changelog-master.xml 和 table 文件夹
/// 5. 创建一张示例表
pub fn install(
  project_root_dir: &Path,
  module_name: &str,
  base_package: &str,
  author: &str,
) -> Result<CodeGenerateResult> {
  let mut files: Vec<FileInfo> = Vec::new();

  let root_pom_xml_path = update_root_pom_xml(project_root_dir)?;
  files.push(FileInfo {
    path: root_pom_xml_path.into_string().unwrap(),
    operation: FileOperation::Modify,
    message: "修改成功".to_string(),
  });

  let module_pom_xml_path = update_module_pom_xml(project_root_dir, module_name)?;
  files.push(FileInfo {
    path: module_pom_xml_path.into_string().unwrap(),
    operation: FileOperation::Modify,
    message: "修改成功".to_string(),
  });

  let liquibase_config_java_path =
    new_liquibase_config_java(project_root_dir, module_name, base_package)?;
  files.push(FileInfo {
    path: liquibase_config_java_path.into_string().unwrap(),
    operation: FileOperation::Add,
    message: "新增成功".to_string(),
  });

  let demo_table_id = Local::now().format("%Y%m%d%H%M").to_string();
  let liquibase_config_xml_path =
    new_liquibase_config_xml(project_root_dir, module_name, demo_table_id.as_str())?;
  files.push(FileInfo {
    path: liquibase_config_xml_path.into_string().unwrap(),
    operation: FileOperation::Add,
    message: "新增成功".to_string(),
  });

  let demo_table_create_xml_path = new_liquibase_demo_table_create_xml(
    project_root_dir,
    module_name,
    author,
    demo_table_id.as_str(),
  )?;
  files.push(FileInfo {
    path: demo_table_create_xml_path.into_string().unwrap(),
    operation: FileOperation::Add,
    message: "新增成功".to_string(),
  });

  Ok(CodeGenerateResult { files })
}

fn update_root_pom_xml(project_root_dir: &Path) -> Result<PathBuf> {
  let liquibase_core_version = "5.0.4";

  // 往根目录的 pom.xml 中添加依赖节点
  let root_pom_path = project_root_dir.join("pom.xml");
  let mut root_pom_reader = Reader::from_file(&root_pom_path)?;
  root_pom_reader.config_mut().trim_text(false);
  let indent_char = b' ';
  let indent_size = 4;
  let mut root_pom_writer =
    Writer::new_with_indent(Cursor::new(Vec::new()), indent_char, indent_size);
  let mut buf = Vec::new();

  let indent_str = String::from_utf8(vec![indent_char; indent_size])?;

  loop {
    match root_pom_reader.read_event_into(&mut buf) {
      Ok(Event::Start(e)) => {
        root_pom_writer.write_event(Event::Start(e))?;
      }
      Ok(Event::End(e)) => {
        match e.name().as_ref() {
          "properties" => {
            root_pom_writer.write_event(Event::Text(BytesText::new(indent_str.as_str())))?;
            root_pom_writer.write_event(Event::Start(BytesStart::new("liquibase.version")))?;
            root_pom_writer.write_event(Event::Text(BytesText::new(liquibase_core_version)))?;
            root_pom_writer.write_event(Event::End(BytesEnd::new("liquibase.version")))?;
          }
          "dependencies" => {
            root_pom_writer.write_indent()?;
            root_pom_writer.write_event(Event::Comment(BytesText::new("liquibase")))?;

            root_pom_writer.write_event(Event::Start(BytesStart::new("dependency")))?;

            root_pom_writer.write_event(Event::Start(BytesStart::new("groupId")))?;
            root_pom_writer.write_event(Event::Text(BytesText::new("org.liquibase")))?;
            root_pom_writer.write_event(Event::End(BytesEnd::new("groupId")))?;

            root_pom_writer.write_event(Event::Start(BytesStart::new("artifactId")))?;
            root_pom_writer.write_event(Event::Text(BytesText::new("liquibase-core")))?;
            root_pom_writer.write_event(Event::End(BytesEnd::new("artifactId")))?;

            root_pom_writer.write_event(Event::Start(BytesStart::new("version")))?;
            root_pom_writer.write_event(Event::Text(BytesText::new("${liquibase.version}")))?;
            root_pom_writer.write_event(Event::End(BytesEnd::new("version")))?;

            root_pom_writer.write_event(Event::End(BytesEnd::new("dependency")))?;
          }
          _ => {}
        }

        root_pom_writer.write_event(Event::End(e))?;
      }
      Ok(Event::Eof) => break,
      Ok(event) => root_pom_writer.write_event(event)?,
      Err(e) => return Err(e.into()),
    }
    buf.clear();
  }

  let result = root_pom_writer.into_inner().into_inner();
  fs::write(&root_pom_path, result)?;

  Ok(root_pom_path)
}

fn update_module_pom_xml(project_root_dir: &Path, module_name: &str) -> Result<PathBuf> {
  let pom_path = project_root_dir.join(module_name).join("pom.xml");
  let mut pom_reader = Reader::from_file(&pom_path)?;
  pom_reader.config_mut().trim_text(false);
  let indent_char = b' ';
  let indent_size = 4;
  let mut pom_writer = Writer::new_with_indent(Cursor::new(Vec::new()), indent_char, indent_size);
  let mut buf = Vec::new();

  loop {
    match pom_reader.read_event_into(&mut buf) {
      Ok(Event::Start(e)) => {
        pom_writer.write_event(Event::Start(e))?;
      }
      Ok(Event::End(e)) => {
        if e.name().as_ref() == "dependencies" {
          pom_writer.write_indent()?;
          pom_writer.write_event(Event::Comment(BytesText::new("liquibase")))?;

          pom_writer.write_event(Event::Start(BytesStart::new("dependency")))?;

          pom_writer.write_event(Event::Start(BytesStart::new("groupId")))?;
          pom_writer.write_event(Event::Text(BytesText::new("org.liquibase")))?;
          pom_writer.write_event(Event::End(BytesEnd::new("groupId")))?;

          pom_writer.write_event(Event::Start(BytesStart::new("artifactId")))?;
          pom_writer.write_event(Event::Text(BytesText::new("liquibase-core")))?;
          pom_writer.write_event(Event::End(BytesEnd::new("artifactId")))?;

          pom_writer.write_event(Event::End(BytesEnd::new("dependency")))?;
        }

        pom_writer.write_event(Event::End(e))?;
      }
      Ok(Event::Eof) => break,
      Ok(event) => pom_writer.write_event(event)?,
      Err(e) => return Err(e.into()),
    }
    buf.clear();
  }

  let result = pom_writer.into_inner().into_inner();
  fs::write(&pom_path, result)?;

  Ok(pom_path)
}

fn new_liquibase_config_java(
  project_root_dir: &Path,
  module_name: &str,
  base_package: &str,
) -> Result<PathBuf> {
  let class_content = format!(
    r#"package {base_package}.core.config;

import liquibase.integration.spring.SpringLiquibase;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.context.annotation.Profile;
import org.springframework.core.io.DefaultResourceLoader;
import javax.sql.DataSource;

@Configuration
public class LiquibaseConfig {{

    private static final String LIQUIBASE_CHANGELOG = "classpath:/db/changelog/db.changelog-master.xml";

    @Profile("!test")
    @Bean
    public SpringLiquibase create(DataSource dataSource) {{
        SpringLiquibase liquibase = new SpringLiquibase();
        liquibase.setChangeLog(LIQUIBASE_CHANGELOG);
        liquibase.setDataSource(dataSource);
        liquibase.setShouldRun(true);
        liquibase.setResourceLoader(new DefaultResourceLoader());
        return liquibase;
    }}

    @Profile("test")
    @Bean
    public SpringLiquibase createForTest(DataSource dataSource) {{
        SpringLiquibase liquibase = new SpringLiquibase();
        liquibase.setChangeLog(LIQUIBASE_CHANGELOG);
        liquibase.setDataSource(dataSource);
        liquibase.setContexts("test");
        liquibase.setShouldRun(true);
        liquibase.setResourceLoader(new DefaultResourceLoader());
        return liquibase;
    }}
}}
"#
  );

  let class_dir_path = project_root_dir
    .join(module_name)
    .join("src/main/java")
    .join(base_package.replace(".", "/"))
    .join("core/config");
  fs::create_dir_all(&class_dir_path)?;

  let file_path = class_dir_path.join("LiquibaseConfig.java");
  fs::write(&file_path, class_content)?;

  Ok(file_path)
}

// FIXME: 每次生成时，文件名称和id不要变，不然会尝试重复创建已存在的表，然后报错
fn new_liquibase_config_xml(
  project_root_dir: &Path,
  module_name: &str,
  demo_table_id: &str,
) -> Result<PathBuf> {
  let xml_content = format!(
    r#"<?xml version="1.0" encoding="UTF-8"?>
<databaseChangeLog
        xmlns="http://www.liquibase.org/xml/ns/dbchangelog"
        xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
        xsi:schemaLocation="http://www.liquibase.org/xml/ns/dbchangelog http://www.liquibase.org/xml/ns/dbchangelog/dbchangelog-latest.xsd">

    <property dbms="mysql" name="clob" value="text"/>
    <property dbms="mysql" name="datetime" value="DATETIME"/>

    <include file="db/changelog/table/examples/demo_table/{demo_table_id}_create.xml"/>
</databaseChangeLog>
"#
  );

  let xml_dir_path = project_root_dir
    .join(module_name)
    .join("src/main/resources/db/changelog");
  fs::create_dir_all(&xml_dir_path)?;

  let file_path = xml_dir_path.join("db.changelog-master.xml");
  fs::write(&file_path, xml_content)?;

  Ok(file_path)
}

fn new_liquibase_demo_table_create_xml(
  project_root_dir: &Path,
  module_name: &str,
  author: &str,
  demo_table_id: &str,
) -> Result<PathBuf> {
  let xml_content = format!(
    r#"<?xml version="1.0" encoding="UTF-8"?>
<databaseChangeLog
        xmlns="http://www.liquibase.org/xml/ns/dbchangelog"
        xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
        xsi:schemaLocation="http://www.liquibase.org/xml/ns/dbchangelog http://www.liquibase.org/xml/ns/dbchangelog/dbchangelog-latest.xsd">

    <changeSet author="{author}" id="{demo_table_id}">
        <createTable tableName="demo_table" remarks="示例表">
            <column name="id" type="bigint" autoIncrement="true" remarks="主键">
                <constraints primaryKey="true" nullable="false" primaryKeyName="pk_demo_table"/>
            </column>
            <column name="f_int" remarks="整数" type="int"/>
            <column name="f_float" remarks="单精度浮点数" type="float"/>
            <column name="f_date" remarks="日期" type="date"/>
            <column name="f_time" remarks="时间" type="time"/>
            <column name="f_datetime" remarks="日期和时间" type="datetime"/>
            <column name="f_char" remarks="定长字符串" type="char(2)"/>
            <column name="f_varchar" remarks="变长字符串" type="varchar(32)"/>
            <column name="file_name" remarks="文件名" type="varchar(128)"/>
            <column name="f_blob" remarks="文件" type="blob"/>
            <column name="f_text" remarks="长文本数据" type="text"/>
            <column name="version" remarks="版本号" type="int" defaultValueNumeric="0"/>
            <column name="create_by" remarks="创建者标识" type="bigint">
                <constraints nullable="false"/>
            </column>
            <column name="create_time" remarks="创建时间" type="datetime">
                <constraints nullable="false"/>
            </column>
            <column name="update_by" remarks="更新者标识" type="bigint"/>
            <column name="update_time" remarks="最近修改时间 " type="datetime"/>
        </createTable>
    </changeSet>
</databaseChangeLog>
"#
  );

  let xml_dir_path = project_root_dir
    .join(module_name)
    .join("src/main/resources/db/changelog/table/examples/demo_table");
  fs::create_dir_all(&xml_dir_path)?;

  let file_path = xml_dir_path.join(format!("{demo_table_id}_create.xml"));
  fs::write(&file_path, xml_content)?;

  Ok(file_path)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::java_project::module::new_module;
  #[test]
  fn test_update_root_pom_xml() {
    update_root_pom_xml("resources/project1".as_ref()).unwrap();
  }

  #[test]
  fn test_update_module_pom_xml() {
    update_module_pom_xml("resources/project1".as_ref(), "ruoyi-admin").unwrap();
  }

  #[test]
  fn test_new_liquibase_config_java() {
    new_liquibase_config_java("resources/project1".as_ref(), "server", "com.ruoyi.demo").unwrap();
  }

  #[test]
  fn test_new_liquibase_config_xml() {
    new_liquibase_config_xml("resources/project1".as_ref(), "server", "202608301700").unwrap();
  }

  #[test]
  fn test_new_liquibase_demo_table_create_xml() {
    new_liquibase_demo_table_create_xml(
      "resources/project1".as_ref(),
      "server",
      "zhangsan",
      "202608301700",
    )
    .unwrap();
  }

  #[test]
  fn test_install() {
    install(
      "D:\\sources\\markdown-lang\\ide-plugins\\vscode\\server".as_ref(),
      "ruoyi-test",
      "com.ruoyi.test",
      "cx",
    )
    .unwrap();
  }
}
