use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi]
pub struct PageInfo {
  pub parent_keys: Vec<String>,
  pub page_key: String,
  pub page_name: String,
}

// 以下代码是 AI 生成，仅供参考
// ==================== 基础通用类型 ====================

/// 通用数值/字符串联合类型，对应 TS 的 `number | string`
#[derive(Debug, Clone)]
#[napi]
pub enum NumberOrString {
  Number(f64),
  String(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[napi(string_enum)]
pub enum UIParagraphType {
  None,
  Data,
  Ui,
  Changelog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[napi(string_enum)]
pub enum MatchOperation {
  Contains,
  Equal,
  Between,
  ItemEqual,
  ItemContains,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[napi(string_enum)]
pub enum UIComponentKey {
  TextInput,
  NumberInput,
  Textarea,
  DatePicker,
  Select,
  DictSelect,
  DictCheckbox,
  DictRadio,
  ImageUpload,
  FileUpload,
  Editor,
}
//
// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// #[napi(string_enum="")]
// pub enum UIParamActionKey {
//     #[napi(rename = "list_query")]
//     ListQuery,
//     #[napi(rename = "form_reset")]
//     FormReset,
// }
//
// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// #[napi]
// pub enum UIActionKey {
//     #[napi(rename = "create")]
//     Create,
//     #[napi(rename = "update")]
//     Update,
//     #[napi(rename = "remove")]
//     Remove,
//     #[napi(rename = "view")]
//     View,
//     #[napi(rename = "export")]
//     Export,
//     #[napi(rename = "show_search")]
//     ShowSearch,
//     #[napi(rename = "list_query")]
//     ListQuery,
//     #[napi(rename = "set_table_columns")]
//     SetTableColumns,
// }
//
// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// #[napi]
// pub enum ColumnComponentKey {
//     #[napi(rename = "Text")]
//     Text,
//     #[napi(rename = "Number")]
//     Number,
//     #[napi(rename = "Currency")]
//     Currency,
//     #[napi(rename = "Date")]
//     Date,
//     #[napi(rename = "DateTime")]
//     DateTime,
//     #[napi(rename = "Image")]
//     Image,
//     #[napi(rename = "Dict")]
//     Dict,
// }
//
// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// #[napi]
// pub enum DatePickerMode {
//     #[napi(rename = "year")]
//     Year,
//     #[napi(rename = "years")]
//     Years,
//     #[napi(rename = "month")]
//     Month,
//     #[napi(rename = "months")]
//     Months,
//     #[napi(rename = "date")]
//     Date,
//     #[napi(rename = "dates")]
//     Dates,
//     #[napi(rename = "datetime")]
//     DateTime,
//     #[napi(rename = "week")]
//     Week,
//     #[napi(rename = "datetimerange")]
//     DateTimeRange,
//     #[napi(rename = "daterange")]
//     DateRange,
//     #[napi(rename = "monthrange")]
//     MonthRange,
//     #[napi(rename = "yearrange")]
//     YearRange,
// }
//
// // ==================== 页面顶层结构 ====================
//
#[derive(Debug, Clone)]
#[napi(object)]
pub struct UIPage {
  pub parent_paths: Vec<String>,
  pub content: String, // UIPageContent,
}
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct UIPageContent {
//     pub view: Option<UIView>,
// }
//
// /// 视图模式判别联合：目录模式 / 模板模式
// #[derive(Debug, Clone)]
// #[napi(discriminant = "type")]
// pub enum UIView {
//     #[napi(rename = "outline")]
//     Outline(UIOutline),
//     #[napi(rename = "html")]
//     Html(UIHtml),
// }
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct UIOutline {
//     pub root: CPage,
// }
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct UIHtml {
//     pub content: String,
// }
//
// // ==================== 页面根组件 ====================
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CPage {
//     pub children: Vec<CPageChild>,
// }
//
// /// CPage 子节点判别联合
// #[derive(Debug, Clone)]
// #[napi(discriminant = "type")]
// pub enum CPageChild {
//     #[napi(rename = "table")]
//     Table(CTable),
//     #[napi(rename = "dialog")]
//     Dialog(CDialog),
//     #[napi(rename = "form")]
//     Form(CForm),
// }
//
// // ==================== 表格组件体系 ====================
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CTable {
//     pub params: Option<CTableParamSlot>,
//     pub actions: Option<CTableActionSlot>,
//     pub columns: Option<CTableColumnSlot>,
//     pub pagination: Option<bool>,
// }
//
// // --- 查询参数区 ---
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CTableParamSlot {
//     pub name: String,
//     pub children: Vec<CTableParamChild>,
// }
//
// /// 表格参数区子节点判别联合
// #[derive(Debug, Clone)]
// #[napi(discriminant = "type")]
// pub enum CTableParamChild {
//     #[napi(rename = "tableParam")]
//     ParamItem(CTableParamItem),
//     #[napi(rename = "tableParamAction")]
//     ParamAction(CTableParamAction),
// }
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CTableParamItem {
//     pub property: String,
//     pub label: String,
//     pub operation: MatchOperation,
//     pub component: UIComponentKey,
//     pub placeholder: Option<String>,
//     pub data: Option<String>,
//     pub value_field: Option<String>,
//     pub label_field: Option<String>,
// }
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CTableParamAction {
//     pub actions: Vec<UIParamActionKey>,
// }
//
// // --- 操作按钮区 ---
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CTableActionSlot {
//     pub start: Option<CTableActionStartSlot>,
//     pub end: Option<CTableActionEndSlot>,
// }
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CTableActionStartSlot {
//     pub children: Vec<CTableAction>,
// }
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CTableActionEndSlot {
//     pub children: Vec<CTableAction>,
// }
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CTableAction {
//     pub name: UIActionKey,
// }
//
// // --- 表格列区 ---
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CTableColumnSlot {
//     pub children: Vec<CTableColumnChild>,
// }
//
// /// 表格列区子节点判别联合
// #[derive(Debug, Clone)]
// #[napi(discriminant = "type")]
// pub enum CTableColumnChild {
//     #[napi(rename = "tableColumn")]
//     Column(CTableColumn),
//     #[napi(rename = "tableColumnActions")]
//     ColumnActions(CTableColumnActions),
// }
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CTableColumn {
//     pub property: Option<String>,
//     pub label: String,
//     pub component: Option<ColumnComponentKey>,
//     pub width: Option<NumberOrString>,
//     pub data: Option<String>,
//     pub precision: Option<i32>,
//     pub thousandth: Option<bool>,
// }
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CTableColumnActions {
//     pub actions: Vec<CTableColumnAction>,
// }
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CTableColumnAction {
//     pub name: UIActionKey,
// }
//
// // ==================== 对话框组件 ====================
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CDialog {
//     pub width: Option<NumberOrString>,
//     pub children: Vec<CDialogChild>,
// }
//
// /// 对话框子节点判别联合
// #[derive(Debug, Clone)]
// #[napi(discriminant = "type")]
// pub enum CDialogChild {
//     #[napi(rename = "table")]
//     Table(CTable),
//     #[napi(rename = "form")]
//     Form(CForm),
// }
//
// // ==================== 表单组件体系 ====================
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CForm {
//     pub name: Option<String>,
//     pub column_count: i32,
//     pub label_width: Option<NumberOrString>,
//     pub fields: Vec<CFormFieldItem>,
// }
//
// /// 表单项判别联合（含所有字段组件 + 空白占位）
// #[derive(Debug, Clone)]
// #[napi(discriminant = "type")]
// pub enum CFormFieldItem {
//     #[napi(rename = "TextInput")]
//     TextInput(CFormTextInput),
//     #[napi(rename = "NumberInput")]
//     NumberInput(CFormNumberInput),
//     #[napi(rename = "Textarea")]
//     Textarea(CFormTextarea),
//     #[napi(rename = "DatePicker")]
//     DatePicker(CFormDatePicker),
//     #[napi(rename = "Select")]
//     Select(CFormSelect),
//     #[napi(rename = "DictSelect")]
//     DictSelect(CFormDictSelect),
//     #[napi(rename = "DictCheckbox")]
//     DictCheckbox(CFormDictCheckbox),
//     #[napi(rename = "DictRadio")]
//     DictRadio(CFormDictRadio),
//     #[napi(rename = "ImageUpload")]
//     ImageUpload(CFormImageUpload),
//     #[napi(rename = "FileUpload")]
//     FileUpload(CFormFileUpload),
//     #[napi(rename = "Editor")]
//     Editor(CFormEditor),
//     #[napi(rename = "BlankItem")]
//     BlankItem(CBlankItem),
// }
//
// // --- 各表单项结构体（CFormField 字段直接展开，对齐 TS 继承语义） ---
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CFormTextInput {
//     pub property: Option<String>,
//     pub label: String,
//     pub required: bool,
//     pub span: i32,
//     pub minlength: Option<i32>,
//     pub maxlength: Option<i32>,
// }
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CFormNumberInput {
//     pub property: Option<String>,
//     pub label: String,
//     pub required: bool,
//     pub span: i32,
//     pub min: f64,
//     pub max: f64,
//     pub precision: i32,
//     pub unit: Option<String>,
// }
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CFormTextarea {
//     pub property: Option<String>,
//     pub label: String,
//     pub required: bool,
//     pub span: i32,
//     pub rows: Option<i32>,
// }
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CFormDatePicker {
//     pub property: Option<String>,
//     pub label: String,
//     pub required: bool,
//     pub span: i32,
//     pub mode: DatePickerMode,
// }
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CFormSelect {
//     pub property: Option<String>,
//     pub label: String,
//     pub required: bool,
//     pub span: i32,
//     pub options: String,
//     pub value_field: String,
//     pub label_field: String,
//     pub unit: Option<String>,
// }
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CFormDictSelect {
//     pub property: Option<String>,
//     pub label: String,
//     pub required: bool,
//     pub span: i32,
//     pub dict_type: String,
// }
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CFormDictCheckbox {
//     pub property: Option<String>,
//     pub label: String,
//     pub required: bool,
//     pub span: i32,
//     pub dict_type: String,
// }
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CFormDictRadio {
//     pub property: Option<String>,
//     pub label: String,
//     pub required: bool,
//     pub span: i32,
//     pub dict_type: String,
// }
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CFormImageUpload {
//     pub property: Option<String>,
//     pub label: String,
//     pub required: bool,
//     pub span: i32,
// }
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CFormFileUpload {
//     pub property: Option<String>,
//     pub label: String,
//     pub required: bool,
//     pub span: i32,
// }
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CFormEditor {
//     pub property: Option<String>,
//     pub label: String,
//     pub required: bool,
//     pub span: i32,
// }
//
// #[derive(Debug, Clone)]
// #[napi(object)]
// pub struct CBlankItem {
//     pub span: Option<i32>,
// }
//
// // ==================== 顶层全节点联合 ====================
//
// /// 所有组件节点的总联合类型，完全对应 TS 的 `CNode`
// #[derive(Debug, Clone)]
// #[napi(discriminant = "type")]
// pub enum CNode {
//     #[napi(rename = "page")]
//     Page(CPage),
//     #[napi(rename = "table")]
//     Table(CTable),
//     #[napi(rename = "dialog")]
//     Dialog(CDialog),
//     #[napi(rename = "form")]
//     Form(CForm),
//     #[napi(rename = "TextInput")]
//     FormTextInput(CFormTextInput),
//     #[napi(rename = "NumberInput")]
//     FormNumberInput(CFormNumberInput),
//     #[napi(rename = "Textarea")]
//     FormTextarea(CFormTextarea),
//     #[napi(rename = "DatePicker")]
//     FormDatePicker(CFormDatePicker),
//     #[napi(rename = "Select")]
//     FormSelect(CFormSelect),
//     #[napi(rename = "DictSelect")]
//     FormDictSelect(CFormDictSelect),
//     #[napi(rename = "DictCheckbox")]
//     FormDictCheckbox(CFormDictCheckbox),
//     #[napi(rename = "DictRadio")]
//     FormDictRadio(CFormDictRadio),
//     #[napi(rename = "ImageUpload")]
//     FormImageUpload(CFormImageUpload),
//     #[napi(rename = "FileUpload")]
//     FormFileUpload(CFormFileUpload),
//     #[napi(rename = "Editor")]
//     FormEditor(CFormEditor),
//     #[napi(rename = "BlankItem")]
//     BlankItem(CBlankItem),
//     #[napi(rename = "tableParamSlot")]
//     TableParamSlot(CTableParamSlot),
//     #[napi(rename = "tableParam")]
//     TableParamItem(CTableParamItem),
//     #[napi(rename = "tableParamAction")]
//     TableParamAction(CTableParamAction),
//     #[napi(rename = "tableActionSlot")]
//     TableActionSlot(CTableActionSlot),
//     #[napi(rename = "tableActionStartSlot")]
//     TableActionStartSlot(CTableActionStartSlot),
//     #[napi(rename = "tableActionEndSlot")]
//     TableActionEndSlot(CTableActionEndSlot),
//     #[napi(rename = "tableAction")]
//     TableAction(CTableAction),
//     #[napi(rename = "tableColumnSlot")]
//     TableColumnSlot(CTableColumnSlot),
//     #[napi(rename = "tableColumn")]
//     TableColumn(CTableColumn),
//     #[napi(rename = "tableColumnActions")]
//     TableColumnActions(CTableColumnActions),
//     #[napi(rename = "tableColumnAction")]
//     TableColumnAction(CTableColumnAction),
// }
