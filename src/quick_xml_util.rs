use anyhow::Result;
use quick_xml::Writer;
use quick_xml::events::{BytesText, Event};
use std::io::Cursor;

pub(crate) fn write_two_new_lines(writer: &mut Writer<Cursor<Vec<u8>>>) -> Result<()> {
  write_new_lines(writer, 2)
}

pub(crate) fn write_one_new_line(writer: &mut Writer<Cursor<Vec<u8>>>) -> Result<()> {
  write_new_lines(writer, 1)
}

pub(crate) fn write_new_lines(
  writer: &mut Writer<Cursor<Vec<u8>>>,
  line_count: usize,
) -> Result<()> {
  assert!(line_count > 0);

  let line_chars = "\n".repeat(line_count);
  writer.write_event(Event::Text(BytesText::new(line_chars.as_str())))?;
  Ok(())
}

pub(crate) fn write_new_line_ident(writer: &mut Writer<Cursor<Vec<u8>>>) -> Result<()> {
  writer.write_event(Event::Text(BytesText::new("\n\n    ")))?;
  Ok(())
}

pub(crate) fn write_new_line_then_n_ident(
  writer: &mut Writer<Cursor<Vec<u8>>>,
  level: usize,
) -> Result<()> {
  writer.write_event(Event::Text(BytesText::new(
    format!("\n{}", "    ".repeat(level)).as_str(),
  )))?;
  Ok(())
}
