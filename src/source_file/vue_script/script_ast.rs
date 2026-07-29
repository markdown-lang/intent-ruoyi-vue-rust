use oxc_allocator::{Allocator, ArenaBox, ArenaVec, GetAllocator, IntoIn,  Vec};
use oxc_ast::ast::{Argument, ArrayExpression, ArrowFunctionExpression, AssignmentExpression, AssignmentOperator, AssignmentTarget, BindingIdentifier, BindingPattern, BindingProperty, BindingRestElement, BlockStatement, BooleanLiteral, CallExpression, CatchClause, CatchParameter, Declaration, Directive, ExportNamedDeclaration, Expression, ExpressionStatement, FormalParameter, FormalParameterKind, FormalParameterRest, FormalParameters, FunctionBody, IdentifierName, IdentifierReference, ImportDeclaration, ImportDeclarationSpecifier, ImportDefaultSpecifier, ImportOrExportKind, ImportSpecifier, MemberExpression, ModuleExportName, NumberBase, NumericLiteral, ObjectExpression, ObjectPattern, ObjectPropertyKind, Program, PropertyKey, PropertyKind, ReturnStatement, Statement, StaticMemberExpression, StringLiteral, TSArrayType, TSBooleanKeyword, TSInterfaceBody, TSInterfaceDeclaration, TSInterfaceHeritage, TSNumberKeyword, TSPropertySignature, TSSignature, TSStringKeyword, TSType, TSTypeAliasDeclaration, TSTypeAnnotation, TSTypeName, TSTypeParameterDeclaration, TSTypeParameterInstantiation, TSTypeReference, TSUnknownKeyword, TemplateLiteral, TryStatement, VariableDeclaration, VariableDeclarationKind, VariableDeclarator, WithClause};
use oxc_ast::builder::{AstBuilder, GetAstBuilder};
use oxc_ast::{Comment, CommentContent, CommentKind};
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_span::{SPAN, SourceType};

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

