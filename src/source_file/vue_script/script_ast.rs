use std::borrow::Cow;
use std::fmt::Arguments;
use oxc_allocator::{Allocator, ArenaBox, ArenaVec, GetAllocator};
use oxc_ast::ast::{Argument, ArrayExpression, ArrowFunctionExpression, BindingIdentifier, BindingPattern, BlockStatement, BooleanLiteral, CallExpression, Directive, Expression, ExpressionStatement, FormalParameter, FormalParameterKind, FormalParameterRest, FormalParameters, FunctionBody, IdentifierName, IdentifierReference, ImportDeclaration, ImportDeclarationSpecifier, ImportDefaultSpecifier, ImportOrExportKind, ImportSpecifier, MemberExpression, ModuleExportName, NumberBase, NumericLiteral, Program, Statement, StaticMemberExpression, StringLiteral, TSArrayType, TSBooleanKeyword, TSNumberKeyword, TSStringKeyword, TSType, TSTypeAnnotation, TSTypeName, TSTypeParameterDeclaration, TSTypeParameterInstantiation, TSTypeReference, VariableDeclaration, VariableDeclarationKind, VariableDeclarator, WithClause};
use oxc_ast::{AstBuilder, Comment};
use oxc_ast::builder::GetAstBuilder;
use oxc_codegen::Codegen;
use oxc_span::{SourceType, SPAN};

pub enum NamedImportItem<'a> {
   /// 普通值导入：`name`
   Value(&'a str),
   /// 类型导入：`type name`
   Type(&'a str),
}

pub struct ScriptAst<'a> {
   pub builder: AstBuilder<'a>,
   pub statements: ArenaVec<'a, Statement<'a>>,
   pub comments: ArenaVec<'a, Comment>,
   pub directives: ArenaVec<'a, Directive<'a>>,
}

impl<'a> GetAstBuilder<'a> for ScriptAst<'a> {
   type Builder = AstBuilder<'a>;

   fn builder(&self) -> &AstBuilder<'a> {
      &self.builder
   }
}

impl <'a> GetAllocator<'a> for ScriptAst<'a> {
   fn allocator(&self) -> &'a Allocator {
      self.builder().allocator()
   }
}

impl<'a> ScriptAst<'a> {
   pub fn new(allocator: &'a Allocator) -> Self {
      let builder = AstBuilder::new(allocator);
      let statements = ArenaVec::new_in(&builder);
      let comments = ArenaVec::new_in(&builder);
      let directives = ArenaVec::new_in(&builder);
      Self {
         builder,
         statements,
         comments,
         directives,
      }
   }

   //region import
   /// 默认导入
   /// `import local from "source";`
   pub fn add_import_default(&mut self, source: &'a str, default_import: &'a str) {
      // 创建模块源字符串字面量
      let source_literal = StringLiteral::new(SPAN, source, None, self);

      // 绑定标识符，默认导入的本地变量名
      let local = BindingIdentifier::new(SPAN, default_import, self);
      // 默认导入说明符
      let specifier = ImportDefaultSpecifier::boxed(SPAN, local, self);
      // 包装为统一的导入说明符枚举
      let specifier = ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier);
      // 创建存放说明符的列表
      let mut specifiers = ArenaVec::new_in(self);
      specifiers.push(specifier);

      let import_declaration = ImportDeclaration::boxed(
         SPAN,
         Some(specifiers),
         source_literal,
         None,
         None::<WithClause<'a>>,
         ImportOrExportKind::Value,
         self);
      
