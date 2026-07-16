use napi::bindgen_prelude::*;
use napi_derive::napi;
use oxc_allocator::Allocator;
use crate::source_file::vue_script::script_ast::ScriptAst;
use crate::ui_types::UIPage;

#[napi]
pub fn generate_vue_script(ui_page: UIPage) -> Result<String> {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);

    script_ast.add_import_named_value("vue", &["ref", "reactive"]);
    let code = script_ast.to_code();
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_generate_vue_script() {
        let ui_page = UIPage {
            parent_paths: vec![],
            content: "".to_string(),
        };
        let actual = generate_vue_script(ui_page).unwrap();
        let expected = r#"import { ref, reactive } from "vue";
"#;
        assert_eq!(expected, actual);
    }
}