impl<'a> GetAllocator<'a> for ScriptAst<'a> {
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
      self,
    );

    self
      .statements
      .push(Statement::ImportDeclaration(import_declaration));
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
      let specifier =
        ImportSpecifier::boxed(SPAN, imported, local, ImportOrExportKind::Value, self);
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

    self
      .statements
      .push(Statement::ImportDeclaration(import_declaration));
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
      let specifier =
        ImportSpecifier::boxed(SPAN, imported, local, ImportOrExportKind::Value, self);
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

    self
      .statements
      .push(Statement::ImportDeclaration(import_declaration));
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

    self
      .statements
      .push(Statement::ImportDeclaration(import_declaration));
  }
  //endregion

  //region argument 实参
  pub fn new_argument_string(&self, value: &'a str) -> Argument<'a> {
    let literal = StringLiteral::boxed(SPAN, value, None, self);
    Argument::StringLiteral(literal)
  }

  pub fn new_argument_decimal(&self, value: f64) -> Argument<'a> {
    let literal = NumericLiteral::boxed(SPAN, value, None, NumberBase::Decimal, self);
    Argument::NumericLiteral(literal)
  }

  pub fn new_argument_float(&self, value: f64) -> Argument<'a> {
    let literal = NumericLiteral::boxed(SPAN, value, None, NumberBase::Float, self);
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

  //region 基本类型表达式
  fn new_expression_string(&self, value: &'a str) -> Expression<'a> {
    Expression::StringLiteral(StringLiteral::boxed(SPAN, value, None, self))
  }

  fn new_expression_decimal(&self, value: i64) -> Expression<'a> {
    Expression::NumericLiteral(NumericLiteral::boxed(SPAN, value as f64, None, NumberBase::Decimal, self))
  }

  fn new_expression_float(&self, value: f64) -> Expression<'a> {
    Expression::NumericLiteral(NumericLiteral::boxed(SPAN, value, None, NumberBase::Float, self))
  }

  fn new_expression_boolean(&self, value: bool) -> Expression<'a> {
    Expression::BooleanLiteral(BooleanLiteral::boxed(SPAN, value, self))
  }
  //endregion

  //region const
  pub fn add_const_string(&mut self, name: &'a str, value: &'a str) {
    let init_expr = self.new_expression_string(value);
    self.add_named_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
  }

  pub fn add_const_integer(&mut self, name: &'a str, value: i64) {
    let init_expr = self.new_expression_decimal(value);
    self.add_named_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
  }

  pub fn add_const_float(&mut self, name: &'a str, value: f64) {
    let init_expr = self.new_expression_float(value);
    self.add_named_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
  }

  pub fn add_const_boolean(&mut self, name: &'a str, value: bool) {
    let value_literal = BooleanLiteral::boxed(SPAN, value, self);
    let init_expr = Expression::BooleanLiteral(value_literal);

    self.add_named_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
  }

  pub fn add_const_ref_boolean(&mut self, name: &'a str, value: bool) {
    let argument = self.new_argument_boolean(value);
    let ts_type = TSType::TSBooleanKeyword(TSBooleanKeyword::boxed(SPAN, self));

    let init_expr = self.call_ref(ts_type, argument);

    self.add_named_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
  }

  pub fn add_const_ref_string(&mut self, name: &'a str, value: &'a str) {
    // 函数的输入参数,字符串字面量
    let argument = self.new_argument_string(value);
    let ts_type = TSType::TSStringKeyword(TSStringKeyword::boxed(SPAN, self));

    let init_expr = self.call_ref(ts_type, argument);

    self.add_named_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
  }

  pub fn add_const_ref_number(&mut self, name: &'a str, value: f64) {
    let argument = self.new_argument_decimal(value);
    let ts_type = TSType::TSNumberKeyword(TSNumberKeyword::boxed(SPAN, self));

    let init_expr = self.call_ref(ts_type, argument);

    self.add_named_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
  }

  pub fn add_const_ref_string_array(&mut self, name: &'a str) {
    let empty_array_argument = self.new_argument_empty_array();

    let ts_type = TSType::TSStringKeyword(TSStringKeyword::boxed(SPAN, self));
    let ts_array_type = TSType::TSArrayType(TSArrayType::boxed(SPAN, ts_type, self));

    let init_expr = self.call_ref(ts_array_type, empty_array_argument);

    self.add_named_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
  }

  pub fn add_const_ref_number_array(&mut self, name: &'a str) {
    let empty_array_argument = self.new_argument_empty_array();

    let ts_type = TSType::TSNumberKeyword(TSNumberKeyword::boxed(SPAN, self));
    let ts_array_type = TSType::TSArrayType(TSArrayType::boxed(SPAN, ts_type, self));

    let init_expr = self.call_ref(ts_array_type, empty_array_argument);

    self.add_named_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
  }

  pub fn add_const_ref_object_array(&mut self, name: &'a str, type_name: &'a str) {
    let empty_array_argument = self.new_argument_empty_array();

    let ident_ref = IdentifierReference::boxed(SPAN, type_name, self);
    let type_ref = TSTypeReference::boxed(
      SPAN,
      TSTypeName::IdentifierReference(ident_ref),
      None::<TSTypeParameterInstantiation>,
      self,
    );
    let ts_type = TSType::TSTypeReference(type_ref);
    let ts_array_type = TSType::TSArrayType(TSArrayType::boxed(SPAN, ts_type, self));

    let init_expr = self.call_ref(ts_array_type, empty_array_argument);

    self.add_named_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
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
    let type_parameters_instantiation =
      TSTypeParameterInstantiation::new(SPAN, type_parameters, self);

    // 组装完整表达式
    let call_expr = CallExpression::boxed(
      SPAN,
      ref_expr,
      Some(type_parameters_instantiation),
      input_arguments,
      false,
      self,
    );

    Expression::CallExpression(call_expr)
  }

  fn add_named_variable_declaration(
    &mut self,
    kind: VariableDeclarationKind,
    name: &'a str,
    init_expr: Expression<'a>,
  ) {
    let binding_identifier = BindingIdentifier::boxed(SPAN, name, self);
    let id = BindingPattern::BindingIdentifier(binding_identifier);

    self.add_variable_declaration(kind, id, init_expr);
  }

  fn add_variable_declaration(
    &mut self,
    kind: VariableDeclarationKind,
    id: BindingPattern<'a>,
    init_expr: Expression<'a>,
  ) {
    let declarator = VariableDeclarator::new(
      SPAN,
      kind,
      id,
      None::<TSTypeAnnotation<'a>>,
      Some(init_expr),
      false,
      self,
    );

    let mut declarations = ArenaVec::new_in(self);
    declarations.push(declarator);

    let var_declaration = VariableDeclaration::boxed(SPAN, kind, declarations, false, self);

    self
      .statements
      .push(Statement::VariableDeclaration(var_declaration));
  }
  //endregion

  //region set ref value

  fn new_set_ref_value(&self, name: &'a str, value: Expression<'a>) -> Statement<'a> {
    let ident_ref = IdentifierReference::boxed(SPAN, name, self);
    let object_expr = Expression::Identifier(ident_ref);
    let method_name_ident = IdentifierName::new(SPAN, "value", self);
    let static_member =
      StaticMemberExpression::boxed(SPAN, object_expr, method_name_ident, false, self);
    let left_target = AssignmentTarget::StaticMemberExpression(static_member);

    let assign_expr =
      AssignmentExpression::boxed(SPAN, AssignmentOperator::Assign, left_target, value, self);
    let expression = Expression::AssignmentExpression(assign_expr);

    Statement::ExpressionStatement(ExpressionStatement::boxed(SPAN, expression, self))
  }
  pub fn new_set_ref_string_value(&self, name: &'a str, value: &'a str) -> Statement<'a> {
    let right_expr = self.new_expression_string(value);
    self.new_set_ref_value(name, right_expr)
  }

  pub fn new_set_ref_decimal_value(&self, name: &'a str, value: i64) -> Statement<'a> {
    let right_expr = self.new_expression_decimal(value);
    self.new_set_ref_value(name, right_expr)
  }

  pub fn new_set_ref_boolean_value(&self, name: &'a str, value: bool) -> Statement<'a> {
    let bool_literal = BooleanLiteral::boxed(SPAN, value, self);
    let right_expr = Expression::BooleanLiteral(bool_literal);

    self.new_set_ref_value(name, right_expr)
  }

  pub fn add_set_ref_string_value(&mut self, name: &'a str, value: &'a str) {
    let statement = self.new_set_ref_string_value(name, value);
    self.statements.push(statement);
  }

  pub fn add_set_ref_decimal_value(&mut self, name: &'a str, value: i64) {
    let statement = self.new_set_ref_decimal_value(name, value);
    self.statements.push(statement);
  }

  pub fn add_set_ref_boolean_value(&mut self, name: &'a str, value: bool) {
    let statement = self.new_set_ref_boolean_value(name, value);
    self.statements.push(statement);
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
      self,
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
      self,
    )
  }

  fn new_call_object_method_statement(
    &self,
    object_name: &'a str,
    method_name: &'a str,
    args: ArenaVec<'a, Argument<'a>>,
  ) -> Statement<'a> {
    let expression = self.new_call_object_method_expression(object_name, method_name, args);
    Statement::ExpressionStatement(ExpressionStatement::boxed(SPAN, expression, self))
  }

  fn new_call_object_method_expression(
    &self,
    object_name: &'a str,
    method_name: &'a str,
    args: ArenaVec<'a, Argument<'a>>,
  ) -> Expression<'a> {
    let object_expr = Expression::Identifier(IdentifierReference::boxed(SPAN, object_name, self));
    let method_name_ident = IdentifierName::new(SPAN, method_name, self);
    let static_member =
      StaticMemberExpression::boxed(SPAN, object_expr, method_name_ident, false, self);
    let callee = Expression::StaticMemberExpression(static_member);

    let call_expr = CallExpression::boxed(
      SPAN,
      callee,
      None::<TSTypeParameterInstantiation>,
      args,
      false,
      self,
    );

    Expression::CallExpression(call_expr)
  }

  pub fn append_statement(
    &self,
    function_body: &mut ArenaBox<'a, FunctionBody<'a>>,
    statement: Statement<'a>,
  ) {
    function_body.statements.push(statement);
  }

  pub fn append_formal_parameter(
    &self,
    parameters: &mut ArenaBox<'a, FormalParameters<'a>>,
    parameter: FormalParameter<'a>,
  ) {
    parameters.items.push(parameter);
  }

  pub fn append_argument(
    &self,
    arguments: &mut ArenaVec<'a, Argument<'a>>,
    argument: Argument<'a>,
  ) {
    arguments.push(argument);
  }

  fn new_call_console_log(&self, args: ArenaVec<'a, Argument<'a>>) -> Statement<'a> {
    self.new_call_object_method_statement("console", "log", args)
  }

  /// 以add开头的方法，都是直接在root节点中添加子节点
  fn add_call_console_log(&mut self, args: ArenaVec<'a, Argument<'a>>) {
    let statement = self.new_call_object_method_statement("console", "log", args);
    self.statements.push(statement);
  }

  fn add_call_use_dict(&mut self, dict_names: &[&'a str]) {
    if dict_names.is_empty() {
      return;
    }

    let props = ArenaVec::from_iter_in(
      dict_names.iter().map(|name| {
        BindingProperty::new(
          SPAN,
          PropertyKey::StaticIdentifier(IdentifierName::boxed(SPAN, *name, self)),
          BindingPattern::BindingIdentifier(BindingIdentifier::boxed(SPAN, *name, self)),
          true,
          false,
          self,
        )
      }),
      self,
    );

    let obj_pattern = BindingPattern::ObjectPattern(ObjectPattern::boxed(
      SPAN,
      props,
      None::<BindingRestElement>,
      self,
    ));

    let call_args = ArenaVec::from_iter_in(
      dict_names
        .iter()
        .map(|&name| self.new_argument_string(name)),
      self,
    );

    let callee = Expression::Identifier(IdentifierReference::boxed(SPAN, "useDict", self));

    let call_expr = CallExpression::boxed(
      SPAN,
      callee,
      None::<TSTypeParameterInstantiation>,
      call_args,
      false,
      self,
    );
    let expression = Expression::CallExpression(call_expr);

    let kind = VariableDeclarationKind::Const;

    let declarator = VariableDeclarator::new(
      SPAN,
      kind,
      obj_pattern,
      None::<TSTypeAnnotation<'a>>,
      Some(expression),
      false,
      self,
    );

    let mut declarations = ArenaVec::new_in(self);
    declarations.push(declarator);

    let var_declaration = VariableDeclaration::boxed(SPAN, kind, declarations, false, self);

    self
      .statements
      .push(Statement::VariableDeclaration(var_declaration));
  }

  pub fn add_arrow_function(
    &mut self,
    name: &'a str,
    params: ArenaBox<'a, FormalParameters<'a>>,
    body: ArenaBox<'a, FunctionBody<'a>>,
  ) {
    let arrow_expr = ArrowFunctionExpression::boxed(
      SPAN,
      false,
      false,
      None::<TSTypeParameterDeclaration>,
      params,
      None::<TSTypeAnnotation>,
      body,
      self,
    );
    let init_expr = Expression::ArrowFunctionExpression(arrow_expr);

    self.add_named_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
  }

  pub fn add_arrow_async_function(
    &mut self,
    name: &'a str,
    params: ArenaBox<'a, FormalParameters<'a>>,
    body: ArenaBox<'a, FunctionBody<'a>>,
  ) {
    let arrow_expr = ArrowFunctionExpression::boxed(
      SPAN,
      false,
      true,
      None::<TSTypeParameterDeclaration>,
      params,
      None::<TSTypeAnnotation>,
      body,
      self,
    );
    let init_expr = Expression::ArrowFunctionExpression(arrow_expr);

    self.add_named_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
  }
  //endregion

  //region interface
  pub fn add_interface(
    &mut self,
    name: &'a str,
    body: TSInterfaceBody<'a>,
    base_type_names: &[&'a str],
  ) {
    let bind_ident = BindingIdentifier::new(SPAN, name, self);

    let extends = ArenaVec::from_iter_in(
      base_type_names.iter().map(|name| {
        let expr = Expression::Identifier(IdentifierReference::boxed(SPAN, *name, self));
        TSInterfaceHeritage::new(SPAN, expr, None::<TSTypeParameterInstantiation>, self)
      }),
      self,
    );

    let interface_declaration = TSInterfaceDeclaration::boxed(
      SPAN,
      bind_ident,
      None::<TSTypeParameterDeclaration>,
      extends,
      body,
      false,
      self,
    );

    let export_declaration = ExportNamedDeclaration::boxed(
      SPAN,
      Some(Declaration::TSInterfaceDeclaration(interface_declaration)),
      ArenaVec::new_in(self),
      None,
      ImportOrExportKind::Value,
      None::<WithClause>,
      self,
    );

    let statement = Statement::ExportNamedDeclaration(export_declaration);
    self.statements.push(statement);
  }

  pub fn new_empty_interface_body(&self) -> TSInterfaceBody<'a> {
    TSInterfaceBody::new(SPAN, ArenaVec::new_in(self), self)
  }

  pub fn append_property_string(
    &self,
    body: &mut TSInterfaceBody<'a>,
    name: &'a str,
    optional: bool,
  ) {
    let reference = IdentifierReference::boxed(SPAN, name, self);

    let ts_type = TSType::TSStringKeyword(TSStringKeyword::boxed(SPAN, self));

    let signature = TSPropertySignature::boxed(
      SPAN,
      false,
      optional,
      false,
      PropertyKey::Identifier(reference),
      Some(TSTypeAnnotation::boxed(SPAN, ts_type, self)),
      self,
    );

    body.body.push(TSSignature::TSPropertySignature(signature));
  }

  pub fn append_property_number(
    &self,
    body: &mut TSInterfaceBody<'a>,
    name: &'a str,
    optional: bool,
  ) {
    let reference = IdentifierReference::boxed(SPAN, name, self);

    let ts_type = TSType::TSNumberKeyword(TSNumberKeyword::boxed(SPAN, self));

    let signature = TSPropertySignature::boxed(
      SPAN,
      false,
      optional,
      false,
      PropertyKey::Identifier(reference),
      Some(TSTypeAnnotation::boxed(SPAN, ts_type, self)),
      self,
    );

    body.body.push(TSSignature::TSPropertySignature(signature));
  }

  pub fn append_property_boolean(
    &self,
    body: &mut TSInterfaceBody<'a>,
    name: &'a str,
    optional: bool,
  ) {
    let reference = IdentifierReference::boxed(SPAN, name, self);

    let ts_type = TSType::TSBooleanKeyword(TSBooleanKeyword::boxed(SPAN, self));

    let signature = TSPropertySignature::boxed(
      SPAN,
      false,
      optional,
      false,
      PropertyKey::Identifier(reference),
      Some(TSTypeAnnotation::boxed(SPAN, ts_type, self)),
      self,
    );

    body.body.push(TSSignature::TSPropertySignature(signature));
  }
  //endregion

  //region request
  pub fn new_call_request_get_expression(&self, url: &'a str, config: &[&'a str]) -> Expression<'a> {
    let url_arg = if is_template_string(url) {
      let parser = Parser::new(self.allocator(), url, SourceType::ts());
      let url_expr = parser.parse_expression().unwrap();
      match url_expr {
        Expression::TemplateLiteral(template_literal) => {
          Argument::TemplateLiteral(template_literal)
        }
        other => panic!("expected TemplateLiteral, got {:?}", other),
      }
    } else {
      self.new_argument_string(url)
    };

    let mut vec = ArenaVec::new_in(self);
    vec.push(url_arg);

    if !config.is_empty() {
      let config_expr = self.boxed_object_expression(config);
      let config_arg = Argument::ObjectExpression(config_expr);
      vec.push(config_arg);
    }

    self.new_call_object_method_expression("request", "get", vec)
  }

  pub fn new_call_request_get_statement(&self, url: &'a str, config: &[&'a str]) -> Statement<'a> {
    let url_arg = if is_template_string(url) {
      let parser = Parser::new(self.allocator(), url, SourceType::ts());
      let url_expr = parser.parse_expression().unwrap();
      match url_expr {
        Expression::TemplateLiteral(template_literal) => {
          Argument::TemplateLiteral(template_literal)
        }
        other => panic!("expected TemplateLiteral, got {:?}", other),
      }
    } else {
      self.new_argument_string(url)
    };

    let mut vec = ArenaVec::new_in(self);
    vec.push(url_arg);

    if !config.is_empty() {
      let config_expr = self.boxed_object_expression(config);
      let config_arg = Argument::ObjectExpression(config_expr);
      vec.push(config_arg);
    }

    self.new_call_object_method_statement("request", "get", vec)
  }

  pub fn new_call_request_post_statement(&self, url: &'a str, data: &'a str) -> Statement<'a> {
    let url_arg = if is_template_string(url) {
      let parser = Parser::new(self.allocator(), url, SourceType::ts());
      let url_expr = parser.parse_expression().unwrap();
      match url_expr {
        Expression::TemplateLiteral(template_literal) => {
          Argument::TemplateLiteral(template_literal)
        }
        other => panic!("expected TemplateLiteral, got {:?}", other),
      }
    } else {
      self.new_argument_string(url)
    };

    let mut vec = ArenaVec::new_in(self);
    vec.push(url_arg);

    let data_arg = self.new_argument_identifier(data);
    vec.push(data_arg);

    self.new_call_object_method_statement("request", "post", vec)
  }

  pub fn new_call_request_put_statement(&self, url: &'a str, data: &'a str) -> Statement<'a> {
    let url_arg = if is_template_string(url) {
      let parser = Parser::new(self.allocator(), url, SourceType::ts());
      let url_expr = parser.parse_expression().unwrap();
      match url_expr {
        Expression::TemplateLiteral(template_literal) => {
          Argument::TemplateLiteral(template_literal)
        }
        other => panic!("expected TemplateLiteral, got {:?}", other),
      }
    } else {
      self.new_argument_string(url)
    };

    let mut vec = ArenaVec::new_in(self);
    vec.push(url_arg);

    let data_arg = self.new_argument_identifier(data);
    vec.push(data_arg);

    self.new_call_object_method_statement("request", "put", vec)
  }

  pub fn new_call_request_delete_statement(&self, url: &'a str) -> Statement<'a> {
    let url_arg = if is_template_string(url) {
      let parser = Parser::new(self.allocator(), url, SourceType::ts());
      let url_expr = parser.parse_expression().unwrap();
      match url_expr {
        Expression::TemplateLiteral(template_literal) => {
          Argument::TemplateLiteral(template_literal)
        }
        other => panic!("expected TemplateLiteral, got {:?}", other),
      }
    } else {
      self.new_argument_string(url)
    };

    let mut vec = ArenaVec::new_in(self);
    vec.push(url_arg);

    self.new_call_object_method_statement("request", "delete", vec)
  }

  fn boxed_object_expression(
    &self,
    names: &[&'a str],
  ) -> oxc_allocator::Box<'a, ObjectExpression<'a>> {
    let properties = ArenaVec::from_iter_in(
      names.iter().map(|name| {
        ObjectPropertyKind::new_object_property(
          SPAN,
          PropertyKind::Init,
          PropertyKey::StaticIdentifier(IdentifierName::boxed(SPAN, *name, self)),
          Expression::Identifier(IdentifierReference::boxed(SPAN, *name, self)),
          false,
          true,
          false,
          self,
        )
      }),
      self,
    );

    ObjectExpression::boxed(SPAN, properties, self)
  }
  //endregion

  //region type alias
  pub fn add_generic_type_alias(&mut self, alias: &'a str, type_names: &[&'a str]) -> Statement<'a> {
    let current_type = self.new_generic_type(type_names);

    let ts_type_alias = TSTypeAliasDeclaration::boxed(
      SPAN,
      BindingIdentifier::new(SPAN, alias, self),
      None::<TSTypeParameterDeclaration>,
      current_type,
      false,
      self,
    );
    Statement::TSTypeAliasDeclaration(ts_type_alias)
  }
  //endregion

  //region util
  fn new_return_statement(&self, argument: Option<Expression<'a>>) -> Statement<'a> {
    let return_statement = ReturnStatement::boxed(SPAN, argument, self);
    Statement::ReturnStatement(return_statement)
  }

  fn new_generic_type(&mut self, type_names: &[&'a str]) -> TSType<'a> {
    assert!(!type_names.is_empty(), "type_names cannot be empty");

    // 从后往前循环
    let mut iter = type_names.iter().rev();

    let mut current_type = {
      let last_type_name = iter.next().unwrap();
      let last_ident_ref = IdentifierReference::boxed(SPAN, *last_type_name, self);
      let last_type_ref = TSTypeReference::boxed(
        SPAN,
        TSTypeName::IdentifierReference(last_ident_ref),
        None::<TSTypeParameterInstantiation>,
        self
      );
      TSType::TSTypeReference(last_type_ref)
    };

    for outer_type_name in iter {
      let outer_type_ident = IdentifierReference::boxed(SPAN, *outer_type_name, self);
      let outer_ts_name = TSTypeName::IdentifierReference(outer_type_ident);

      let type_params = TSTypeParameterInstantiation::boxed(
        SPAN,
        ArenaVec::from_value_in(current_type, self),
        self
      );
      current_type = TSType::TSTypeReference(TSTypeReference::boxed(SPAN, outer_ts_name, Some(type_params), self));
    }

    current_type
  }

  fn new_try_catch_finally_statement(
    &self,
    try_statements: impl IntoIterator<Item = Statement<'a>>,
    catch_statements: impl IntoIterator<Item = Statement<'a>>,
    finally_statements: impl IntoIterator<Item = Statement<'a>>
  ) -> Statement<'a> {
    self.new_try_statement(try_statements, Some(catch_statements), Some(finally_statements))
  }

  fn new_try_statement(
    &self,
    try_statements: impl IntoIterator<Item = Statement<'a>>,
    catch_statements: Option<impl IntoIterator<Item = Statement<'a>>>,
    finally_statements: Option<impl IntoIterator<Item = Statement<'a>>>
  ) -> Statement<'a>  {
    // try block
    let mut try_body_vec = ArenaVec::new_in(self);
    try_body_vec.extend(try_statements);
    let try_body = BlockStatement::new(
      SPAN,
      try_body_vec,
      self
    );
    let try_body_box = ArenaBox::new_in(try_body, self);

    // catch clause
    let catch_parameter = CatchParameter::new(
      SPAN,
      BindingPattern::BindingIdentifier(BindingIdentifier::boxed(SPAN, "e", self)),
      None::<TSTypeAnnotation>,
      self
    );

    let catch_clause_option = if let Some(catch_statements) = catch_statements {
      let mut catch_body_vec = ArenaVec::new_in(self);
      catch_body_vec.extend(catch_statements);
      let catch_body = BlockStatement::new(
        SPAN,
        catch_body_vec,
        self,
      );
      Some(CatchClause::new(
        SPAN,
        Some(catch_parameter),
        ArenaBox::new_in(catch_body, self),
        self
      ))
    } else {
      None
    };

    // finally block
    let finally_body_option = if let Some(finally_statements) = finally_statements {
      let mut finally_body_vec = ArenaVec::new_in(self);
      finally_body_vec.extend(finally_statements);
      let finally_body = BlockStatement::new(
        SPAN,
        finally_body_vec,
        self,
      );
      Some(ArenaBox::new_in(finally_body, self))
    } else {
      None
    };

    // try-catch-finally
    Statement::new_try_statement(
      SPAN,
      try_body_box,
      catch_clause_option,
      finally_body_option,
      self
    )
  }
  //endregion

  //region 注释
  pub fn add_line_comment(&mut self, comment: &str) {
    let comment1 = Comment::new(0, 10, CommentKind::Line);
    self.comments.push(comment1);
  }
  //endregion

  pub fn append(&mut self, statement: Statement<'a>) {
    self.statements.push(statement);
  }

  pub fn to_code(self) -> String {
    let program = Program::new(
      SPAN,
      SourceType::ts(),
      "",
      self.comments,
      None,
      self.directives,
      self.statements,
      &self.builder,
    );
    let codegen_return = Codegen::new().build(&program);
    codegen_return.code
  }
}