      self.statements.push(Statement::ImportDeclaration(import_declaration));
   }

   /// 命名导入
   /// `import { a, b } from "source";`
   pub fn add_import_named_value(&mut self, source: &'a str, named_imports: &[&'a str]) {
      if named_imports.is_empty() {
         return;
      }

      let source_literal = StringLiteral::new(SPAN, source, None, self);

      let mut specifiers = ArenaVec::new_in(self);
      for &name in named_imports {
         // 本地作用域绑定名称

         let imported = ModuleExportName::IdentifierName(IdentifierName::new(SPAN, name, self));
         let local = BindingIdentifier::new(SPAN, name, self);

         // 同名导入（无 as 别名）时 imported 传 None，代码生成自动复用 local 名称
         let specifier = ImportSpecifier::boxed(SPAN, imported, local, ImportOrExportKind::Value, self);
         specifiers.push(ImportDeclarationSpecifier::ImportSpecifier(specifier));
      }

      let import_declaration = ImportDeclaration::boxed(
         SPAN,
         Some(specifiers),
         source_literal,
         None,
         None::<WithClause<'a>>,
         ImportOrExportKind::Value,
         self,
      );

      self.statements.push(Statement::ImportDeclaration(import_declaration));
   }

   /// 命名导入
   /// `import type { a, b } from "source";`
   pub fn add_import_named_type(&mut self, source: &'a str, named_imports: &[&'a str]) {
      if named_imports.is_empty() {
         return;
      }

      let source_literal = StringLiteral::new(SPAN, source, None, self);

      let mut specifiers = ArenaVec::new_in(self);
      for &name in named_imports {
         // 本地作用域绑定名称

         let imported = ModuleExportName::IdentifierName(IdentifierName::new(SPAN, name, self));
         let local = BindingIdentifier::new(SPAN, name, self);

         // 同名导入（无 as 别名）时 imported 传 None，代码生成自动复用 local 名称
         let specifier = ImportSpecifier::boxed(SPAN, imported, local, ImportOrExportKind::Value, self);
         specifiers.push(ImportDeclarationSpecifier::ImportSpecifier(specifier));
      }

      let import_declaration = ImportDeclaration::boxed(
         SPAN,
         Some(specifiers),
         source_literal,
         None,
         None::<WithClause<'a>>,
         ImportOrExportKind::Type,
         self,
      );

      self.statements.push(Statement::ImportDeclaration(import_declaration));
   }

   /// 混合命名导入（支持值与 type 项混排）
   /// 示例：`import { a, type b } from "source";`
   pub fn add_import_named_all(&mut self, source: &'a str, named_imports: &[NamedImportItem<'a>]) {
      if named_imports.is_empty() {
         return;
      }

      let source_literal = StringLiteral::new(SPAN, source, None, self);
      let mut specifiers = ArenaVec::new_in(self);

      for item in named_imports {
         let (name, kind) = match item {
            NamedImportItem::Value(name) => (*name, ImportOrExportKind::Value),
            NamedImportItem::Type(name) => (*name, ImportOrExportKind::Type),
         };

         let imported = ModuleExportName::IdentifierName(IdentifierName::new(SPAN, name, self));
         let local = BindingIdentifier::new(SPAN, name, self);
         let specifier = ImportSpecifier::boxed(SPAN, imported, local, kind, self);
         specifiers.push(ImportDeclarationSpecifier::ImportSpecifier(specifier));
      }

      let import_declaration = ImportDeclaration::boxed(
         SPAN,
         Some(specifiers),
         source_literal,
         None,
         None::<WithClause<'a>>,
         // 混合导入顶层必须为 Value，type 修饰由单个 specifier 控制
         ImportOrExportKind::Value,
         self,
      );

      self.statements.push(Statement::ImportDeclaration(import_declaration));
   }
   //endregion

   //region argument 实参
   pub fn new_argument_string(&self, value: &'a str) -> Argument<'a> {
      let literal = StringLiteral::boxed(SPAN, value, None, self);
      Argument::StringLiteral(literal)
   }

   pub fn new_argument_number(&self, value: f64) -> Argument<'a> {
      let literal = NumericLiteral::boxed(SPAN, value, None, NumberBase::Decimal, self);
      Argument::NumericLiteral(literal)
   }

   pub fn new_argument_boolean(&self, value: bool) -> Argument<'a> {
      let literal = BooleanLiteral::boxed(SPAN, value, self);
      Argument::BooleanLiteral(literal)
   }

   pub fn new_argument_identifier(&self, name: &'a str) -> Argument<'a> {
      Argument::Identifier(IdentifierReference::boxed(SPAN, name, self))
   }

   pub fn new_argument_empty_array(&self) -> Argument<'a> {
      let empty_array = ArrayExpression::boxed(SPAN, ArenaVec::new_in(self), self);
      Argument::ArrayExpression(empty_array)
   }
   //endregion

   //region const
   pub fn add_const_string(&mut self, name: &'a str, value: &'a str) {
      let value_literal = StringLiteral::boxed(SPAN, value, None, self);
      let init_expr = Expression::StringLiteral(value_literal);

      self.add_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
   }

   pub fn add_const_integer(&mut self, name: &'a str, value: i64) {
      let value_literal = NumericLiteral::boxed(SPAN, value as f64, None, NumberBase::Decimal, self);
      let init_expr = Expression::NumericLiteral(value_literal);

      self.add_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
   }

   pub fn add_const_float(&mut self, name: &'a str, value: f64) {
      let value_literal = NumericLiteral::boxed(SPAN, value, None, NumberBase::Float, self);
      let init_expr = Expression::NumericLiteral(value_literal);

      self.add_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
   }

   pub fn add_const_boolean(&mut self, name: &'a str, value: bool) {
      let value_literal = BooleanLiteral::boxed(SPAN, value, self);
      let init_expr = Expression::BooleanLiteral(value_literal);

      self.add_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
   }

   pub fn add_const_ref_boolean(&mut self, name: &'a str, value: bool) {
      let argument = self.new_argument_boolean(value);
      let ts_type = TSType::TSBooleanKeyword(TSBooleanKeyword::boxed(SPAN, self));

      let init_expr = self.call_ref(ts_type, argument);

      self.add_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
   }

   pub fn add_const_ref_string(&mut self, name: &'a str, value: &'a str) {
      // 函数的输入参数,字符串字面量
      let argument = self.new_argument_string(value);
      let ts_type = TSType::TSStringKeyword(TSStringKeyword::boxed(SPAN, self));

      let init_expr = self.call_ref(ts_type, argument);

      self.add_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
   }

   pub fn add_const_ref_number(&mut self, name: &'a str, value: f64) {
      let argument = self.new_argument_number(value);
      let ts_type = TSType::TSNumberKeyword(TSNumberKeyword::boxed(SPAN, self));

      let init_expr = self.call_ref(ts_type, argument);

      self.add_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
   }

   pub fn add_const_ref_string_array(&mut self, name: &'a str) {
      let empty_array_argument = self.new_argument_empty_array();

      let ts_type = TSType::TSStringKeyword(TSStringKeyword::boxed(SPAN, self));
      let ts_array_type = TSType::TSArrayType(TSArrayType::boxed(SPAN, ts_type, self));

      let init_expr = self.call_ref(ts_array_type, empty_array_argument);

      self.add_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
   }

   pub fn add_const_ref_number_array(&mut self, name: &'a str) {
      let empty_array_argument = self.new_argument_empty_array();

      let ts_type = TSType::TSNumberKeyword(TSNumberKeyword::boxed(SPAN, self));
      let ts_array_type = TSType::TSArrayType(TSArrayType::boxed(SPAN, ts_type, self));

      let init_expr = self.call_ref(ts_array_type, empty_array_argument);

      self.add_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
   }

   pub fn add_const_ref_object_array(&mut self, name: &'a str, type_name: &'a str) {
      let empty_array_argument = self.new_argument_empty_array();

      let ident_ref = IdentifierReference::boxed(SPAN, type_name, self);
      let type_ref = TSTypeReference::boxed(
         SPAN,
         TSTypeName::IdentifierReference(ident_ref),
         None::<TSTypeParameterInstantiation>,
         self
      );
      let ts_type = TSType::TSTypeReference(type_ref);
      let ts_array_type = TSType::TSArrayType(TSArrayType::boxed(SPAN, ts_type, self));

      let init_expr = self.call_ref(ts_array_type, empty_array_argument);

      self.add_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
   }

   fn call_ref(&mut self, ts_type: TSType<'a>, argument: Argument<'a>) -> Expression<'a> {
      // 调用 ref 函数
      let ref_ident_reference = IdentifierReference::boxed(SPAN, "ref", self);
      let ref_expr = Expression::Identifier(ref_ident_reference);

      let mut input_arguments = ArenaVec::new_in(self);
      input_arguments.push(argument);

      // 函数的泛型参数
      let mut type_parameters = ArenaVec::new_in(self);
      type_parameters.push(ts_type);
      let type_parameters_instantiation = TSTypeParameterInstantiation::new(SPAN, type_parameters, self);

      // 组装完整表达式
      let call_expr = CallExpression::boxed(
         SPAN,
         ref_expr,
         Some(type_parameters_instantiation),
         input_arguments,
         false,
         self
      );

      Expression::CallExpression(call_expr)
   }

   fn add_variable_declaration(&mut self, kind: VariableDeclarationKind, name: &'a str, init_expr: Expression<'a>) {
      let binding_identifier = BindingIdentifier::boxed(SPAN, name, self);
      let id = BindingPattern::BindingIdentifier(binding_identifier);

      let declarator = VariableDeclarator::new(
         SPAN,
         kind,
         id,
         None::<TSTypeAnnotation<'a>>,
         Some(init_expr),
         false,
         self
      );

      let mut declarations = ArenaVec::new_in(self);
      declarations.push(declarator);

      let var_declaration = VariableDeclaration::boxed(
         SPAN,
         kind,
         declarations,
         false,
         self
      );

      self.statements.push(Statement::VariableDeclaration(var_declaration));
   }
   //endregion

   //region arrow function


   pub fn new_empty_function_arguments(&self) -> oxc_allocator::Vec<'a, Argument<'a>> {
      ArenaVec::new_in(self)
   }

   pub fn new_empty_function_body(&self) -> oxc_allocator::Box<'a, FunctionBody<'a>> {
      FunctionBody::boxed(SPAN, ArenaVec::new_in(self), ArenaVec::new_in(self), self)
   }

   pub fn new_empty_arrow_function_params(&self) -> oxc_allocator::Box<'a, FormalParameters<'a>> {
      FormalParameters::boxed(
         SPAN,
         FormalParameterKind::ArrowFormalParameters,
         ArenaVec::new_in(self),
         None::<FormalParameterRest>,
         self
      )
   }

   pub fn new_formal_string_parameter(&self, name: &'a str) -> FormalParameter<'a> {
      let bind_ident = BindingIdentifier::boxed(SPAN, name, self);
      let pattern = BindingPattern::BindingIdentifier(bind_ident);

      let ts_type = TSType::TSStringKeyword(TSStringKeyword::boxed(SPAN, self));
      let ts_type_anno = TSTypeAnnotation::boxed(SPAN, ts_type, self);

      FormalParameter::new(
         SPAN,
         ArenaVec::new_in(self),
         pattern,
         Some(ts_type_anno),
         None::<Expression>,
         false,
         None,
         false,
         false,
         self
      )
   }

   fn new_call_object_method(&self, object_name: &'a str, method_name: &'a str, args: ArenaVec<'a, Argument<'a>>) -> Statement<'a> {
      let object_expr = Expression::Identifier(IdentifierReference::boxed(SPAN, object_name, self));
      let method_name_ident = IdentifierName::new(SPAN, method_name, self);
      let static_member = StaticMemberExpression::boxed(SPAN, object_expr, method_name_ident, false, self);
      let callee = Expression::StaticMemberExpression(static_member);

      let call_expr = CallExpression::boxed(
         SPAN,
         callee,
         None::<TSTypeParameterInstantiation>,
         args,
         false,
         self);
      let expression = Expression::CallExpression(call_expr);

      Statement::ExpressionStatement(ExpressionStatement::boxed(SPAN, expression, self))
   }

   pub fn append_statement(&self, function_body: &mut ArenaBox<'a, FunctionBody<'a>>, statement: Statement<'a>) {
      function_body.statements.push(statement);
   }

   pub fn append_formal_parameter(&self, parameters: &mut ArenaBox<'a, FormalParameters<'a>>, parameter: FormalParameter<'a> ) {
      parameters.items.push(parameter);
   }

   pub fn append_argument(&self, arguments: &mut ArenaVec<'a, Argument<'a>>, argument: Argument<'a>) {
      arguments.push(argument);
   }

   fn new_call_console_log(&self, args: ArenaVec<'a, Argument<'a>>) -> Statement<'a> {
      self.new_call_object_method("console", "log", args)
   }

   /// 以add开头的方法，都是直接在root节点中添加子节点
   fn add_call_console_log(&mut self, args: ArenaVec<'a, Argument<'a>>) {
      let statement = self.new_call_object_method("console", "log", args);
      self.statements.push(statement);
   }

   pub fn add_arrow_function(&mut self, name: &'a str, params: ArenaBox<'a, FormalParameters<'a>>, body: ArenaBox<'a, FunctionBody<'a>>) {
      let arrow_expr = ArrowFunctionExpression::boxed(
         SPAN,
         false,
         false,
         None::<TSTypeParameterDeclaration>,
         params,
         None::<TSTypeAnnotation>,
         body,
         self
      );
      let init_expr = Expression::ArrowFunctionExpression(arrow_expr);

      self.add_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
   }
   //endregion

   pub fn to_code(self) -> String {
      let program = Program::new(
         SPAN,
         SourceType::ts(),
         "",
         self.comments,
         None,
         self.directives,
         self.statements,
         &self.builder
      );
      let codegen_return = Codegen::new().build(&program);
      codegen_return.code
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn test_empty() {
      let allocator = Allocator::new();
      let script_ast = ScriptAst::new(&allocator);
      let actual_code = script_ast.to_code();
      assert_eq!(actual_code, "");
   }

   #[test]
   fn test_import_default() {
      let allocator = Allocator::new();
      let mut script_ast = ScriptAst::new(&allocator);
      script_ast.add_import_default("module-a", "a");
      let actual_code = script_ast.to_code();
      assert_eq!(actual_code, "import a from \"module-a\";\n");
   }

   #[test]
   fn test_import_named_value() {
      let allocator = Allocator::new();
      let mut script_ast = ScriptAst::new(&allocator);
      script_ast.add_import_named_value("source", &["a", "b"]);
      let actual_code = script_ast.to_code();
      assert_eq!(actual_code, "import { a, b } from \"source\";\n");
   }

   #[test]
   fn test_import_named_type() {
      let allocator = Allocator::new();
      let mut script_ast = ScriptAst::new(&allocator);
      script_ast.add_import_named_type("source", &["a", "b"]);
      let actual_code = script_ast.to_code();
      assert_eq!(actual_code, "import type { a, b } from \"source\";\n");
   }

   #[test]
   fn test_add_import_named_all() {
      let allocator = Allocator::new();
      let mut script_ast = ScriptAst::new(&allocator);
      script_ast.add_import_named_all("source", &[NamedImportItem::Value("a"), NamedImportItem::Type("b")]);
      let actual_code = script_ast.to_code();
      assert_eq!(actual_code, "import { a, type b } from \"source\";\n");
   }

   #[test]
   fn test_add_const_string() {
      let allocator = Allocator::new();
      let mut script_ast = ScriptAst::new(&allocator);
      script_ast.add_const_string("a", "b");
      let actual_code = script_ast.to_code();
      assert_eq!(actual_code, "const a = \"b\";\n");
   }

   #[test]
   fn test_add_const_integer() {
      let allocator = Allocator::new();
      let mut script_ast = ScriptAst::new(&allocator);
      script_ast.add_const_integer("a", 1);
      let actual_code = script_ast.to_code();
      assert_eq!(actual_code, "const a = 1;\n");
   }

   #[test]
   fn test_add_const_float() {
      let allocator = Allocator::new();
      let mut script_ast = ScriptAst::new(&allocator);
      script_ast.add_const_float("a", 1.2);
      let actual_code = script_ast.to_code();
      assert_eq!(actual_code, "const a = 1.2;\n");
   }

   #[test]
   fn test_add_const_boolean() {
      let allocator = Allocator::new();
      let mut script_ast = ScriptAst::new(&allocator);
      script_ast.add_const_boolean("a", false);
      let actual_code = script_ast.to_code();
      assert_eq!(actual_code, "const a = false;\n");
   }

   #[test]
   fn test_add_const_ref_string() {
      let allocator = Allocator::new();
      let mut script_ast = ScriptAst::new(&allocator);
      script_ast.add_const_ref_string("a", "");
      let actual_code = script_ast.to_code();
      assert_eq!(actual_code, "const a = ref<string>(\"\");\n");
   }

   #[test]
   fn test_add_const_ref_number() {
      let allocator = Allocator::new();
      let mut script_ast = ScriptAst::new(&allocator);
      script_ast.add_const_ref_number("a", 0.0);
      let actual_code = script_ast.to_code();
      assert_eq!(actual_code, "const a = ref<number>(0);\n");
   }

   #[test]
   fn test_add_const_ref_boolean() {
      let allocator = Allocator::new();
      let mut script_ast = ScriptAst::new(&allocator);
      script_ast.add_const_ref_boolean("a", false);
      let actual_code = script_ast.to_code();
      assert_eq!(actual_code, "const a = ref<boolean>(false);\n");
   }

   #[test]
   fn test_add_const_ref_string_array() {
      let allocator = Allocator::new();
      let mut script_ast = ScriptAst::new(&allocator);
      script_ast.add_const_ref_string_array("a");
      let actual_code = script_ast.to_code();
      assert_eq!(actual_code, "const a = ref<string[]>([]);\n");
   }

   #[test]
   fn test_add_const_ref_number_array() {
      let allocator = Allocator::new();
      let mut script_ast = ScriptAst::new(&allocator);
      script_ast.add_const_ref_number_array("a");
      let actual_code = script_ast.to_code();
      assert_eq!(actual_code, "const a = ref<number[]>([]);\n");
   }

   #[test]
   fn test_add_const_ref_object_array() {
      let allocator = Allocator::new();
      let mut script_ast = ScriptAst::new(&allocator);
      script_ast.add_const_ref_object_array("a", "UserInfo");
      let actual_code = script_ast.to_code();
      assert_eq!(actual_code, "const a = ref<UserInfo[]>([]);\n");
   }

   #[test]
   fn test_add_arrow_function_empty() {
      let allocator = Allocator::new();
      let mut script_ast = ScriptAst::new(&allocator);
      let parameters = script_ast.new_empty_arrow_function_params();
      let function_body = script_ast.new_empty_function_body();
      script_ast.add_arrow_function("a", parameters, function_body);
      let actual_code = script_ast.to_code();
      assert_eq!(actual_code, "const a = () => {};\n");
   }

   #[test]
   fn test_new_call_console_log() {
      let allocator = Allocator::new();
      let mut script_ast = ScriptAst::new(&allocator);
      let parameters = script_ast.new_empty_arrow_function_params();
      let function_body = script_ast.new_empty_function_body();

      let mut arguments = script_ast.new_empty_function_arguments();
      script_ast.append_argument(&mut arguments, script_ast.new_argument_string("a"));
      script_ast.append_argument(&mut arguments, script_ast.new_argument_number(1.0));
      script_ast.append_argument(&mut arguments, script_ast.new_argument_boolean(true));

      script_ast.add_call_console_log(arguments);
      let actual_code = script_ast.to_code();
      assert_eq!(actual_code, "console.log(\"a\", 1, true);\n");
   }

   #[test]
   fn test_add_arrow_function_one_parameter_one_statement() {
      let allocator = Allocator::new();
      let mut script_ast = ScriptAst::new(&allocator);
      let mut parameters = script_ast.new_empty_arrow_function_params();
      let parameter = script_ast.new_formal_string_parameter("b");
      script_ast.append_formal_parameter(&mut parameters, parameter);
      let mut function_body = script_ast.new_empty_function_body();

      let mut function_arguments = script_ast.new_empty_function_arguments();
      let argument = script_ast.new_argument_identifier("b");
      script_ast.append_argument(&mut function_arguments, argument);
      let console_log = script_ast.new_call_console_log(function_arguments);
      script_ast.append_statement(&mut function_body, console_log);
      script_ast.add_arrow_function("a", parameters, function_body);
      let actual_code = script_ast.to_code();
      assert_eq!(actual_code, "const a = (b: string) => {\n\tconsole.log(b);\n};\n");
   }
}
