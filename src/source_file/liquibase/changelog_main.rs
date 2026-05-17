use anyhow::Result;
use quick_xml::events::{BytesStart, BytesText, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

pub fn append_include_tag_to_changelog_file(
    changelog_file_path: &str,
    include_file_path: &str,
) -> Result<()> {
    let xml_file_path = PathBuf::from(changelog_file_path);
    if !xml_file_path.exists() {
        return Err(anyhow::anyhow!("文件不存在"));
    }

    let modified_xml = add_include_tag(changelog_file_path, &[include_file_path])?;
    fs::write(xml_file_path, modified_xml)?;

    Ok(())
}

pub fn append_include_tag_list_to_changelog_file(
    changelog_file_path: &str,
    include_file_path_list: &[&str],
) -> Result<()> {
    let xml_file_path = PathBuf::from(changelog_file_path);
    if !xml_file_path.exists() {
        return Err(anyhow::anyhow!("文件不存在"));
    }

    let modified_xml = add_include_tag(changelog_file_path, include_file_path_list)?;
    fs::write(xml_file_path, modified_xml)?;

    Ok(())
}

fn add_include_tag(changelog_file_path: &str, include_file_path_list: &[&str]) -> Result<String> {
    let mut reader = Reader::from_file(changelog_file_path)?;

    reader.config_mut().trim_text(false);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"databaseChangeLog" => {
                writer.write_event(Event::Start(e.clone()))?;
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"databaseChangeLog" => {
                for &include_file_path in include_file_path_list {
                    writer.write_event(Event::Text(BytesText::new("    ")))?;
                    // 在关闭标签前添加新的 include 节点
                    let mut include = BytesStart::new("include");
                    include.push_attribute(("file", include_file_path));
                    writer.write_event(Event::Empty(include))?;

                    // 写入换行符保持格式
                    writer.write_event(Event::Text(BytesText::new("\n")))?;
                }
                // 写入原有的关闭标签
                writer.write_event(Event::End(e.clone()))?;
            }
            Ok(Event::Eof) => break,
            Ok(event) => writer.write_event(event)?,
            Err(e) => return Err(e.into()),
        }
        buf.clear();
    }
    let result = String::from_utf8(writer.into_inner().into_inner())?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_include_tag_to_changelog_file() {
        append_include_tag_to_changelog_file("resources/db.changelog-master.xml", "db/a.xml")
            .unwrap();
        append_include_tag_list_to_changelog_file(
            "resources/db.changelog-master.xml",
            &["db/b.xml", "db/c.xml"],
        )
        .unwrap();
    }
}