fn is_template_string(content: &str) -> bool {
  content.len() >= 2 && content.starts_with("`") && content.ends_with("`")
}

#[cfg(test)]
mod tests {
  use oxc_ast::ast::TryStatement;
  use super::*;

  #[test]
  fn test_is_template_string() {
    assert!(!is_template_string("`"));
    assert!(!is_template_string("Hello World!"));
    assert!(!is_template_string("`Hello World!"));
    assert!(!is_template_string("Hello World!`"));
    assert!(is_template_string("``"));
    assert!(is_template_string("`Hello World!`"));
  }

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
    script_ast.add_import_named_all(
      "source",
      &[NamedImportItem::Value("a"), NamedImportItem::Type("b")],
    );
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
  fn test_add_arrow_async_function_empty() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let parameters = script_ast.new_empty_arrow_function_params();
    let function_body = script_ast.new_empty_function_body();
    script_ast.add_arrow_async_function("a", parameters, function_body);
    let actual_code = script_ast.to_code();
    assert_eq!(actual_code, "const a = async () => {};\n");
  }

  #[test]
  fn test_add_call_console_log() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let parameters = script_ast.new_empty_arrow_function_params();
    let function_body = script_ast.new_empty_function_body();

    let mut arguments = script_ast.new_empty_function_arguments();
    script_ast.append_argument(&mut arguments, script_ast.new_argument_string("a"));
    script_ast.append_argument(&mut arguments, script_ast.new_argument_decimal(1.0));
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
    assert_eq!(
      actual_code,
      "const a = (b: string) => {\n\tconsole.log(b);\n};\n"
    );
  }

  #[test]
  fn test_add_set_ref_string_value() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_set_ref_string_value("a", "b");
    let actual_code = script_ast.to_code();
    assert_eq!(actual_code, "a.value = \"b\";\n");
  }

  #[test]
  fn test_add_set_ref_decimal_value() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_set_ref_decimal_value("a", 1);
    let actual_code = script_ast.to_code();
    assert_eq!(actual_code, "a.value = 1;\n");
  }

  #[test]
  fn test_add_set_ref_boolean_value() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_set_ref_boolean_value("a", true);
    let actual_code = script_ast.to_code();
    assert_eq!(actual_code, "a.value = true;\n");
  }

  // get_object_data_api
  // get_array_data_api
  // get_pagable_data_api
  // get_status_data
  // get_response_data
  #[test]
  fn test_new_call_request_get_string() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);

    let request_get = script_ast.new_call_request_get_statement("url", &[]);
    script_ast.append(request_get);
    let actual_code = script_ast.to_code();
    assert_eq!(actual_code, "request.get(\"url\");\n");
  }

  #[test]
  fn test_new_call_request_get_string_params() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);

    let request_get = script_ast.new_call_request_get_statement("url", &["params"]);
    script_ast.append(request_get);
    let actual_code = script_ast.to_code();
    assert_eq!(actual_code, "request.get(\"url\", { params });\n");
  }

  #[test]
  fn test_new_call_request_get_template_string() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);

    let request_get = script_ast.new_call_request_get_statement("`base_url/${id}`", &[]);
    script_ast.append(request_get);
    let actual_code = script_ast.to_code();
    assert_eq!(actual_code, "request.get(`base_url/${id}`);\n");
  }

  #[test]
  fn test_new_call_request_post_url_string_data() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let request_post = script_ast.new_call_request_post_statement("url", "data");
    script_ast.append(request_post);
    let actual_code = script_ast.to_code();
    assert_eq!(actual_code, "request.post(\"url\", data);\n");
  }

  #[test]
  fn test_new_call_request_post_url_string_template_data() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let request_post = script_ast.new_call_request_post_statement("`url/${var1}`", "data");
    script_ast.append(request_post);
    let actual_code = script_ast.to_code();
    assert_eq!(actual_code, "request.post(`url/${var1}`, data);\n");
  }

  #[test]
  fn test_new_call_request_put_url_string_data() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let request_post = script_ast.new_call_request_put_statement("url", "data");
    script_ast.append(request_post);
    let actual_code = script_ast.to_code();
    assert_eq!(actual_code, "request.put(\"url\", data);\n");
  }

  #[test]
  fn test_new_call_request_put_url_string_template_data() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let request_post = script_ast.new_call_request_put_statement("`url/${var1}`", "data");
    script_ast.append(request_post);
    let actual_code = script_ast.to_code();
    assert_eq!(actual_code, "request.put(`url/${var1}`, data);\n");
  }

  #[test]
  fn test_new_call_request_delete_url_string() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let request_post = script_ast.new_call_request_delete_statement("url");
    script_ast.append(request_post);
    let actual_code = script_ast.to_code();
    assert_eq!(actual_code, "request.delete(\"url\");\n");
  }

  #[test]
  fn test_new_call_request_delete_url_string_template() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let request_post = script_ast.new_call_request_delete_statement("`url/${var1}`");
    script_ast.append(request_post);
    let actual_code = script_ast.to_code();
    assert_eq!(actual_code, "request.delete(`url/${var1}`);\n");
  }

  #[test]
  fn test_new_object_expression() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let obj_expr = script_ast.boxed_object_expression(&["param1", "param2"]);

    let statement = Statement::ExpressionStatement(ExpressionStatement::boxed(
      SPAN,
      Expression::ObjectExpression(obj_expr),
      script_ast.builder(),
    ));
    script_ast.append(statement);
    let actual_code = script_ast.to_code();
    assert_eq!(actual_code, "({\n\tparam1,\n\tparam2\n});\n");
  }

  #[test]
  fn test_new_return_statement_no_argument() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);

    let statement = script_ast.new_return_statement(None);
    script_ast.append(statement);
    let actual_code = script_ast.to_code();
    assert_eq!(actual_code, "return;\n");
  }

  #[test]
  fn test_new_return_statement_string_argument() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let str_expr = script_ast.new_expression_string("a");
    let statement = script_ast.new_return_statement(Some(str_expr));
    script_ast.append(statement);
    let actual_code = script_ast.to_code();
    assert_eq!(actual_code, "return \"a\";\n");
  }

  #[test]
  fn test_new_return_statement_request_get() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);

    let request_get_expr = script_ast.new_call_request_get_expression("url", &[]);
    let statement = script_ast.new_return_statement(Some(request_get_expr));
    script_ast.append(statement);
    let actual_code = script_ast.to_code();
    assert_eq!(actual_code, "return request.get(\"url\");\n");
  }

  #[test]
  fn test_add_generic_type_alias() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let promise_user_type = script_ast.add_generic_type_alias("NewType", &["Promise", "User"]);
    script_ast.append(promise_user_type);
    let actual_code = script_ast.to_code();
    assert_eq!(actual_code, "type NewType = Promise<User>;\n");
  }

  #[test]
  fn test_add_call_use_dict() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);

    let id_arg = script_ast.new_argument_identifier("id");
    script_ast.add_call_use_dict(&["a", "b"]);
    let actual_code = script_ast.to_code();
    assert_eq!(actual_code, "const { a, b } = useDict(\"a\", \"b\");\n");
  }

  #[test]
  fn test_add_interface() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);

    let mut interface_body = script_ast.new_empty_interface_body();

    script_ast.append_property_string(&mut interface_body, "a", true);
    script_ast.append_property_number(&mut interface_body, "b", true);
    script_ast.append_property_boolean(&mut interface_body, "c", true);

    script_ast.add_interface("TheType", interface_body, &["TheBase1", "TheBase2"]);
    let actual_code = script_ast.to_code();
    assert_eq!(
      actual_code,
      "export interface TheType extends TheBase1, TheBase2 {\n\ta?: string;\n\tb?: number;\n\tc?: boolean;\n}\n"
    );
  }

  #[test]
  fn test_new_try_catch_finally_statement() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let statement = script_ast.new_try_catch_finally_statement(
      [],
      [],
      []
    );
    script_ast.append(statement);
    let actual_code = script_ast.to_code();
    assert_eq!(actual_code, "try {} catch (e) {} finally {}\n");
  }

  #[test]
  fn test_new_try_statement() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let statement = script_ast.new_try_statement(
      [],
      None::<[Statement; 0]>,
      None::<[Statement; 0]>
    );
    script_ast.append(statement);
    let actual_code = script_ast.to_code();
    assert_eq!(actual_code, "try {}\n");
  }

  #[test]
  fn test_add_comment() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_line_comment("我是注释");
    let actual_code = script_ast.to_code();
    assert_eq!(actual_code, "// 我是注释\n");
  }
}
