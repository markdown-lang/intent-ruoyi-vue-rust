use anyhow::Result;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

pub fn new_module(
  project_root_dir: &Path,
  module_name: &str,
  module_description: Option<String>,
  base_package: &str,
) -> Result<String> {
  let module_path = project_root_dir.join(module_name);

  let main_java_base_package_path = module_path
    .join("src/main/java")
    .join(base_package.replace(".", "/"));
  let main_resources_path = module_path.join("src/main/resources");

  let dirs = [&main_java_base_package_path, &main_resources_path];

  for dir in dirs {
    fs::create_dir_all(dir)?;
  }

  let description = module_description.unwrap_or("".to_string());
  let description = description.as_str();

  let root_pom_gav = read_root_pom_gav(project_root_dir);
  if let Some(root_pom_gav) = root_pom_gav {
    new_module_pom_xml(module_name, description, root_pom_gav, &module_path)?;
  }

  update_root_pom_xml(project_root_dir, module_name, description)?;
  update_ruoyi_admin_pom_xml(project_root_dir, module_name, description)?;
  new_biz_entity_java(&module_path, base_package)?;

  Ok("".to_string())
}

/// pom 构件坐标
struct ArtifactCoordinates {
  group_id: String,
  artifact_id: String,
  version: String,
}

fn read_root_pom_gav(project_root_dir: &Path) -> Option<ArtifactCoordinates> {
  let root_pom_path = project_root_dir.join("pom.xml");
  if !root_pom_path.exists() {
    return None;
  }
  let mut root_pom_reader = match Reader::from_file(&root_pom_path) {
    Ok(r) => r,
    Err(_) => return None,
  };
  root_pom_reader.config_mut().trim_text(false);
  let mut buf = Vec::new();

  let mut in_project = false;

  let mut in_group_id = false;
  let mut in_artifact_id = false;
  let mut in_version = false;

  let mut group_id = None;
  let mut artifact_id = None;
  let mut version = None;

  loop {
    match root_pom_reader.read_event_into(&mut buf) {
      Ok(Event::Start(e)) => match e.name().as_ref() {
        "project" => in_project = true,
        "groupId" if in_project => in_group_id = true,
        "artifactId" if in_project => in_artifact_id = true,
        "version" if in_project => in_version = true,
        _ => {}
      },
      Ok(Event::Text(e)) => {
        let txt = e.xml10_content();
        if in_group_id {
          group_id = Some(txt.to_string());
        }
        if in_artifact_id {
          artifact_id = Some(txt.to_string());
        }
        if in_version {
          version = Some(txt.to_string());
        }
      }
      Ok(Event::End(e)) => match e.name().as_ref() {
        "project" => in_project = false,
        "groupId" => in_group_id = false,
        "artifactId" => in_artifact_id = false,
        "version" => in_version = false,
        _ => {}
      },
      Ok(Event::Eof) => break,
      Ok(e) => return None,
      Err(e) => return None,
    }

    if group_id.is_some() && artifact_id.is_some() && version.is_some() {
      break;
    }

    buf.clear();
  }

  Some(ArtifactCoordinates {
    group_id: group_id.unwrap(),
    artifact_id: artifact_id.unwrap(),
    version: version.unwrap(),
  })
}

fn new_module_pom_xml(
  module_name: &str,
  module_description: &str,
  root_pom_gav: ArtifactCoordinates,
  module_path: &Path,
) -> Result<()> {
  let parent_artifact_id = root_pom_gav.artifact_id;
  let parent_group_id = root_pom_gav.group_id;
  let parent_version = root_pom_gav.version;

  let pom_content = format!(
    r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <parent>
        <artifactId>{parent_artifact_id}</artifactId>
        <groupId>{parent_group_id}</groupId>
        <version>{parent_version}</version>
    </parent>
    <modelVersion>4.0.0</modelVersion>

    <artifactId>{module_name}</artifactId>

    <description>
        {module_description}
    </description>

    <dependencies>

        <!-- 通用工具-->
        <dependency>
            <groupId>com.ruoyi</groupId>
            <artifactId>ruoyi-common</artifactId>
        </dependency>

    </dependencies>

</project>
"#
  );
  fs::write(module_path.join("pom.xml"), pom_content)?;
  Ok(())
}

