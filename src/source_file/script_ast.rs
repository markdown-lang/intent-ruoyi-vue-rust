use heck::ToUpperCamelCase;
use oxc_allocator::{Allocator, ArenaBox, ArenaVec, CloneIn, GetAllocator};
use oxc_ast::ast::{Argument, ArrayExpressionElement, ArrowFunctionBody, AssignmentOperator, AssignmentTarget, BinaryOperator, BindingIdentifier, BindingPattern, BindingProperty, BindingRestElement, BlockStatement, CatchClause, CatchParameter, Declaration, Directive, ExportNamedDeclaration, ExportSpecifier, Expression, FormalParameter, FormalParameterKind, FormalParameterRest, FormalParameters, Function, FunctionBody, FunctionType, IdentifierName, ImportDeclarationSpecifier, ImportOrExportKind, LogicalOperator, ModuleExportName, NumberBase, ObjectExpression, ObjectProperty, ObjectPropertyKind, Program, PropertyKey, PropertyKind, ReturnStatement, Statement, StringLiteral, TSInterfaceBody, TSInterfaceHeritage, TSSignature, TSType, TSTypeAnnotation, TSTypeName, TSTypeParameterDeclaration, TSTypeParameterInstantiation, TSUnionType, VariableDeclarationKind, VariableDeclarator, WithClause};
use oxc_ast::builder::{AstBuilder, GetAstBuilder};
use oxc_ast::{Comment, CommentKind};
use oxc_codegen::{Codegen, CodegenOptions, IndentChar};
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
    // 默认导入说明符，包装为统一的导入说明符枚举
    let specifier = ImportDeclarationSpecifier::new_import_default_specifier(SPAN, local, self);

    // 创建存放说明符的列表
    let specifiers = ArenaVec::from_value_in(specifier, self);

    let statement = Statement::new_import_declaration(
      SPAN,
      Some(specifiers),
      source_literal,
      None,
      None,
      ImportOrExportKind::Value,
      self,
    );

    self.append_to_root(statement);
  }

  /// 命名导入
  /// `import { a, b } from "source";`
  pub fn add_import_named_value(&mut self, source: &'a str, named_imports: &[&'a str]) {
    if named_imports.is_empty() {
      return;
    }

    let source_literal = StringLiteral::new(SPAN, source, None, self);

    let specifiers = ArenaVec::from_iter_in(
      named_imports.iter().map(|&name| {
        // 本地作用域绑定名称
        let imported = ModuleExportName::new_identifier_name(SPAN, name, self);
        let local = BindingIdentifier::new(SPAN, name, self);
        // 同名导入（无 as 别名）时 imported 传 None，代码生成自动复用 local 名称
        ImportDeclarationSpecifier::new_import_specifier(
          SPAN,
          imported,
          local,
          ImportOrExportKind::Value,
          self,
        )
      }),
      self,
    );

    let statement = Statement::new_import_declaration(
      SPAN,
      Some(specifiers),
      source_literal,
      None,
      None,
      ImportOrExportKind::Value,
      self,
    );

    self.append_to_root(statement);
  }

  /// 命名导入
  /// `import type { a, b } from "source";`
  pub fn add_import_named_type(&mut self, source: &'a str, named_imports: &[&'a str]) {
    if named_imports.is_empty() {
      return;
    }

    let source_literal = StringLiteral::new(SPAN, source, None, self);

    let specifiers = ArenaVec::from_iter_in(
      named_imports.iter().map(|&name| {
        // 本地作用域绑定名称
        let imported = ModuleExportName::new_identifier_name(SPAN, name, self);
        let local = BindingIdentifier::new(SPAN, name, self);
        // 同名导入（无 as 别名）时 imported 传 None，代码生成自动复用 local 名称
        ImportDeclarationSpecifier::new_import_specifier(
          SPAN,
          imported,
          local,
          ImportOrExportKind::Value,
          self,
        )
      }),
      self,
    );

    let statement = Statement::new_import_declaration(
      SPAN,
      Some(specifiers),
      source_literal,
      None,
      None,
      ImportOrExportKind::Type,
      self,
    );

    self.append_to_root(statement);
  }

  /// 混合命名导入（支持值与 type 项混排）
  /// 示例：`import { a, type b } from "source";`
  pub fn add_import_named_all(&mut self, source: &'a str, named_imports: &[NamedImportItem<'a>]) {
    if named_imports.is_empty() {
      return;
    }

    let source_literal = StringLiteral::new(SPAN, source, None, self);

    let specifiers = ArenaVec::from_iter_in(
      named_imports.iter().map(|item| {
        let (name, kind) = match item {
          NamedImportItem::Value(name) => (*name, ImportOrExportKind::Value),
          NamedImportItem::Type(name) => (*name, ImportOrExportKind::Type),
        };
        // 本地作用域绑定名称
        let imported = ModuleExportName::new_identifier_name(SPAN, name, self);
        let local = BindingIdentifier::new(SPAN, name, self);
        // 同名导入（无 as 别名）时 imported 传 None，代码生成自动复用 local 名称
        ImportDeclarationSpecifier::new_import_specifier(SPAN, imported, local, kind, self)
      }),
      self,
    );

    let statement = Statement::new_import_declaration(
      SPAN,
      Some(specifiers),
      source_literal,
      None,
      None,
      // 混合导入顶层必须为 Value，type 修饰由单个 specifier 控制
      ImportOrExportKind::Value,
      self,
    );

    self.append_to_root(statement);
  }
  //endregion

  //region argument 实参
  #[inline]
  pub fn new_argument_string(&self, value: &'a str) -> Argument<'a> {
    Argument::new_string_literal(SPAN, value, None, self)
  }

  #[inline]
  pub fn new_argument_decimal(&self, value: f64) -> Argument<'a> {
    Argument::new_numeric_literal(SPAN, value, None, NumberBase::Decimal, self)
  }

  #[inline]
  pub fn new_argument_float(&self, value: f64) -> Argument<'a> {
    Argument::new_numeric_literal(SPAN, value, None, NumberBase::Float, self)
  }

  #[inline]
  pub fn new_argument_boolean(&self, value: bool) -> Argument<'a> {
    Argument::new_boolean_literal(SPAN, value, self)
  }

  #[inline]
  pub fn new_argument_identifier(&self, name: &'a str) -> Argument<'a> {
    Argument::new_identifier(SPAN, name, self)
  }

  #[inline]
  pub fn new_argument_empty_array(&self) -> Argument<'a> {
    Argument::new_array_expression(SPAN, ArenaVec::new_in(self), self)
  }
  //endregion

  //region 基本类型表达式
  #[inline]
  fn new_expression_string(&self, value: &'a str) -> Expression<'a> {
    Expression::new_string_literal(SPAN, value, None, self)
  }

  #[inline]
  fn new_expression_decimal(&self, value: i64) -> Expression<'a> {
    Expression::new_numeric_literal(SPAN, value as f64, None, NumberBase::Decimal, self)
  }

  #[inline]
  fn new_expression_float(&self, value: f64) -> Expression<'a> {
    Expression::new_numeric_literal(SPAN, value, None, NumberBase::Float, self)
  }

  #[inline]
  fn new_expression_boolean(&self, value: bool) -> Expression<'a> {
    Expression::new_boolean_literal(SPAN, value, self)
  }

  #[inline]
  fn new_expression_array(&self, elements: impl IntoIterator<Item = ArrayExpressionElement<'a>>) -> Expression<'a> {
    Expression::new_array_expression(SPAN, ArenaVec::from_iter_in(elements, self), self)
  }

  #[inline]
  fn new_expression_identifier(&self, name: &'a str) -> Expression<'a> {
    Expression::new_identifier(SPAN, name, self)
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
    let init_expr = self.new_expression_boolean(value);
    self.add_named_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
  }

  pub fn add_const_ref_boolean(&mut self, name: &'a str, value: bool) {
    let ts_type = TSType::new_ts_boolean_keyword(SPAN, self);
    let argument = self.new_argument_boolean(value);

    let init_expr = self.call_ref(ts_type, argument);

    self.add_named_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
  }

  pub fn add_const_ref_string(&mut self, name: &'a str, value: &'a str) {
    let ts_type = TSType::new_ts_string_keyword(SPAN, self);
    let argument = self.new_argument_string(value);

    let init_expr = self.call_ref(ts_type, argument);

    self.add_named_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
  }

  pub fn add_const_ref_number(&mut self, name: &'a str, value: f64) {
    let ts_type = TSType::new_ts_number_keyword(SPAN, self);
    let argument = self.new_argument_decimal(value);

    let init_expr = self.call_ref(ts_type, argument);

    self.add_named_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
  }

  pub fn add_const_ref_string_array(&mut self, name: &'a str) {
    let empty_array_argument = self.new_argument_empty_array();

    let ts_type = TSType::new_ts_string_keyword(SPAN, self);
    let ts_array_type = TSType::new_ts_array_type(SPAN, ts_type, self);

    let init_expr = self.call_ref(ts_array_type, empty_array_argument);

    self.add_named_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
  }

  pub fn add_const_ref_number_array(&mut self, name: &'a str) {
    let empty_array_argument = self.new_argument_empty_array();

    let ts_type = TSType::new_ts_number_keyword(SPAN, self);
    let ts_array_type = TSType::new_ts_array_type(SPAN, ts_type, self);

    let init_expr = self.call_ref(ts_array_type, empty_array_argument);

    self.add_named_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
  }

  pub fn add_const_ref_object_array(&mut self, name: &'a str, type_name: &'a str) {
    let empty_array_argument = self.new_argument_empty_array();

    let ident_ref = TSTypeName::new_identifier_reference(SPAN, type_name, self);
    let ts_type = TSType::new_ts_type_reference(
      SPAN,
      ident_ref,
      None,
      self
    );
    let ts_array_type = TSType::new_ts_array_type(SPAN, ts_type, self);

    let init_expr = self.call_ref(ts_array_type, empty_array_argument);

    self.add_named_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
  }

  fn call_ref(&self, ts_type: TSType<'a>, argument: Argument<'a>) -> Expression<'a> {
    // 调用 ref 函数
    let ref_expr = Expression::new_identifier(SPAN, "ref", self);

    let input_arguments = ArenaVec::from_value_in(argument, self);
    // 函数的泛型参数
    let type_parameters = ArenaVec::from_value_in(ts_type, self);
    let type_parameters_instantiation = TSTypeParameterInstantiation::boxed(
      SPAN,
      type_parameters,
      self
    );

    // 组装完整表达式
    Expression::new_call_expression(
      SPAN,
      ref_expr,
      Some(type_parameters_instantiation),
      input_arguments,
      false,
      self,
    )
  }

  fn call_reactive(&self, properties: ArenaVec<'a, ObjectPropertyKind<'a>>) -> Expression<'a> {
    let reactive_expr = Expression::new_identifier(SPAN, "reactive", self);

    let argument = Argument::new_object_expression(SPAN, properties, self);
    let input_arguments = ArenaVec::from_value_in(argument, self);

    Expression::new_call_expression(
      SPAN,
      reactive_expr,
      None,
      input_arguments,
      false,
      self,
    )
  }

  pub fn add_const_reactive_object(
    &mut self,
    var_name: &'a str,
    type_names: &[&'a str],
    object_properties: impl IntoIterator<Item = ObjectPropertyKind<'a>>,
  ) {
    let init_expr = self.call_reactive(ArenaVec::from_iter_in(object_properties, self));
    self.add_named_typed_variable_declaration(VariableDeclarationKind::Const, var_name, type_names, init_expr);
  }

  pub fn new_decimal_object_property(&self, name: &'a str, value: f64) -> ObjectPropertyKind<'a> {
    ObjectPropertyKind::new_object_property(
      SPAN,
      PropertyKind::Init,
      PropertyKey::new_static_identifier(SPAN, name, self),
      self.new_expression_float(value),
      false,
      true,
      false,
      self,
    )
  }

  pub fn new_string_object_property(&self, name: &'a str, value: &'a str) -> ObjectPropertyKind<'a> {
    ObjectPropertyKind::new_object_property(
      SPAN,
      PropertyKind::Init,
      PropertyKey::new_static_identifier(SPAN, name, self),
      self.new_expression_string(value),
      false,
      true,
      false,
      self,
    )
  }

  pub fn new_boolean_object_property(&self, name: &'a str, value: bool) -> ObjectPropertyKind<'a> {
    ObjectPropertyKind::new_object_property(
      SPAN,
      PropertyKind::Init,
      PropertyKey::new_static_identifier(SPAN, name, self),
      self.new_expression_boolean(value),
      false,
      true,
      false,
      self,
    )
  }

  pub fn new_undefined_object_property(&self, name: &'a str) -> ObjectPropertyKind<'a> {
    ObjectPropertyKind::new_object_property(
      SPAN,
      PropertyKind::Init,
      PropertyKey::new_static_identifier(SPAN, name, self),
      Expression::new_identifier(SPAN, "undefined", self),
      false,
      true,
      false,
      self,
    )
  }

  pub fn new_array_object_property(&self, name: &'a str, elements: impl IntoIterator<Item = ArrayExpressionElement<'a>>) -> ObjectPropertyKind<'a> {
    ObjectPropertyKind::new_object_property(
      SPAN,
      PropertyKind::Init,
      PropertyKey::new_static_identifier(SPAN, name, self),
      Expression::new_array_expression(SPAN, ArenaVec::from_iter_in(elements, self), self),
      false,
      true,
      false,
      self,
    )
  }
  
  pub fn new_array_object_element(&self, object_properties: impl IntoIterator<Item = ObjectPropertyKind<'a>>,) -> ArrayExpressionElement<'a> {
    ArrayExpressionElement::new_object_expression(SPAN, ArenaVec::from_iter_in(object_properties, self), self)
  }

  fn add_named_variable_declaration(
    &mut self,
    kind: VariableDeclarationKind,
    name: &'a str,
    init_expr: Expression<'a>,
  ) {
    let id = BindingPattern::new_binding_identifier(SPAN, name, self);
    self.add_variable_declaration(kind, id, init_expr);
  }

  fn add_named_typed_variable_declaration(
    &mut self,
    kind: VariableDeclarationKind,
    var_name: &'a str,
    type_names: &[&'a str],
    init_expr: Expression<'a>,
  ) {
    let id = BindingPattern::new_binding_identifier(SPAN, var_name, self);
    self.add_typed_variable_declaration(kind, id, type_names, init_expr);
  }

  fn add_variable_declaration(
    &mut self,
    kind: VariableDeclarationKind,
    id: BindingPattern<'a>,
    init_expr: Expression<'a>,
  ) {
    let declarator = VariableDeclarator::new(
      SPAN,
      id,
      None,
      Some(init_expr),
      false,
      self,
    );

    let declarations = ArenaVec::from_value_in(declarator, self);

    let statement = Statement::new_variable_declaration(SPAN, kind, declarations, false, self);
    self.append_to_root(statement);
  }

  fn add_typed_variable_declaration(
    &mut self,
    kind: VariableDeclarationKind,
    id: BindingPattern<'a>,
    type_names: &[&'a str],
    init_expr: Expression<'a>,
  ) {
    let ts_type = self.new_generic_type(type_names);
    let declarator = VariableDeclarator::new(
      SPAN,
      id,
      Some(TSTypeAnnotation::boxed(SPAN, ts_type, self)),
      Some(init_expr),
      false,
      self,
    );

    let declarations = ArenaVec::from_value_in(declarator, self);

    let statement = Statement::new_variable_declaration(SPAN, kind, declarations, false, self);
    self.append_to_root(statement);
  }
  //endregion

  //region set ref value

  fn new_set_ref_value(&self, name: &'a str, value: Expression<'a>) -> Statement<'a> {
    let object_expr = Expression::new_identifier(SPAN, name, self);

    let method_name_ident = IdentifierName::new(SPAN, "value", self);
    let left_target = AssignmentTarget::new_static_member_expression(
      SPAN,
      object_expr,
      method_name_ident,
      false,
      self,
    );

    let expression = Expression::new_assignment_expression(
      SPAN,
      AssignmentOperator::Assign,
      left_target,
      value,
      self,
    );

    Statement::new_expression_statement(SPAN, expression, self)
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
    let right_expr = self.new_expression_boolean(value);
    self.new_set_ref_value(name, right_expr)
  }

  pub fn new_set_ref_identifier_value(&self, name: &'a str, value: &'a str) -> Statement<'a> {
    let right_expr = self.new_expression_identifier(value);
    self.new_set_ref_value(name, right_expr)
  }

  pub fn new_clear_ref_object_property(&self, object_name: &'a str, property_key: &'a str) -> Statement<'a> {
    let object_expr = Expression::new_identifier(SPAN, object_name, self);

    let value_ident = IdentifierName::new(SPAN, "value", self);
    let object_value_expr = Expression::new_static_member_expression(
      SPAN,
      object_expr,
      value_ident,
      false,
      self,
    );

    let property_key_ident = IdentifierName::new(SPAN, property_key, self);

    let left_target = AssignmentTarget::new_static_member_expression(
      SPAN,
      object_value_expr,
      property_key_ident,
      false,
      self,
    );

    let expression = Expression::new_assignment_expression(
      SPAN,
      AssignmentOperator::Assign,
      left_target,
      Expression::new_object_expression(
        SPAN,
        ArenaVec::new_in(self),
        self,
      ),
      self,
    );

    Statement::new_expression_statement(SPAN, expression, self)
  }

  pub fn new_check_ref_value_is_blank(&self, name: &'a str) -> Expression<'a> {
    let object_expr = Expression::new_identifier(SPAN, name, self);
    let value_ident = IdentifierName::new(SPAN, "value", self);
    let object_value_expr = Expression::new_static_member_expression(
      SPAN,
      object_expr,
      value_ident,
      false,
      self,
    );

    let not_null_expr = Expression::new_binary_expression(
      SPAN,
      Expression::new_null_literal(SPAN, self),
      BinaryOperator::Inequality,
      object_value_expr.clone_in(self.allocator()),
      self,
    );

    let not_empty_string_expr = Expression::new_binary_expression(
      SPAN,
      Expression::new_string_literal(SPAN, "", None, self),
      BinaryOperator::Inequality,
      object_value_expr,
      self,
    );

    Expression::new_logical_expression(
      SPAN,
      not_null_expr,
      LogicalOperator::And,
      not_empty_string_expr,
      self,
    )
  }

  fn new_left_member_assign_target(&self, parts: &[&'a str]) -> AssignmentTarget<'a> {
    assert!(!parts.is_empty());
    if parts.len() == 1 {
      AssignmentTarget::new_assignment_target_identifier(SPAN, parts[0], self)
    } else {
      let mut expr = Expression::new_identifier(SPAN, parts[0], self);
      let middle_parts = &parts[1..parts.len() - 1];
      for part in middle_parts {
        let part = *part;
        if is_computed_member(part) {
          if let Some(key) = part.strip_prefix('[').and_then(|part| part.strip_suffix(']')) {
            let prop_expr = if let Ok(num) = key.parse::<f64>() {
              Expression::new_numeric_literal(SPAN, num, None, NumberBase::Decimal, self)
            } else {
              Expression::new_string_literal(SPAN, key, None, self)
            };
            expr = Expression::new_computed_member_expression(
              SPAN,
              expr,
              prop_expr,
              false,
              self,
            )
          }
        } else {
          expr = Expression::new_static_member_expression(
            SPAN,
            expr,
            IdentifierName::new(SPAN, part, self),
            false,
            self,
          );
        }
      }
      let last_part = parts[parts.len() - 1];
      if is_computed_member(last_part) {
        let key = strip_computed_key(last_part);
        let prop_expr = if let Ok(num) = key.parse::<f64>() {
          Expression::new_numeric_literal(SPAN, num, None, NumberBase::Decimal, self)
        } else {
          Expression::new_string_literal(SPAN, key, None, self)
        };
        AssignmentTarget::new_computed_member_expression(
          SPAN,
          expr,
          prop_expr,
          false,
          self,
        )
      } else {
        AssignmentTarget::new_static_member_expression(
          SPAN,
          expr,
          IdentifierName::new(SPAN, last_part, self),
          false,
          self,
        )
      }
    }
  }

  fn new_right_member_expression(&self, parts: &[&'a str]) -> Expression<'a> {
    assert!(!parts.is_empty());
    let mut expr = Expression::new_identifier(SPAN, parts[0], self);
    for part in parts.iter().skip(1) {
      let part = *part;
      if is_computed_member(part) {
        if let Some(key) = part.strip_prefix('[').and_then(|part| part.strip_suffix(']')) {
          let prop_expr = if let Ok(num) = key.parse::<f64>() {
            Expression::new_numeric_literal(SPAN, num, None, NumberBase::Decimal, self)
          } else {
            Expression::new_string_literal(SPAN, key, None, self)
          };
          expr = Expression::new_computed_member_expression(
            SPAN,
            expr,
            prop_expr,
            false,
            self,
          )
        }
      } else {
        expr = Expression::new_static_member_expression(
          SPAN,
          expr,
          IdentifierName::new(SPAN, part, self),
          false,
          self,
        );
      }
    }

    expr
  }

  pub fn new_set_member_value(&self, member_parts: &[&'a str], value_parts: &[&'a str]) -> Statement<'a> {
    let left_target = self.new_left_member_assign_target(member_parts);
    let right_expr = self.new_right_member_expression(value_parts);
    let expression = Expression::new_assignment_expression(
      SPAN,
      AssignmentOperator::Assign,
      left_target,
      right_expr,
      self,
    );
    Statement::new_expression_statement(SPAN, expression, self)
  }

  pub fn add_set_ref_string_value(&mut self, name: &'a str, value: &'a str) {
    let statement = self.new_set_ref_string_value(name, value);
    self.append_to_root(statement);
  }

  pub fn add_set_ref_decimal_value(&mut self, name: &'a str, value: i64) {
    let statement = self.new_set_ref_decimal_value(name, value);
    self.append_to_root(statement);
  }

  pub fn add_set_ref_boolean_value(&mut self, name: &'a str, value: bool) {
    let statement = self.new_set_ref_boolean_value(name, value);
    self.append_to_root(statement);
  }

  //endregion

  //region arrow function
  pub fn new_formal_string_parameter(&self, name: &'a str) -> FormalParameter<'a> {
    let pattern = BindingPattern::new_binding_identifier(SPAN, name, self);

    let ts_type = TSType::new_ts_string_keyword(SPAN, self);
    let ts_type_anno = TSTypeAnnotation::boxed(SPAN, ts_type, self);

    FormalParameter::new(
      SPAN,
      ArenaVec::new_in(self),
      pattern,
      Some(ts_type_anno),
      None,
      false,
      None,
      false,
      false,
      self,
    )
  }

  pub fn new_formal_number_parameter(&self, name: &'a str) -> FormalParameter<'a> {
    let pattern = BindingPattern::new_binding_identifier(SPAN, name, self);

    let ts_type = TSType::new_ts_number_keyword(SPAN, self);
    let ts_type_anno = TSTypeAnnotation::boxed(SPAN, ts_type, self);

    FormalParameter::new(
      SPAN,
      ArenaVec::new_in(self),
      pattern,
      Some(ts_type_anno),
      None,
      false,
      None,
      false,
      false,
      self,
    )
  }

  pub fn new_formal_type_parameter(&self, param_name: &'a str, type_name: &'a str) -> FormalParameter<'a> {
    let pattern = BindingPattern::new_binding_identifier(SPAN, param_name, self);

    let ts_type = TSType::new_ts_type_reference(
      SPAN,
      TSTypeName::new_identifier_reference(SPAN, type_name, self),
      None,
      self,
    );
    let ts_type_anno = TSTypeAnnotation::boxed(SPAN, ts_type, self);

    FormalParameter::new(
      SPAN,
      ArenaVec::new_in(self),
      pattern,
      Some(ts_type_anno),
      None,
      false,
      None,
      false,
      false,
      self,
    )
  }

  pub fn new_ts_number_type(&self) -> TSType<'a> {
    TSType::new_ts_number_keyword(SPAN, self)
  }

  pub fn new_ts_string_type(&self) -> TSType<'a> {
    TSType::new_ts_string_keyword(SPAN, self)
  }

  pub fn new_ts_boolean_type(&self) -> TSType<'a> {
    TSType::new_ts_boolean_keyword(SPAN, self)
  }

  pub fn new_ts_array_number_type(&self) -> TSType<'a> {
    TSType::new_ts_array_type(SPAN, TSType::new_ts_number_keyword(SPAN, self), self)
  }

  pub fn new_ts_array_string_type(&self) -> TSType<'a> {
    TSType::new_ts_array_type(SPAN, TSType::new_ts_string_keyword(SPAN, self), self)
  }

  pub fn new_ts_array_boolean_type(&self) -> TSType<'a> {
    TSType::new_ts_array_type(SPAN, TSType::new_ts_boolean_keyword(SPAN, self), self)
  }

  pub fn new_formal_union_types_parameter(&self, param_name: &'a str, ts_types: impl IntoIterator<Item = TSType<'a>>) -> FormalParameter<'a> {
    let pattern = BindingPattern::new_binding_identifier(SPAN, param_name, self);

    let ts_type = self.new_union_type(ts_types);

    let ts_type_anno = TSTypeAnnotation::boxed(SPAN, ts_type, self);

    FormalParameter::new(
      SPAN,
      ArenaVec::new_in(self),
      pattern,
      Some(ts_type_anno),
      None,
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
    args: impl IntoIterator<Item = Argument<'a>>,
  ) -> Statement<'a> {
    let expression = self.new_call_object_method_expression(object_name, method_name, args);
    Statement::new_expression_statement(SPAN, expression, self)
  }

  fn new_call_object_method_expression(
    &self,
    object_name: &'a str,
    method_name: &'a str,
    args: impl IntoIterator<Item = Argument<'a>>,
  ) -> Expression<'a> {
    let object_expr = Expression::new_identifier(SPAN, object_name, self);
    let method_name_ident = IdentifierName::new(SPAN, method_name, self);

    let callee =
      Expression::new_static_member_expression(SPAN, object_expr, method_name_ident, false, self);

    Expression::new_call_expression(
      SPAN,
      callee,
      None,
      ArenaVec::from_iter_in(args, self),
      false,
      self,
    )
  }

  pub fn new_call_console_log(&self, args: impl IntoIterator<Item = Argument<'a>>) -> Statement<'a> {
    self.new_call_object_method_statement("console", "log", args)
  }

  pub fn new_call_console_error(&self, args: impl IntoIterator<Item = Argument<'a>>) -> Statement<'a> {
    self.new_call_object_method_statement("console", "error", args)
  }

  /// 以add开头的方法，都是直接在root节点中添加子节点
  fn add_call_console_log(&mut self, args: impl IntoIterator<Item = Argument<'a>>) {
    let statement = self.new_call_object_method_statement("console", "log", args);
    self.append_to_root(statement);
  }

  pub fn add_call_use_dict(&mut self, dict_names: &[&'a str]) {
    if dict_names.is_empty() {
      return;
    }

    let obj_pattern = self.new_binding_object_pattern(dict_names);

    let call_args = ArenaVec::from_iter_in(
      dict_names
        .iter()
        .map(|&name| self.new_argument_string(name)),
      self,
    );

    let callee = Expression::new_identifier(SPAN, "useDict", self);

    let expression = Expression::new_call_expression(
      SPAN,
      callee,
      None,
      call_args,
      false,
      self,
    );

    let kind = VariableDeclarationKind::Const;

    let declarator = VariableDeclarator::new(
      SPAN,
      obj_pattern,
      None,
      Some(expression),
      false,
      self,
    );

    let declarations = ArenaVec::from_value_in(declarator, self);

    let statement = Statement::new_variable_declaration(
      SPAN,
      kind,
      declarations,
      false,
      self
    );
    self.append_to_root(statement);
  }

  pub fn add_arrow_function(
    &mut self,
    name: &'a str,
    params: impl IntoIterator<Item = FormalParameter<'a>>,
    body_statements: impl IntoIterator<Item = Statement<'a>>,
  ) {
    let formal_params = FormalParameters::boxed(
      SPAN,
      FormalParameterKind::ArrowFormalParameters,
      ArenaVec::from_iter_in(params, self),
      None,
      self,
    );

    let body = ArrowFunctionBody::new_function_body(
      SPAN,
      ArenaVec::new_in(self),
      ArenaVec::from_iter_in(body_statements, self),
      self,
    );

    let init_expr = Expression::new_arrow_function_expression(
      SPAN,
      false,
      None,
      formal_params,
      None,
      body,
      self,
    );

    self.add_named_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
  }

  pub fn add_arrow_async_function(
    &mut self,
    name: &'a str,
    params: impl IntoIterator<Item = FormalParameter<'a>>,
    body_statements: impl IntoIterator<Item = Statement<'a>>,
  ) {
    let formal_params = FormalParameters::boxed(
      SPAN,
      FormalParameterKind::ArrowFormalParameters,
      ArenaVec::from_iter_in(params, self),
      None,
      self,
    );

    let body = ArrowFunctionBody::new_function_body(
      SPAN,
      ArenaVec::new_in(self),
      ArenaVec::from_iter_in(body_statements, self),
      self,
    );

    let init_expr = Expression::new_arrow_function_expression(
      SPAN,
      true,
      None,
      formal_params,
      None,
      body,
      self,
    );

    self.add_named_variable_declaration(VariableDeclarationKind::Const, name, init_expr);
  }
  //endregion

  //region interface
  pub fn add_interface(
    &mut self,
    name: &'a str,
    properties: impl IntoIterator<Item = TSSignature<'a>>,
    base_type_names: &[&'a str],
  ) {
    let bind_ident = BindingIdentifier::new(SPAN, name, self);

    let extends = ArenaVec::from_iter_in(
      base_type_names.iter().map(|name| {
        let type_name = TSTypeName::new_identifier_reference(SPAN, *name, self);
        TSInterfaceHeritage::new(SPAN, type_name, None, self)
      }),
      self,
    );

    let body = TSInterfaceBody::boxed(SPAN, ArenaVec::from_iter_in(properties, self), self);

    let interface_declaration = Declaration::new_ts_interface_declaration(
      SPAN,
      bind_ident,
      None,
      extends,
      body,
      false,
      self,
    );

    let statement = Statement::new_export_declaration(
      SPAN,
      interface_declaration,
      self,
    );

    self.append_to_root(statement);
  }

  pub fn new_interface_property_string(&self, name: &'a str, optional: bool) -> TSSignature<'a> {
    let ts_type = TSType::new_ts_string_keyword(SPAN, self);

    TSSignature::new_ts_property_signature(
      SPAN,
      false,
      optional,
      false,
      PropertyKey::new_identifier(SPAN, name, self),
      Some(TSTypeAnnotation::boxed(SPAN, ts_type, self)),
      self,
    )
  }

  pub fn new_interface_property_number(&self, name: &'a str, optional: bool) -> TSSignature<'a> {
    let ts_type = TSType::new_ts_number_keyword(SPAN, self);

    TSSignature::new_ts_property_signature(
      SPAN,
      false,
      optional,
      false,
      PropertyKey::new_identifier(SPAN, name, self),
      Some(TSTypeAnnotation::boxed(SPAN, ts_type, self)),
      self,
    )
  }

  pub fn new_interface_property_boolean(&self, name: &'a str, optional: bool) -> TSSignature<'a> {
    let ts_type = TSType::new_ts_boolean_keyword(SPAN, self);

    TSSignature::new_ts_property_signature(
      SPAN,
      false,
      optional,
      false,
      PropertyKey::new_identifier(SPAN, name, self),
      Some(TSTypeAnnotation::boxed(SPAN, ts_type, self)),
      self,
    )
  }

  /// 引用外部类型
  pub fn new_interface_property_type(&self, name: &'a str, type_name: &'a str, optional: bool) -> TSSignature<'a> {
    let ident_ref = TSTypeName::new_identifier_reference(SPAN, type_name, self);
    let ts_type = TSType::new_ts_type_reference(SPAN, ident_ref, None, self);

    TSSignature::new_ts_property_signature(
      SPAN,
      false,
      optional,
      false,
      PropertyKey::new_identifier(SPAN, name, self),
      Some(TSTypeAnnotation::boxed(SPAN, ts_type, self)),
      self,
    )
  }

  pub fn new_interface_property_array_type(&self, name: &'a str, type_name: &'a str, optional: bool) -> TSSignature<'a> {
    let ident_ref = TSTypeName::new_identifier_reference(SPAN, type_name, self);
    let ts_type = TSType::new_ts_type_reference(SPAN, ident_ref, None, self);
    let ts_array_type = TSType::new_ts_array_type(SPAN, ts_type, self);

    TSSignature::new_ts_property_signature(
      SPAN,
      false,
      optional,
      false,
      PropertyKey::new_identifier(SPAN, name, self),
      Some(TSTypeAnnotation::boxed(SPAN, ts_array_type, self)),
      self,
    )
  }

  pub fn new_interface_property_any(&self, name: &'a str, optional: bool) -> TSSignature<'a> {
    let ts_type = TSType::new_ts_any_keyword(SPAN, self);
    TSSignature::new_ts_property_signature(
      SPAN,
      false,
      optional,
      false,
      PropertyKey::new_identifier(SPAN, name, self),
      Some(TSTypeAnnotation::boxed(SPAN, ts_type, self)),
      self,
    )
  }

  /// 内部嵌套类型
  pub fn new_interface_property_type_literal(&self, name: &'a str, members: impl IntoIterator<Item = TSSignature<'a>>, optional: bool) -> TSSignature<'a> {
    let ts_type = TSType::new_ts_type_literal(
      SPAN,
      ArenaVec::from_iter_in(members, self),
      self,
    );
    TSSignature::new_ts_property_signature(
      SPAN,
      false,
      optional,
      false,
      PropertyKey::new_identifier(SPAN, name, self),
      Some(TSTypeAnnotation::boxed(SPAN, ts_type, self)),
      self,
    )
  }
  //endregion

  //region request
  pub fn new_call_request_get_expression(
    &self,
    url: &'a str,
    config: &[&'a str],
  ) -> Expression<'a> {
    let url_arg = self.parse_url(url);

    let mut args = ArenaVec::new_in(self);
    args.push(url_arg);

    if !config.is_empty() {
      let config_expr = self.boxed_object_expression(config);
      let config_arg = Argument::ObjectExpression(config_expr);

      args.push(config_arg);
    }

    self.new_call_object_method_expression("request", "get", args)
  }

  pub fn new_call_request_get_statement(&self, url: &'a str, config: &[&'a str]) -> Statement<'a> {
    let url_arg = self.parse_url(url);

    let mut args = ArenaVec::new_in(self);
    args.push(url_arg);

    if !config.is_empty() {
      let config_expr = self.boxed_object_expression(config);
      let config_arg = Argument::ObjectExpression(config_expr);
      args.push(config_arg);
    }

    self.new_call_object_method_statement("request", "get", args)
  }

  pub fn new_return_request_get_statement(&self, url: &'a str, config: &[&'a str]) -> Statement<'a> {
    let expression = self.new_call_request_get_expression(url, config);
    Statement::new_return_statement(
      SPAN,
      Some(expression),
      self
    )
  }

  pub fn new_return_request_post_statement(&self, url: &'a str, data: &'a str) -> Statement<'a> {
    let expression = self.new_call_request_post_expression(url, data);
    Statement::new_return_statement(
      SPAN,
      Some(expression),
      self
    )
  }

  pub fn new_return_request_put_statement(&self, url: &'a str, data: &'a str) -> Statement<'a> {
    let expression = self.new_call_request_put_expression(url, data);
    Statement::new_return_statement(
      SPAN,
      Some(expression),
      self
    )
  }

  pub fn new_return_request_delete_statement(&self, url: &'a str) -> Statement<'a> {
    let expression = self.new_call_request_delete_expression(url);
    Statement::new_return_statement(
      SPAN,
      Some(expression),
      self
    )
  }

  pub fn new_call_request_post_expression(&self, url: &'a str, data: &'a str) -> Expression<'a> {
    let url_arg = self.parse_url(url);

    let mut args = ArenaVec::new_in(self);
    args.push(url_arg);

    let data_arg = self.new_argument_identifier(data);
    args.push(data_arg);

    self.new_call_object_method_expression("request", "post", args)
  }

  pub fn new_call_request_post_statement(&self, url: &'a str, data: &'a str) -> Statement<'a> {
    let url_arg = self.parse_url(url);

    let mut args = ArenaVec::new_in(self);
    args.push(url_arg);

    let data_arg = self.new_argument_identifier(data);
    args.push(data_arg);

    self.new_call_object_method_statement("request", "post", args)
  }

  fn parse_url(&self, url: &'a str) -> Argument<'a> {
    if is_template_string(url) {
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
    }
  }

  pub fn new_call_request_put_statement(&self, url: &'a str, data: &'a str) -> Statement<'a> {
    let url_arg = self.parse_url(url);

    let mut args = ArenaVec::new_in(self);
    args.push(url_arg);

    let data_arg = self.new_argument_identifier(data);
    args.push(data_arg);

    self.new_call_object_method_statement("request", "put", args)
  }

  pub fn new_call_request_put_expression(&self, url: &'a str, data: &'a str) -> Expression<'a> {
    let url_arg = self.parse_url(url);

    let mut args = ArenaVec::new_in(self);
    args.push(url_arg);

    let data_arg = self.new_argument_identifier(data);
    args.push(data_arg);

    self.new_call_object_method_expression("request", "put", args)
  }

  pub fn new_call_request_delete_statement(&self, url: &'a str) -> Statement<'a> {
    let url_arg = self.parse_url(url);

    let mut args = ArenaVec::new_in(self);
    args.push(url_arg);

    self.new_call_object_method_statement("request", "delete", args)
  }

  pub fn new_call_request_delete_expression(&self, url: &'a str) -> Expression<'a> {
    let url_arg = self.parse_url(url);

    let mut args = ArenaVec::new_in(self);
    args.push(url_arg);

    self.new_call_object_method_expression("request", "delete", args)
  }

  pub fn add_api_function(
    &mut self,
    function_name: &'a str,
    input_params: impl IntoIterator<Item = FormalParameter<'a>>,
    return_type_names: &[&'a str],
    body_statements: impl IntoIterator<Item = Statement<'a>>
  ) {
    let mut all_return_type_names = ArenaVec::with_capacity_in(return_type_names.len() + 1, self);
    all_return_type_names.push("Promise");
    all_return_type_names.extend_from_slice(return_type_names);
    let return_type = TSTypeAnnotation::boxed(
      SPAN,
      self.new_generic_type(all_return_type_names.as_slice()),
      self,
    );
    let body = FunctionBody::boxed(
      SPAN,
      ArenaVec::new_in(self),
      ArenaVec::from_iter_in(body_statements, self),
      self,
    );

    let formal_parameters = FormalParameters::boxed(
      SPAN,
      FormalParameterKind::FormalParameter,
      ArenaVec::from_iter_in(input_params, self),
      None,
      self,
    );

    let declaration = Declaration::new_function_declaration(
      SPAN,
      FunctionType::FunctionDeclaration,
      Some(BindingIdentifier::new(SPAN, function_name, self)),
      false,
      false,
      false,
      None,
      None,
      formal_parameters,
      Some(return_type),
      Some(body),
      self,
    );

    let statement = Statement::new_export_declaration(
      SPAN,
      declaration,
      self,
    );

    self.append_to_root(statement);
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
          PropertyKey::new_static_identifier(SPAN, *name, self),
          Expression::new_identifier(SPAN, *name, self),
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

  pub fn new_call_fetch_data_list_api(&self, function_name: &'a str, query_params_name: &'a str) -> Statement<'a> {
    let left_id = self.new_binding_object_pattern(&["rows", "total"]);

    let callee = Expression::new_identifier(SPAN, function_name, self);

    let query_param_value = Argument::new_static_member_expression(
      SPAN,
      Expression::new_identifier(SPAN, query_params_name, self),
      IdentifierName::new(SPAN, "value", self),
      false,
      self,
    );
    let call_args = ArenaVec::from_value_in(query_param_value, self);

    let await_arg= Expression::new_call_expression(
      SPAN,
      callee,
      None,
      call_args,
      false,
      self,
    );

    let right_expr = Expression::new_await_expression(
      SPAN,
      await_arg,
      self,
    );

    let var_decl = VariableDeclarator::new(
      SPAN,
      left_id,
      None,
      Some(right_expr),
      false,
      self,
    );

    Statement::new_variable_declaration(
      SPAN,
      VariableDeclarationKind::Const,
      ArenaVec::from_value_in(var_decl, self),
      false,
      self,
    )
  }

  fn new_binding_object_pattern(&self, keys: &[&'a str]) -> BindingPattern<'a> {
    let obj_props = ArenaVec::from_iter_in(keys.iter().map(|key| {
      BindingProperty::new(
        SPAN,
        PropertyKey::new_static_identifier(SPAN, *key, self),
        BindingPattern::new_binding_identifier(SPAN, *key, self),
        true,
        false,
        self,
      )
    }), self);

    BindingPattern::new_object_pattern(
      SPAN,
      obj_props,
      None,
      self,
    )
  }
  //endregion

  //region type alias
  pub fn add_generic_type_alias(
    &mut self,
    alias: &'a str,
    type_names: &[&'a str],
  ) -> Statement<'a> {
    let current_type = self.new_generic_type(type_names);
    Statement::new_ts_type_alias_declaration(
      SPAN,
      BindingIdentifier::new(SPAN, alias, self),
      None,
      current_type,
      false,
      self,
    )
  }

  pub fn add_union_type_alias(
    &mut self,
    alias: &'a str,
    ts_types: impl IntoIterator<Item = TSType<'a>>
  ) -> Statement<'a> {
    let current_type = self.new_union_type(ts_types);
    Statement::new_ts_type_alias_declaration(
      SPAN,
      BindingIdentifier::new(SPAN, alias, self),
      None,
      current_type,
      false,
      self,
    )
  }
  //endregion

  //region util
  #[inline]
  fn new_return_statement(&self, argument: Option<Expression<'a>>) -> Statement<'a> {
    Statement::new_return_statement(SPAN, argument, self)
  }

  fn new_generic_type(&self, type_names: &[&'a str]) -> TSType<'a> {
    assert!(!type_names.is_empty(), "type_names cannot be empty");

    // 从后往前循环
    let mut iter = type_names.iter().rev();

    let mut current_type = {
      let last_type_name = iter.next().unwrap();
      TSType::new_ts_type_reference(
        SPAN,
        TSTypeName::new_identifier_reference(SPAN, *last_type_name, self),
        None,
        self,
      )
    };

    for outer_type_name in iter {
      let outer_ts_name = TSTypeName::new_identifier_reference(SPAN, *outer_type_name, self);

      let type_params = TSTypeParameterInstantiation::boxed(
        SPAN,
        ArenaVec::from_value_in(current_type, self),
        self,
      );
      current_type = TSType::new_ts_type_reference(SPAN, outer_ts_name, Some(type_params), self);
    }

    current_type
  }

  fn new_union_type(&self, ts_types: impl IntoIterator<Item = TSType<'a>>) -> TSType<'a> {
    TSType::new_ts_union_type(
      SPAN,
      ArenaVec::from_iter_in(ts_types, self),
      self
    )
  }

  pub fn new_try_catch_finally_statement(
    &self,
    try_statements: impl IntoIterator<Item = Statement<'a>>,
    catch_statements: impl IntoIterator<Item = Statement<'a>>,
    finally_statements: impl IntoIterator<Item = Statement<'a>>,
  ) -> Statement<'a> {
    self.new_try_statement(
      try_statements,
      Some(catch_statements),
      Some(finally_statements),
    )
  }

  fn new_try_statement(
    &self,
    try_statements: impl IntoIterator<Item = Statement<'a>>,
    catch_statements: Option<impl IntoIterator<Item = Statement<'a>>>,
    finally_statements: Option<impl IntoIterator<Item = Statement<'a>>>,
  ) -> Statement<'a> {
    // try block
    let try_body_vec = ArenaVec::from_iter_in(try_statements, self);
    let try_body = BlockStatement::new(SPAN, try_body_vec, self);
    let try_body_box = ArenaBox::new_in(try_body, self);

    // catch clause
    let catch_parameter = CatchParameter::new(
      SPAN,
      BindingPattern::BindingIdentifier(BindingIdentifier::boxed(SPAN, "e", self)),
      None,
      self,
    );

    let catch_clause_option = if let Some(catch_statements) = catch_statements {
      let catch_body_vec = ArenaVec::from_iter_in(catch_statements, self);
      let catch_body = BlockStatement::new(SPAN, catch_body_vec, self);
      Some(CatchClause::boxed(
        SPAN,
        Some(catch_parameter),
        ArenaBox::new_in(catch_body, self),
        self,
      ))
    } else {
      None
    };

    // finally block
    let finally_body_option = if let Some(finally_statements) = finally_statements {
      let finally_body_vec = ArenaVec::from_iter_in(finally_statements, self);
      let finally_body = BlockStatement::new(SPAN, finally_body_vec, self);
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
      self,
    )
  }

  #[inline]
  fn new_compare_identifier_string_expression(
    &self,
    identifier_name: &'a str,
    value: &'a str,
    operator: BinaryOperator,
  ) -> Expression<'a> {
    Expression::new_binary_expression(
      SPAN,
      Expression::new_identifier(SPAN, identifier_name, self),
      operator,
      Expression::new_string_literal(SPAN, value, None, self),
      self,
    )
  }

  #[inline]
  fn new_compare_identifier_decimal_expression(
    &self,
    identifier_name: &'a str,
    value: i64,
    operator: BinaryOperator,
  ) -> Expression<'a> {
    Expression::new_binary_expression(
      SPAN,
      Expression::new_identifier(SPAN, identifier_name, self),
      operator,
      Expression::new_numeric_literal(SPAN, value as f64, None, NumberBase::Decimal, self),
      self,
    )
  }

  #[inline]
  fn new_compare_identifier_float_expression(
    &self,
    identifier_name: &'a str,
    value: f64,
    operator: BinaryOperator,
  ) -> Expression<'a> {
    Expression::new_binary_expression(
      SPAN,
      Expression::new_identifier(SPAN, identifier_name, self),
      operator,
      Expression::new_numeric_literal(SPAN, value, None, NumberBase::Float, self),
      self,
    )
  }

  #[inline]
  fn new_compare_identifier_boolean_expression(
    &self,
    identifier_name: &'a str,
    value: bool,
    operator: BinaryOperator,
  ) -> Expression<'a> {
    if value {
      Expression::new_identifier(SPAN, identifier_name, self)
    } else {
      Expression::new_binary_expression(
        SPAN,
        Expression::new_identifier(SPAN, identifier_name, self),
        operator,
        Expression::new_boolean_literal(SPAN, value, self),
        self,
      )
    }
  }
  //endregion

  //region if
  pub fn new_if_statement(
    &self,
    test: Expression<'a>,
    if_body_statements: impl IntoIterator<Item = Statement<'a>>,
  ) -> Statement<'a> {
    let if_body_vec = ArenaVec::from_iter_in(if_body_statements, self);

    Statement::new_if_statement(
      SPAN,
      test,
      Statement::new_block_statement(SPAN, if_body_vec, self),
      None,
      self,
    )
  }

  pub fn new_if_else_statement(
    &self,
    test: Expression<'a>,
    if_body_statements: impl IntoIterator<Item = Statement<'a>>,
    else_body_statements: impl IntoIterator<Item = Statement<'a>>,
  ) -> Statement<'a> {
    let if_body_vec = ArenaVec::from_iter_in(if_body_statements, self);
    let else_body_vec = ArenaVec::from_iter_in(else_body_statements, self);

    Statement::new_if_statement(
      SPAN,
      test,
      Statement::new_block_statement(SPAN, if_body_vec, self),
      Some(Statement::new_block_statement(SPAN, else_body_vec, self)),
      self,
    )
  }

  pub fn new_if_elseif_statement(
    &self,
    test: Expression<'a>,
    if_body_statements: impl IntoIterator<Item = Statement<'a>>,
    else_if_statement: Statement<'a>,
  ) -> Statement<'a> {
    let if_body_vec = ArenaVec::from_iter_in(if_body_statements, self);

    Statement::new_if_statement(
      SPAN,
      test,
      Statement::new_block_statement(SPAN, if_body_vec, self),
      Some(else_if_statement),
      self,
    )
  }
  //endregion

  //region 注释
  pub fn add_line_comment(&mut self, comment: &str) {
    let comment1 = Comment::new(0, 10, CommentKind::Line);
    self.comments.push(comment1);
  }
  //endregion

  pub fn append_to_root(&mut self, statement: Statement<'a>) {
    self.statements.push(statement);
  }

  pub fn append_expression_to_root(&mut self, expression: Expression<'a>) {
    let statement = Statement::new_expression_statement(SPAN, expression, self);
    self.append_to_root(statement);
  }

  pub fn get_code(self) -> String {
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
    let codegen_return = Codegen::new()
      .with_options(CodegenOptions {
        single_quote: false,
        minify: false,
        comments: Default::default(),
        source_map_path: None,
        indent_char: IndentChar::Space,
        indent_width: 2,
        initial_indent: 0,
      })
      .build(&program);

    codegen_return.code
  }
}

