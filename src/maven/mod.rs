use quick_xml::Reader;
use quick_xml::events::Event;
use std::path::Path;
// 如果文件夹不存在，返回false
// 如果项目根目录下没有 pom.xml 文件，返回 false
// 如果 pom.xml 中的 packaging 的值不是 pom，返回 false
// 如果存在 modules 节点，返回 true
// 返回 false
pub fn is_maven_multiple_module_project(project_root_path: &str) -> bool {
  let root_dir = Path::new(project_root_path);
  if !root_dir.is_dir() {
    return false;
  }
  let pom_path = root_dir.join("pom.xml");
  if !pom_path.exists() {
    return false;
  }
  let mut pom_reader = match Reader::from_file(pom_path) {
    Ok(reader) => reader,
    Err(_) => return false,
  };
  pom_reader.config_mut().trim_text(true);

  let mut buf = Vec::new();
  let mut in_packaging = false;
  let mut in_modules = false;
  let mut in_module = false;
  let mut packaging_value = String::new();
  let mut modules_count = 0;

  loop {
    match pom_reader.read_event_into(&mut buf) {
      Ok(Event::Start(e)) => match e.name().as_ref() {
        "packaging" => in_packaging = true,
        "modules" => in_modules = true,
        "module" if in_modules => in_module = true,
        _ => {}
      },
      Ok(Event::Text(e)) => {
        let text = e.xml10_content();
        if in_packaging {
          packaging_value = text.to_string();
        }
        if in_module {
          modules_count += 1;
        }
      }
      Ok(Event::End(e)) => match e.name().as_ref() {
        "packaging" => in_packaging = false,
        "modules" => in_modules = false,
        "module" => in_module = false,
        _ => {}
      },
      Ok(Event::Eof) => break,
      Err(_) => return false,
      _ => {}
    }

    // 判断完就退出
    if !packaging_value.is_empty() {
      if packaging_value != "pom" {
        break;
      }
      if modules_count > 0 {
        break;
      }
    }

    buf.clear();
  }

  packaging_value == "pom" && modules_count > 0
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_is_maven_multiple_module_project() {}
}