fn new_biz_entity_java(module_path: &Path, base_package: &str) -> Result<String> {
  let class_content = format!(
    r#"package {base_package}.core.domain;

import com.ruoyi.common.core.domain.BaseEntity;

/// 业务 Entity 基类
public class BizEntity extends BaseEntity {{

    /// 代理主键
    private Long id;
    /// 乐观锁
    private Integer version;

    public Long getId() {{
        return id;
    }}

    public Long setId(Long id) {{
        this.id = id;
    }}

    public Integer getVersion() {{
        return version;
    }}

    public Integer setVersion(Integer version) {{
        this.version = version;
    }}

    public Long getCreateUserId() {{
        if (super.getCreateBy() != null) {{
            return Long.valueOf(super.getCreateBy());
        }} else {{
            return null;
        }}
    }}

    public void setCreateUserId(Long userId) {{
        if (userId != null) {{
            super.setCreateBy(userId.toString());
        }}
    }}

    public Long getUpdateUserId() {{
        if (super.getUpdateBy() != null) {{
            return Long.valueOf(super.getUpdateBy());
        }} else {{
            return null;
        }}
    }}

    public void setUpdateUserId(Long userId) {{
        if (userId != null) {{
            super.setUpdateBy(userId.toString());
        }}
    }}
}}
"#
  );

  let class_dir_path = module_path
    .join("src/main/java")
    .join(base_package.replace(".", "/"))
    .join("core/domain");
  fs::create_dir_all(&class_dir_path)?;
  fs::write(class_dir_path.join("BizEntity.java"), class_content)?;

  Ok("".to_string())
}

fn update_root_pom_xml(
  project_root_dir: &Path,
  module_name: &str,
  module_description: &str,
) -> Result<()> {
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
          "dependencies" => {
            root_pom_writer.write_indent()?;
            root_pom_writer.write_event(Event::Comment(BytesText::new(module_description)))?;

            root_pom_writer.write_event(Event::Start(BytesStart::new("dependency")))?;

            root_pom_writer.write_event(Event::Start(BytesStart::new("groupId")))?;
            root_pom_writer.write_event(Event::Text(BytesText::new("com.ruoyi")))?;
            root_pom_writer.write_event(Event::End(BytesEnd::new("groupId")))?;

            root_pom_writer.write_event(Event::Start(BytesStart::new("artifactId")))?;
            root_pom_writer.write_event(Event::Text(BytesText::new(module_name)))?;
            root_pom_writer.write_event(Event::End(BytesEnd::new("artifactId")))?;

            root_pom_writer.write_event(Event::Start(BytesStart::new("version")))?;
            root_pom_writer.write_event(Event::Text(BytesText::new("${ruoyi.version}")))?;
            root_pom_writer.write_event(Event::End(BytesEnd::new("version")))?;

            root_pom_writer.write_event(Event::End(BytesEnd::new("dependency")))?;
          }
          "modules" => {
            root_pom_writer.write_event(Event::Text(BytesText::new(indent_str.as_str())))?;
            root_pom_writer.write_event(Event::Start(BytesStart::new("module")))?;
            root_pom_writer.write_event(Event::Text(BytesText::new(module_name)))?;
            root_pom_writer.write_event(Event::End(BytesEnd::new("module")))?;
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
  Ok(())
}

fn update_ruoyi_admin_pom_xml(
  project_root_dir: &Path,
  module_name: &str,
  module_description: &str,
) -> Result<()> {
  // 往 ruoyi_admin 的 pom.xml 中添加依赖节点
  let pom_path = project_root_dir.join("ruoyi_admin/pom.xml");
  let mut pom_reader = Reader::from_file(&pom_path)?;
  pom_reader.config_mut().trim_text(false);
  let indent_char = b' ';
  let indent_size = 4;
  let mut pom_writer = Writer::new_with_indent(Cursor::new(Vec::new()), indent_char, indent_size);
  let mut buf = Vec::new();

  let indent_str = String::from_utf8(vec![indent_char; indent_size])?;

  loop {
    match pom_reader.read_event_into(&mut buf) {
      Ok(Event::Start(e)) => {
        pom_writer.write_event(Event::Start(e))?;
      }
      Ok(Event::End(e)) => {
        if e.name().as_ref() == "dependencies" {
          pom_writer.write_indent()?;
          pom_writer.write_event(Event::Comment(BytesText::new(module_description)))?;

          pom_writer.write_event(Event::Start(BytesStart::new("dependency")))?;

          pom_writer.write_event(Event::Start(BytesStart::new("groupId")))?;
          pom_writer.write_event(Event::Text(BytesText::new("com.ruoyi")))?;
          pom_writer.write_event(Event::End(BytesEnd::new("groupId")))?;

          pom_writer.write_event(Event::Start(BytesStart::new("artifactId")))?;
          pom_writer.write_event(Event::Text(BytesText::new(module_name)))?;
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
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_update_root_pom_xml() {
    update_root_pom_xml("resources/project1".as_ref(), "ruoyi-test", "测试模块").unwrap();
  }

  #[test]
  fn test_update_ruoyi_admin_pom_xml() {
    update_ruoyi_admin_pom_xml("resources/project1".as_ref(), "ruoyi-test", "测试模块").unwrap();
  }

  #[test]
  fn test_new_biz_entity_java() {
    new_biz_entity_java("resources/project1/server".as_ref(), "com.ruoyi.demo").unwrap();
  }
}