fn is_template_string(content: &str) -> bool {
  content.len() >= 2 && content.starts_with("`") && content.ends_with("`")
}

fn is_computed_member(name: &str) -> bool {
  name.len() >= 2 && name.starts_with("[") && name.ends_with("]")
}

fn strip_computed_key(s: &str) -> &str {
  &s[1..s.len() - 1]
}

#[cfg(test)]
mod tests {
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
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "");
  }

  #[test]
  fn test_import_default() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_import_default("module-a", "a");
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "import a from \"module-a\";\n");
  }

  #[test]
  fn test_import_named_value() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_import_named_value("source", &["a", "b"]);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "import { a, b } from \"source\";\n");
  }

  #[test]
  fn test_import_named_type() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_import_named_type("source", &["a", "b"]);
    let actual_code = script_ast.get_code();
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
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "import { a, type b } from \"source\";\n");
  }

  #[test]
  fn test_add_const_string() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_const_string("a", "b");
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "const a = \"b\";\n");
  }

  #[test]
  fn test_add_const_integer() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_const_integer("a", 1);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "const a = 1;\n");
  }

  #[test]
  fn test_add_const_float() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_const_float("a", 1.2);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "const a = 1.2;\n");
  }

  #[test]
  fn test_add_const_boolean() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_const_boolean("a", false);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "const a = false;\n");
  }

  #[test]
  fn test_add_const_ref_string() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_const_ref_string("a", "");
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "const a = ref<string>(\"\");\n");
  }

  #[test]
  fn test_add_const_ref_number() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_const_ref_number("a", 0.0);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "const a = ref<number>(0);\n");
  }

  #[test]
  fn test_add_const_ref_boolean() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_const_ref_boolean("a", false);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "const a = ref<boolean>(false);\n");
  }

  #[test]
  fn test_add_const_ref_string_array() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_const_ref_string_array("a");
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "const a = ref<string[]>([]);\n");
  }

  #[test]
  fn test_add_const_ref_number_array() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_const_ref_number_array("a");
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "const a = ref<number[]>([]);\n");
  }

  #[test]
  fn test_add_const_ref_object_array() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_const_ref_object_array("a", "UserInfo");
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "const a = ref<UserInfo[]>([]);\n");
  }

  #[test]
  fn test_add_reactive_object() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_const_reactive_object("a", &["UserInfo"], [
      script_ast.new_decimal_object_property("pageNum", 1.0),
      script_ast.new_undefined_object_property("a"),
    ]);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "const a: UserInfo = reactive({\n  pageNum: 1,\n  a: undefined\n});\n");
  }

  #[test]
  fn test_add_reactive_object_generic_type() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_const_reactive_object("a", &["TheType", "UserInfo"], [
      script_ast.new_decimal_object_property("pageNum", 1.0),
      script_ast.new_undefined_object_property("a"),
    ]);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "const a: TheType<UserInfo> = reactive({\n  pageNum: 1,\n  a: undefined\n});\n");
  }

  #[test]
  fn test_add_arrow_function_empty() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_arrow_function("a", [], []);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "const a = () => {};\n");
  }

  #[test]
  fn test_add_arrow_async_function_empty() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_arrow_async_function("a", [], []);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "const a = async () => {};\n");
  }

  #[test]
  fn test_add_call_console_log() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);

    script_ast.add_call_console_log([
      script_ast.new_argument_string("a"),
      script_ast.new_argument_decimal(1.0),
      script_ast.new_argument_boolean(true),
    ]);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "console.log(\"a\", 1, true);\n");
  }

  #[test]
  fn test_add_arrow_function_one_parameter_one_statement() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);

    let console_log = script_ast.new_call_console_log([script_ast.new_argument_identifier("b")]);
    script_ast.add_arrow_function(
      "a",
      [script_ast.new_formal_string_parameter("b")],
      [console_log],
    );
    let actual_code = script_ast.get_code();
    assert_eq!(
      actual_code,
      "const a = (b: string) => {\n  console.log(b);\n};\n"
    );
  }

  #[test]
  fn test_add_set_ref_string_value() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_set_ref_string_value("a", "b");
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "a.value = \"b\";\n");
  }

  #[test]
  fn test_add_set_ref_decimal_value() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_set_ref_decimal_value("a", 1);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "a.value = 1;\n");
  }

  #[test]
  fn test_add_set_ref_boolean_value() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_set_ref_boolean_value("a", true);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "a.value = true;\n");
  }

  #[test]
  fn test_new_clear_ref_object_property() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let statement = script_ast.new_clear_ref_object_property("a", "b");
    script_ast.append_to_root(statement);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "a.value.b = {};\n");
  }

  #[test]
  fn test_new_check_ref_value_is_blank() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let expression = script_ast.new_check_ref_value_is_blank("a");
    script_ast.append_expression_to_root(expression);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "null != a.value && \"\" != a.value;\n");
  }

  #[test]
  fn test_new_right_member_expression_computed_string() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let expression = script_ast.new_right_member_expression(&["a", "b", "[c]"]);
    script_ast.append_expression_to_root(expression);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "a.b[\"c\"];\n");
  }

  #[test]
  fn test_new_right_member_expression_computed_number() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let expression = script_ast.new_right_member_expression(&["a", "b", "[0]"]);
    script_ast.append_expression_to_root(expression);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "a.b[0];\n");
  }

  #[test]
  fn test_new_set_member_value() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let expression = script_ast.new_set_member_value(&["a", "b", "[c]"], &["d", "e", "[0]"]);
    script_ast.append_to_root(expression);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "a.b[\"c\"] = d.e[0];\n");
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
    script_ast.append_to_root(request_get);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "request.get(\"url\");\n");
  }

  #[test]
  fn test_new_call_request_get_string_params() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);

    let request_get = script_ast.new_call_request_get_statement("url", &["params"]);
    script_ast.append_to_root(request_get);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "request.get(\"url\", { params });\n");
  }

  #[test]
  fn test_new_call_request_get_template_string() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);

    let request_get = script_ast.new_call_request_get_statement("`base_url/${id}`", &[]);
    script_ast.append_to_root(request_get);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "request.get(`base_url/${id}`);\n");
  }

  #[test]
  fn test_new_call_request_post_url_string_data() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let request_post = script_ast.new_call_request_post_statement("url", "data");
    script_ast.append_to_root(request_post);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "request.post(\"url\", data);\n");
  }

  #[test]
  fn test_new_call_request_post_url_string_template_data() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let request_post = script_ast.new_call_request_post_statement("`url/${var1}`", "data");
    script_ast.append_to_root(request_post);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "request.post(`url/${var1}`, data);\n");
  }

  #[test]
  fn test_new_call_request_put_url_string_data() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let request_post = script_ast.new_call_request_put_statement("url", "data");
    script_ast.append_to_root(request_post);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "request.put(\"url\", data);\n");
  }

  #[test]
  fn test_new_call_request_put_url_string_template_data() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let request_post = script_ast.new_call_request_put_statement("`url/${var1}`", "data");
    script_ast.append_to_root(request_post);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "request.put(`url/${var1}`, data);\n");
  }

  #[test]
  fn test_new_call_request_delete_url_string() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let request_post = script_ast.new_call_request_delete_statement("url");
    script_ast.append_to_root(request_post);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "request.delete(\"url\");\n");
  }

  #[test]
  fn test_new_call_request_delete_url_string_template() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let request_post = script_ast.new_call_request_delete_statement("`url/${var1}`");
    script_ast.append_to_root(request_post);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "request.delete(`url/${var1}`);\n");
  }

  #[test]
  fn test_new_api_function() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);

    script_ast.add_api_function(
      "fetchDataList",
      [],
      &[],
      []
    );

    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "export function fetchDataList(): Promise {}\n");
  }

  #[test]
  fn test_new_call_fetch_data_list_api() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);

    let statement = script_ast.new_call_fetch_data_list_api("fetchDataList", "queryParams");
    script_ast.append_to_root(statement);

    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "const { rows, total } = await fetchDataList(queryParams.value);\n");
  }

  #[test]
  fn test_new_object_expression() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let obj_expr = script_ast.boxed_object_expression(&["param1", "param2"]);

    let statement = Statement::new_expression_statement(
      SPAN,
      Expression::ObjectExpression(obj_expr),
      script_ast.builder(),
    );

    script_ast.append_to_root(statement);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "({\n  param1,\n  param2\n});\n");
  }

  #[test]
  fn test_new_return_statement_no_argument() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);

    let statement = script_ast.new_return_statement(None);
    script_ast.append_to_root(statement);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "return;\n");
  }

  #[test]
  fn test_new_return_statement_string_argument() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let str_expr = script_ast.new_expression_string("a");
    let statement = script_ast.new_return_statement(Some(str_expr));
    script_ast.append_to_root(statement);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "return \"a\";\n");
  }

  #[test]
  fn test_new_return_statement_request_get() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);

    let request_get_expr = script_ast.new_call_request_get_expression("url", &[]);
    let statement = script_ast.new_return_statement(Some(request_get_expr));
    script_ast.append_to_root(statement);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "return request.get(\"url\");\n");
  }

  #[test]
  fn test_add_generic_type_alias() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let promise_user_type = script_ast.add_generic_type_alias("NewType", &["Promise", "User"]);
    script_ast.append_to_root(promise_user_type);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "type NewType = Promise<User>;\n");
  }

  #[test]
  fn test_add_union_type_alias() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let promise_user_type = script_ast.add_union_type_alias("NewType", [
      script_ast.new_ts_number_type(),
      script_ast.new_ts_array_number_type()
    ]);
    script_ast.append_to_root(promise_user_type);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "type NewType = number | number[];\n");
  }

  #[test]
  fn test_add_call_use_dict() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);

    script_ast.add_call_use_dict(&["a", "b"]);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "const { a, b } = useDict(\"a\", \"b\");\n");
  }

  #[test]
  fn test_add_interface() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);

    script_ast.add_interface(
      "TheType",
      [
        script_ast.new_interface_property_string("a", true),
        script_ast.new_interface_property_number("b", true),
        script_ast.new_interface_property_boolean("c", true),
        script_ast.new_interface_property_any("d", true),
      ],
      &["TheBase1", "TheBase2"],
    );

    let actual_code = script_ast.get_code();
    assert_eq!(
      actual_code,
      "export interface TheType extends TheBase1, TheBase2 {\n  a?: string;\n  b?: number;\n  c?: boolean;\n  d?: any;\n}\n"
    );
  }

  #[test]
  fn test_new_try_catch_finally_statement() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let statement = script_ast.new_try_catch_finally_statement([], [], []);
    script_ast.append_to_root(statement);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "try {} catch (e) {} finally {}\n");
  }

  #[test]
  fn test_new_try_statement() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let statement =
      script_ast.new_try_statement([], None::<[Statement; 0]>, None::<[Statement; 0]>);
    script_ast.append_to_root(statement);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "try {}\n");
  }

  #[test]
  fn test_new_compare_identifier_string_expression() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let expression =
      script_ast.new_compare_identifier_string_expression("a", "b", BinaryOperator::StrictEquality);
    script_ast.append_to_root(Statement::new_expression_statement(
      SPAN,
      expression,
      &script_ast,
    ));
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "a === \"b\";\n");
  }

  #[test]
  fn test_new_compare_identifier_decimal_expression() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let expression = script_ast.new_compare_identifier_decimal_expression(
      "a",
      200,
      BinaryOperator::StrictEquality,
    );
    script_ast.append_to_root(Statement::new_expression_statement(
      SPAN,
      expression,
      &script_ast,
    ));
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "a === 200;\n");
  }

  #[test]
  fn test_new_compare_identifier_float_expression() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let expression = script_ast.new_compare_identifier_float_expression(
      "a",
      200.1,
      BinaryOperator::StrictEquality,
    );
    script_ast.append_to_root(Statement::new_expression_statement(
      SPAN,
      expression,
      &script_ast,
    ));
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "a === 200.1;\n");
  }

  #[test]
  fn test_new_compare_identifier_boolean_expression_false() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let expression = script_ast.new_compare_identifier_boolean_expression(
      "a",
      false,
      BinaryOperator::StrictEquality,
    );
    script_ast.append_to_root(Statement::new_expression_statement(
      SPAN,
      expression,
      &script_ast,
    ));
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "a === false;\n");
  }

  #[test]
  fn test_new_compare_identifier_boolean_expression_true() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let expression = script_ast.new_compare_identifier_boolean_expression(
      "a",
      true,
      BinaryOperator::StrictEquality,
    );
    script_ast.append_to_root(Statement::new_expression_statement(
      SPAN,
      expression,
      &script_ast,
    ));
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "a;\n");
  }

  #[test]
  fn test_new_if_statement() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let test = script_ast.new_compare_identifier_boolean_expression(
      "a",
      true,
      BinaryOperator::StrictEquality,
    );
    let statement = script_ast.new_if_statement(test, []);
    script_ast.append_to_root(statement);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "if (a) {}\n");
  }

  #[test]
  fn test_new_if_else_statement() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let test = script_ast.new_compare_identifier_boolean_expression(
      "a",
      true,
      BinaryOperator::StrictEquality,
    );
    let statement = script_ast.new_if_else_statement(test, [], []);
    script_ast.append_to_root(statement);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "if (a) {} else {}\n");
  }

  #[test]
  fn test_new_if_elseif_statement() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    let test1 = script_ast.new_compare_identifier_boolean_expression(
      "a",
      true,
      BinaryOperator::StrictEquality,
    );
    let test2 = script_ast.new_compare_identifier_boolean_expression(
      "b",
      true,
      BinaryOperator::StrictEquality,
    );
    let last_if_statement = script_ast.new_if_else_statement(test2, [], []);
    let statement = script_ast.new_if_elseif_statement(test1, [], last_if_statement);
    script_ast.append_to_root(statement);
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "if (a) {} else if (b) {} else {}\n");
  }

  #[test]
  #[ignore]
  fn test_add_comment() {
    let allocator = Allocator::new();
    let mut script_ast = ScriptAst::new(&allocator);
    script_ast.add_line_comment("我是注释");
    let actual_code = script_ast.get_code();
    assert_eq!(actual_code, "// 我是注释\n");
  }
}
