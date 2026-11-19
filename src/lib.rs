//! Interpreter for the source-attested subset of the historical Jeem language.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

type VariableCell = Rc<RefCell<Option<Value>>>;
pub type FileHandle = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileMode {
    Read,
    Write,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: {}",
            self.span.line, self.span.column, self.message
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    Integer,
    SmallInteger,
    Real,
    Logical,
    Character,
    Text,
    Pointer,
    File,
    Array,
    IntegerArray,
    SmallIntegerArray,
    RealArray,
    LogicalArray,
    CharacterArray,
    Record(u16),
    RecordArray(u16),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Integer(i64),
    Real(f64),
    Logical(bool),
    Character(char),
    Text(String),
    Array(Vec<VariableCell>),
    Record {
        type_id: u16,
        fields: HashMap<String, VariableCell>,
    },
    File(Option<FileHandle>),
    /// Opaque pointer values are represented for host integration. The paper
    /// does not define allocation, dereference, or null-pointer syntax.
    Pointer(Option<u64>),
}

impl Value {
    /// Returns a deep copy of an initialized field of a record value.
    pub fn record_field(&self, name: &str) -> Option<Option<Value>> {
        let Self::Record { fields, .. } = self else {
            return None;
        };
        fields
            .get(name)
            .map(|cell| cell.borrow().as_ref().map(clone_value_deep))
    }

    pub fn record_type_id(&self) -> Option<u16> {
        match self {
            Self::Record { type_id, .. } => Some(*type_id),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer(value) => write!(f, "{value}"),
            Self::Real(value) => write!(f, "{value}"),
            Self::Logical(value) => write!(f, "{}", if *value { "صواب" } else { "خطأ" }),
            Self::Character(value) => write!(f, "{value}"),
            Self::Text(value) => write!(f, "{value}"),
            Self::Array(_) => write!(f, "<صفيفة>"),
            Self::Record { .. } => write!(f, "<مركب>"),
            Self::File(Some(_)) => write!(f, "<ملف مفتوح>"),
            Self::File(None) => write!(f, "<ملف مغلق>"),
            Self::Pointer(Some(value)) => write!(f, "<مؤشر:{value}>"),
            Self::Pointer(None) => write!(f, "<مؤشر>"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Program {
    declarations: HashMap<String, (Type, Span)>,
    arrays: HashMap<String, ArraySpec>,
    record_types: HashMap<u16, RecordDefinition>,
    constants: HashMap<String, Value>,
    routines: HashMap<String, Routine>,
    libraries: HashSet<String>,
    statements: Vec<Stmt>,
}

#[derive(Clone, Debug)]
struct TypeAlias {
    ty: Type,
    array_spec: Option<ArraySpec>,
}

#[derive(Clone, Debug)]
struct Parameter {
    name: String,
    ty: Type,
    constant: bool,
}

#[derive(Clone, Debug)]
struct Routine {
    parameters: Vec<Parameter>,
    return_type: Option<Type>,
    visible_globals: HashMap<String, (Type, Span)>,
    visible_constants: HashMap<String, Value>,
    constants: HashMap<String, Value>,
    visible_arrays: HashMap<String, ArraySpec>,
    declarations: HashMap<String, (Type, Span)>,
    arrays: HashMap<String, ArraySpec>,
    statements: Vec<Stmt>,
    span: Span,
}

#[derive(Clone, Debug)]
struct RecordField {
    ty: Type,
    array_spec: Option<ArraySpec>,
}

#[derive(Clone, Debug)]
struct RecordDefinition {
    name: String,
    fields: HashMap<String, RecordField>,
}

#[derive(Clone, Debug)]
struct ArraySpec {
    element_type: Type,
    dimensions: Vec<usize>,
}

#[derive(Clone, Debug)]
enum LValue {
    Variable {
        name: String,
        span: Span,
    },
    Index {
        name: String,
        indexes: Vec<Expr>,
        span: Span,
    },
    IndexedField {
        name: String,
        indexes: Vec<Expr>,
        fields: Vec<String>,
        span: Span,
    },
    Field {
        name: String,
        fields: Vec<String>,
        span: Span,
    },
}

#[derive(Clone, Debug)]
enum Stmt {
    Assign {
        target: LValue,
        value: Expr,
        span: Span,
    },
    Read {
        targets: Vec<LValue>,
        span: Span,
    },
    Write {
        parts: Vec<OutputPart>,
        span: Span,
    },
    While {
        condition: Condition,
        body: Vec<Stmt>,
        span: Span,
    },
    RepeatUntil {
        condition: Condition,
        body: Vec<Stmt>,
        span: Span,
    },
    Counted {
        count: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    For {
        name: String,
        start: Expr,
        end: Expr,
        step: Option<Expr>,
        body: Vec<Stmt>,
        span: Span,
    },
    Call {
        name: String,
        arguments: Vec<Expr>,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    Terminate {
        span: Span,
    },
    If {
        condition: Condition,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
        span: Span,
    },
    Draw {
        geometry: DrawGeometry,
        span: Span,
    },
    OpenFile {
        name: String,
        path: Expr,
        mode: FileMode,
        span: Span,
    },
    ReadFile {
        name: String,
        targets: Vec<LValue>,
        span: Span,
    },
    WriteFile {
        name: String,
        parts: Vec<OutputPart>,
        span: Span,
    },
    CloseFile {
        name: String,
        span: Span,
    },
}

#[derive(Clone, Debug)]
enum OutputPart {
    Expression(Expr),
    Text(String),
    Newline,
}

#[derive(Clone, Debug)]
enum DrawGeometry {
    Point(Expr, Expr),
    Line(Vec<(Expr, Expr)>),
    Circle {
        center: (Expr, Expr),
        diameter: Expr,
    },
    Ellipse {
        center: (Expr, Expr),
        diameters: (Expr, Expr),
    },
    Rectangle {
        from: (Expr, Expr),
        to: (Expr, Expr),
    },
    Triangle([(Expr, Expr); 3]),
    StrokeColor([Expr; 3]),
    FillColor([Expr; 3]),
    FillCircle {
        center: (Expr, Expr),
        diameter: Expr,
    },
    FillEllipse {
        center: (Expr, Expr),
        diameters: (Expr, Expr),
    },
    FillRectangle {
        from: (Expr, Expr),
        to: (Expr, Expr),
    },
    FillTriangle([(Expr, Expr); 3]),
    FillCanvas(Option<[Expr; 3]>),
    Label {
        position: Option<(Expr, Expr)>,
        values: Vec<OutputPart>,
    },
}

#[derive(Clone, Debug)]
enum Expr {
    Integer(i64),
    Real(f64),
    Logical(bool),
    Text(String),
    Character(char),
    Variable(String, Span),
    Index(String, Vec<Expr>, Span),
    IndexedField(String, Vec<Expr>, Vec<String>, Span),
    Field(String, Vec<String>, Span),
    Call(String, Vec<Expr>, Span),
    Unary(Box<Expr>, Unary, Span),
    Binary(Box<Expr>, Arithmetic, Box<Expr>, Span),
}

#[derive(Clone, Debug)]
enum Condition {
    Expression(Expr),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Arithmetic {
    Add,
    Subtract,
    Multiply,
    Divide,
    IntegerDivide,
    Power,
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
    Equal,
    NotEqual,
    And,
    Or,
}

#[derive(Clone, Copy, Debug)]
enum Unary {
    Negate,
    Not,
    Absolute,
}

#[derive(Clone, Debug)]
enum ControlFlow {
    Continue,
    Return(Option<Value>),
    Halt,
}

#[derive(Clone, Debug)]
struct SourceLine {
    number: usize,
    text: String,
}

struct Parser {
    lines: Vec<SourceLine>,
    at: usize,
    declarations: HashMap<String, (Type, Span)>,
    arrays: HashMap<String, ArraySpec>,
    record_types: HashMap<u16, RecordDefinition>,
    record_names: HashMap<String, u16>,
    type_aliases: HashMap<String, TypeAlias>,
    constants: HashMap<String, Value>,
    post_condition: Option<Condition>,
    routines: HashMap<String, Routine>,
    libraries: HashSet<String>,
    local_declarations: Option<HashMap<String, (Type, Span)>>,
    local_arrays: Option<HashMap<String, ArraySpec>>,
    local_constants: Option<HashMap<String, Value>>,
    in_routine: bool,
}

pub fn parse(source: &str) -> Result<Program, Vec<Diagnostic>> {
    parse_with_library_loader(source, |_| Ok(None))
}

/// Parses a Jeem program and resolves nonstandard `أدرج` files with `loader`.
/// The loader receives a library name without its `.jeem` extension.
pub fn parse_with_library_loader<F>(source: &str, mut loader: F) -> Result<Program, Vec<Diagnostic>>
where
    F: FnMut(&str) -> Result<Option<String>, String>,
{
    let expanded = expand_source(source, &mut loader, &mut HashMap::new(), &mut Vec::new(), 0)
        .map_err(|error| vec![error])?;
    let lines = expanded
        .lines()
        .enumerate()
        .map(|(index, text)| SourceLine {
            number: index + 1,
            text: text.to_owned(),
        })
        .collect();
    let mut parser = Parser {
        lines,
        at: 0,
        declarations: HashMap::new(),
        arrays: HashMap::new(),
        record_types: HashMap::new(),
        record_names: HashMap::new(),
        type_aliases: HashMap::new(),
        constants: HashMap::new(),
        post_condition: None,
        routines: HashMap::new(),
        libraries: HashSet::new(),
        local_declarations: None,
        local_arrays: None,
        local_constants: None,
        in_routine: false,
    };
    let statements = parser.block(false).map_err(|error| vec![error])?;
    Ok(Program {
        declarations: parser.declarations,
        arrays: parser.arrays,
        record_types: parser.record_types,
        constants: parser.constants,
        routines: parser.routines,
        libraries: parser.libraries,
        statements,
    })
}

fn expand_source<F>(
    source: &str,
    loader: &mut F,
    substitutions: &mut HashMap<String, String>,
    include_stack: &mut Vec<String>,
    depth: usize,
) -> Result<String, Diagnostic>
where
    F: FnMut(&str) -> Result<Option<String>, String>,
{
    if depth > 64 {
        return Err(diagnostic(
            Span { line: 1, column: 1 },
            "library inclusion exceeds the nesting limit",
        ));
    }
    let uncommented = strip_multiline_comments(source)?;
    let uncommented = strip_line_comments(&uncommented);
    let mut output = String::new();
    for (index, raw_line) in uncommented.lines().enumerate() {
        let line_number = index + 1;
        let line = substitute_identifiers(raw_line, substitutions);
        let text = line.trim();
        if let Some(directive) = text.strip_prefix("بدل ") {
            let Some((name, replacement_source)) = directive.split_once(" بـ ") else {
                return Err(diagnostic(
                    Span {
                        line: line_number,
                        column: 1,
                    },
                    "replacement directive needs `بدل <name> بـ <quoted text>`",
                ));
            };
            let name = name.trim();
            let replacement = parse_text_constant(replacement_source.trim()).ok_or_else(|| {
                diagnostic(
                    Span {
                        line: line_number,
                        column: 1,
                    },
                    "replacement directive needs a valid quoted text constant",
                )
            })?;
            if !valid_macro_name(name) {
                return Err(diagnostic(
                    Span {
                        line: line_number,
                        column: 1,
                    },
                    "replacement name must be one identifier",
                ));
            }
            substitutions.insert(name.to_owned(), replacement);
            output.push('\n');
            continue;
        }
        if let Some(directive) = text.strip_prefix("البديل ") {
            let Some((name, replacement_source)) = directive.split_once('=') else {
                return Err(diagnostic(
                    Span {
                        line: line_number,
                        column: 1,
                    },
                    "synonym definition needs `البديل <name> = <quoted text>`",
                ));
            };
            let name = name.trim();
            let replacement = parse_text_constant(replacement_source.trim()).ok_or_else(|| {
                diagnostic(
                    Span {
                        line: line_number,
                        column: 1,
                    },
                    "synonym definition needs a valid quoted text constant",
                )
            })?;
            if !valid_macro_name(name) {
                return Err(diagnostic(
                    Span {
                        line: line_number,
                        column: 1,
                    },
                    "synonym name must be one identifier",
                ));
            }
            substitutions.insert(name.to_owned(), replacement);
            output.push('\n');
            continue;
        }
        if let Some(directive) = text.strip_prefix("أدرج ") {
            let name = parse_text_constant(directive.trim()).ok_or_else(|| {
                diagnostic(
                    Span {
                        line: line_number,
                        column: 1,
                    },
                    "library inclusion needs a quoted library name",
                )
            })?;
            output.push_str("أدرج \"");
            output.push_str(&name);
            output.push_str("\"\n");
            if let Some(definitions) = builtin_library_substitutions(&name) {
                substitutions.extend(definitions);
                continue;
            }
            if include_stack.iter().any(|included| included == &name) {
                return Err(diagnostic(
                    Span {
                        line: line_number,
                        column: 1,
                    },
                    format!("cyclic library inclusion for `{name}`"),
                ));
            }
            let included = loader(&name).map_err(|message| {
                diagnostic(
                    Span {
                        line: line_number,
                        column: 1,
                    },
                    format!("cannot load library `{name}`: {message}"),
                )
            })?;
            let Some(included) = included else {
                return Err(diagnostic(
                    Span {
                        line: line_number,
                        column: 1,
                    },
                    format!("library `{name}` was not found"),
                ));
            };
            include_stack.push(name.clone());
            output.push_str(&expand_source(
                &included,
                loader,
                substitutions,
                include_stack,
                depth + 1,
            )?);
            include_stack.pop();
            continue;
        }
        output.push_str(&line);
        output.push('\n');
    }
    Ok(output)
}

fn strip_line_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| {
            let mut quoted = false;
            let mut end = line.len();
            let mut characters = line.char_indices().peekable();
            while let Some((index, character)) = characters.next() {
                if character == '"' && !is_escaped_quote(line, index) {
                    quoted = !quoted;
                } else if !quoted
                    && character == '!'
                    && characters.peek().is_some_and(|(_, next)| *next == '!')
                {
                    end = index;
                    break;
                }
            }
            &line[..end]
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn substitute_identifiers(source: &str, substitutions: &HashMap<String, String>) -> String {
    let mut output = String::with_capacity(source.len());
    let mut token_start = None;
    let mut quoted = false;
    for (index, character) in source.char_indices() {
        if character == '"' && !is_escaped_quote(source, index) {
            if let Some(start) = token_start.take() {
                let token = &source[start..index];
                output.push_str(substitutions.get(token).map_or(token, String::as_str));
            }
            quoted = !quoted;
            output.push(character);
        } else if !quoted && is_identifier_character(character) {
            token_start.get_or_insert(index);
        } else {
            if let Some(start) = token_start.take() {
                let token = &source[start..index];
                output.push_str(substitutions.get(token).map_or(token, String::as_str));
            }
            output.push(character);
        }
    }
    if let Some(start) = token_start {
        let token = &source[start..];
        output.push_str(substitutions.get(token).map_or(token, String::as_str));
    }
    output
}

fn valid_macro_name(source: &str) -> bool {
    let mut characters = source.chars();
    characters.next().is_some_and(is_arabic_letter) && characters.all(is_identifier_character)
}

fn is_identifier_character(character: char) -> bool {
    is_arabic_letter(character)
        || is_arabic_mark(character)
        || character == '_'
        || character.is_ascii_digit()
        || is_arabic_digit(character)
}

fn parse_text_constant(source: &str) -> Option<String> {
    let inner = source.strip_prefix('"')?.strip_suffix('"')?;
    let mut output = String::with_capacity(inner.len());
    let mut characters = inner.char_indices().peekable();
    while let Some((index, character)) = characters.next() {
        if character == '"' {
            return None;
        }
        if character == '/' && characters.peek().is_some_and(|(_, next)| *next == '"') {
            characters.next();
            output.push('"');
        } else {
            output.push(character);
        }
        let _ = index;
    }
    Some(output)
}

fn is_escaped_quote(source: &str, index: usize) -> bool {
    index > 0 && source.as_bytes().get(index - 1) == Some(&b'/')
}

fn builtin_library_substitutions(name: &str) -> Option<HashMap<String, String>> {
    let definitions = match name {
        "رياضيات" | "حروف" | "ألوان" | "الأعداد العقدية" | "المترادفات" =>
        {
            let mut definitions = HashMap::new();
            if name == "ألوان" {
                for (name, replacement) in [
                    ("الأحمر", "(100،0،0)"),
                    ("الأخضر", "(0،100،0)"),
                    ("الأزرق", "(0،0،100)"),
                    ("الأصفر", "(100،100،0)"),
                    ("السياني", "(0،100،100)"),
                    ("البنفسجي", "(100،0،100)"),
                    ("السماوي", "(71،92،100)"),
                    ("الفستقي", "(65،100،65)"),
                    ("البرتقالي", "(100،45،16)"),
                    ("الأبيض", "(100،100،100)"),
                    ("الأسود", "(0،0،0)"),
                    ("الرمادي", "(60،60،60)"),
                    ("البني", "(51،31،0)"),
                ] {
                    definitions.insert(name.to_owned(), replacement.to_owned());
                }
            }
            if name == "المترادفات" {
                for (name, replacement) in [
                    ("المتغير", "المتحول"),
                    ("المتغيرين", "المتحول"),
                    ("المتغيرات", "المتحول"),
                    ("المتحولين", "المتحول"),
                    ("المتحولات", "المتحول"),
                    ("الإجرائية", "الخوارزمية"),
                    ("الدالة", "التابع"),
                    ("دع", "أجعل"),
                    ("كرر", "أكرر"),
                    ("مرات", "مرة"),
                    ("بداية", "{"),
                    ("نهاية", "}"),
                    ("البداية", "{"),
                    ("النهاية", "}"),
                    ("هو", ":"),
                    ("هي", ":"),
                    ("هما", ":"),
                    ("صحيحة", "صحيح"),
                    ("صحيحين", "صحيح"),
                    ("حقيقية", "حقيقي"),
                    ("نقيض", "عكس"),
                    ("النقطة", "نقطة"),
                    ("مستقيما", "مستقيم"),
                    ("خط", "مستقيم"),
                    ("جدول", "صفيفة"),
                ] {
                    definitions.insert(name.to_owned(), replacement.to_owned());
                }
            }
            Some(definitions)
        }
        _ => None,
    }?;
    Some(definitions)
}

fn strip_multiline_comments(source: &str) -> Result<String, Diagnostic> {
    let bytes = source.as_bytes();
    let mut output = bytes.to_vec();
    let mut at = 0;
    let mut line = 1;
    let mut column = 1;
    let mut quoted = false;
    let mut comment_start = None;
    while at < bytes.len() {
        if comment_start.is_none() && bytes[at] == b'"' && !is_escaped_quote(source, at) {
            quoted = !quoted;
        }
        if !quoted && comment_start.is_none() && bytes[at..].starts_with(b"*(") {
            comment_start = Some((line, column));
            output[at] = b' ';
            output[at + 1] = b' ';
            at += 2;
            column += 2;
            continue;
        }
        if comment_start.is_some() {
            if bytes[at..].starts_with(b")*") {
                output[at] = b' ';
                output[at + 1] = b' ';
                at += 2;
                column += 2;
                comment_start = None;
                continue;
            }
            if bytes[at] != b'\n' {
                output[at] = b' ';
            }
        }
        if bytes[at] == b'\n' {
            line += 1;
            column = 1;
            quoted = false;
        } else {
            column += 1;
        }
        at += 1;
    }
    if let Some((comment_line, comment_column)) = comment_start {
        return Err(Diagnostic {
            span: Span {
                line: comment_line,
                column: comment_column,
            },
            message: "unterminated `*( ... )*` comment".to_owned(),
        });
    }
    String::from_utf8(output).map_err(|_| Diagnostic {
        span: Span { line: 1, column: 1 },
        message: "source is not valid UTF-8".to_owned(),
    })
}

pub fn check(program: &Program) -> Result<(), Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    check_statements(
        &program.statements,
        &program.declarations,
        &program.constants,
        &program.routines,
        &program.record_types,
        &mut diagnostics,
    );
    for routine in program.routines.values() {
        let mut declarations = routine.visible_globals.clone();
        declarations.extend(routine.declarations.clone());
        let mut constants = routine.visible_constants.clone();
        constants.extend(routine.constants.clone());
        check_statements(
            &routine.statements,
            &declarations,
            &constants,
            &program.routines,
            &program.record_types,
            &mut diagnostics,
        );
        check_return_types(
            &routine.statements,
            routine.return_type,
            &declarations,
            &constants,
            &program.routines,
            &program.record_types,
            &mut diagnostics,
        );
    }
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(diagnostics)
    }
}

fn check_statements(
    statements: &[Stmt],
    declarations: &HashMap<String, (Type, Span)>,
    constants: &HashMap<String, Value>,
    routines: &HashMap<String, Routine>,
    record_types: &HashMap<u16, RecordDefinition>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for statement in statements {
        match statement {
            Stmt::Assign {
                target,
                value,
                span,
            } => {
                let expected = lvalue_type(
                    target,
                    declarations,
                    constants,
                    routines,
                    record_types,
                    diagnostics,
                );
                if matches!(target, LValue::Variable { name, .. } if constants.contains_key(name)) {
                    let name = match target {
                        LValue::Variable { name, .. } => name,
                        LValue::Index { .. }
                        | LValue::IndexedField { .. }
                        | LValue::Field { .. } => unreachable!(),
                    };
                    diagnostics.push(diagnostic(
                        *span,
                        format!("cannot assign to constant `{name}`"),
                    ));
                } else if let Some(expected) = expected {
                    if array_element_type(expected).is_some() || expected == Type::Array {
                        diagnostics
                            .push(diagnostic(*span, "whole-array assignment is not defined"));
                    }
                    if let Some(actual) = expression_type(
                        value,
                        declarations,
                        constants,
                        routines,
                        record_types,
                        diagnostics,
                    ) && !assignment_compatible(expected, actual)
                    {
                        diagnostics.push(diagnostic(
                            *span,
                            format!("cannot assign {actual:?} to {expected:?} target"),
                        ));
                    }
                } else {
                    check_expr(
                        value,
                        declarations,
                        constants,
                        routines,
                        record_types,
                        diagnostics,
                    );
                }
            }
            Stmt::Read { targets, span } => {
                for target in targets {
                    match lvalue_type(target, declarations, constants, routines, record_types, diagnostics) {
                        Some(Type::Pointer) => diagnostics.push(diagnostic(
                            *span,
                            "pointer input and file-reference syntax are not defined by the paper",
                        )),
                        Some(Type::File) => diagnostics.push(diagnostic(
                            *span,
                            "file input commands are mentioned, but their syntax and behavior are not specified",
                        )),
                        Some(Type::Logical) => diagnostics.push(diagnostic(
                            *span,
                            "the source defines input for numeric and text values, not logical values",
                        )),
                        Some(ty) if array_element_type(ty).is_some() => {
                            if ty != Type::CharacterArray {
                                diagnostics.push(diagnostic(
                                    *span,
                                    "only character arrays have a documented text-input form",
                                ));
                            }
                        }
                        Some(_) => {}
                        None => {}
                    }
                }
            }
            Stmt::Write { parts, .. } => {
                for part in parts {
                    if let OutputPart::Expression(expression) = part {
                        if let Some(ty) = expression_type(
                            expression,
                            declarations,
                            constants,
                            routines,
                            record_types,
                            diagnostics,
                        ) && (ty == Type::Logical
                            || array_element_type(ty).is_some()
                            || matches!(
                                ty,
                                Type::Array | Type::Record(_) | Type::File | Type::Pointer
                            ))
                        {
                            diagnostics.push(diagnostic(
                                expression_span(expression),
                                "this value type cannot be written directly in the source language",
                            ));
                        }
                    }
                }
            }
            Stmt::While {
                condition, body, ..
            } => {
                check_condition(
                    condition,
                    declarations,
                    constants,
                    routines,
                    record_types,
                    diagnostics,
                );
                check_statements(
                    body,
                    declarations,
                    constants,
                    routines,
                    record_types,
                    diagnostics,
                );
            }
            Stmt::RepeatUntil {
                condition, body, ..
            } => {
                check_statements(
                    body,
                    declarations,
                    constants,
                    routines,
                    record_types,
                    diagnostics,
                );
                check_condition(
                    condition,
                    declarations,
                    constants,
                    routines,
                    record_types,
                    diagnostics,
                );
            }
            Stmt::Counted { count, body, span } => {
                if let Some(actual) = expression_type(
                    count,
                    declarations,
                    constants,
                    routines,
                    record_types,
                    diagnostics,
                ) && !matches!(actual, Type::Integer | Type::SmallInteger)
                {
                    diagnostics.push(diagnostic(*span, "a counted loop needs an integer count"));
                }
                check_statements(
                    body,
                    declarations,
                    constants,
                    routines,
                    record_types,
                    diagnostics,
                );
            }
            Stmt::For {
                name,
                start,
                end,
                step,
                body,
                span,
            } => {
                match declarations.get(name) {
                    Some((Type::Integer | Type::SmallInteger, declared_at))
                        if declared_at.line <= span.line => {}
                    Some((Type::Integer | Type::SmallInteger, _)) => diagnostics.push(diagnostic(
                        *span,
                        format!("variable {name} is not declared yet"),
                    )),
                    Some(_) => diagnostics.push(diagnostic(
                        *span,
                        "a range-loop variable must have an integer type",
                    )),
                    None => diagnostics.push(diagnostic(
                        *span,
                        format!("undeclared range-loop variable `{name}`"),
                    )),
                }
                for expression in [Some(start), Some(end), step.as_ref()]
                    .into_iter()
                    .flatten()
                {
                    if let Some(actual) = expression_type(
                        expression,
                        declarations,
                        constants,
                        routines,
                        record_types,
                        diagnostics,
                    ) && !matches!(actual, Type::Integer | Type::SmallInteger)
                    {
                        diagnostics.push(diagnostic(
                            *span,
                            "range-loop bounds and step must be integers",
                        ));
                    }
                }
                check_statements(
                    body,
                    declarations,
                    constants,
                    routines,
                    record_types,
                    diagnostics,
                );
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                check_condition(
                    condition,
                    declarations,
                    constants,
                    routines,
                    record_types,
                    diagnostics,
                );
                check_statements(
                    then_body,
                    declarations,
                    constants,
                    routines,
                    record_types,
                    diagnostics,
                );
                check_statements(
                    else_body,
                    declarations,
                    constants,
                    routines,
                    record_types,
                    diagnostics,
                );
            }
            Stmt::Draw { geometry, span } => {
                for expression in drawing_numeric_expressions(geometry) {
                    if let Some(actual) = expression_type(
                        expression,
                        declarations,
                        constants,
                        routines,
                        record_types,
                        diagnostics,
                    ) && !matches!(actual, Type::Integer | Type::SmallInteger | Type::Real)
                    {
                        diagnostics.push(diagnostic(
                            *span,
                            "drawing coordinates, sizes, and color channels must be numeric",
                        ));
                    }
                }
            }
            Stmt::OpenFile {
                name, path, span, ..
            } => {
                match declarations.get(name) {
                    Some((Type::File, declared_at)) if declared_at.line <= span.line => {}
                    Some((Type::File, _)) => diagnostics.push(diagnostic(
                        *span,
                        format!("file variable {name} is not declared yet"),
                    )),
                    Some(_) => diagnostics.push(diagnostic(
                        *span,
                        format!("`{name}` is not declared as a file"),
                    )),
                    None => diagnostics.push(diagnostic(
                        *span,
                        format!("undeclared file variable `{name}`"),
                    )),
                }
                if let Some(actual) = expression_type(
                    path,
                    declarations,
                    constants,
                    routines,
                    record_types,
                    diagnostics,
                ) && actual != Type::Text
                {
                    diagnostics.push(diagnostic(
                        expression_span(path),
                        "file path must be a quoted text value",
                    ));
                }
            }
            Stmt::ReadFile {
                name,
                targets,
                span,
            } => {
                check_file_variable(name, *span, declarations, diagnostics);
                for target in targets {
                    match lvalue_type(
                        target,
                        declarations,
                        constants,
                        routines,
                        record_types,
                        diagnostics,
                    ) {
                        Some(Type::Pointer | Type::File | Type::Record(_) | Type::Array) => {
                            diagnostics.push(diagnostic(
                                *span,
                                "file input requires a scalar target or a character array",
                            ));
                        }
                        Some(ty)
                            if array_element_type(ty).is_some() && ty != Type::CharacterArray =>
                        {
                            diagnostics.push(diagnostic(
                                *span,
                                "file text input requires a character array",
                            ));
                        }
                        Some(_) | None => {}
                    }
                }
            }
            Stmt::WriteFile { name, parts, span } => {
                check_file_variable(name, *span, declarations, diagnostics);
                for part in parts {
                    if let OutputPart::Expression(expression) = part {
                        if let Some(ty) = expression_type(
                            expression,
                            declarations,
                            constants,
                            routines,
                            record_types,
                            diagnostics,
                        ) && (array_element_type(ty).is_some()
                            || matches!(
                                ty,
                                Type::Array | Type::Record(_) | Type::File | Type::Pointer
                            ))
                        {
                            diagnostics.push(diagnostic(
                                expression_span(expression),
                                "this value type cannot be written directly to a file",
                            ));
                        }
                    }
                }
            }
            Stmt::CloseFile { name, span } => {
                check_file_variable(name, *span, declarations, diagnostics);
            }
            Stmt::Call {
                name,
                arguments,
                span,
            } => {
                check_call_arguments(
                    name,
                    arguments,
                    *span,
                    declarations,
                    constants,
                    routines,
                    record_types,
                    diagnostics,
                );
            }
            Stmt::Return { value, .. } => {
                if let Some(value) = value {
                    check_expr(
                        value,
                        declarations,
                        constants,
                        routines,
                        record_types,
                        diagnostics,
                    );
                }
            }
            Stmt::Terminate { .. } => {}
        }
    }
}

fn add_point_expressions<'a>(expressions: &mut Vec<&'a Expr>, point: &'a (Expr, Expr)) {
    expressions.push(&point.0);
    expressions.push(&point.1);
}

fn drawing_numeric_expressions(geometry: &DrawGeometry) -> Vec<&Expr> {
    let mut expressions = Vec::new();
    match geometry {
        DrawGeometry::Point(x, y) => {
            expressions.extend([x, y]);
        }
        DrawGeometry::Line(points) => {
            for point in points {
                add_point_expressions(&mut expressions, point);
            }
        }
        DrawGeometry::Triangle(points) => {
            for point in points {
                add_point_expressions(&mut expressions, point);
            }
        }
        DrawGeometry::Circle { center, diameter }
        | DrawGeometry::FillCircle { center, diameter } => {
            add_point_expressions(&mut expressions, center);
            expressions.push(diameter);
        }
        DrawGeometry::Ellipse { center, diameters }
        | DrawGeometry::FillEllipse { center, diameters } => {
            add_point_expressions(&mut expressions, center);
            add_point_expressions(&mut expressions, diameters);
        }
        DrawGeometry::Rectangle { from, to } | DrawGeometry::FillRectangle { from, to } => {
            add_point_expressions(&mut expressions, from);
            add_point_expressions(&mut expressions, to);
        }
        DrawGeometry::StrokeColor(channels) | DrawGeometry::FillColor(channels) => {
            expressions.extend(channels.iter());
        }
        DrawGeometry::FillTriangle(points) => {
            for point in points {
                add_point_expressions(&mut expressions, point);
            }
        }
        DrawGeometry::FillCanvas(Some(channels)) => expressions.extend(channels.iter()),
        DrawGeometry::FillCanvas(None) => {}
        DrawGeometry::Label { position, .. } => {
            if let Some(position) = position {
                add_point_expressions(&mut expressions, position);
            }
        }
    }
    expressions
}

fn check_file_variable(
    name: &str,
    at: Span,
    declarations: &HashMap<String, (Type, Span)>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match declarations.get(name) {
        Some((Type::File, declared_at)) if declared_at.line <= at.line => {}
        Some((Type::File, _)) => diagnostics.push(diagnostic(
            at,
            format!("file variable {name} is not declared yet"),
        )),
        Some(_) => diagnostics.push(diagnostic(at, format!("`{name}` is not a file"))),
        None => diagnostics.push(diagnostic(at, format!("undeclared file variable `{name}`"))),
    }
}

fn check_condition(
    condition: &Condition,
    declarations: &HashMap<String, (Type, Span)>,
    constants: &HashMap<String, Value>,
    routines: &HashMap<String, Routine>,
    record_types: &HashMap<u16, RecordDefinition>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Condition::Expression(expression) = condition;
    if let Some(actual) = expression_type(
        expression,
        declarations,
        constants,
        routines,
        record_types,
        diagnostics,
    ) && actual != Type::Logical
    {
        diagnostics.push(diagnostic(
            expression_span(expression),
            "condition must be logical",
        ));
    }
}

fn assignment_compatible(expected: Type, actual: Type) -> bool {
    expected == actual
        || matches!(expected, Type::Real) && matches!(actual, Type::Integer | Type::SmallInteger)
        || expected == Type::SmallInteger && actual == Type::Integer
        || matches!(expected, Type::Integer) && actual == Type::SmallInteger
}

fn parameter_compatible(expected: Type, actual: Type) -> bool {
    if expected == Type::CharacterArray && actual == Type::Text {
        return true;
    }
    if expected == Type::Real {
        return matches!(
            actual,
            Type::Integer | Type::SmallInteger | Type::Real | Type::Character
        );
    }
    if matches!(
        expected,
        Type::Integer | Type::SmallInteger | Type::Character
    ) {
        return matches!(actual, Type::Integer | Type::SmallInteger | Type::Character);
    }
    assignment_compatible(expected, actual)
}

fn array_type(element_type: Type) -> Option<Type> {
    match element_type {
        Type::Integer => Some(Type::IntegerArray),
        Type::SmallInteger => Some(Type::SmallIntegerArray),
        Type::Real => Some(Type::RealArray),
        Type::Logical => Some(Type::LogicalArray),
        Type::Character => Some(Type::CharacterArray),
        Type::Record(type_id) => Some(Type::RecordArray(type_id)),
        _ => None,
    }
}

fn array_element_type(array_type: Type) -> Option<Type> {
    match array_type {
        Type::IntegerArray => Some(Type::Integer),
        Type::SmallIntegerArray => Some(Type::SmallInteger),
        Type::RealArray => Some(Type::Real),
        Type::LogicalArray => Some(Type::Logical),
        Type::CharacterArray => Some(Type::Character),
        Type::RecordArray(type_id) => Some(Type::Record(type_id)),
        _ => None,
    }
}

fn resolve_record_field_type(
    mut current: Type,
    fields: &[String],
    record_types: &HashMap<u16, RecordDefinition>,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    for (index, field_name) in fields.iter().enumerate() {
        let Type::Record(type_id) = current else {
            diagnostics.push(diagnostic(
                span,
                format!("`{field_name}` is selected from a non-record value"),
            ));
            return None;
        };
        let Some(record) = record_types.get(&type_id) else {
            diagnostics.push(diagnostic(
                span,
                format!("record type {type_id} is unavailable"),
            ));
            return None;
        };
        let Some(field) = record.fields.get(field_name) else {
            diagnostics.push(diagnostic(
                span,
                format!("record `{}` has no field `{field_name}`", record.name),
            ));
            return None;
        };
        if index + 1 < fields.len() && field.array_spec.is_some() {
            diagnostics.push(diagnostic(
                span,
                "an array record field must be indexed before selecting a nested field",
            ));
            return None;
        }
        current = field
            .array_spec
            .as_ref()
            .and_then(|spec| array_type(spec.element_type))
            .unwrap_or(field.ty);
    }
    Some(current)
}

fn check_return_types(
    statements: &[Stmt],
    return_type: Option<Type>,
    declarations: &HashMap<String, (Type, Span)>,
    constants: &HashMap<String, Value>,
    routines: &HashMap<String, Routine>,
    record_types: &HashMap<u16, RecordDefinition>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for statement in statements {
        match statement {
            Stmt::Return { value, span } => match (return_type, value) {
                (None, Some(_)) => {
                    diagnostics.push(diagnostic(*span, "algorithm cannot return a value"))
                }
                (Some(_), None) => {
                    diagnostics.push(diagnostic(*span, "function return needs a value"))
                }
                (Some(expected), Some(value)) => {
                    if let Some(actual) = expression_type(
                        value,
                        declarations,
                        constants,
                        routines,
                        record_types,
                        diagnostics,
                    ) && !assignment_compatible(expected, actual)
                    {
                        diagnostics.push(diagnostic(
                            *span,
                            "function return type does not match its declaration",
                        ));
                    }
                }
                (None, None) => {}
            },
            Stmt::If {
                then_body,
                else_body,
                ..
            } => {
                check_return_types(
                    then_body,
                    return_type,
                    declarations,
                    constants,
                    routines,
                    record_types,
                    diagnostics,
                );
                check_return_types(
                    else_body,
                    return_type,
                    declarations,
                    constants,
                    routines,
                    record_types,
                    diagnostics,
                );
            }
            Stmt::While { body, .. }
            | Stmt::RepeatUntil { body, .. }
            | Stmt::Counted { body, .. }
            | Stmt::For { body, .. } => {
                check_return_types(
                    body,
                    return_type,
                    declarations,
                    constants,
                    routines,
                    record_types,
                    diagnostics,
                );
            }
            _ => {}
        }
    }
}

fn value_type(value: &Value) -> Type {
    match value {
        Value::Integer(_) => Type::Integer,
        Value::Real(_) => Type::Real,
        Value::Logical(_) => Type::Logical,
        Value::Character(_) => Type::Character,
        Value::Text(_) => Type::Text,
        Value::Array(_) => Type::Array,
        Value::Record { type_id, .. } => Type::Record(*type_id),
        Value::File(_) => Type::File,
        Value::Pointer(_) => Type::Pointer,
    }
}

fn expression_type(
    expression: &Expr,
    declarations: &HashMap<String, (Type, Span)>,
    constants: &HashMap<String, Value>,
    routines: &HashMap<String, Routine>,
    record_types: &HashMap<u16, RecordDefinition>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    match expression {
        Expr::Integer(_) => Some(Type::Integer),
        Expr::Real(_) => Some(Type::Real),
        Expr::Logical(_) => Some(Type::Logical),
        Expr::Text(_) => Some(Type::Text),
        Expr::Character(_) => Some(Type::Character),
        Expr::Variable(name, span) => declarations
            .get(name)
            .filter(|(_, declared_at)| declared_at.line <= span.line)
            .map(|(ty, _)| *ty)
            .or_else(|| constants.get(name).map(value_type))
            .or_else(|| {
                diagnostics.push(diagnostic(*span, format!("undeclared name `{name}`")));
                None
            }),
        Expr::Index(name, indexes, span) => {
            let Some((array_type, declared_at)) = declarations.get(name) else {
                diagnostics.push(diagnostic(*span, format!("undeclared array `{name}`")));
                return None;
            };
            if declared_at.line > span.line {
                diagnostics.push(diagnostic(
                    *span,
                    format!("array {name} is not declared yet"),
                ));
            }
            let Some(element_type) = array_element_type(*array_type) else {
                diagnostics.push(diagnostic(*span, format!("`{name}` is not an array")));
                return None;
            };
            for index in indexes {
                if let Some(actual) = expression_type(
                    index,
                    declarations,
                    constants,
                    routines,
                    record_types,
                    diagnostics,
                ) && !matches!(actual, Type::Integer | Type::SmallInteger)
                {
                    diagnostics.push(diagnostic(
                        expression_span(index),
                        "array index must be an integer",
                    ));
                }
            }
            Some(element_type)
        }
        Expr::IndexedField(name, indexes, fields, span) => {
            let Some((Type::RecordArray(type_id), declared_at)) = declarations.get(name) else {
                diagnostics.push(diagnostic(
                    *span,
                    format!("`{name}` is not an array of records"),
                ));
                return None;
            };
            if declared_at.line > span.line {
                diagnostics.push(diagnostic(
                    *span,
                    format!("array {name} is not declared yet"),
                ));
            }
            for index in indexes {
                if let Some(actual) = expression_type(
                    index,
                    declarations,
                    constants,
                    routines,
                    record_types,
                    diagnostics,
                ) && !matches!(actual, Type::Integer | Type::SmallInteger)
                {
                    diagnostics.push(diagnostic(
                        expression_span(index),
                        "array index must be an integer",
                    ));
                }
            }
            resolve_record_field_type(
                Type::Record(*type_id),
                fields,
                record_types,
                *span,
                diagnostics,
            )
        }
        Expr::Field(name, fields, span) => {
            let root_type = declarations
                .get(name)
                .filter(|(_, declared_at)| declared_at.line <= span.line)
                .map(|(ty, _)| *ty)
                .or_else(|| constants.get(name).map(value_type));
            let Some(root_type) = root_type else {
                diagnostics.push(diagnostic(*span, format!("undeclared name `{name}`")));
                return None;
            };
            resolve_record_field_type(root_type, fields, record_types, *span, diagnostics)
        }
        Expr::Call(name, arguments, span) => {
            let return_type = check_call_arguments(
                name,
                arguments,
                *span,
                declarations,
                constants,
                routines,
                record_types,
                diagnostics,
            );
            if routines
                .get(name)
                .is_some_and(|routine| routine.return_type.is_none())
                || builtin_is_procedure(name)
            {
                diagnostics.push(diagnostic(
                    *span,
                    "an algorithm without a return value cannot be used in an expression",
                ));
                None
            } else {
                return_type
            }
        }
        Expr::Unary(operand, operator, span) => {
            let Some(operand_type) = expression_type(
                operand,
                declarations,
                constants,
                routines,
                record_types,
                diagnostics,
            ) else {
                return None;
            };
            match operator {
                Unary::Negate | Unary::Absolute
                    if matches!(
                        operand_type,
                        Type::Integer | Type::SmallInteger | Type::Real
                    ) =>
                {
                    Some(operand_type)
                }
                Unary::Not if operand_type == Type::Logical => Some(Type::Logical),
                Unary::Negate | Unary::Absolute => {
                    diagnostics.push(diagnostic(*span, "numeric unary operator needs a number"));
                    None
                }
                Unary::Not => {
                    diagnostics.push(diagnostic(*span, "`نفي` needs a logical value"));
                    None
                }
            }
        }
        Expr::Binary(left, operator, right, span) => {
            let left_type = expression_type(
                left,
                declarations,
                constants,
                routines,
                record_types,
                diagnostics,
            )?;
            let right_type = expression_type(
                right,
                declarations,
                constants,
                routines,
                record_types,
                diagnostics,
            )?;
            match operator {
                Arithmetic::Greater
                | Arithmetic::GreaterOrEqual
                | Arithmetic::Less
                | Arithmetic::LessOrEqual
                    if matches!(left_type, Type::Integer | Type::SmallInteger | Type::Real)
                        && matches!(
                            right_type,
                            Type::Integer | Type::SmallInteger | Type::Real
                        ) =>
                {
                    Some(Type::Logical)
                }
                Arithmetic::Greater
                | Arithmetic::GreaterOrEqual
                | Arithmetic::Less
                | Arithmetic::LessOrEqual => {
                    diagnostics.push(diagnostic(*span, "comparison needs two numeric values"));
                    None
                }
                Arithmetic::Equal | Arithmetic::NotEqual
                    if left_type == right_type
                        || matches!(left_type, Type::Integer | Type::SmallInteger | Type::Real)
                            && matches!(
                                right_type,
                                Type::Integer | Type::SmallInteger | Type::Real
                            ) =>
                {
                    Some(Type::Logical)
                }
                Arithmetic::Equal | Arithmetic::NotEqual => {
                    diagnostics.push(diagnostic(*span, "equality needs values of the same type"));
                    None
                }
                Arithmetic::And | Arithmetic::Or
                    if left_type == Type::Logical && right_type == Type::Logical =>
                {
                    Some(Type::Logical)
                }
                Arithmetic::And | Arithmetic::Or => {
                    diagnostics.push(diagnostic(*span, "logical operators need logical values"));
                    None
                }
                Arithmetic::Add
                | Arithmetic::Subtract
                | Arithmetic::Multiply
                | Arithmetic::Power
                    if matches!(left_type, Type::Integer | Type::SmallInteger | Type::Real)
                        && matches!(
                            right_type,
                            Type::Integer | Type::SmallInteger | Type::Real
                        ) =>
                {
                    if *operator == Arithmetic::Power
                        && left_type == Type::Integer
                        && right_type == Type::Integer
                    {
                        Some(Type::Integer)
                    } else if *operator == Arithmetic::Divide
                        || left_type == Type::Real
                        || right_type == Type::Real
                    {
                        Some(Type::Real)
                    } else if left_type == Type::SmallInteger && right_type == Type::SmallInteger {
                        Some(Type::SmallInteger)
                    } else {
                        Some(Type::Integer)
                    }
                }
                Arithmetic::IntegerDivide
                    if matches!(left_type, Type::Integer | Type::SmallInteger)
                        && matches!(right_type, Type::Integer | Type::SmallInteger) =>
                {
                    Some(Type::Integer)
                }
                Arithmetic::IntegerDivide => {
                    diagnostics.push(diagnostic(
                        *span,
                        "integer quotient operator needs integer values",
                    ));
                    None
                }
                Arithmetic::Divide
                    if matches!(left_type, Type::Integer | Type::SmallInteger | Type::Real)
                        && matches!(
                            right_type,
                            Type::Integer | Type::SmallInteger | Type::Real
                        ) =>
                {
                    Some(Type::Real)
                }
                Arithmetic::Divide => {
                    diagnostics.push(diagnostic(*span, "division needs numeric values"));
                    None
                }
                _ => {
                    diagnostics.push(diagnostic(
                        *span,
                        "arithmetic operator needs numeric values",
                    ));
                    None
                }
            }
        }
    }
}

fn expression_span(expression: &Expr) -> Span {
    match expression {
        Expr::Variable(_, span)
        | Expr::Index(_, _, span)
        | Expr::IndexedField(_, _, _, span)
        | Expr::Field(_, _, span)
        | Expr::Call(_, _, span)
        | Expr::Unary(_, _, span)
        | Expr::Binary(_, _, _, span) => *span,
        Expr::Integer(_)
        | Expr::Real(_)
        | Expr::Logical(_)
        | Expr::Text(_)
        | Expr::Character(_) => Span::default(),
    }
}

fn check_expr(
    expression: &Expr,
    declarations: &HashMap<String, (Type, Span)>,
    constants: &HashMap<String, Value>,
    routines: &HashMap<String, Routine>,
    record_types: &HashMap<u16, RecordDefinition>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let _ = expression_type(
        expression,
        declarations,
        constants,
        routines,
        record_types,
        diagnostics,
    );
}

fn lvalue_type(
    target: &LValue,
    declarations: &HashMap<String, (Type, Span)>,
    constants: &HashMap<String, Value>,
    routines: &HashMap<String, Routine>,
    record_types: &HashMap<u16, RecordDefinition>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    match target {
        LValue::Variable { name, span } => {
            if constants.contains_key(name) {
                diagnostics.push(diagnostic(
                    *span,
                    format!("cannot write to constant `{name}`"),
                ));
                return None;
            }
            match declarations.get(name) {
                Some((ty, declared_at)) if declared_at.line <= span.line => Some(*ty),
                Some(_) => {
                    diagnostics.push(diagnostic(
                        *span,
                        format!("variable {name} is not declared yet"),
                    ));
                    None
                }
                None => {
                    diagnostics.push(diagnostic(*span, format!("undeclared variable `{name}`")));
                    None
                }
            }
        }
        LValue::Index {
            name,
            indexes,
            span,
        } => {
            let Some((ty, declared_at)) = declarations.get(name) else {
                diagnostics.push(diagnostic(*span, format!("undeclared array `{name}`")));
                for index in indexes {
                    check_expr(
                        index,
                        declarations,
                        constants,
                        routines,
                        record_types,
                        diagnostics,
                    );
                }
                return None;
            };
            if declared_at.line > span.line {
                diagnostics.push(diagnostic(
                    *span,
                    format!("array {name} is not declared yet"),
                ));
            }
            let Some(element_type) = array_element_type(*ty) else {
                diagnostics.push(diagnostic(*span, format!("`{name}` is not an array")));
                return None;
            };
            for index in indexes {
                if let Some(actual) = expression_type(
                    index,
                    declarations,
                    constants,
                    routines,
                    record_types,
                    diagnostics,
                ) && !matches!(actual, Type::Integer | Type::SmallInteger)
                {
                    diagnostics.push(diagnostic(
                        expression_span(index),
                        "array index must be an integer",
                    ));
                }
            }
            Some(element_type)
        }
        LValue::IndexedField {
            name,
            indexes,
            fields,
            span,
        } => {
            if constants.contains_key(name) {
                diagnostics.push(diagnostic(
                    *span,
                    format!("cannot write to constant `{name}`"),
                ));
                return None;
            }
            let Some((Type::RecordArray(type_id), declared_at)) = declarations.get(name) else {
                diagnostics.push(diagnostic(
                    *span,
                    format!("`{name}` is not an array of records"),
                ));
                return None;
            };
            if declared_at.line > span.line {
                diagnostics.push(diagnostic(
                    *span,
                    format!("array {name} is not declared yet"),
                ));
            }
            for index in indexes {
                if let Some(actual) = expression_type(
                    index,
                    declarations,
                    constants,
                    routines,
                    record_types,
                    diagnostics,
                ) && !matches!(actual, Type::Integer | Type::SmallInteger)
                {
                    diagnostics.push(diagnostic(
                        expression_span(index),
                        "array index must be an integer",
                    ));
                }
            }
            resolve_record_field_type(
                Type::Record(*type_id),
                fields,
                record_types,
                *span,
                diagnostics,
            )
        }
        LValue::Field { name, fields, span } => {
            if constants.contains_key(name) {
                diagnostics.push(diagnostic(
                    *span,
                    format!("cannot write to constant `{name}`"),
                ));
                return None;
            }
            let Some((root_type, declared_at)) = declarations.get(name) else {
                diagnostics.push(diagnostic(*span, format!("undeclared record `{name}`")));
                return None;
            };
            if declared_at.line > span.line {
                diagnostics.push(diagnostic(
                    *span,
                    format!("record {name} is not declared yet"),
                ));
            }
            resolve_record_field_type(*root_type, fields, record_types, *span, diagnostics)
        }
    }
}

fn check_call_arguments(
    name: &str,
    arguments: &[Expr],
    call_span: Span,
    declarations: &HashMap<String, (Type, Span)>,
    constants: &HashMap<String, Value>,
    routines: &HashMap<String, Routine>,
    record_types: &HashMap<u16, RecordDefinition>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    if let Some(routine) = routines.get(name) {
        if call_span.line < routine.span.line {
            diagnostics.push(diagnostic(
                call_span,
                format!("routine {name} is not defined yet"),
            ));
        }
        if arguments.len() != routine.parameters.len() {
            diagnostics.push(diagnostic(
                call_span,
                format!(
                    "`{name}` expects {} arguments but received {}",
                    routine.parameters.len(),
                    arguments.len()
                ),
            ));
        }
        for (argument, parameter) in arguments.iter().zip(&routine.parameters) {
            if let Some(actual) = expression_type(
                argument,
                declarations,
                constants,
                routines,
                record_types,
                diagnostics,
            ) && !parameter_compatible(parameter.ty, actual)
            {
                diagnostics.push(diagnostic(
                    expression_span(argument),
                    format!(
                        "argument type does not match parameter `{}`",
                        parameter.name
                    ),
                ));
            }
        }
        routine.return_type
    } else if is_builtin_name(name) {
        check_builtin_call(
            name,
            arguments,
            call_span,
            declarations,
            constants,
            routines,
            record_types,
            diagnostics,
        )
    } else {
        diagnostics.push(diagnostic(
            call_span,
            format!("unknown routine or built-in `{name}`"),
        ));
        None
    }
}

fn builtin_is_procedure(name: &str) -> bool {
    matches!(
        name,
        "أنسخ_كلمة"
            | "نسخ_كلمة"
            | "أنتقي_اللون"
            | "أجعل_عقدي"
            | "أجمع_عقدي"
            | "أضرب_عقدي"
            | "مرافق_عقدي"
            | "أرسم_عقدي"
            | "أقرأ_عقدي"
            | "أكتب_عقدي"
    )
}

fn check_builtin_call(
    name: &str,
    arguments: &[Expr],
    call_span: Span,
    declarations: &HashMap<String, (Type, Span)>,
    constants: &HashMap<String, Value>,
    routines: &HashMap<String, Routine>,
    record_types: &HashMap<u16, RecordDefinition>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    let expected_types: &[&[Type]] = match name {
        "حدس" | "حدع" => &[],
        "جذر" | "صحيح" | "جب" | "تجب" | "لغ" | "ظل" | "تظل" => {
            &[&[Type::Integer, Type::SmallInteger, Type::Real]]
        }
        "باقي" | "تدوير" => &[
            &[Type::Integer, Type::SmallInteger],
            &[Type::Integer, Type::SmallInteger],
        ],
        "أبجدي" => &[&[Type::Character]],
        "عكس" => &[&[Type::Logical]],
        "طول_الكلمة" => &[&[Type::CharacterArray]],
        "أنسخ_كلمة" | "نسخ_كلمة" => {
            &[&[Type::CharacterArray], &[Type::CharacterArray]]
        }
        "أقارن_كلمتين" => &[&[Type::CharacterArray], &[Type::CharacterArray]],
        "أقرأ_لوحة_ألوان" => &[&[Type::RecordArray(0)], &[Type::CharacterArray]],
        "أنتقي_اللون" => &[&[Type::Integer, Type::SmallInteger]],
        "أجعل_عقدي" => &[
            &[Type::Record(0)],
            &[Type::Integer, Type::SmallInteger, Type::Real],
            &[Type::Integer, Type::SmallInteger, Type::Real],
        ],
        "أجمع_عقدي" | "أضرب_عقدي" => {
            &[&[Type::Record(0)], &[Type::Record(0)], &[Type::Record(0)]]
        }
        "مرافق_عقدي" => &[&[Type::Record(0)], &[Type::Record(0)]],
        "طويلة_عقدي" | "أرسم_عقدي" | "أقرأ_عقدي" | "أكتب_عقدي" => {
            &[&[Type::Record(0)]]
        }
        _ => {
            diagnostics.push(diagnostic(
                call_span,
                format!(
                    "predefined name `{name}` is listed by the source, but its callable contract is unavailable"
                ),
            ));
            return None;
        }
    };
    if arguments.len() != expected_types.len() {
        diagnostics.push(diagnostic(
            call_span,
            format!(
                "built-in `{name}` expects {} arguments but received {}",
                expected_types.len(),
                arguments.len()
            ),
        ));
    }
    let mut argument_types = Vec::with_capacity(arguments.len());
    for argument in arguments {
        argument_types.push(expression_type(
            argument,
            declarations,
            constants,
            routines,
            record_types,
            diagnostics,
        ));
    }
    for (index, (actual, accepted)) in argument_types.iter().zip(expected_types).enumerate() {
        let Some(actual) = actual else {
            continue;
        };
        let matches = accepted.iter().any(|expected| match (expected, actual) {
            (Type::Record(0), Type::Record(_)) => true,
            (Type::RecordArray(0), Type::RecordArray(_)) => true,
            (expected, actual) => expected == actual,
        });
        if !matches {
            diagnostics.push(diagnostic(
                expression_span(&arguments[index]),
                format!("argument type is invalid for built-in `{name}`"),
            ));
        }
    }
    match name {
        "حدس"
        | "حدع"
        | "باقي"
        | "تدوير"
        | "صحيح"
        | "طول_الكلمة"
        | "أقارن_كلمتين"
        | "أقرأ_لوحة_ألوان" => Some(Type::Integer),
        "أبجدي" | "عكس" => Some(Type::Logical),
        "جذر" | "جب" | "تجب" | "لغ" | "ظل" | "تظل" | "طويلة_عقدي" => {
            Some(Type::Real)
        }
        _ => None,
    }
}

impl Parser {
    fn block(&mut self, nested: bool) -> Result<Vec<Stmt>, Diagnostic> {
        let mut statements = Vec::new();
        while self.at < self.lines.len() {
            let line = self.lines[self.at].clone();
            let text = line.text.trim();
            if text.is_empty() || text.starts_with("!!") {
                self.at += 1;
                continue;
            }
            if text == "}" {
                if !nested {
                    return Err(diagnostic(span(&line, 1), "unexpected `}`"));
                }
                self.at += 1;
                return Ok(statements);
            }
            if nested
                && let Some(condition_source) = text.strip_prefix('}').map(str::trim_start)
                && let Some(condition_source) = condition_source.strip_prefix("حتى")
            {
                self.post_condition = Some(parse_condition(&line, condition_source.trim())?);
                self.at += 1;
                return Ok(statements);
            }
            if text == "{" {
                return Err(diagnostic(
                    span(&line, 1),
                    "unexpected `{`; expected a statement header",
                ));
            }
            if let Some(header) = text.strip_prefix("الخوارزمية ") {
                self.parse_routine(&line, header, false, nested)?;
                continue;
            }
            if let Some(header) = text.strip_prefix("التابع ") {
                self.parse_routine(&line, header, true, nested)?;
                continue;
            }
            if let Some(header) = text.strip_prefix("النوع ") {
                self.parse_type_declaration(&line, header, nested)?;
                continue;
            }
            if let Some(declaration) = text.strip_prefix("المتحول ") {
                self.declare(&line, declaration)?;
                self.at += 1;
                continue;
            }
            if let Some(constant) = text.strip_prefix("الثابت ") {
                self.declare_constant(&line, constant)?;
                self.at += 1;
                continue;
            }
            if let Some(rest) = text.strip_prefix("أجعل ") {
                statements.push(parse_assignment(&line, rest)?);
                self.at += 1;
                continue;
            }
            if let Some((name, _)) = text.split_once('=')
                && parse_lvalue(&line, name.trim()).is_ok()
            {
                statements.push(parse_assignment(&line, text)?);
                self.at += 1;
                continue;
            }
            if text == "أنتهي" {
                statements.push(Stmt::Terminate {
                    span: span(&line, 1),
                });
                self.at += 1;
                continue;
            }
            if text == "أرجع" || text.starts_with("أرجع ") {
                if !self.in_routine {
                    return Err(diagnostic(
                        span(&line, 1),
                        "`أرجع` is only valid inside an algorithm or function",
                    ));
                }
                let value = text
                    .strip_prefix("أرجع")
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| parse_expr(&line, value))
                    .transpose()?;
                statements.push(Stmt::Return {
                    value,
                    span: span(&line, 1),
                });
                self.at += 1;
                continue;
            }
            if let Some(rest) = text.strip_prefix("أقرأ ") {
                let targets = split_output_parts(rest)
                    .into_iter()
                    .map(|target| parse_lvalue(&line, target.trim()))
                    .collect::<Result<Vec<_>, _>>()?;
                if targets.is_empty() {
                    return Err(diagnostic(
                        span(&line, 1),
                        "`أقرأ` needs at least one variable",
                    ));
                }
                statements.push(Stmt::Read {
                    targets,
                    span: span(&line, 1),
                });
                self.at += 1;
                continue;
            }
            if let Some(rest) = text.strip_prefix("أكتب ") {
                statements.push(parse_write(&line, rest)?);
                self.at += 1;
                continue;
            }
            if let Some(rest) = text.strip_prefix("أدرج ") {
                let library = parse_text_constant(rest.trim()).ok_or_else(|| {
                    diagnostic(
                        span(&line, 1),
                        "library inclusion needs a quoted library name",
                    )
                })?;
                self.register_builtin_library(&line, &library)?;
                self.libraries.insert(library);
                self.at += 1;
                continue;
            }
            if text.starts_with("أرسم ") || text.starts_with("ألون ") || text.starts_with("أنقش ")
            {
                for geometry in parse_draw_geometries(&line, text)? {
                    statements.push(Stmt::Draw {
                        geometry,
                        span: span(&line, 1),
                    });
                }
                self.at += 1;
                continue;
            }
            if let Some(rest) = text.strip_prefix("أفتح ") {
                let (name, path, mode) = if let Some((name, path)) = rest.split_once(" للقراءة من ")
                {
                    (name.trim(), path.trim(), FileMode::Read)
                } else if let Some((name, path)) = rest.split_once(" للكتابة في ") {
                    (name.trim(), path.trim(), FileMode::Write)
                } else {
                    return Err(diagnostic(
                        span(&line, 1),
                        "expected `للـقراءة من` or `للكتابة في` in file-open statement",
                    ));
                };
                if !valid_name(name) || path.is_empty() {
                    return Err(diagnostic(span(&line, 1), "invalid file-open statement"));
                }
                statements.push(Stmt::OpenFile {
                    name: name.to_owned(),
                    path: parse_expr(&line, path)?,
                    mode,
                    span: span(&line, 1),
                });
                self.at += 1;
                continue;
            }
            if let Some(rest) = text.strip_prefix("أقرأ من ") {
                let Some((name, targets)) = rest.split_once(':') else {
                    return Err(diagnostic(
                        span(&line, 1),
                        "expected `:` after file name in `أقرأ من`",
                    ));
                };
                let name = name.trim();
                if !valid_name(name) {
                    return Err(diagnostic(span(&line, 1), "invalid file variable name"));
                }
                let targets = split_output_parts(targets)
                    .into_iter()
                    .map(|target| parse_lvalue(&line, target.trim()))
                    .collect::<Result<Vec<_>, _>>()?;
                if targets.is_empty() {
                    return Err(diagnostic(
                        span(&line, 1),
                        "`أقرأ من` needs at least one target",
                    ));
                }
                statements.push(Stmt::ReadFile {
                    name: name.to_owned(),
                    targets,
                    span: span(&line, 1),
                });
                self.at += 1;
                continue;
            }
            if let Some(rest) = text.strip_prefix("أكتب في ") {
                let Some((name, values)) = rest.split_once(':') else {
                    return Err(diagnostic(
                        span(&line, 1),
                        "expected `:` after file name in `أكتب في`",
                    ));
                };
                let name = name.trim();
                if !valid_name(name) {
                    return Err(diagnostic(span(&line, 1), "invalid file variable name"));
                }
                let parts = parse_output_parts(&line, values)?;
                statements.push(Stmt::WriteFile {
                    name: name.to_owned(),
                    parts,
                    span: span(&line, 1),
                });
                self.at += 1;
                continue;
            }
            if let Some(name) = text.strip_prefix("أغلق ") {
                let name = name.trim();
                if !valid_name(name) {
                    return Err(diagnostic(span(&line, 1), "invalid file variable name"));
                }
                statements.push(Stmt::CloseFile {
                    name: name.to_owned(),
                    span: span(&line, 1),
                });
                self.at += 1;
                continue;
            }
            if let Some(rest) = text.strip_prefix("إذا ") {
                let condition = parse_condition(&line, rest)?;
                self.at += 1;
                self.open_brace()?;
                let then_body = self.block(true)?;
                self.skip_ignored();
                let else_body = if self
                    .lines
                    .get(self.at)
                    .is_some_and(|line| line.text.trim() == "وإلا")
                {
                    self.at += 1;
                    self.open_brace()?;
                    self.block(true)?
                } else {
                    Vec::new()
                };
                statements.push(Stmt::If {
                    condition,
                    then_body,
                    else_body,
                    span: span(&line, 1),
                });
                continue;
            }
            if let Some(rest) = text.strip_prefix("أكرر طالما ") {
                let condition = parse_condition(&line, rest)?;
                self.at += 1;
                self.open_brace()?;
                let body = self.block(true)?;
                statements.push(Stmt::While {
                    condition,
                    body,
                    span: span(&line, 1),
                });
                continue;
            }
            if let Some(rest) = text.strip_prefix("أكرر من ") {
                let (name, start, end, step) = parse_for_header(&line, rest)?;
                self.at += 1;
                self.open_brace()?;
                let body = self.block(true)?;
                statements.push(Stmt::For {
                    name,
                    start,
                    end,
                    step,
                    body,
                    span: span(&line, 1),
                });
                continue;
            }
            if text == "أكرر" {
                self.at += 1;
                self.open_brace()?;
                self.post_condition = None;
                let body = self.block(true)?;
                let condition = self.post_condition.take().ok_or_else(|| {
                    diagnostic(
                        span(&line, 1),
                        "postcondition loop must close with `}حتى <condition>`",
                    )
                })?;
                statements.push(Stmt::RepeatUntil {
                    condition,
                    body,
                    span: span(&line, 1),
                });
                continue;
            }
            if let Some(rest) = text.strip_prefix("أكرر ")
                && let Some(count_source) = rest.strip_suffix(" مرة")
            {
                let count = parse_expr(&line, count_source)?;
                self.at += 1;
                self.open_brace()?;
                let body = self.block(true)?;
                statements.push(Stmt::Counted {
                    count,
                    body,
                    span: span(&line, 1),
                });
                continue;
            }
            let call_source = text.strip_prefix("أنفذ ").unwrap_or(text);
            if let Some((name, arguments)) = parse_call(&line, call_source)? {
                statements.push(Stmt::Call {
                    name,
                    arguments,
                    span: span(&line, 1),
                });
                self.at += 1;
                continue;
            }
            if [
                "أرسم",
                "ألون",
                "أنقش",
                "أفتح",
                "أغلق",
                "أدرج",
                "بدل",
                "أنفذ",
                "أرجع",
                "أنتهي",
                "الخوارزمية",
                "التابع",
            ]
            .iter()
            .any(|keyword| text.starts_with(keyword))
            {
                return Err(diagnostic(
                    span(&line, 1),
                    format!(
                        "`{text}` belongs to a documented feature whose source does not provide a complete parseable form"
                    ),
                ));
            }
            return Err(diagnostic(
                span(&line, 1),
                format!("unsupported statement `{text}`"),
            ));
        }
        if nested {
            return Err(Diagnostic {
                span: Span {
                    line: self.lines.last().map_or(1, |line| line.number),
                    column: 1,
                },
                message: "expected `}` to close `أكرر طالما`".to_owned(),
            });
        }
        Ok(statements)
    }

    fn parse_routine(
        &mut self,
        line: &SourceLine,
        header: &str,
        is_function: bool,
        nested: bool,
    ) -> Result<(), Diagnostic> {
        if nested || self.in_routine {
            return Err(diagnostic(
                span(line, 1),
                "routine definitions must be at program level",
            ));
        }
        let (name, parameters, return_type) = parse_routine_header(
            line,
            header,
            is_function,
            &self.record_names,
            &self.type_aliases,
        )?;
        if self.routines.contains_key(&name)
            || self.declarations.contains_key(&name)
            || self.constants.contains_key(&name)
            || self.record_names.contains_key(&name)
            || self.type_aliases.contains_key(&name)
        {
            return Err(diagnostic(
                span(line, 1),
                format!("duplicate program name `{name}`"),
            ));
        }
        let visible_globals = self.declarations.clone();
        let visible_constants = self.constants.clone();
        let visible_arrays = self.arrays.clone();
        let mut local_declarations = HashMap::new();
        for parameter in &parameters {
            local_declarations.insert(parameter.name.clone(), (parameter.ty, span(line, 1)));
        }
        self.local_declarations = Some(local_declarations);
        self.local_arrays = Some(HashMap::new());
        self.local_constants = Some(HashMap::new());
        self.in_routine = true;
        self.at += 1;
        let routine_body = (|| {
            self.open_brace()?;
            self.block(true)
        })();
        self.in_routine = false;
        let declarations = self.local_declarations.take().unwrap_or_default();
        let arrays = self.local_arrays.take().unwrap_or_default();
        let constants = self.local_constants.take().unwrap_or_default();
        let statements = routine_body?;
        self.routines.insert(
            name.clone(),
            Routine {
                parameters,
                return_type,
                visible_globals,
                visible_constants,
                constants,
                visible_arrays,
                declarations,
                arrays,
                statements,
                span: span(line, 1),
            },
        );
        Ok(())
    }

    fn parse_type_declaration(
        &mut self,
        line: &SourceLine,
        header: &str,
        nested: bool,
    ) -> Result<(), Diagnostic> {
        let Some((name, type_source)) = header.split_once(':') else {
            return Err(diagnostic(span(line, 1), "type declaration needs `:`"));
        };
        if type_source.trim().starts_with("صفيفة") {
            if nested || self.in_routine {
                return Err(diagnostic(
                    span(line, 1),
                    "type aliases must be declared at program level",
                ));
            }
            let name = name.trim();
            if !valid_name(name)
                || self.record_names.contains_key(name)
                || self.type_aliases.contains_key(name)
                || self.declarations.contains_key(name)
                || self.constants.contains_key(name)
                || self.routines.contains_key(name)
            {
                return Err(diagnostic(
                    span(line, 1),
                    "invalid or duplicate type alias name",
                ));
            }
            let constants = self.visible_constants();
            let (ty, array_spec) = parse_declaration_type(
                line,
                type_source.trim(),
                &self.record_names,
                &self.type_aliases,
                &constants,
            )?;
            self.type_aliases
                .insert(name.to_owned(), TypeAlias { ty, array_spec });
            self.at += 1;
            return Ok(());
        }
        self.parse_record_type(line, header, nested)
    }

    fn register_builtin_library(
        &mut self,
        line: &SourceLine,
        library: &str,
    ) -> Result<(), Diagnostic> {
        match library {
            "رياضيات" => {
                for (name, value) in [
                    ("بي", Value::Real(3.141593)),
                    ("صفر", Value::Integer(0)),
                    ("أكبر_عدد_صغير", Value::Integer(1 << 15)),
                    ("أكبر_عدد_صحيح", Value::Integer(1 << 31)),
                    ("أكبر_عدد_حقيقي", Value::Real(1.7e308)),
                    ("أصغر_عدد_حقيقي", Value::Real(1.7e-308)),
                ] {
                    if self.constants.contains_key(name)
                        || self.declarations.contains_key(name)
                        || self.routines.contains_key(name)
                    {
                        return Err(diagnostic(
                            span(line, 1),
                            format!("duplicate declaration `{name}` from library `{library}`"),
                        ));
                    }
                    self.constants.insert(name.to_owned(), value);
                }
            }
            "حروف" => {
                self.insert_builtin_array_alias(
                    line,
                    "كلمة",
                    Type::CharacterArray,
                    Type::Character,
                    64,
                )?;
            }
            "ألوان" => {
                let type_id = self.insert_builtin_record(
                    line,
                    "لون",
                    [
                        ("ح", Type::Integer),
                        ("خ", Type::Integer),
                        ("ز", Type::Integer),
                    ],
                )?;
                self.insert_builtin_array_alias(
                    line,
                    "لوحة_ألوان",
                    Type::RecordArray(type_id),
                    Type::Record(type_id),
                    64,
                )?;
                self.insert_builtin_array_alias(
                    line,
                    "اسم",
                    Type::CharacterArray,
                    Type::Character,
                    255,
                )?;
                if self.constants.contains_key("عدد_الألوان_الأعظمي") {
                    return Err(diagnostic(
                        span(line, 1),
                        "duplicate color-library constant",
                    ));
                }
                self.constants
                    .insert("عدد_الألوان_الأعظمي".to_owned(), Value::Integer(64));
            }
            "الأعداد العقدية" => {
                self.insert_builtin_record(line, "عقدي", [("ح", Type::Real), ("خ", Type::Real)])?;
            }
            "المترادفات" => {}
            _ => {}
        }
        Ok(())
    }

    fn insert_builtin_record<const N: usize>(
        &mut self,
        line: &SourceLine,
        name: &str,
        field_specs: [(&str, Type); N],
    ) -> Result<u16, Diagnostic> {
        if self.record_names.contains_key(name)
            || self.type_aliases.contains_key(name)
            || self.declarations.contains_key(name)
            || self.constants.contains_key(name)
            || self.routines.contains_key(name)
        {
            return Err(diagnostic(
                span(line, 1),
                format!("duplicate type name `{name}`"),
            ));
        }
        let type_id = self
            .record_types
            .keys()
            .copied()
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| diagnostic(span(line, 1), "too many record types"))?;
        let fields = field_specs
            .into_iter()
            .map(|(name, ty)| {
                (
                    name.to_owned(),
                    RecordField {
                        ty,
                        array_spec: None,
                    },
                )
            })
            .collect();
        self.record_names.insert(name.to_owned(), type_id);
        self.record_types.insert(
            type_id,
            RecordDefinition {
                name: name.to_owned(),
                fields,
            },
        );
        Ok(type_id)
    }

    fn insert_builtin_array_alias(
        &mut self,
        line: &SourceLine,
        name: &str,
        ty: Type,
        element_type: Type,
        length: usize,
    ) -> Result<(), Diagnostic> {
        if self.type_aliases.contains_key(name)
            || self.record_names.contains_key(name)
            || self.declarations.contains_key(name)
            || self.constants.contains_key(name)
            || self.routines.contains_key(name)
        {
            return Err(diagnostic(
                span(line, 1),
                format!("duplicate type alias `{name}`"),
            ));
        }
        self.type_aliases.insert(
            name.to_owned(),
            TypeAlias {
                ty,
                array_spec: Some(ArraySpec {
                    element_type,
                    dimensions: vec![length],
                }),
            },
        );
        Ok(())
    }

    fn parse_record_type(
        &mut self,
        line: &SourceLine,
        header: &str,
        nested: bool,
    ) -> Result<(), Diagnostic> {
        if nested || self.in_routine {
            return Err(diagnostic(
                span(line, 1),
                "record types must be declared at program level",
            ));
        }
        let Some((name, kind_source)) = header.split_once(':') else {
            return Err(diagnostic(
                span(line, 1),
                "record type header needs `: مركب`, `سجل`, `كيان`, or `وحدة`",
            ));
        };
        let name = name.trim();
        if !valid_name(name)
            || self.record_names.contains_key(name)
            || self.type_aliases.contains_key(name)
            || self.declarations.contains_key(name)
            || self.constants.contains_key(name)
            || self.routines.contains_key(name)
        {
            return Err(diagnostic(
                span(line, 1),
                "invalid or duplicate record type name",
            ));
        }

        self.at += 1;
        let mut kind = kind_source.trim().to_owned();
        if kind.is_empty() {
            self.skip_ignored();
            let kind_line = self.lines.get(self.at).ok_or_else(|| {
                diagnostic(span(line, 1), "record type needs a record body marker")
            })?;
            kind = kind_line.text.trim().to_owned();
            self.at += 1;
        }
        let inline_open = kind.ends_with('{');
        if inline_open {
            kind.pop();
            kind = kind.trim().to_owned();
        }
        if !matches!(kind.as_str(), "مركب" | "سجل" | "كيان" | "وحدة") {
            return Err(diagnostic(
                span(line, 1),
                "record body marker must be `مركب`, `سجل`, `كيان`, or `وحدة`",
            ));
        }
        if !inline_open {
            self.open_brace()?;
        }

        let mut fields = HashMap::new();
        let mut closed = false;
        while self.at < self.lines.len() {
            let field_line = self.lines[self.at].clone();
            let text = field_line.text.trim();
            if text.is_empty() || text.starts_with("!!") {
                self.at += 1;
                continue;
            }
            if text == "}" {
                self.at += 1;
                closed = true;
                break;
            }
            let Some((field_names, type_source)) = text.split_once(':') else {
                return Err(diagnostic(
                    span(&field_line, 1),
                    "record field declaration needs `: <type>`",
                ));
            };
            let constants = self.visible_constants();
            let (ty, array_spec) = parse_declaration_type(
                &field_line,
                type_source.trim(),
                &self.record_names,
                &self.type_aliases,
                &constants,
            )?;
            for field_name in field_names.split(['،', ',']).map(str::trim) {
                if !valid_name(field_name) {
                    return Err(diagnostic(
                        span(&field_line, 1),
                        "invalid record field name",
                    ));
                }
                if fields
                    .insert(
                        field_name.to_owned(),
                        RecordField {
                            ty,
                            array_spec: array_spec.clone(),
                        },
                    )
                    .is_some()
                {
                    return Err(diagnostic(
                        span(&field_line, 1),
                        format!("duplicate record field `{field_name}`"),
                    ));
                }
            }
            self.at += 1;
        }
        if !closed {
            return Err(diagnostic(
                span(line, 1),
                "record type is missing its closing `}`",
            ));
        }
        if fields.is_empty() {
            return Err(diagnostic(
                span(line, 1),
                "record type needs at least one field",
            ));
        }
        let type_id = self
            .record_types
            .keys()
            .copied()
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| diagnostic(span(line, 1), "too many record types"))?;
        self.record_names.insert(name.to_owned(), type_id);
        self.record_types.insert(
            type_id,
            RecordDefinition {
                name: name.to_owned(),
                fields,
            },
        );
        Ok(())
    }

    fn declare(&mut self, line: &SourceLine, declaration: &str) -> Result<(), Diagnostic> {
        let Some((names, type_name)) = declaration.split_once(':') else {
            return Err(diagnostic(
                span(line, 1),
                "expected `:` and a documented type in variable declaration",
            ));
        };
        let constants = self.visible_constants();
        let (ty, array_spec) = parse_declaration_type(
            line,
            type_name.trim(),
            &self.record_names,
            &self.type_aliases,
            &constants,
        )?;
        for name in names.split(['،', ',']).map(str::trim) {
            if !valid_name(name) {
                return Err(diagnostic(span(line, 1), "invalid Arabic variable name"));
            }
            let collision = self.constants.contains_key(name)
                || self.record_names.contains_key(name)
                || self.type_aliases.contains_key(name)
                || self
                    .local_constants
                    .as_ref()
                    .is_some_and(|constants| constants.contains_key(name))
                || self.routines.contains_key(name);
            let duplicate = if let Some(locals) = &mut self.local_declarations {
                let duplicate = locals
                    .insert(name.to_owned(), (ty, span(line, 1)))
                    .is_some();
                if let Some(spec) = &array_spec {
                    self.local_arrays
                        .get_or_insert_with(HashMap::new)
                        .insert(name.to_owned(), spec.clone());
                }
                duplicate
            } else {
                let duplicate = self
                    .declarations
                    .insert(name.to_owned(), (ty, span(line, 1)))
                    .is_some();
                if let Some(spec) = &array_spec {
                    self.arrays.insert(name.to_owned(), spec.clone());
                }
                duplicate
            };
            if collision || duplicate {
                return Err(diagnostic(
                    span(line, 1),
                    format!("duplicate variable declaration `{name}`"),
                ));
            }
        }
        Ok(())
    }

    fn declare_constant(&mut self, line: &SourceLine, declaration: &str) -> Result<(), Diagnostic> {
        let Some((name, value_source)) = declaration.split_once('=') else {
            return Err(diagnostic(
                span(line, 1),
                "expected `=` in constant declaration",
            ));
        };
        let name = name.trim();
        if !valid_name(name) {
            return Err(diagnostic(span(line, 1), "invalid Arabic constant name"));
        }
        if self.declarations.contains_key(name)
            || self.record_names.contains_key(name)
            || self.type_aliases.contains_key(name)
            || self
                .local_declarations
                .as_ref()
                .is_some_and(|locals| locals.contains_key(name))
            || self.constants.contains_key(name)
            || self
                .local_constants
                .as_ref()
                .is_some_and(|constants| constants.contains_key(name))
            || self.routines.contains_key(name)
        {
            return Err(diagnostic(
                span(line, 1),
                format!("duplicate declaration `{name}`"),
            ));
        }
        let expression = parse_expr(line, value_source.trim())?;
        let mut visible_constants = self.constants.clone();
        if let Some(local_constants) = &self.local_constants {
            visible_constants.extend(local_constants.clone());
        }
        let value = eval_constant_expression(&expression, &visible_constants)
            .map_err(|error| diagnostic(span(line, 1), error))?;
        if let Some(local_constants) = &mut self.local_constants {
            local_constants.insert(name.to_owned(), value);
        } else {
            self.constants.insert(name.to_owned(), value);
        }
        Ok(())
    }

    fn skip_ignored(&mut self) {
        while let Some(line) = self.lines.get(self.at) {
            let text = line.text.trim();
            if text.is_empty() || text.starts_with("!!") {
                self.at += 1;
            } else {
                break;
            }
        }
    }

    fn visible_constants(&self) -> HashMap<String, Value> {
        let mut visible = self.constants.clone();
        if let Some(local) = &self.local_constants {
            visible.extend(local.clone());
        }
        visible
    }

    fn open_brace(&mut self) -> Result<(), Diagnostic> {
        while self.at < self.lines.len() {
            let line = &self.lines[self.at];
            let text = line.text.trim();
            if text.is_empty() || text.starts_with("!!") {
                self.at += 1;
                continue;
            }
            if text == "{" {
                self.at += 1;
                return Ok(());
            }
            return Err(diagnostic(
                span(line, 1),
                "expected `{` on a separate line after the block header",
            ));
        }
        Err(Diagnostic {
            span: Span {
                line: self.lines.last().map_or(1, |line| line.number),
                column: 1,
            },
            message: "expected `{` after block header".to_owned(),
        })
    }
}

fn parse_assignment(line: &SourceLine, text: &str) -> Result<Stmt, Diagnostic> {
    let Some((name, expression)) = text.split_once('=') else {
        return Err(diagnostic(
            span(line, 1),
            "expected `=` in `أجعل` assignment",
        ));
    };
    let target = parse_lvalue(line, name.trim())?;
    Ok(Stmt::Assign {
        target,
        value: parse_expr(line, expression)?,
        span: span(line, 1),
    })
}

fn parse_lvalue(line: &SourceLine, source: &str) -> Result<LValue, Diagnostic> {
    let source = source.trim();
    if let Some(open) = source.find('[') {
        let Some(close) = matching_delimiter(source, open, '[', ']') else {
            return Err(diagnostic(span(line, 1), "array index is not closed"));
        };
        let name = source[..open].trim();
        let suffix = source[close + 1..].trim();
        if !valid_name(name) {
            return Err(diagnostic(span(line, 1), "invalid array element reference"));
        }
        let indexes = split_output_parts(&source[open + 1..close])
            .into_iter()
            .map(|index| parse_expr(line, index.trim()))
            .collect::<Result<Vec<_>, _>>()?;
        if indexes.is_empty() {
            return Err(diagnostic(span(line, 1), "array reference needs an index"));
        }
        if !suffix.is_empty() {
            let field_source = suffix
                .strip_prefix('.')
                .ok_or_else(|| diagnostic(span(line, 1), "invalid indexed field reference"))?;
            let fields = parse_field_suffix(field_source)
                .ok_or_else(|| diagnostic(span(line, 1), "invalid indexed field path"))?;
            return Ok(LValue::IndexedField {
                name: name.to_owned(),
                indexes,
                fields,
                span: span(line, 1),
            });
        }
        return Ok(LValue::Index {
            name: name.to_owned(),
            indexes,
            span: span(line, 1),
        });
    }
    if let Some((name, fields)) = parse_field_path(source)
        && !fields.is_empty()
    {
        return Ok(LValue::Field {
            name,
            fields,
            span: span(line, 1),
        });
    }
    if !valid_name(source) {
        return Err(diagnostic(
            span(line, 1),
            "expected an Arabic variable name",
        ));
    }
    Ok(LValue::Variable {
        name: source.to_owned(),
        span: span(line, 1),
    })
}

fn parse_for_header(
    line: &SourceLine,
    source: &str,
) -> Result<(String, Expr, Expr, Option<Expr>), Diagnostic> {
    let Some((initialization, range)) = source.split_once("إلى") else {
        return Err(diagnostic(
            span(line, 1),
            "expected `إلى` in a range-loop header",
        ));
    };
    let Some((name, start)) = initialization.split_once('=') else {
        return Err(diagnostic(
            span(line, 1),
            "expected `<variable> = <start>` after `أكرر من`",
        ));
    };
    let name = name.trim();
    if !valid_name(name) {
        return Err(diagnostic(span(line, 1), "invalid range-loop variable"));
    }
    let (end, step) = match range.split_once("بخطوة") {
        Some((end, step)) => (end.trim(), Some(parse_expr(line, step.trim())?)),
        None => (range.trim(), None),
    };
    if end.is_empty() {
        return Err(diagnostic(span(line, 1), "expected an inclusive range end"));
    }
    Ok((
        name.to_owned(),
        parse_expr(line, start.trim())?,
        parse_expr(line, end)?,
        step,
    ))
}

fn parse_routine_header(
    line: &SourceLine,
    source: &str,
    is_function: bool,
    record_names: &HashMap<String, u16>,
    type_aliases: &HashMap<String, TypeAlias>,
) -> Result<(String, Vec<Parameter>, Option<Type>), Diagnostic> {
    let Some(open) = source.find('(') else {
        return Err(diagnostic(
            span(line, 1),
            "routine header needs a parameter list",
        ));
    };
    let Some(close) = matching_paren(source, open) else {
        return Err(diagnostic(
            span(line, 1),
            "routine parameter list is not closed",
        ));
    };
    let name = source[..open].trim();
    if !valid_routine_name(name) {
        return Err(diagnostic(span(line, 1), "invalid routine name"));
    }
    let suffix = source[close + 1..].trim();
    let return_type = if is_function {
        let (_, ty) = suffix
            .split_once("-->")
            .or_else(|| suffix.split_once("->"))
            .or_else(|| suffix.split_once('→'))
            .ok_or_else(|| diagnostic(span(line, 1), "function header needs `-> <type>`"))?;
        let ty = parse_type(line, ty.trim(), record_names, type_aliases)?;
        if !matches!(
            ty,
            Type::Integer
                | Type::SmallInteger
                | Type::Real
                | Type::Logical
                | Type::Character
                | Type::Record(_)
        ) {
            return Err(diagnostic(
                span(line, 1),
                "function return type must be scalar",
            ));
        }
        Some(ty)
    } else {
        if !suffix.is_empty() {
            return Err(diagnostic(
                span(line, 1),
                "algorithm header cannot have a return type",
            ));
        }
        None
    };
    let mut parameters = Vec::new();
    let mut pending_names = Vec::new();
    let normalized = source[open + 1..close]
        .replace(" – ", ";")
        .replace(" - ", ";");
    for group in normalized.split(';') {
        for part in group.split(['،', ',']).map(str::trim) {
            if part.is_empty() {
                continue;
            }
            if let Some((names, ty)) = part.split_once(':') {
                let ty = parse_parameter_type(line, ty.trim(), record_names, type_aliases)?;
                let mut all_names = std::mem::take(&mut pending_names);
                all_names.extend(names.split(['،', ',']).map(str::trim).map(str::to_owned));
                for raw_name in all_names {
                    let (constant, name) = match raw_name.strip_prefix("الثابت ") {
                        Some(name) => (true, name.trim()),
                        None => (false, raw_name.trim()),
                    };
                    if !valid_name(name) {
                        return Err(diagnostic(span(line, 1), "invalid routine parameter name"));
                    }
                    parameters.push(Parameter {
                        name: name.to_owned(),
                        ty,
                        constant,
                    });
                }
            } else {
                pending_names.push(part.to_owned());
            }
        }
    }
    if !pending_names.is_empty() {
        return Err(diagnostic(
            span(line, 1),
            "routine parameter group needs a type",
        ));
    }
    let mut seen = HashMap::new();
    for parameter in &parameters {
        if seen.insert(parameter.name.as_str(), ()).is_some() {
            return Err(diagnostic(span(line, 1), "duplicate routine parameter"));
        }
    }
    Ok((name.to_owned(), parameters, return_type))
}

fn parse_parameter_type(
    line: &SourceLine,
    source: &str,
    record_names: &HashMap<String, u16>,
    type_aliases: &HashMap<String, TypeAlias>,
) -> Result<Type, Diagnostic> {
    if let Some(rest) = source.strip_prefix("صفيفة") {
        let element_type = rest.trim().strip_prefix("من ").ok_or_else(|| {
            diagnostic(
                span(line, 1),
                "array parameter type needs `صفيفة من <element type>`",
            )
        })?;
        let element_type = parse_type(line, element_type.trim(), record_names, type_aliases)?;
        return array_type(element_type).ok_or_else(|| {
            diagnostic(
                span(line, 1),
                "array parameters need a supported scalar element type",
            )
        });
    }
    parse_type(line, source, record_names, type_aliases)
}

fn parse_type(
    line: &SourceLine,
    name: &str,
    record_names: &HashMap<String, u16>,
    type_aliases: &HashMap<String, TypeAlias>,
) -> Result<Type, Diagnostic> {
    if let Some(type_id) = record_names.get(name) {
        return Ok(Type::Record(*type_id));
    }
    if let Some(alias) = type_aliases.get(name) {
        return Ok(alias.ty);
    }
    match name {
        "صحيح" => Ok(Type::Integer),
        "صغير" => Ok(Type::SmallInteger),
        "حقيقي" => Ok(Type::Real),
        "منطقي" => Ok(Type::Logical),
        "حرف" => Ok(Type::Character),
        "مؤشر" => Ok(Type::Pointer),
        "ملف" => Ok(Type::File),
        other => Err(diagnostic(
            span(line, 1),
            format!("type `{other}` has no executable definition in this profile"),
        )),
    }
}

fn parse_declaration_type(
    line: &SourceLine,
    source: &str,
    record_names: &HashMap<String, u16>,
    type_aliases: &HashMap<String, TypeAlias>,
    constants: &HashMap<String, Value>,
) -> Result<(Type, Option<ArraySpec>), Diagnostic> {
    if let Some(alias) = type_aliases.get(source) {
        return Ok((alias.ty, alias.array_spec.clone()));
    }
    if let Some(rest) = source.strip_prefix("صفيفة") {
        let rest = rest.trim_start();
        if !rest.starts_with('[') {
            return Err(diagnostic(
                span(line, 1),
                "array declaration needs a size in square brackets",
            ));
        }
        let Some(close) = matching_delimiter(rest, 0, '[', ']') else {
            return Err(diagnostic(span(line, 1), "array size list is not closed"));
        };
        let dimensions = split_output_parts(&rest[1..close])
            .into_iter()
            .map(|size| {
                let expression = parse_expr(line, size.trim())?;
                let dimension = eval_constant_expression(&expression, constants)
                    .map_err(|message| diagnostic(span(line, 1), message))?;
                match dimension {
                    Value::Integer(size) => usize::try_from(size)
                        .ok()
                        .filter(|size| *size > 0)
                        .ok_or_else(|| {
                            diagnostic(span(line, 1), "array sizes must be positive integers")
                        }),
                    _ => Err(diagnostic(
                        span(line, 1),
                        "array sizes must be integer constant expressions",
                    )),
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        let element_source = rest[close + 1..]
            .trim()
            .strip_prefix("من ")
            .ok_or_else(|| diagnostic(span(line, 1), "array type needs `من <element type>`"))?;
        let element_type = parse_type(line, element_source.trim(), record_names, type_aliases)?;
        let ty = array_type(element_type).ok_or_else(|| {
            diagnostic(span(line, 1), "arrays of this element type are not defined")
        })?;
        let size = dimensions
            .iter()
            .try_fold(1usize, |product, dimension| product.checked_mul(*dimension));
        if size.is_none_or(|size| size > 1_000_000) {
            return Err(diagnostic(
                span(line, 1),
                "array allocation size is too large",
            ));
        }
        return Ok((
            ty,
            Some(ArraySpec {
                element_type,
                dimensions,
            }),
        ));
    }
    let ty = match source {
        "صحيح" => Type::Integer,
        "صغير" => Type::SmallInteger,
        "حقيقي" => Type::Real,
        "منطقي" => Type::Logical,
        "حرف" => Type::Character,
        "مؤشر" => Type::Pointer,
        "ملف" => Type::File,
        other => {
            return record_names
                .get(other)
                .copied()
                .map(|type_id| (Type::Record(type_id), None))
                .ok_or_else(|| {
                    diagnostic(
                        span(line, 1),
                        format!("type `{other}` has no source-backed declaration form"),
                    )
                });
        }
    };
    Ok((ty, None))
}

fn parse_call(line: &SourceLine, source: &str) -> Result<Option<(String, Vec<Expr>)>, Diagnostic> {
    let source = source.trim();
    let Some(open) = source.find('(') else {
        return Ok(None);
    };
    let Some(close) = matching_paren(source, open) else {
        return Err(diagnostic(
            span(line, 1),
            "call argument list is not closed",
        ));
    };
    if !source[close + 1..].trim().is_empty() {
        return Ok(None);
    }
    let name = source[..open].trim();
    if !valid_routine_name(name) && !is_builtin_name(name) {
        return Ok(None);
    }
    let arguments = split_output_parts(&source[open + 1..close])
        .into_iter()
        .map(str::trim)
        .filter(|argument| !argument.is_empty())
        .map(|argument| parse_expr(line, argument))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some((name.to_owned(), arguments)))
}

fn valid_routine_name(source: &str) -> bool {
    if valid_identifier(source) {
        return true;
    }
    let mut has_identifier = false;
    for word in source.split_whitespace() {
        if valid_identifier(word) {
            has_identifier = true;
        } else if normalize_digits(word).parse::<u64>().is_err() {
            return false;
        }
    }
    has_identifier && !source.trim().is_empty()
}

fn valid_name(source: &str) -> bool {
    valid_identifier(source) || valid_routine_name(source)
}

fn is_builtin_name(name: &str) -> bool {
    matches!(
        name,
        "جب" | "تجب"
            | "جذر"
            | "لغ"
            | "ظل"
            | "تظل"
            | "باقي"
            | "تدوير"
            | "عجب"
            | "عتجب"
            | "عظل"
            | "حدس"
            | "حدع"
            | "جديد"
            | "صحيح"
            | "أبجدي"
            | "عكس"
            | "طول_الكلمة"
            | "أنسخ_كلمة"
            | "نسخ_كلمة"
            | "أقارن_كلمتين"
            | "أقرأ_لوحة_ألوان"
            | "أنتقي_اللون"
            | "أجعل_عقدي"
            | "أجمع_عقدي"
            | "أضرب_عقدي"
            | "مرافق_عقدي"
            | "طويلة_عقدي"
            | "أرسم_عقدي"
            | "أقرأ_عقدي"
            | "أكتب_عقدي"
    )
}

fn parse_draw_geometries(line: &SourceLine, source: &str) -> Result<Vec<DrawGeometry>, Diagnostic> {
    if let Some(rest) = source.strip_prefix("أرسم باللون ") {
        let (channels, trailing) = parse_color_triplet_prefix(line, rest)?;
        let mut geometries = vec![DrawGeometry::StrokeColor(channels)];
        let trailing = strip_drawing_separator(trailing);
        if !trailing.is_empty() {
            let statement = format!("أرسم {trailing}");
            geometries.push(parse_draw_geometry(line, &statement)?);
        }
        return Ok(geometries);
    }
    if let Some(rest) = source.strip_prefix("ألون باللون ") {
        let (channels, trailing) = parse_color_triplet_prefix(line, rest)?;
        let trailing = strip_drawing_separator(trailing);
        if trailing == "الورقة" {
            return Ok(vec![DrawGeometry::FillCanvas(Some(channels))]);
        }
        let mut geometries = vec![DrawGeometry::FillColor(channels)];
        if !trailing.is_empty() {
            let statement = format!("ألون {trailing}");
            geometries.push(parse_draw_geometry(line, &statement)?);
        }
        return Ok(geometries);
    }
    Ok(vec![parse_draw_geometry(line, source)?])
}

fn strip_drawing_separator(source: &str) -> &str {
    source
        .trim_start()
        .strip_prefix('،')
        .or_else(|| source.trim_start().strip_prefix(','))
        .map(str::trim_start)
        .unwrap_or_else(|| source.trim_start())
}

fn parse_draw_geometry(line: &SourceLine, source: &str) -> Result<DrawGeometry, Diagnostic> {
    if let Some(rest) = source.strip_prefix("أرسم النقطة ") {
        let (x, y) = parse_coordinate_pair(line, rest)?;
        return Ok(DrawGeometry::Point(x, y));
    }
    if let Some(rest) = source.strip_prefix("أرسم مستقيم من ") {
        let mut points = Vec::new();
        let (first, mut rest) = parse_coordinate_pair_prefix(line, rest)?;
        points.push(first);
        while let Some(after_connector) = rest.strip_prefix("إلى ") {
            let (point, remaining) = parse_coordinate_pair_prefix(line, after_connector)?;
            points.push(point);
            rest = remaining;
        }
        if !rest.is_empty() || points.len() < 2 {
            return Err(diagnostic(
                span(line, 1),
                "a line needs at least two coordinate pairs joined by `إلى`",
            ));
        }
        return Ok(DrawGeometry::Line(points));
    }
    if let Some(rest) = source.strip_prefix("أرسم دائرة مركزها ") {
        let (center, rest) = parse_coordinate_pair_prefix(line, rest)?;
        let diameter = rest
            .strip_prefix("قطرها ")
            .ok_or_else(|| diagnostic(span(line, 1), "expected `قطرها` after circle center"))?;
        return Ok(DrawGeometry::Circle {
            center,
            diameter: parse_expr(line, diameter)?,
        });
    }
    if let Some(rest) = source.strip_prefix("أرسم إهليلج مركزه ") {
        let (center, rest) = parse_coordinate_pair_prefix(line, rest)?;
        let diameters = rest
            .strip_prefix("قطريه ")
            .ok_or_else(|| diagnostic(span(line, 1), "expected `قطريه` after ellipse center"))?;
        return Ok(DrawGeometry::Ellipse {
            center,
            diameters: parse_coordinate_pair(line, diameters)?,
        });
    }
    if let Some(rest) = source.strip_prefix("أرسم مستطيل من ") {
        return Ok(DrawGeometry::Rectangle {
            from: parse_coordinate_pair_prefix(line, rest)?.0,
            to: parse_coordinate_pair_after_connector(line, rest, "إلى ")?,
        });
    }
    if let Some(rest) = source.strip_prefix("أرسم مثلث رؤوسه ") {
        return Ok(DrawGeometry::Triangle(parse_three_points(line, rest)?));
    }
    if let Some(rest) = source.strip_prefix("ألون دائرة مركزها ") {
        let (center, rest) = parse_coordinate_pair_prefix(line, rest)?;
        let diameter = rest
            .strip_prefix("قطرها ")
            .ok_or_else(|| diagnostic(span(line, 1), "expected `قطرها` after circle center"))?;
        return Ok(DrawGeometry::FillCircle {
            center,
            diameter: parse_expr(line, diameter)?,
        });
    }
    if let Some(rest) = source.strip_prefix("ألون إهليلج مركزه ") {
        let (center, rest) = parse_coordinate_pair_prefix(line, rest)?;
        let diameters = rest
            .strip_prefix("قطريه ")
            .ok_or_else(|| diagnostic(span(line, 1), "expected `قطريه` after ellipse center"))?;
        return Ok(DrawGeometry::FillEllipse {
            center,
            diameters: parse_coordinate_pair(line, diameters)?,
        });
    }
    if let Some(rest) = source.strip_prefix("ألون مستطيل من ") {
        return Ok(DrawGeometry::FillRectangle {
            from: parse_coordinate_pair_prefix(line, rest)?.0,
            to: parse_coordinate_pair_after_connector(line, rest, "إلى ")?,
        });
    }
    if let Some(rest) = source.strip_prefix("ألون مثلث رؤوسه ") {
        return Ok(DrawGeometry::FillTriangle(parse_three_points(line, rest)?));
    }
    if source == "ألون الورقة" {
        return Ok(DrawGeometry::FillCanvas(None));
    }
    if let Some(rest) = source.strip_prefix("أنقش ") {
        let (position, values_source) = if let Some(after_at) = rest.strip_prefix("عند ") {
            let (position, remaining) = parse_coordinate_pair_prefix(line, after_at)?;
            (Some(position), remaining)
        } else {
            (None, rest.trim())
        };
        let values = parse_output_parts(line, values_source)?;
        if values
            .iter()
            .any(|part| matches!(part, OutputPart::Newline))
        {
            return Err(diagnostic(
                span(line, 1),
                "`سطر` is not a drawing-label value",
            ));
        }
        return Ok(DrawGeometry::Label { position, values });
    }
    Err(diagnostic(
        span(line, 1),
        format!("unsupported drawing command `{source}`"),
    ))
}

fn parse_coordinate_pair(line: &SourceLine, source: &str) -> Result<(Expr, Expr), Diagnostic> {
    let (pair, rest) = parse_coordinate_pair_prefix(line, source)?;
    if !rest.is_empty() {
        return Err(diagnostic(
            span(line, 1),
            "unexpected text after coordinates",
        ));
    }
    Ok(pair)
}

fn parse_coordinate_pair_prefix<'a>(
    line: &SourceLine,
    source: &'a str,
) -> Result<((Expr, Expr), &'a str), Diagnostic> {
    let source = source.trim_start();
    if !source.starts_with('(') {
        return Err(diagnostic(span(line, 1), "expected `(x, y)` coordinates"));
    }
    let close = matching_paren(source, 0)
        .ok_or_else(|| diagnostic(span(line, 1), "coordinate pair is not closed"))?;
    let coordinates = split_output_parts(&source[1..close]);
    if coordinates.len() != 2 {
        return Err(diagnostic(
            span(line, 1),
            "coordinate pair needs two values",
        ));
    }
    Ok((
        (
            parse_expr(line, coordinates[0])?,
            parse_expr(line, coordinates[1])?,
        ),
        source[close + 1..].trim_start(),
    ))
}

fn parse_coordinate_pair_after_connector(
    line: &SourceLine,
    source: &str,
    connector: &str,
) -> Result<(Expr, Expr), Diagnostic> {
    let (_, rest) = parse_coordinate_pair_prefix(line, source)?;
    let rest = rest.strip_prefix(connector).ok_or_else(|| {
        diagnostic(
            span(line, 1),
            format!("expected `{}` between coordinates", connector.trim()),
        )
    })?;
    parse_coordinate_pair(line, rest)
}

fn parse_three_points(line: &SourceLine, source: &str) -> Result<[(Expr, Expr); 3], Diagnostic> {
    let mut rest = source.trim_start();
    let mut points = Vec::with_capacity(3);
    for index in 0..3 {
        let (point, remaining) = parse_coordinate_pair_prefix(line, rest)?;
        points.push(point);
        rest = remaining;
        if index < 2 {
            rest = rest.strip_prefix(' ').unwrap_or(rest);
        }
    }
    if !rest.trim().is_empty() {
        return Err(diagnostic(
            span(line, 1),
            "unexpected text after triangle vertices",
        ));
    }
    points
        .try_into()
        .map_err(|_| diagnostic(span(line, 1), "triangle needs three vertices"))
}

fn parse_color_triplet_prefix<'a>(
    line: &SourceLine,
    source: &'a str,
) -> Result<([Expr; 3], &'a str), Diagnostic> {
    let source = source.trim_start();
    if !source.starts_with('(') {
        return Err(diagnostic(span(line, 1), "expected `(red, green, blue)`"));
    }
    let close = matching_paren(source, 0)
        .ok_or_else(|| diagnostic(span(line, 1), "RGB color is not closed"))?;
    let channels = split_output_parts(&source[1..close]);
    if channels.len() != 3 {
        return Err(diagnostic(span(line, 1), "RGB color needs three channels"));
    }
    let parsed = [
        parse_expr(line, channels[0])?,
        parse_expr(line, channels[1])?,
        parse_expr(line, channels[2])?,
    ];
    Ok((parsed, source[close + 1..].trim()))
}

fn parse_write(line: &SourceLine, text: &str) -> Result<Stmt, Diagnostic> {
    let parts = parse_output_parts(line, text)?;
    Ok(Stmt::Write {
        parts,
        span: span(line, 1),
    })
}

fn parse_output_parts(line: &SourceLine, text: &str) -> Result<Vec<OutputPart>, Diagnostic> {
    let mut parts = Vec::new();
    for part in split_output_parts(text) {
        let part = part.trim();
        if part == "سطر" {
            parts.push(OutputPart::Newline);
        } else if part.starts_with('"') && part.ends_with('"') && part.len() >= 2 {
            let text = parse_text_constant(part).ok_or_else(|| {
                diagnostic(
                    span(line, 1),
                    "nested quotes are unsupported in output statements",
                )
            })?;
            parts.push(OutputPart::Text(text));
        } else {
            parts.push(OutputPart::Expression(parse_expr(line, part)?));
        }
    }
    if parts.is_empty() {
        return Err(diagnostic(
            span(line, 1),
            "output statement needs at least one item",
        ));
    }
    Ok(parts)
}

fn split_output_parts(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut parenthesis_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut quoted = false;
    for (index, character) in text.char_indices() {
        if character == '"' && !is_escaped_quote(text, index) {
            quoted = !quoted;
        } else if !quoted && character == '(' {
            parenthesis_depth += 1;
        } else if !quoted && character == ')' {
            parenthesis_depth = parenthesis_depth.saturating_sub(1);
        } else if !quoted && character == '[' {
            bracket_depth += 1;
        } else if !quoted && character == ']' {
            bracket_depth = bracket_depth.saturating_sub(1);
        } else if !quoted
            && parenthesis_depth == 0
            && bracket_depth == 0
            && matches!(character, ',' | '،')
        {
            parts.push(&text[start..index]);
            start = index + character.len_utf8();
        }
    }
    parts.push(&text[start..]);
    parts
}

fn parse_expr(line: &SourceLine, source: &str) -> Result<Expr, Diagnostic> {
    let source = source.trim();
    if source.is_empty() {
        return Err(diagnostic(span(line, 1), "expected an expression"));
    }
    if let Some(inner) = source.strip_prefix("نفي ") {
        return Ok(Expr::Unary(
            Box::new(parse_expr(line, inner)?),
            Unary::Not,
            span(line, 1),
        ));
    }
    if source.starts_with("||") && source.ends_with("||") && source.len() >= 4 {
        return Ok(Expr::Unary(
            Box::new(parse_expr(line, &source[2..source.len() - 2])?),
            Unary::Absolute,
            span(line, 1),
        ));
    }
    if let Some(rest) = source
        .strip_prefix('-')
        .or_else(|| source.strip_prefix('−'))
    {
        return Ok(Expr::Unary(
            Box::new(parse_expr(line, rest)?),
            Unary::Negate,
            span(line, 1),
        ));
    }
    let groups: &[(&[(&str, Arithmetic)], bool)] = &[
        (&[("أو", Arithmetic::Or)], false),
        (&[("و", Arithmetic::And)], false),
        (
            &[("=؟", Arithmetic::Equal), ("<>", Arithmetic::NotEqual)],
            false,
        ),
        (
            &[
                (">=", Arithmetic::GreaterOrEqual),
                ("≥", Arithmetic::GreaterOrEqual),
                ("<=", Arithmetic::LessOrEqual),
                ("≤", Arithmetic::LessOrEqual),
                (">", Arithmetic::Greater),
                ("<", Arithmetic::Less),
            ],
            false,
        ),
        (
            &[
                ("+", Arithmetic::Add),
                ("-", Arithmetic::Subtract),
                ("−", Arithmetic::Subtract),
            ],
            false,
        ),
        (
            &[
                ("×", Arithmetic::Multiply),
                ("÷", Arithmetic::Divide),
                ("\\", Arithmetic::IntegerDivide),
            ],
            false,
        ),
        (&[("^", Arithmetic::Power)], true),
    ];
    for (operators, right_associative) in groups {
        if let Some((index, width, operator)) =
            find_top_level_operator(source, operators, *right_associative)
        {
            let left = source[..index].trim();
            let right = source[index + width..].trim();
            if left.is_empty() || right.is_empty() {
                return Err(diagnostic(span(line, 1), "incomplete expression"));
            }
            return Ok(Expr::Binary(
                Box::new(parse_expr(line, left)?),
                operator,
                Box::new(parse_expr(line, right)?),
                span(line, 1),
            ));
        }
    }
    parse_atom(line, source)
}

fn parse_atom(line: &SourceLine, source: &str) -> Result<Expr, Diagnostic> {
    let source = source.trim();
    if source.starts_with('(') {
        if let Some(close) = matching_paren(source, 0) {
            let suffix = source[close + 1..].trim();
            if suffix.is_empty() {
                return parse_expr(line, &source[1..close]);
            }
        }
    }
    if source.starts_with('"') && source.ends_with('"') && source.len() >= 2 {
        if let Some(text) = parse_text_constant(source) {
            return Ok(Expr::Text(text));
        } else {
            return Err(diagnostic(span(line, 1), "nested quotes are unsupported"));
        }
    }
    if source.len() >= 2 {
        let mut chars = source.chars();
        if let (Some(quote), Some(value), Some(end)) = (chars.next(), chars.next(), chars.next())
            && matches!(quote, '\'' | '‘' | '’')
            && end == quote
            && chars.next().is_none()
        {
            return Ok(Expr::Character(value));
        }
    }
    if source == "صواب" {
        return Ok(Expr::Logical(true));
    }
    if source == "خطأ" {
        return Ok(Expr::Logical(false));
    }
    if matches!(source, "حدس" | "حدع") {
        return Ok(Expr::Call(source.to_owned(), Vec::new(), span(line, 1)));
    }
    if let Some((name, arguments)) = parse_call(line, source)? {
        return Ok(Expr::Call(name, arguments, span(line, 1)));
    }
    if let Some(open) = source.find('[') {
        if let Some(close) = matching_delimiter(source, open, '[', ']') {
            let name = source[..open].trim();
            if valid_name(name) {
                let indexes = split_output_parts(&source[open + 1..close])
                    .into_iter()
                    .map(|index| parse_expr(line, index.trim()))
                    .collect::<Result<Vec<_>, _>>()?;
                if indexes.is_empty() {
                    return Err(diagnostic(span(line, 1), "array reference needs an index"));
                }
                let suffix = source[close + 1..].trim();
                if !suffix.is_empty() {
                    let field_source = suffix.strip_prefix('.').ok_or_else(|| {
                        diagnostic(span(line, 1), "invalid indexed field reference")
                    })?;
                    let fields = parse_field_suffix(field_source)
                        .ok_or_else(|| diagnostic(span(line, 1), "invalid indexed field path"))?;
                    return Ok(Expr::IndexedField(
                        name.to_owned(),
                        indexes,
                        fields,
                        span(line, 1),
                    ));
                }
                return Ok(Expr::Index(name.to_owned(), indexes, span(line, 1)));
            }
        }
    }
    if let Some((name, fields)) = parse_field_path(source)
        && !fields.is_empty()
    {
        return Ok(Expr::Field(name, fields, span(line, 1)));
    }
    let normalized = normalize_digits(source);
    if let Ok(value) = normalized.parse::<i64>() {
        return Ok(Expr::Integer(value));
    }
    if normalized.contains('.') {
        if let Ok(value) = normalized.parse::<f64>()
            && value.is_finite()
        {
            return Ok(Expr::Real(value));
        }
    }
    if valid_name(source) {
        return Ok(Expr::Variable(source.to_owned(), span(line, 1)));
    }
    Err(diagnostic(
        span(line, 1),
        format!("unsupported expression `{source}`"),
    ))
}

fn parse_field_path(source: &str) -> Option<(String, Vec<String>)> {
    let parts = source.split('.').map(str::trim).collect::<Vec<_>>();
    if parts.len() < 2 || !valid_name(parts[0]) {
        return None;
    }
    Some((
        parts[0].to_owned(),
        parse_field_suffix(&source[source.find('.')? + 1..])?,
    ))
}

fn parse_field_suffix(source: &str) -> Option<Vec<String>> {
    let fields = source.split('.').map(str::trim).collect::<Vec<_>>();
    if fields.is_empty() || fields.iter().any(|field| !valid_name(field)) {
        return None;
    }
    Some(fields.into_iter().map(str::to_owned).collect())
}

fn parse_condition(line: &SourceLine, source: &str) -> Result<Condition, Diagnostic> {
    Ok(Condition::Expression(parse_expr(line, source)?))
}

fn find_top_level_operator(
    source: &str,
    operators: &[(&str, Arithmetic)],
    right_associative: bool,
) -> Option<(usize, usize, Arithmetic)> {
    let mut depth = 0usize;
    let mut quoted = false;
    let mut absolute = false;
    let mut found = None;
    let mut characters = source.char_indices().peekable();
    while let Some((index, character)) = characters.next() {
        if character == '"' && !is_escaped_quote(source, index) {
            quoted = !quoted;
            continue;
        }
        if quoted {
            continue;
        }
        if character == '|' && characters.peek().is_some_and(|(_, next)| *next == '|') {
            characters.next();
            absolute = !absolute;
            continue;
        }
        if absolute {
            continue;
        }
        match character {
            '(' => {
                depth += 1;
                continue;
            }
            ')' => {
                depth = depth.saturating_sub(1);
                continue;
            }
            _ => {}
        }
        if depth > 0 {
            continue;
        }
        for (token, operator) in operators {
            let is_word = token.chars().all(|value| value.is_alphabetic());
            if !source[index..].starts_with(token)
                || (is_word && !has_word_boundaries(source, index, token.len()))
            {
                continue;
            }
            if matches!(*operator, Arithmetic::Subtract) {
                let previous = source[..index].trim_end().chars().next_back();
                if previous.is_none_or(|value| {
                    matches!(
                        value,
                        '+' | '-' | '−' | '×' | '÷' | '\\' | '^' | '<' | '>' | '=' | 'و'
                    ) || value.is_whitespace()
                }) {
                    continue;
                }
            }
            found = Some((index, token.len(), *operator));
            if right_associative {
                return found;
            }
            break;
        }
    }
    found
}

fn has_word_boundaries(source: &str, start: usize, width: usize) -> bool {
    let before = source[..start].chars().next_back();
    let after = source[start + width..].chars().next();
    before.is_none_or(char::is_whitespace) && after.is_none_or(char::is_whitespace)
}

fn matching_paren(source: &str, open: usize) -> Option<usize> {
    matching_delimiter(source, open, '(', ')')
}

fn matching_delimiter(source: &str, open: usize, opening: char, closing: char) -> Option<usize> {
    let mut depth = 0usize;
    let mut quoted = false;
    for (index, character) in source.char_indices().skip_while(|(index, _)| *index < open) {
        match character {
            '"' if !is_escaped_quote(source, index) => quoted = !quoted,
            value if value == opening && !quoted => depth += 1,
            value if value == closing && !quoted => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn valid_identifier(source: &str) -> bool {
    const RESERVED: &[&str] = &[
        "النوع",
        "المتحول",
        "الثابت",
        "الخوارزمية",
        "التابع",
        "أجعل",
        "أكتب",
        "أقرأ",
        "أرسم",
        "ألون",
        "أنقش",
        "أنفذ",
        "أكرر",
        "إذا",
        "أرجع",
        "أنتهي",
        "أفتح",
        "أغلق",
        "أدرج",
        "بدل",
        "في",
        "عند",
        "طالما",
        "من",
        "إلى",
        "حتى",
        "وإلا",
        "صحيح",
        "حقيقي",
        "صغير",
        "منطقي",
        "حرف",
        "ملف",
        "صفيفة",
        "مركب",
        "صنف",
        "وحدة",
        "مؤشر",
        "نفي",
        "صواب",
        "خطأ",
        "سطر",
        "الورقة",
        "مرة",
        "بخطوة",
        "للقراءة",
        "للكتابة",
        "النقطة",
        "مستقيم",
        "دائرة",
        "مستطيل",
        "مثلث",
        "إهليلج",
        "مركزها",
        "قطرها",
        "مركزه",
        "قطريه",
        "رؤوسه",
        "باللون",
        "بدقة",
        "جب",
        "تجب",
        "جذر",
        "لغ",
        "عجب",
        "عتجب",
        "عظل",
        "حدس",
        "حدع",
        "جديد",
        "صحيح",
    ];
    if RESERVED.contains(&source) {
        return false;
    }
    let mut characters = source.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    is_arabic_letter(first)
        && characters.all(|character| {
            is_arabic_letter(character)
                || is_arabic_mark(character)
                || character == '_'
                || character.is_ascii_digit()
                || is_arabic_digit(character)
        })
}

fn is_arabic_letter(character: char) -> bool {
    matches!(
        character as u32,
        0x0621..=0x063A
            | 0x0641..=0x064A
            | 0x066E..=0x066F
            | 0x0671..=0x06D3
            | 0x06D5
            | 0x06EE..=0x06EF
            | 0x0750..=0x077F
            | 0x08A0..=0x08C9
    )
}

fn is_jeem_alphabetic_letter(character: char) -> bool {
    matches!(character as u32, 0x0621..=0x063A | 0x0641..=0x064A)
}

fn is_arabic_mark(character: char) -> bool {
    matches!(character as u32, 0x064B..=0x065F | 0x0670 | 0x06D6..=0x06ED)
}

fn is_arabic_digit(character: char) -> bool {
    matches!(character as u32, 0x0660..=0x0669 | 0x06F0..=0x06F9)
}

fn normalize_digits(source: &str) -> String {
    source
        .chars()
        .map(|character| match character as u32 {
            0x0660..=0x0669 => char::from(b'0' + (character as u32 - 0x0660) as u8),
            0x06F0..=0x06F9 => char::from(b'0' + (character as u32 - 0x06F0) as u8),
            _ => character,
        })
        .collect()
}

fn span(line: &SourceLine, column: usize) -> Span {
    Span {
        line: line.number,
        column,
    }
}

fn diagnostic(span: Span, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        span,
        message: message.into(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeError {
    pub span: Span,
    pub message: String,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: {}",
            self.span.line, self.span.column, self.message
        )
    }
}

fn open_file_handle(
    name: &str,
    variables: &HashMap<String, VariableCell>,
    at: Span,
) -> Result<FileHandle, RuntimeError> {
    match variables.get(name).and_then(|cell| cell.borrow().clone()) {
        Some(Value::File(Some(handle))) => Ok(handle),
        Some(Value::File(None)) => Err(runtime_error(at, format!("file `{name}` is closed"))),
        _ => Err(runtime_error(
            at,
            format!("file `{name}` has not been opened"),
        )),
    }
}

fn builtin_library(name: &str) -> Option<&'static str> {
    match name {
        "ظل" | "تظل" | "باقي" | "تدوير" => Some("رياضيات"),
        "أبجدي" | "عكس" | "طول_الكلمة" | "أنسخ_كلمة" | "نسخ_كلمة" | "أقارن_كلمتين" => {
            Some("حروف")
        }
        "أقرأ_لوحة_ألوان" | "أنتقي_اللون" => Some("ألوان"),
        "أجعل_عقدي"
        | "أجمع_عقدي"
        | "أضرب_عقدي"
        | "مرافق_عقدي"
        | "طويلة_عقدي"
        | "أرسم_عقدي"
        | "أقرأ_عقدي"
        | "أكتب_عقدي" => Some("الأعداد العقدية"),
        _ => None,
    }
}

fn character_array_text(value: &Value, span: Span) -> Result<String, RuntimeError> {
    let Value::Array(cells) = value else {
        return Err(runtime_error(span, "expected a character array"));
    };
    let mut text = String::new();
    for cell in cells {
        let Some(value) = cell.borrow().clone() else {
            return Err(runtime_error(
                span,
                "character array contains an uninitialized cell",
            ));
        };
        let Value::Character(character) = value else {
            return Err(runtime_error(span, "array is not a character array"));
        };
        if character == '\0' {
            return Ok(text);
        }
        text.push(character);
    }
    Err(runtime_error(
        span,
        "character array has no zero terminator",
    ))
}

fn write_character_array(
    target: &VariableCell,
    text: &str,
    span: Span,
) -> Result<(), RuntimeError> {
    let mut value = target.borrow_mut();
    let Some(Value::Array(cells)) = value.as_mut() else {
        return Err(runtime_error(
            span,
            "target is not an initialized character array",
        ));
    };
    let characters = text.chars().collect::<Vec<_>>();
    if characters.len() >= cells.len() {
        return Err(runtime_error(span, "word does not fit in the target array"));
    }
    for cell in cells.iter() {
        *cell.borrow_mut() = Some(Value::Character('\0'));
    }
    for (cell, character) in cells.iter().zip(characters) {
        *cell.borrow_mut() = Some(Value::Character(character));
    }
    Ok(())
}

fn complex_components(
    value: &Value,
    records: &HashMap<u16, RecordDefinition>,
    span: Span,
) -> Result<(u16, f64, f64), RuntimeError> {
    let type_id = complex_record_type_id(value, records, span)?;
    let Value::Record { fields, .. } = value else {
        unreachable!("complex_record_type_id validates record values")
    };
    let component = |name: &str| -> Result<f64, RuntimeError> {
        let value = fields
            .get(name)
            .and_then(|cell| cell.borrow().clone())
            .ok_or_else(|| {
                runtime_error(span, format!("complex field `{name}` is uninitialized"))
            })?;
        numeric_to_f64(&value)
            .ok_or_else(|| runtime_error(span, format!("complex field `{name}` must be real")))
    };
    Ok((type_id, component("ح")?, component("خ")?))
}

fn complex_record_type_id(
    value: &Value,
    records: &HashMap<u16, RecordDefinition>,
    span: Span,
) -> Result<u16, RuntimeError> {
    let Value::Record { type_id, .. } = value else {
        return Err(runtime_error(span, "expected a complex-number record"));
    };
    if records
        .get(type_id)
        .is_none_or(|record| record.name != "عقدي")
    {
        return Err(runtime_error(span, "record is not of type `عقدي`"));
    }
    Ok(*type_id)
}

fn write_record_real_fields(
    target: &VariableCell,
    type_id: u16,
    values: [(&str, f64); 2],
    span: Span,
) -> Result<(), RuntimeError> {
    let value = target.borrow();
    let Some(value) = value.as_ref() else {
        return Err(runtime_error(span, "record target is uninitialized"));
    };
    write_record_fields(
        value,
        type_id,
        values.map(|(name, value)| (name, Value::Real(value))),
        span,
    )
}

fn write_record_integer_fields(
    target: &mut Value,
    type_id: u16,
    values: [(&str, i64); 3],
    span: Span,
) -> Result<(), RuntimeError> {
    write_record_fields(
        target,
        type_id,
        values.map(|(name, value)| (name, Value::Integer(value))),
        span,
    )
}

fn write_record_fields<const N: usize>(
    target: &Value,
    type_id: u16,
    values: [(&str, Value); N],
    span: Span,
) -> Result<(), RuntimeError> {
    let Value::Record {
        type_id: actual_type,
        fields,
    } = target
    else {
        return Err(runtime_error(span, "target is not a record"));
    };
    if *actual_type != type_id {
        return Err(runtime_error(span, "record type does not match target"));
    }
    for (name, value) in values {
        let cell = fields
            .get(name)
            .ok_or_else(|| runtime_error(span, format!("record has no field `{name}`")))?;
        *cell.borrow_mut() = Some(value);
    }
    Ok(())
}

pub trait Host {
    fn input(&mut self) -> Result<String, String>;
    fn output(&mut self, text: &str) -> Result<(), String>;

    fn draw_circle(&mut self, _circle: Circle) -> Result<(), String> {
        Err("drawing service is unavailable".to_owned())
    }

    fn draw_command(&mut self, command: DrawingCommand) -> Result<(), String> {
        if let DrawingCommand::Circle { center, diameter } = command {
            self.draw_circle(Circle {
                center_x: center.x,
                center_y: center.y,
                diameter,
            })
        } else {
            Err("drawing service is unavailable".to_owned())
        }
    }

    fn open_file(&mut self, _path: &str, _mode: FileMode) -> Result<FileHandle, String> {
        Err("file service is unavailable".to_owned())
    }

    fn read_file_value(&mut self, _handle: FileHandle) -> Result<String, String> {
        Err("file-reading service is unavailable".to_owned())
    }

    fn write_file_text(&mut self, _handle: FileHandle, _text: &str) -> Result<(), String> {
        Err("file-writing service is unavailable".to_owned())
    }

    fn close_file(&mut self, _handle: FileHandle) -> Result<(), String> {
        Err("file service is unavailable".to_owned())
    }

    fn drawing_bounds(&mut self) -> Result<(i64, i64), String> {
        Err("drawing surface dimensions are unavailable".to_owned())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgb {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DrawingCommand {
    Point(Point),
    Line(Vec<Point>),
    Circle {
        center: Point,
        diameter: f64,
    },
    Ellipse {
        center: Point,
        diameters: Point,
    },
    Rectangle {
        from: Point,
        to: Point,
    },
    Triangle([Point; 3]),
    SetStrokeColor(Rgb),
    SetFillColor(Rgb),
    FillCircle {
        center: Point,
        diameter: f64,
    },
    FillEllipse {
        center: Point,
        diameters: Point,
    },
    FillRectangle {
        from: Point,
        to: Point,
    },
    FillTriangle([Point; 3]),
    FillCanvas(Option<Rgb>),
    Label {
        position: Option<Point>,
        values: Vec<String>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Circle {
    pub center_x: f64,
    pub center_y: f64,
    pub diameter: f64,
}

#[derive(Clone, Debug)]
pub struct Execution {
    pub variables: HashMap<String, Value>,
    pub executed_steps: u64,
}

pub struct Interpreter<H: Host> {
    host: H,
    step_limit: u64,
    call_depth: usize,
    active_arrays: HashMap<String, ArraySpec>,
    active_records: HashMap<u16, RecordDefinition>,
    active_libraries: HashSet<String>,
}

impl<H: Host> Interpreter<H> {
    pub fn new(host: H) -> Self {
        Self {
            host,
            step_limit: 1_000_000,
            call_depth: 0,
            active_arrays: HashMap::new(),
            active_records: HashMap::new(),
            active_libraries: HashSet::new(),
        }
    }

    pub fn with_step_limit(mut self, step_limit: u64) -> Self {
        self.step_limit = step_limit.max(1);
        self
    }

    pub fn run(mut self, program: &Program) -> Result<Execution, RuntimeError> {
        let mut variables = HashMap::new();
        self.active_arrays = program.arrays.clone();
        self.active_records = program.record_types.clone();
        self.active_libraries = program.libraries.clone();
        for (name, spec) in &program.arrays {
            variables.insert(
                name.clone(),
                Rc::new(RefCell::new(Some(make_array_value(spec)))),
            );
        }
        for (name, (ty, _)) in &program.declarations {
            if let Type::Record(type_id) = ty {
                variables.insert(
                    name.clone(),
                    Rc::new(RefCell::new(Some(make_record_value(
                        *type_id,
                        &self.active_records,
                    )))),
                );
            }
        }
        let mut executed_steps = 0;
        let mut control = ControlFlow::Continue;
        self.execute_block(
            &program.statements,
            &program.declarations,
            &program.constants,
            &program.routines,
            &mut variables,
            &mut executed_steps,
            &mut control,
        )?;
        if matches!(control, ControlFlow::Return(_)) {
            return Err(runtime_error(Span::default(), "return outside a routine"));
        }
        Ok(Execution {
            variables: variables
                .into_iter()
                .filter_map(|(name, cell)| cell.borrow().clone().map(|value| (name, value)))
                .collect(),
            executed_steps,
        })
    }

    fn execute_block(
        &mut self,
        statements: &[Stmt],
        declarations: &HashMap<String, (Type, Span)>,
        constants: &HashMap<String, Value>,
        routines: &HashMap<String, Routine>,
        variables: &mut HashMap<String, VariableCell>,
        executed_steps: &mut u64,
        control: &mut ControlFlow,
    ) -> Result<(), RuntimeError> {
        for statement in statements {
            if !matches!(control, ControlFlow::Continue) {
                break;
            }
            let at = *statement_span(statement);
            self.tick(*executed_steps, at)?;
            *executed_steps += 1;
            match statement {
                Stmt::Assign { target, value, .. } => {
                    if matches!(target, LValue::Variable { name, .. } if constants.contains_key(name))
                    {
                        return Err(runtime_error(at, "cannot assign to a constant"));
                    }
                    let value = self.eval(
                        value,
                        declarations,
                        variables,
                        constants,
                        routines,
                        executed_steps,
                        control,
                    )?;
                    let ty = runtime_lvalue_type(target, declarations, &self.active_records)?;
                    if array_element_type(ty).is_some() {
                        return Err(runtime_error(at, "whole-array assignment is not defined"));
                    }
                    let value = coerce_assignment(value, ty).ok_or_else(|| {
                        runtime_error(at, "assigned value has an incompatible type")
                    })?;
                    let cell = self.lvalue_cell(
                        target,
                        declarations,
                        variables,
                        constants,
                        routines,
                        executed_steps,
                        control,
                    )?;
                    *cell.borrow_mut() = Some(value);
                }
                Stmt::Read { targets, span } => {
                    for target in targets {
                        let input = self.host.input().map_err(|message| RuntimeError {
                            span: *span,
                            message: format!("input failed: {message}"),
                        })?;
                        if let LValue::Variable { name, .. } = target
                            && let Some(spec) = self.active_arrays.get(name)
                        {
                            let Value::Array(cells) = variables
                                .get(name)
                                .and_then(|cell| cell.borrow().clone())
                                .ok_or_else(|| runtime_error(*span, "array is unavailable"))?
                            else {
                                return Err(runtime_error(*span, "array storage is invalid"));
                            };
                            read_character_array(&cells, spec, &input)
                                .map_err(|message| runtime_error(*span, message))?;
                            continue;
                        }
                        let ty = runtime_lvalue_type(target, declarations, &self.active_records)?;
                        let value = parse_input_value(ty, &input)
                            .map_err(|message| runtime_error(*span, message))?;
                        let cell = self.lvalue_cell(
                            target,
                            declarations,
                            variables,
                            constants,
                            routines,
                            executed_steps,
                            control,
                        )?;
                        *cell.borrow_mut() = Some(value);
                    }
                }
                Stmt::Write { parts, span } => {
                    for part in parts {
                        let text = match part {
                            OutputPart::Text(text) => text.clone(),
                            OutputPart::Expression(expr) => self
                                .eval(
                                    expr,
                                    declarations,
                                    variables,
                                    constants,
                                    routines,
                                    executed_steps,
                                    control,
                                )?
                                .to_string(),
                            OutputPart::Newline => "\n".to_owned(),
                        };
                        self.host.output(&text).map_err(|message| RuntimeError {
                            span: *span,
                            message: format!("output failed: {message}"),
                        })?;
                    }
                }
                Stmt::While {
                    condition,
                    body,
                    span,
                } => loop {
                    if !matches!(control, ControlFlow::Continue) {
                        break;
                    }
                    self.tick(*executed_steps, *span)?;
                    *executed_steps += 1;
                    if !self.eval_condition(
                        condition,
                        declarations,
                        variables,
                        constants,
                        routines,
                        executed_steps,
                        control,
                    )? {
                        break;
                    }
                    self.execute_block(
                        body,
                        declarations,
                        constants,
                        routines,
                        variables,
                        executed_steps,
                        control,
                    )?;
                },
                Stmt::RepeatUntil {
                    condition,
                    body,
                    span,
                } => loop {
                    if !matches!(control, ControlFlow::Continue) {
                        break;
                    }
                    self.tick(*executed_steps, *span)?;
                    *executed_steps += 1;
                    self.execute_block(
                        body,
                        declarations,
                        constants,
                        routines,
                        variables,
                        executed_steps,
                        control,
                    )?;
                    if !matches!(control, ControlFlow::Continue)
                        || self.eval_condition(
                            condition,
                            declarations,
                            variables,
                            constants,
                            routines,
                            executed_steps,
                            control,
                        )?
                    {
                        break;
                    }
                },
                Stmt::Counted { count, body, span } => {
                    let count = self.eval(
                        count,
                        declarations,
                        variables,
                        constants,
                        routines,
                        executed_steps,
                        control,
                    )?;
                    let Value::Integer(count) = count else {
                        return Err(runtime_error(*span, "counted loop requires an integer"));
                    };
                    for _ in 0..count.max(0) {
                        if !matches!(control, ControlFlow::Continue) {
                            break;
                        }
                        self.tick(*executed_steps, *span)?;
                        *executed_steps += 1;
                        self.execute_block(
                            body,
                            declarations,
                            constants,
                            routines,
                            variables,
                            executed_steps,
                            control,
                        )?;
                    }
                }
                Stmt::For {
                    name,
                    start,
                    end,
                    step,
                    body,
                    span,
                } => {
                    let start = self.eval(
                        start,
                        declarations,
                        variables,
                        constants,
                        routines,
                        executed_steps,
                        control,
                    )?;
                    let end = self.eval(
                        end,
                        declarations,
                        variables,
                        constants,
                        routines,
                        executed_steps,
                        control,
                    )?;
                    let step = match step {
                        Some(expr) => self.eval(
                            expr,
                            declarations,
                            variables,
                            constants,
                            routines,
                            executed_steps,
                            control,
                        )?,
                        None => Value::Integer(1),
                    };
                    let (Value::Integer(mut current), Value::Integer(end), Value::Integer(step)) =
                        (start, end, step)
                    else {
                        return Err(runtime_error(*span, "range values must be integers"));
                    };
                    if step == 0 {
                        return Err(runtime_error(*span, "range step cannot be zero"));
                    }
                    loop {
                        if !matches!(control, ControlFlow::Continue)
                            || if step > 0 {
                                current > end
                            } else {
                                current < end
                            }
                        {
                            break;
                        }
                        self.tick(*executed_steps, *span)?;
                        *executed_steps += 1;
                        let ty = declarations.get(name).map(|(ty, _)| *ty).ok_or_else(|| {
                            runtime_error(*span, format!("undeclared loop variable {name}"))
                        })?;
                        let value = coerce_assignment(Value::Integer(current), ty)
                            .ok_or_else(|| runtime_error(*span, "invalid loop variable type"))?;
                        set_variable(variables, name, value);
                        self.execute_block(
                            body,
                            declarations,
                            constants,
                            routines,
                            variables,
                            executed_steps,
                            control,
                        )?;
                        if current == end {
                            break;
                        }
                        current = current
                            .checked_add(step)
                            .ok_or_else(|| runtime_error(*span, "range value overflow"))?;
                    }
                }
                Stmt::Call {
                    name,
                    arguments,
                    span,
                } => {
                    let _ = self.call_routine(
                        name,
                        arguments,
                        *span,
                        declarations,
                        constants,
                        routines,
                        variables,
                        executed_steps,
                        control,
                    )?;
                }
                Stmt::Return { value, .. } => {
                    let value = value
                        .as_ref()
                        .map(|expr| {
                            self.eval(
                                expr,
                                declarations,
                                variables,
                                constants,
                                routines,
                                executed_steps,
                                control,
                            )
                        })
                        .transpose()?;
                    if matches!(control, ControlFlow::Continue) {
                        *control = ControlFlow::Return(value);
                    }
                }
                Stmt::Terminate { .. } => *control = ControlFlow::Halt,
                Stmt::If {
                    condition,
                    then_body,
                    else_body,
                    ..
                } => {
                    let body = if self.eval_condition(
                        condition,
                        declarations,
                        variables,
                        constants,
                        routines,
                        executed_steps,
                        control,
                    )? {
                        then_body
                    } else {
                        else_body
                    };
                    self.execute_block(
                        body,
                        declarations,
                        constants,
                        routines,
                        variables,
                        executed_steps,
                        control,
                    )?;
                }
                Stmt::Draw { geometry, span } => {
                    let command = self.drawing_command(
                        geometry,
                        *span,
                        declarations,
                        variables,
                        constants,
                        routines,
                        executed_steps,
                        control,
                    )?;
                    self.host
                        .draw_command(command)
                        .map_err(|message| RuntimeError {
                            span: *span,
                            message: format!("drawing failed: {message}"),
                        })?;
                }
                Stmt::OpenFile {
                    name,
                    path,
                    mode,
                    span,
                } => {
                    if declarations.get(name).map(|(ty, _)| *ty) != Some(Type::File) {
                        return Err(runtime_error(
                            *span,
                            format!("`{name}` is not a file variable"),
                        ));
                    }
                    let path = match self.eval(
                        path,
                        declarations,
                        variables,
                        constants,
                        routines,
                        executed_steps,
                        control,
                    )? {
                        Value::Text(path) => path,
                        _ => return Err(runtime_error(*span, "file path must be text")),
                    };
                    if matches!(
                        variables.get(name).and_then(|cell| cell.borrow().clone()),
                        Some(Value::File(Some(_)))
                    ) {
                        return Err(runtime_error(
                            *span,
                            format!("file `{name}` is already open"),
                        ));
                    }
                    let handle =
                        self.host
                            .open_file(&path, *mode)
                            .map_err(|message| RuntimeError {
                                span: *span,
                                message: format!("opening file failed: {message}"),
                            })?;
                    set_variable(variables, name, Value::File(Some(handle)));
                }
                Stmt::ReadFile {
                    name,
                    targets,
                    span,
                } => {
                    let handle = open_file_handle(name, variables, *span)?;
                    for target in targets {
                        let ty = runtime_lvalue_type(target, declarations, &self.active_records)?;
                        let input =
                            self.host
                                .read_file_value(handle)
                                .map_err(|message| RuntimeError {
                                    span: *span,
                                    message: format!("reading file failed: {message}"),
                                })?;
                        if let LValue::Variable {
                            name: target_name, ..
                        } = target
                            && let Some(spec) = self.active_arrays.get(target_name)
                        {
                            let Value::Array(cells) = variables
                                .get(target_name)
                                .and_then(|cell| cell.borrow().clone())
                                .ok_or_else(|| runtime_error(*span, "array is unavailable"))?
                            else {
                                return Err(runtime_error(*span, "array storage is invalid"));
                            };
                            read_character_array(&cells, spec, &input)
                                .map_err(|message| runtime_error(*span, message))?;
                            continue;
                        }
                        let value = parse_input_value(ty, &input)
                            .map_err(|message| runtime_error(*span, message))?;
                        let cell = self.lvalue_cell(
                            target,
                            declarations,
                            variables,
                            constants,
                            routines,
                            executed_steps,
                            control,
                        )?;
                        *cell.borrow_mut() = Some(value);
                    }
                }
                Stmt::WriteFile { name, parts, span } => {
                    let handle = open_file_handle(name, variables, *span)?;
                    for part in parts {
                        let text = match part {
                            OutputPart::Text(text) => text.clone(),
                            OutputPart::Expression(expression) => self
                                .eval(
                                    expression,
                                    declarations,
                                    variables,
                                    constants,
                                    routines,
                                    executed_steps,
                                    control,
                                )?
                                .to_string(),
                            OutputPart::Newline => "\n".to_owned(),
                        };
                        self.host
                            .write_file_text(handle, &text)
                            .map_err(|message| RuntimeError {
                                span: *span,
                                message: format!("writing file failed: {message}"),
                            })?;
                    }
                }
                Stmt::CloseFile { name, span } => {
                    let handle = open_file_handle(name, variables, *span)?;
                    self.host
                        .close_file(handle)
                        .map_err(|message| RuntimeError {
                            span: *span,
                            message: format!("closing file failed: {message}"),
                        })?;
                    set_variable(variables, name, Value::File(None));
                }
            }
        }
        Ok(())
    }

    fn lvalue_cell(
        &mut self,
        target: &LValue,
        declarations: &HashMap<String, (Type, Span)>,
        variables: &mut HashMap<String, VariableCell>,
        constants: &HashMap<String, Value>,
        routines: &HashMap<String, Routine>,
        executed_steps: &mut u64,
        control: &mut ControlFlow,
    ) -> Result<VariableCell, RuntimeError> {
        match target {
            LValue::Variable { name, span } => {
                if !declarations.contains_key(name) {
                    return Err(runtime_error(*span, format!("undeclared variable {name}")));
                }
                Ok(variables
                    .entry(name.clone())
                    .or_insert_with(|| Rc::new(RefCell::new(None)))
                    .clone())
            }
            LValue::Index {
                name,
                indexes,
                span,
            } => {
                if constants.contains_key(name) {
                    return Err(runtime_error(*span, "constant array cannot be changed"));
                }
                let mut values = Vec::with_capacity(indexes.len());
                for index in indexes {
                    match self.eval(
                        index,
                        declarations,
                        variables,
                        constants,
                        routines,
                        executed_steps,
                        control,
                    )? {
                        Value::Integer(value) => values.push(value),
                        _ => return Err(runtime_error(*span, "array index must be an integer")),
                    }
                }
                array_element_cell(
                    name,
                    &values,
                    &self.active_arrays,
                    &self.active_records,
                    variables,
                    *span,
                )
            }
            LValue::IndexedField {
                name,
                indexes,
                fields,
                span,
            } => {
                if constants.contains_key(name) {
                    return Err(runtime_error(*span, "constant array cannot be changed"));
                }
                let mut values = Vec::with_capacity(indexes.len());
                for index in indexes {
                    match self.eval(
                        index,
                        declarations,
                        variables,
                        constants,
                        routines,
                        executed_steps,
                        control,
                    )? {
                        Value::Integer(value) => values.push(value),
                        _ => return Err(runtime_error(*span, "array index must be an integer")),
                    }
                }
                let element = array_element_cell(
                    name,
                    &values,
                    &self.active_arrays,
                    &self.active_records,
                    variables,
                    *span,
                )?;
                record_field_cell_from_cell(&element, fields, *span)
            }
            LValue::Field { name, fields, span } => {
                if constants.contains_key(name) {
                    return Err(runtime_error(*span, "constant record cannot be changed"));
                }
                if !declarations.contains_key(name) {
                    return Err(runtime_error(*span, format!("undeclared record `{name}`")));
                }
                record_field_cell(name, fields, variables, *span)
            }
        }
    }

    fn call_builtin(
        &mut self,
        name: &str,
        arguments: &[Expr],
        call_span: Span,
        caller_declarations: &HashMap<String, (Type, Span)>,
        caller_constants: &HashMap<String, Value>,
        routines: &HashMap<String, Routine>,
        variables: &mut HashMap<String, VariableCell>,
        executed_steps: &mut u64,
        caller_control: &mut ControlFlow,
    ) -> Result<Option<Value>, RuntimeError> {
        if let Some(library) = builtin_library(name)
            && !self.active_libraries.contains(library)
        {
            return Err(runtime_error(
                call_span,
                format!("`{name}` requires `أدرج \"{library}\"`"),
            ));
        }
        if name == "حدس" || name == "حدع" {
            if !arguments.is_empty() {
                return Err(runtime_error(
                    call_span,
                    format!("`{name}` takes no arguments"),
                ));
            }
            let (width, height) = self
                .host
                .drawing_bounds()
                .map_err(|message| runtime_error(call_span, message))?;
            return Ok(Some(Value::Integer(if name == "حدس" {
                width
            } else {
                height
            })));
        }
        if matches!(name, "أنسخ_كلمة" | "نسخ_كلمة") {
            self.check_runtime_arity(name, arguments, 2, call_span)?;
            let source = self.eval(
                &arguments[0],
                caller_declarations,
                variables,
                caller_constants,
                routines,
                executed_steps,
                caller_control,
            )?;
            let text = character_array_text(&source, call_span)?;
            let target = self.argument_cell(
                &arguments[1],
                call_span,
                caller_declarations,
                caller_constants,
                routines,
                variables,
                executed_steps,
                caller_control,
            )?;
            write_character_array(&target, &text, call_span)?;
            return Ok(None);
        }
        if name == "طول_الكلمة" {
            self.check_runtime_arity(name, arguments, 1, call_span)?;
            let value = self.eval(
                &arguments[0],
                caller_declarations,
                variables,
                caller_constants,
                routines,
                executed_steps,
                caller_control,
            )?;
            let length = character_array_text(&value, call_span)?.chars().count();
            let length =
                i64::try_from(length).map_err(|_| runtime_error(call_span, "word is too long"))?;
            return Ok(Some(Value::Integer(length)));
        }
        if name == "أقارن_كلمتين" {
            self.check_runtime_arity(name, arguments, 2, call_span)?;
            let values = self.evaluate_arguments(
                arguments,
                caller_declarations,
                caller_constants,
                routines,
                variables,
                executed_steps,
                caller_control,
            )?;
            let left = character_array_text(&values[0], call_span)?;
            let right = character_array_text(&values[1], call_span)?;
            let comparison = match left.chars().cmp(right.chars()) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            };
            return Ok(Some(Value::Integer(comparison)));
        }
        if name == "أقرأ_لوحة_ألوان" {
            self.check_runtime_arity(name, arguments, 2, call_span)?;
            let Expr::Variable(array_name, _) = &arguments[0] else {
                return Err(runtime_error(
                    call_span,
                    "palette output needs a declared array variable",
                ));
            };
            let path_value = self.eval(
                &arguments[1],
                caller_declarations,
                variables,
                caller_constants,
                routines,
                executed_steps,
                caller_control,
            )?;
            let path = character_array_text(&path_value, call_span)?;
            let spec =
                self.active_arrays.get(array_name).cloned().ok_or_else(|| {
                    runtime_error(call_span, "palette array shape is unavailable")
                })?;
            let Type::Record(color_type_id) = spec.element_type else {
                return Err(runtime_error(
                    call_span,
                    "palette array element type is invalid",
                ));
            };
            if self
                .active_records
                .get(&color_type_id)
                .is_none_or(|record| record.name != "لون")
            {
                return Err(runtime_error(
                    call_span,
                    "palette array must contain `لون` values",
                ));
            }
            let cells = match variables
                .get(array_name)
                .and_then(|cell| cell.borrow().clone())
            {
                Some(Value::Array(cells)) => cells,
                _ => return Err(runtime_error(call_span, "palette array storage is invalid")),
            };
            let handle = match self.host.open_file(&path, FileMode::Read) {
                Ok(handle) => handle,
                Err(_) => {
                    self.host
                        .output(" لم أتمكن من فتح ملف الألوان !\n")
                        .map_err(|message| {
                            runtime_error(call_span, format!("output failed: {message}"))
                        })?;
                    return Ok(Some(Value::Integer(0)));
                }
            };
            let read_result = (|| {
                let count = self.read_file_integer(handle, call_span)?;
                let count = usize::try_from(count)
                    .ok()
                    .filter(|count| *count <= cells.len())
                    .ok_or_else(|| {
                        runtime_error(call_span, "palette color count is out of range")
                    })?;
                for element in cells.iter().take(count) {
                    let red = self.read_file_integer(handle, call_span)?;
                    let green = self.read_file_integer(handle, call_span)?;
                    let blue = self.read_file_integer(handle, call_span)?;
                    let mut record = make_record_value(color_type_id, &self.active_records);
                    write_record_integer_fields(
                        &mut record,
                        color_type_id,
                        [("ح", red), ("خ", green), ("ز", blue)],
                        call_span,
                    )?;
                    *element.borrow_mut() = Some(record);
                }
                Ok(Value::Integer(count as i64))
            })();
            let close_result = self.host.close_file(handle);
            return match (read_result, close_result) {
                (Err(error), _) => Err(error),
                (Ok(_), Err(message)) => Err(runtime_error(
                    call_span,
                    format!("closing palette failed: {message}"),
                )),
                (Ok(value), Ok(())) => Ok(Some(value)),
            };
        }
        if name == "أنتقي_اللون" {
            self.check_runtime_arity(name, arguments, 1, call_span)?;
            let values = self.evaluate_arguments(
                arguments,
                caller_declarations,
                caller_constants,
                routines,
                variables,
                executed_steps,
                caller_control,
            )?;
            let Value::Integer(mut palette_index) = values[0] else {
                return Err(runtime_error(call_span, "`أنتقي_اللون` needs an integer"));
            };
            let mut channels = [0.0; 3];
            for channel in &mut channels {
                let quotient = palette_index / 2;
                *channel = (palette_index - quotient * 2) as f64 * 100.0;
                palette_index = quotient;
            }
            let color = Rgb {
                red: channels[0],
                green: channels[1],
                blue: channels[2],
            };
            self.host
                .draw_command(DrawingCommand::SetStrokeColor(color))
                .map_err(|message| {
                    runtime_error(call_span, format!("drawing failed: {message}"))
                })?;
            self.host
                .draw_command(DrawingCommand::SetFillColor(color))
                .map_err(|message| {
                    runtime_error(call_span, format!("drawing failed: {message}"))
                })?;
            return Ok(None);
        }
        if name == "أجعل_عقدي" {
            self.check_runtime_arity(name, arguments, 3, call_span)?;
            let values = self.evaluate_arguments(
                arguments,
                caller_declarations,
                caller_constants,
                routines,
                variables,
                executed_steps,
                caller_control,
            )?;
            let record_type_id =
                complex_record_type_id(&values[0], &self.active_records, call_span)?;
            let real = numeric_to_f64(&values[1])
                .ok_or_else(|| runtime_error(call_span, "real component must be numeric"))?;
            let imaginary = numeric_to_f64(&values[2])
                .ok_or_else(|| runtime_error(call_span, "imaginary component must be numeric"))?;
            let target = self.argument_cell(
                &arguments[0],
                call_span,
                caller_declarations,
                caller_constants,
                routines,
                variables,
                executed_steps,
                caller_control,
            )?;
            write_record_real_fields(
                &target,
                record_type_id,
                [("ح", real), ("خ", imaginary)],
                call_span,
            )?;
            return Ok(None);
        }
        if matches!(name, "أجمع_عقدي" | "أضرب_عقدي" | "مرافق_عقدي") {
            let arity = if name == "مرافق_عقدي" { 2 } else { 3 };
            self.check_runtime_arity(name, arguments, arity, call_span)?;
            let values = self.evaluate_arguments(
                arguments,
                caller_declarations,
                caller_constants,
                routines,
                variables,
                executed_steps,
                caller_control,
            )?;
            let (_, left_real, left_imaginary) =
                complex_components(&values[0], &self.active_records, call_span)?;
            let (result_real, result_imaginary) = if name == "مرافق_عقدي" {
                (left_real, -left_imaginary)
            } else {
                let (_, right_real, right_imaginary) =
                    complex_components(&values[1], &self.active_records, call_span)?;
                if name == "أجمع_عقدي" {
                    (left_real + right_real, left_imaginary + right_imaginary)
                } else {
                    (
                        left_real * right_real - left_imaginary * right_imaginary,
                        left_real * right_imaginary + left_imaginary * right_real,
                    )
                }
            };
            let result_index = arity - 1;
            let result_type_id =
                complex_record_type_id(&values[result_index], &self.active_records, call_span)?;
            let target = self.argument_cell(
                &arguments[result_index],
                call_span,
                caller_declarations,
                caller_constants,
                routines,
                variables,
                executed_steps,
                caller_control,
            )?;
            write_record_real_fields(
                &target,
                result_type_id,
                [("ح", result_real), ("خ", result_imaginary)],
                call_span,
            )?;
            return Ok(None);
        }
        if name == "أقرأ_عقدي" {
            self.check_runtime_arity(name, arguments, 1, call_span)?;
            let target = self.argument_cell(
                &arguments[0],
                call_span,
                caller_declarations,
                caller_constants,
                routines,
                variables,
                executed_steps,
                caller_control,
            )?;
            let value = target
                .borrow()
                .clone()
                .ok_or_else(|| runtime_error(call_span, "complex value is uninitialized"))?;
            let record_type_id = complex_record_type_id(&value, &self.active_records, call_span)?;
            let real_input = self
                .host
                .input()
                .map_err(|message| runtime_error(call_span, format!("input failed: {message}")))?;
            let real = parse_input_value(Type::Real, &real_input)
                .map_err(|message| runtime_error(call_span, message))?;
            let imaginary_input = self
                .host
                .input()
                .map_err(|message| runtime_error(call_span, format!("input failed: {message}")))?;
            let imaginary = parse_input_value(Type::Real, &imaginary_input)
                .map_err(|message| runtime_error(call_span, message))?;
            let (Value::Real(real), Value::Real(imaginary)) = (real, imaginary) else {
                return Err(runtime_error(call_span, "complex input must be numeric"));
            };
            write_record_real_fields(
                &target,
                record_type_id,
                [("ح", real), ("خ", imaginary)],
                call_span,
            )?;
            return Ok(None);
        }
        let values = self.evaluate_arguments(
            arguments,
            caller_declarations,
            caller_constants,
            routines,
            variables,
            executed_steps,
            caller_control,
        )?;
        if name == "طويلة_عقدي" {
            let (_, real, imaginary) =
                complex_components(&values[0], &self.active_records, call_span)?;
            let value = (real * real + imaginary * imaginary).sqrt();
            return value
                .is_finite()
                .then_some(Some(Value::Real(value)))
                .ok_or_else(|| runtime_error(call_span, "complex magnitude is not finite"));
        }
        if name == "أرسم_عقدي" {
            let (_, real, imaginary) =
                complex_components(&values[0], &self.active_records, call_span)?;
            let (width, height) = self
                .host
                .drawing_bounds()
                .map_err(|message| runtime_error(call_span, message))?;
            self.host
                .draw_command(DrawingCommand::Line(vec![
                    Point {
                        x: (width / 2) as f64,
                        y: (height / 2) as f64,
                    },
                    Point {
                        x: (width / 2) as f64 + real,
                        y: (height / 2) as f64 + imaginary,
                    },
                ]))
                .map_err(|message| {
                    runtime_error(call_span, format!("drawing failed: {message}"))
                })?;
            return Ok(None);
        }
        if name == "أكتب_عقدي" {
            let (_, real, imaginary) =
                complex_components(&values[0], &self.active_records, call_span)?;
            self.host
                .output(&Value::Real(real).to_string())
                .map_err(|message| runtime_error(call_span, format!("output failed: {message}")))?;
            if imaginary > 0.0 {
                self.host.output(" + ").map_err(|message| {
                    runtime_error(call_span, format!("output failed: {message}"))
                })?;
            }
            self.host
                .output(&format!("{} ت", Value::Real(imaginary)))
                .map_err(|message| runtime_error(call_span, format!("output failed: {message}")))?;
            return Ok(None);
        }
        let value = evaluate_builtin(name, &values, call_span)?;
        Ok(Some(value))
    }

    fn check_runtime_arity(
        &self,
        name: &str,
        arguments: &[Expr],
        expected: usize,
        span: Span,
    ) -> Result<(), RuntimeError> {
        if arguments.len() == expected {
            Ok(())
        } else {
            Err(runtime_error(
                span,
                format!("`{name}` expects {expected} arguments"),
            ))
        }
    }

    fn evaluate_arguments(
        &mut self,
        arguments: &[Expr],
        declarations: &HashMap<String, (Type, Span)>,
        constants: &HashMap<String, Value>,
        routines: &HashMap<String, Routine>,
        variables: &mut HashMap<String, VariableCell>,
        executed_steps: &mut u64,
        control: &mut ControlFlow,
    ) -> Result<Vec<Value>, RuntimeError> {
        arguments
            .iter()
            .map(|argument| {
                self.eval(
                    argument,
                    declarations,
                    variables,
                    constants,
                    routines,
                    executed_steps,
                    control,
                )
            })
            .collect()
    }

    fn argument_cell(
        &mut self,
        argument: &Expr,
        span: Span,
        declarations: &HashMap<String, (Type, Span)>,
        constants: &HashMap<String, Value>,
        routines: &HashMap<String, Routine>,
        variables: &mut HashMap<String, VariableCell>,
        executed_steps: &mut u64,
        control: &mut ControlFlow,
    ) -> Result<VariableCell, RuntimeError> {
        let target = match argument {
            Expr::Variable(name, _) => LValue::Variable {
                name: name.clone(),
                span,
            },
            Expr::Index(name, indexes, _) => LValue::Index {
                name: name.clone(),
                indexes: indexes.clone(),
                span,
            },
            Expr::IndexedField(name, indexes, fields, _) => LValue::IndexedField {
                name: name.clone(),
                indexes: indexes.clone(),
                fields: fields.clone(),
                span,
            },
            Expr::Field(name, fields, _) => LValue::Field {
                name: name.clone(),
                fields: fields.clone(),
                span,
            },
            _ => return Err(runtime_error(span, "argument must be writable")),
        };
        self.lvalue_cell(
            &target,
            declarations,
            variables,
            constants,
            routines,
            executed_steps,
            control,
        )
    }

    fn read_file_integer(&mut self, handle: FileHandle, span: Span) -> Result<i64, RuntimeError> {
        let input = self
            .host
            .read_file_value(handle)
            .map_err(|message| runtime_error(span, format!("reading palette failed: {message}")))?;
        match parse_input_value(Type::Integer, &input)
            .map_err(|message| runtime_error(span, message))?
        {
            Value::Integer(value) => Ok(value),
            _ => Err(runtime_error(span, "palette file value must be an integer")),
        }
    }

    fn call_routine(
        &mut self,
        name: &str,
        arguments: &[Expr],
        call_span: Span,
        caller_declarations: &HashMap<String, (Type, Span)>,
        caller_constants: &HashMap<String, Value>,
        routines: &HashMap<String, Routine>,
        variables: &mut HashMap<String, VariableCell>,
        executed_steps: &mut u64,
        caller_control: &mut ControlFlow,
    ) -> Result<Option<Value>, RuntimeError> {
        let Some(routine) = routines.get(name).cloned() else {
            if !is_builtin_name(name) {
                return Err(runtime_error(call_span, format!("unknown routine {name}")));
            }
            return self.call_builtin(
                name,
                arguments,
                call_span,
                caller_declarations,
                caller_constants,
                routines,
                variables,
                executed_steps,
                caller_control,
            );
        };
        if arguments.len() != routine.parameters.len() {
            return Err(runtime_error(
                call_span,
                format!("wrong argument count for {name}"),
            ));
        }
        if self.call_depth >= 256 {
            return Err(runtime_error(
                call_span,
                "routine call-depth limit exceeded",
            ));
        }
        enum Binding {
            Reference(VariableCell),
            Value(Value),
            ArrayReference(VariableCell, ArraySpec),
            ArrayValue(Value, ArraySpec),
        }
        let mut bindings = Vec::with_capacity(arguments.len());
        for (argument, parameter) in arguments.iter().zip(&routine.parameters) {
            if array_element_type(parameter.ty).is_some() {
                if let Expr::Variable(actual_name, at) = argument
                    && caller_declarations
                        .get(actual_name)
                        .is_some_and(|(actual_type, _)| *actual_type == parameter.ty)
                {
                    let spec = self
                        .active_arrays
                        .get(actual_name)
                        .cloned()
                        .ok_or_else(|| runtime_error(*at, "array shape is unavailable"))?;
                    let cell = variables
                        .entry(actual_name.clone())
                        .or_insert_with(|| Rc::new(RefCell::new(Some(make_array_value(&spec)))))
                        .clone();
                    let value = cell.borrow().clone();
                    if !matches!(&value, Some(Value::Array(_))) {
                        return Err(runtime_error(*at, "array storage is invalid"));
                    }
                    if parameter.constant {
                        let Some(value) = value else {
                            return Err(runtime_error(*at, "array storage is invalid"));
                        };
                        let value = copy_array_value(value, *at)?;
                        bindings.push(Binding::ArrayValue(value, spec));
                    } else {
                        bindings.push(Binding::ArrayReference(cell, spec));
                    }
                } else {
                    let value = self.eval(
                        argument,
                        caller_declarations,
                        variables,
                        caller_constants,
                        routines,
                        executed_steps,
                        caller_control,
                    )?;
                    let (value, spec) = match (value, parameter.ty) {
                        (Value::Text(text), Type::CharacterArray) => {
                            character_array_from_text(&text, expression_span(argument))?
                        }
                        (Value::Array(_), _) => {
                            return Err(runtime_error(
                                expression_span(argument),
                                "array parameters require a declared array variable",
                            ));
                        }
                        _ => {
                            return Err(runtime_error(
                                expression_span(argument),
                                "argument type mismatch",
                            ));
                        }
                    };
                    bindings.push(Binding::ArrayValue(value, spec));
                }
            } else if !parameter.constant {
                let cell = match argument {
                    Expr::Variable(actual_name, at)
                        if caller_declarations
                            .get(actual_name)
                            .is_some_and(|(actual_type, _)| *actual_type == parameter.ty) =>
                    {
                        if caller_constants.contains_key(actual_name) {
                            return Err(runtime_error(
                                *at,
                                "constant cannot be passed to a mutable parameter",
                            ));
                        }
                        variables
                            .entry(actual_name.clone())
                            .or_insert_with(|| Rc::new(RefCell::new(None)))
                            .clone()
                    }
                    Expr::Index(name, indexes, at)
                        if caller_declarations
                            .get(name)
                            .and_then(|(ty, _)| array_element_type(*ty))
                            == Some(parameter.ty) =>
                    {
                        let mut values = Vec::with_capacity(indexes.len());
                        for index in indexes {
                            match self.eval(
                                index,
                                caller_declarations,
                                variables,
                                caller_constants,
                                routines,
                                executed_steps,
                                caller_control,
                            )? {
                                Value::Integer(value) => values.push(value),
                                _ => {
                                    return Err(runtime_error(
                                        *at,
                                        "array index must be an integer",
                                    ));
                                }
                            }
                        }
                        array_element_cell(
                            name,
                            &values,
                            &self.active_arrays,
                            &self.active_records,
                            variables,
                            *at,
                        )?
                    }
                    Expr::IndexedField(name, indexes, fields, at)
                        if runtime_lvalue_type(
                            &LValue::IndexedField {
                                name: name.clone(),
                                indexes: indexes.clone(),
                                fields: fields.clone(),
                                span: *at,
                            },
                            caller_declarations,
                            &self.active_records,
                        )
                        .is_ok_and(|ty| ty == parameter.ty) =>
                    {
                        self.lvalue_cell(
                            &LValue::IndexedField {
                                name: name.clone(),
                                indexes: indexes.clone(),
                                fields: fields.clone(),
                                span: *at,
                            },
                            caller_declarations,
                            variables,
                            caller_constants,
                            routines,
                            executed_steps,
                            caller_control,
                        )?
                    }
                    Expr::Field(name, fields, at)
                        if runtime_lvalue_type(
                            &LValue::Field {
                                name: name.clone(),
                                fields: fields.clone(),
                                span: *at,
                            },
                            caller_declarations,
                            &self.active_records,
                        )
                        .is_ok_and(|ty| ty == parameter.ty) =>
                    {
                        self.lvalue_cell(
                            &LValue::Field {
                                name: name.clone(),
                                fields: fields.clone(),
                                span: *at,
                            },
                            caller_declarations,
                            variables,
                            caller_constants,
                            routines,
                            executed_steps,
                            caller_control,
                        )?
                    }
                    _ => {
                        let value = self.eval(
                            argument,
                            caller_declarations,
                            variables,
                            caller_constants,
                            routines,
                            executed_steps,
                            caller_control,
                        )?;
                        let value = coerce_parameter(value, parameter.ty)
                            .ok_or_else(|| runtime_error(call_span, "argument type mismatch"))?;
                        bindings.push(Binding::Value(value));
                        continue;
                    }
                };
                bindings.push(Binding::Reference(cell));
            } else {
                let value = self.eval(
                    argument,
                    caller_declarations,
                    variables,
                    caller_constants,
                    routines,
                    executed_steps,
                    caller_control,
                )?;
                let value = coerce_parameter(value, parameter.ty)
                    .ok_or_else(|| runtime_error(call_span, "argument type mismatch"))?;
                bindings.push(Binding::Value(value));
            }
        }
        let mut frame: HashMap<String, VariableCell> = variables
            .iter()
            .filter(|(name, _)| {
                routine.visible_globals.contains_key(*name)
                    && !routine.declarations.contains_key(*name)
            })
            .map(|(name, cell)| (name.clone(), cell.clone()))
            .collect();
        let previous_arrays =
            std::mem::replace(&mut self.active_arrays, routine.visible_arrays.clone());
        self.active_arrays.extend(routine.arrays.clone());
        for (name, spec) in &routine.arrays {
            frame.insert(
                name.clone(),
                Rc::new(RefCell::new(Some(make_array_value(spec)))),
            );
        }
        for (name, (ty, _)) in &routine.declarations {
            if let Type::Record(type_id) = ty {
                frame.insert(
                    name.clone(),
                    Rc::new(RefCell::new(Some(make_record_value(
                        *type_id,
                        &self.active_records,
                    )))),
                );
            }
        }
        let constants = routine.visible_constants.clone();
        for (parameter, binding) in routine.parameters.iter().zip(bindings) {
            match binding {
                Binding::Reference(cell) => {
                    frame.insert(parameter.name.clone(), cell);
                }
                Binding::Value(value) => {
                    frame.insert(parameter.name.clone(), Rc::new(RefCell::new(Some(value))));
                }
                Binding::ArrayReference(cell, spec) => {
                    self.active_arrays.insert(parameter.name.clone(), spec);
                    frame.insert(parameter.name.clone(), cell);
                }
                Binding::ArrayValue(value, spec) => {
                    self.active_arrays.insert(parameter.name.clone(), spec);
                    frame.insert(parameter.name.clone(), Rc::new(RefCell::new(Some(value))));
                }
            }
        }
        let mut declarations = routine.visible_globals.clone();
        declarations.extend(routine.declarations.clone());
        self.call_depth += 1;
        let mut flow = ControlFlow::Continue;
        let result = self.execute_block(
            &routine.statements,
            &declarations,
            &constants,
            routines,
            &mut frame,
            executed_steps,
            &mut flow,
        );
        self.call_depth -= 1;
        self.active_arrays = previous_arrays;
        result?;
        match flow {
            ControlFlow::Halt => {
                *caller_control = ControlFlow::Halt;
                Ok(None)
            }
            ControlFlow::Return(value) => match (routine.return_type, value) {
                (None, None) => Ok(None),
                (None, Some(_)) => Err(runtime_error(
                    routine.span,
                    "algorithm cannot return a value",
                )),
                (Some(_), None) => Err(runtime_error(
                    routine.span,
                    "function returned without a value",
                )),
                (Some(ty), Some(value)) => coerce_assignment(value, ty)
                    .map(Some)
                    .ok_or_else(|| runtime_error(routine.span, "function return type mismatch")),
            },
            ControlFlow::Continue if routine.return_type.is_some() => Err(runtime_error(
                routine.span,
                "function ended without a return value",
            )),
            ControlFlow::Continue => Ok(None),
        }
    }

    fn drawing_command(
        &mut self,
        geometry: &DrawGeometry,
        at: Span,
        declarations: &HashMap<String, (Type, Span)>,
        variables: &mut HashMap<String, VariableCell>,
        constants: &HashMap<String, Value>,
        routines: &HashMap<String, Routine>,
        executed_steps: &mut u64,
        control: &mut ControlFlow,
    ) -> Result<DrawingCommand, RuntimeError> {
        macro_rules! number {
            ($expression:expr) => {{
                let value = self.eval(
                    $expression,
                    declarations,
                    variables,
                    constants,
                    routines,
                    executed_steps,
                    control,
                )?;
                numeric_to_f64(&value)
                    .ok_or_else(|| runtime_error(at, "drawing values must be numeric"))?
            }};
        }
        macro_rules! point {
            ($coordinates:expr) => {{
                let (x, y) = $coordinates;
                Point {
                    x: number!(x),
                    y: number!(y),
                }
            }};
        }
        macro_rules! color {
            ($channels:expr) => {{
                let [red, green, blue] = $channels;
                checked_rgb(number!(red), number!(green), number!(blue), at)?
            }};
        }

        Ok(match geometry {
            DrawGeometry::Point(x, y) => DrawingCommand::Point(Point {
                x: number!(x),
                y: number!(y),
            }),
            DrawGeometry::Line(points) => {
                let mut values = Vec::with_capacity(points.len());
                for coordinates in points {
                    values.push(point!(coordinates));
                }
                DrawingCommand::Line(values)
            }
            DrawGeometry::Circle { center, diameter } => DrawingCommand::Circle {
                center: point!(center),
                diameter: number!(diameter),
            },
            DrawGeometry::Ellipse { center, diameters } => DrawingCommand::Ellipse {
                center: point!(center),
                diameters: point!(diameters),
            },
            DrawGeometry::Rectangle { from, to } => DrawingCommand::Rectangle {
                from: point!(from),
                to: point!(to),
            },
            DrawGeometry::Triangle(points) => {
                let mut values = points.iter();
                DrawingCommand::Triangle([
                    point!(values.next().expect("triangle has three source vertices")),
                    point!(values.next().expect("triangle has three source vertices")),
                    point!(values.next().expect("triangle has three source vertices")),
                ])
            }
            DrawGeometry::StrokeColor(channels) => DrawingCommand::SetStrokeColor(color!(channels)),
            DrawGeometry::FillColor(channels) => DrawingCommand::SetFillColor(color!(channels)),
            DrawGeometry::FillCircle { center, diameter } => DrawingCommand::FillCircle {
                center: point!(center),
                diameter: number!(diameter),
            },
            DrawGeometry::FillEllipse { center, diameters } => DrawingCommand::FillEllipse {
                center: point!(center),
                diameters: point!(diameters),
            },
            DrawGeometry::FillRectangle { from, to } => DrawingCommand::FillRectangle {
                from: point!(from),
                to: point!(to),
            },
            DrawGeometry::FillTriangle(points) => {
                let mut values = points.iter();
                DrawingCommand::FillTriangle([
                    point!(values.next().expect("triangle has three source vertices")),
                    point!(values.next().expect("triangle has three source vertices")),
                    point!(values.next().expect("triangle has three source vertices")),
                ])
            }
            DrawGeometry::FillCanvas(channels) => DrawingCommand::FillCanvas(match channels {
                Some(channels) => Some(color!(channels)),
                None => None,
            }),
            DrawGeometry::Label { position, values } => {
                let position = match position {
                    Some(coordinates) => Some(point!(coordinates)),
                    None => None,
                };
                let mut text_values = Vec::with_capacity(values.len());
                for value in values {
                    match value {
                        OutputPart::Expression(expression) => text_values.push(
                            self.eval(
                                expression,
                                declarations,
                                variables,
                                constants,
                                routines,
                                executed_steps,
                                control,
                            )?
                            .to_string(),
                        ),
                        OutputPart::Text(text) => text_values.push(text.clone()),
                        OutputPart::Newline => {
                            return Err(runtime_error(at, "newline is not a drawing-label value"));
                        }
                    }
                }
                DrawingCommand::Label {
                    position,
                    values: text_values,
                }
            }
        })
    }

    fn eval(
        &mut self,
        expression: &Expr,
        declarations: &HashMap<String, (Type, Span)>,
        variables: &mut HashMap<String, VariableCell>,
        constants: &HashMap<String, Value>,
        routines: &HashMap<String, Routine>,
        executed_steps: &mut u64,
        control: &mut ControlFlow,
    ) -> Result<Value, RuntimeError> {
        match expression {
            Expr::Integer(value) => Ok(Value::Integer(*value)),
            Expr::Real(value) => Ok(Value::Real(*value)),
            Expr::Logical(value) => Ok(Value::Logical(*value)),
            Expr::Text(value) => Ok(Value::Text(value.clone())),
            Expr::Character(value) => Ok(Value::Character(*value)),
            Expr::Variable(name, at) => variables
                .get(name)
                .and_then(|cell| cell.borrow().clone())
                .or_else(|| constants.get(name).cloned())
                .ok_or_else(|| {
                    if declarations.contains_key(name) {
                        runtime_error(*at, format!("variable {name} used before initialization"))
                    } else {
                        runtime_error(*at, format!("undeclared name {name}"))
                    }
                }),
            Expr::Index(name, indexes, at) => {
                let mut values = Vec::with_capacity(indexes.len());
                for index in indexes {
                    match self.eval(
                        index,
                        declarations,
                        variables,
                        constants,
                        routines,
                        executed_steps,
                        control,
                    )? {
                        Value::Integer(value) => values.push(value),
                        _ => return Err(runtime_error(*at, "array index must be an integer")),
                    }
                }
                let cell = array_element_cell(
                    name,
                    &values,
                    &self.active_arrays,
                    &self.active_records,
                    variables,
                    *at,
                )?;
                cell.borrow().clone().ok_or_else(|| {
                    runtime_error(
                        *at,
                        format!("array element {name} used before initialization"),
                    )
                })
            }
            Expr::IndexedField(name, indexes, fields, at) => {
                let mut values = Vec::with_capacity(indexes.len());
                for index in indexes {
                    match self.eval(
                        index,
                        declarations,
                        variables,
                        constants,
                        routines,
                        executed_steps,
                        control,
                    )? {
                        Value::Integer(value) => values.push(value),
                        _ => return Err(runtime_error(*at, "array index must be an integer")),
                    }
                }
                let root = array_element_cell(
                    name,
                    &values,
                    &self.active_arrays,
                    &self.active_records,
                    variables,
                    *at,
                )?;
                let cell = record_field_cell_from_cell(&root, fields, *at)?;
                cell.borrow().clone().ok_or_else(|| {
                    runtime_error(
                        *at,
                        format!("record field `{}` is uninitialized", fields.join(".")),
                    )
                })
            }
            Expr::Field(name, fields, at) => {
                if constants.contains_key(name) {
                    return Err(runtime_error(
                        *at,
                        "constant records cannot be read as fields",
                    ));
                }
                let cell = record_field_cell(name, fields, variables, *at)?;
                cell.borrow().clone().ok_or_else(|| {
                    runtime_error(
                        *at,
                        format!("record field `{}` is uninitialized", fields.join(".")),
                    )
                })
            }
            Expr::Call(name, args, at) => self
                .call_routine(
                    name,
                    args,
                    *at,
                    declarations,
                    constants,
                    routines,
                    variables,
                    executed_steps,
                    control,
                )?
                .ok_or_else(|| {
                    runtime_error(
                        *at,
                        "algorithm without a return value used as an expression",
                    )
                }),
            Expr::Unary(operand, operator, at) => {
                let value = self.eval(
                    operand,
                    declarations,
                    variables,
                    constants,
                    routines,
                    executed_steps,
                    control,
                )?;
                match (operator, value) {
                    (Unary::Not, Value::Logical(value)) => Ok(Value::Logical(!value)),
                    (Unary::Negate, Value::Integer(value)) => value
                        .checked_neg()
                        .map(Value::Integer)
                        .ok_or_else(|| runtime_error(*at, "integer arithmetic overflow")),
                    (Unary::Absolute, Value::Integer(value)) => value
                        .checked_abs()
                        .map(Value::Integer)
                        .ok_or_else(|| runtime_error(*at, "integer arithmetic overflow")),
                    (Unary::Negate, Value::Real(value)) => Ok(Value::Real(-value)),
                    (Unary::Absolute, Value::Real(value)) => Ok(Value::Real(value.abs())),
                    (Unary::Not, _) => {
                        Err(runtime_error(*at, "not operator requires a logical value"))
                    }
                    _ => Err(runtime_error(*at, "numeric unary operator needs a number")),
                }
            }
            Expr::Binary(left, operator, right, at) => {
                let left = self.eval(
                    left,
                    declarations,
                    variables,
                    constants,
                    routines,
                    executed_steps,
                    control,
                )?;
                if *operator == Arithmetic::And && left == Value::Logical(false) {
                    return Ok(Value::Logical(false));
                }
                if *operator == Arithmetic::Or && left == Value::Logical(true) {
                    return Ok(Value::Logical(true));
                }
                let right = self.eval(
                    right,
                    declarations,
                    variables,
                    constants,
                    routines,
                    executed_steps,
                    control,
                )?;
                eval_binary(left, *operator, right, *at)
            }
        }
    }

    fn eval_condition(
        &mut self,
        condition: &Condition,
        declarations: &HashMap<String, (Type, Span)>,
        variables: &mut HashMap<String, VariableCell>,
        constants: &HashMap<String, Value>,
        routines: &HashMap<String, Routine>,
        executed_steps: &mut u64,
        control: &mut ControlFlow,
    ) -> Result<bool, RuntimeError> {
        let Condition::Expression(expression) = condition;
        match self.eval(
            expression,
            declarations,
            variables,
            constants,
            routines,
            executed_steps,
            control,
        )? {
            Value::Logical(value) => Ok(value),
            _ => Err(runtime_error(
                expression_span(expression),
                "condition must be logical",
            )),
        }
    }
    fn tick(&self, executed_steps: u64, span: Span) -> Result<(), RuntimeError> {
        if executed_steps >= self.step_limit {
            Err(runtime_error(span, "execution step limit exceeded"))
        } else {
            Ok(())
        }
    }
}

fn statement_span(statement: &Stmt) -> &Span {
    match statement {
        Stmt::Assign { span, .. }
        | Stmt::Read { span, .. }
        | Stmt::Write { span, .. }
        | Stmt::While { span, .. }
        | Stmt::RepeatUntil { span, .. }
        | Stmt::Counted { span, .. }
        | Stmt::For { span, .. }
        | Stmt::Call { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::Terminate { span }
        | Stmt::If { span, .. }
        | Stmt::Draw { span, .. }
        | Stmt::OpenFile { span, .. }
        | Stmt::ReadFile { span, .. }
        | Stmt::WriteFile { span, .. }
        | Stmt::CloseFile { span, .. } => span,
    }
}

fn set_variable(variables: &mut HashMap<String, VariableCell>, name: &str, value: Value) {
    if let Some(cell) = variables.get(name) {
        *cell.borrow_mut() = Some(value);
    } else {
        variables.insert(name.to_owned(), Rc::new(RefCell::new(Some(value))));
    }
}

fn make_array_value(spec: &ArraySpec) -> Value {
    let total = spec.dimensions.iter().product::<usize>();
    Value::Array((0..total).map(|_| Rc::new(RefCell::new(None))).collect())
}

fn make_record_value(type_id: u16, definitions: &HashMap<u16, RecordDefinition>) -> Value {
    let fields = definitions
        .get(&type_id)
        .map(|definition| {
            definition
                .fields
                .iter()
                .map(|(name, field)| {
                    let initial = if let Some(spec) = &field.array_spec {
                        Some(make_array_value(spec))
                    } else if let Type::Record(nested_type) = field.ty {
                        Some(make_record_value(nested_type, definitions))
                    } else {
                        None
                    };
                    (name.clone(), Rc::new(RefCell::new(initial)))
                })
                .collect()
        })
        .unwrap_or_default();
    Value::Record { type_id, fields }
}

fn clone_value_deep(value: &Value) -> Value {
    match value {
        Value::Array(cells) => Value::Array(
            cells
                .iter()
                .map(|cell| Rc::new(RefCell::new(cell.borrow().as_ref().map(clone_value_deep))))
                .collect(),
        ),
        Value::Record { type_id, fields } => Value::Record {
            type_id: *type_id,
            fields: fields
                .iter()
                .map(|(name, cell)| {
                    (
                        name.clone(),
                        Rc::new(RefCell::new(cell.borrow().as_ref().map(clone_value_deep))),
                    )
                })
                .collect(),
        },
        _ => value.clone(),
    }
}

fn record_field_cell(
    root_name: &str,
    path: &[String],
    variables: &HashMap<String, VariableCell>,
    span: Span,
) -> Result<VariableCell, RuntimeError> {
    let root = variables
        .get(root_name)
        .cloned()
        .ok_or_else(|| runtime_error(span, format!("record `{root_name}` is uninitialized")))?;
    record_field_cell_from_cell(&root, path, span)
}

fn record_field_cell_from_cell(
    root: &VariableCell,
    path: &[String],
    span: Span,
) -> Result<VariableCell, RuntimeError> {
    let mut value = root
        .borrow()
        .clone()
        .ok_or_else(|| runtime_error(span, "record is uninitialized"))?;
    for (index, field_name) in path.iter().enumerate() {
        let Value::Record { fields, .. } = value else {
            return Err(runtime_error(
                span,
                format!("cannot select field `{field_name}` from a non-record value"),
            ));
        };
        let cell = fields
            .get(field_name)
            .cloned()
            .ok_or_else(|| runtime_error(span, format!("record has no field `{field_name}`")))?;
        if index + 1 == path.len() {
            return Ok(cell);
        }
        value = cell.borrow().clone().ok_or_else(|| {
            runtime_error(
                span,
                format!("nested record field `{field_name}` is uninitialized"),
            )
        })?;
    }
    Err(runtime_error(span, "record field path is empty"))
}

fn copy_array_value(value: Value, span: Span) -> Result<Value, RuntimeError> {
    let Value::Array(cells) = value else {
        return Err(runtime_error(span, "array value is invalid"));
    };
    Ok(Value::Array(
        cells
            .iter()
            .map(|cell| Rc::new(RefCell::new(cell.borrow().as_ref().map(clone_value_deep))))
            .collect(),
    ))
}

fn character_array_from_text(text: &str, span: Span) -> Result<(Value, ArraySpec), RuntimeError> {
    let characters = text.chars().collect::<Vec<_>>();
    if characters.len() >= 1_000_000 {
        return Err(runtime_error(
            span,
            "text argument is too large for an array",
        ));
    }
    let mut cells = characters
        .into_iter()
        .map(|character| Rc::new(RefCell::new(Some(Value::Character(character)))))
        .collect::<Vec<_>>();
    cells.push(Rc::new(RefCell::new(Some(Value::Character('\0')))));
    let size = cells.len();
    Ok((
        Value::Array(cells),
        ArraySpec {
            element_type: Type::Character,
            dimensions: vec![size],
        },
    ))
}

fn array_element_cell(
    name: &str,
    indexes: &[i64],
    arrays: &HashMap<String, ArraySpec>,
    record_types: &HashMap<u16, RecordDefinition>,
    variables: &mut HashMap<String, VariableCell>,
    span: Span,
) -> Result<VariableCell, RuntimeError> {
    let spec = arrays
        .get(name)
        .ok_or_else(|| runtime_error(span, format!("`{name}` is not a declared array")))?;
    if indexes.len() != spec.dimensions.len() {
        return Err(runtime_error(
            span,
            format!(
                "array `{name}` expects {} index values but received {}",
                spec.dimensions.len(),
                indexes.len()
            ),
        ));
    }
    let mut offset = 0usize;
    for (index, dimension) in indexes.iter().zip(&spec.dimensions) {
        if *index < 1 || usize::try_from(*index).map_or(true, |index| index > *dimension) {
            return Err(runtime_error(
                span,
                format!("array index out of bounds in `{name}`"),
            ));
        }
        offset = offset
            .checked_mul(*dimension)
            .and_then(|offset| offset.checked_add(*index as usize - 1))
            .ok_or_else(|| runtime_error(span, "array index overflow"))?;
    }
    let root = variables
        .entry(name.to_owned())
        .or_insert_with(|| Rc::new(RefCell::new(Some(make_array_value(spec)))))
        .clone();
    let array = root.borrow().clone();
    let Some(Value::Array(elements)) = array else {
        return Err(runtime_error(
            span,
            format!("`{name}` has invalid array storage"),
        ));
    };
    let cell = elements
        .get(offset)
        .cloned()
        .ok_or_else(|| runtime_error(span, format!("array index out of bounds in `{name}`")))?;
    if cell.borrow().is_none()
        && let Type::Record(type_id) = spec.element_type
    {
        *cell.borrow_mut() = Some(make_record_value(type_id, record_types));
    }
    Ok(cell)
}

fn runtime_lvalue_type(
    target: &LValue,
    declarations: &HashMap<String, (Type, Span)>,
    record_types: &HashMap<u16, RecordDefinition>,
) -> Result<Type, RuntimeError> {
    match target {
        LValue::Variable { name, span } => declarations
            .get(name)
            .map(|(ty, _)| *ty)
            .ok_or_else(|| runtime_error(*span, format!("undeclared variable {name}"))),
        LValue::Index { name, span, .. } => declarations
            .get(name)
            .and_then(|(ty, _)| array_element_type(*ty))
            .ok_or_else(|| runtime_error(*span, format!("`{name}` is not an array"))),
        LValue::IndexedField {
            name, fields, span, ..
        } => {
            let Some((Type::RecordArray(type_id), _)) = declarations.get(name) else {
                return Err(runtime_error(
                    *span,
                    format!("`{name}` is not an array of records"),
                ));
            };
            let mut ty = Type::Record(*type_id);
            for (index, field_name) in fields.iter().enumerate() {
                let Type::Record(type_id) = ty else {
                    return Err(runtime_error(
                        *span,
                        format!("`{field_name}` is selected from a non-record value"),
                    ));
                };
                let definition = record_types.get(&type_id).ok_or_else(|| {
                    runtime_error(*span, format!("record type {type_id} is unavailable"))
                })?;
                let field = definition.fields.get(field_name).ok_or_else(|| {
                    runtime_error(
                        *span,
                        format!("record `{}` has no field `{field_name}`", definition.name),
                    )
                })?;
                ty = field.ty;
                if index + 1 < fields.len() && field.array_spec.is_some() {
                    return Err(runtime_error(
                        *span,
                        "an array record field must be indexed before selecting a nested field",
                    ));
                }
            }
            Ok(ty)
        }
        LValue::Field { name, fields, span } => {
            let Some((root_type, _)) = declarations.get(name) else {
                return Err(runtime_error(*span, format!("undeclared record `{name}`")));
            };
            let mut ty = *root_type;
            for (index, field_name) in fields.iter().enumerate() {
                let Type::Record(type_id) = ty else {
                    return Err(runtime_error(
                        *span,
                        format!("`{field_name}` is selected from a non-record value"),
                    ));
                };
                let definition = record_types.get(&type_id).ok_or_else(|| {
                    runtime_error(*span, format!("record type {type_id} is unavailable"))
                })?;
                let field = definition.fields.get(field_name).ok_or_else(|| {
                    runtime_error(
                        *span,
                        format!("record `{}` has no field `{field_name}`", definition.name),
                    )
                })?;
                ty = field.ty;
                if index + 1 < fields.len() && field.array_spec.is_some() {
                    return Err(runtime_error(
                        *span,
                        "an array record field must be indexed before selecting a nested field",
                    ));
                }
            }
            Ok(ty)
        }
    }
}

fn read_character_array(
    cells: &[VariableCell],
    spec: &ArraySpec,
    input: &str,
) -> Result<(), String> {
    if spec.element_type != Type::Character || spec.dimensions.len() != 1 {
        return Err("text input requires a one-dimensional character array".to_owned());
    }
    let text = input.trim_end_matches(['\r', '\n']);
    let characters = text.chars().collect::<Vec<_>>();
    if characters.len() >= cells.len() {
        return Err(
            "input text does not fit in the character array with its terminator".to_owned(),
        );
    }
    for cell in cells {
        *cell.borrow_mut() = Some(Value::Character('\0'));
    }
    for (cell, character) in cells.iter().zip(characters) {
        *cell.borrow_mut() = Some(Value::Character(character));
    }
    Ok(())
}

fn coerce_assignment(value: Value, expected: Type) -> Option<Value> {
    match (value, expected) {
        (Value::Integer(value), Type::Real) => Some(Value::Real(value as f64)),
        (Value::Integer(value), Type::SmallInteger | Type::Integer) => Some(Value::Integer(value)),
        (Value::Real(value), Type::Real) => Some(Value::Real(value)),
        (Value::Logical(value), Type::Logical) => Some(Value::Logical(value)),
        (Value::Character(value), Type::Character) => Some(Value::Character(value)),
        (Value::File(value), Type::File) => Some(Value::File(value)),
        (Value::Pointer(value), Type::Pointer) => Some(Value::Pointer(value)),
        (value @ Value::Record { type_id, .. }, Type::Record(expected)) if type_id == expected => {
            Some(clone_value_deep(&value))
        }
        _ => None,
    }
}

fn coerce_parameter(value: Value, expected: Type) -> Option<Value> {
    match (value, expected) {
        (Value::Integer(value), Type::Real) => Some(Value::Real(value as f64)),
        (Value::Integer(value), Type::SmallInteger | Type::Integer) => Some(Value::Integer(value)),
        (Value::Integer(value), Type::Character) => {
            char::from_u32(u32::try_from(value).ok()?).map(Value::Character)
        }
        (Value::Real(value), Type::Real) => Some(Value::Real(value)),
        (Value::Logical(value), Type::Logical) => Some(Value::Logical(value)),
        (Value::Character(value), Type::Character) => Some(Value::Character(value)),
        (Value::Character(value), Type::Real) => Some(Value::Real(value as u32 as f64)),
        (Value::Character(value), Type::SmallInteger | Type::Integer) => {
            Some(Value::Integer(value as u32 as i64))
        }
        (Value::File(value), Type::File) => Some(Value::File(value)),
        (Value::Pointer(value), Type::Pointer) => Some(Value::Pointer(value)),
        (value @ Value::Record { type_id, .. }, Type::Record(expected)) if type_id == expected => {
            Some(clone_value_deep(&value))
        }
        _ => None,
    }
}

fn parse_input_value(ty: Type, input: &str) -> Result<Value, String> {
    let trimmed = input.trim();
    let normalized = normalize_digits(trimmed);
    match ty {
        Type::Integer | Type::SmallInteger => normalized
            .parse::<i64>()
            .map(Value::Integer)
            .map_err(|_| "expected an integer input".to_owned()),
        Type::Real => normalized
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .map(Value::Real)
            .ok_or_else(|| "expected a real-number input".to_owned()),
        Type::Logical => match trimmed {
            "صواب" => Ok(Value::Logical(true)),
            "خطأ" => Ok(Value::Logical(false)),
            _ => Err("expected `صواب` or `خطأ`".to_owned()),
        },
        Type::Character => {
            let character_input = input.trim_end_matches(['\r', '\n']);
            let mut chars = character_input.chars();
            let Some(value) = chars.next() else {
                return Err("expected one character".to_owned());
            };
            if chars.next().is_some() {
                Err("character input must contain one Unicode scalar".to_owned())
            } else {
                Ok(Value::Character(value))
            }
        }
        Type::Pointer => Err("pointer input is not defined by the available source".to_owned()),
        Type::File => Err("file input syntax is not defined by the available source".to_owned()),
        Type::Text
        | Type::Array
        | Type::IntegerArray
        | Type::SmallIntegerArray
        | Type::RealArray
        | Type::LogicalArray
        | Type::CharacterArray
        | Type::Record(_)
        | Type::RecordArray(_) => {
            Err("this declaration type has no source-backed input form".to_owned())
        }
    }
}

fn numeric_to_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Integer(value) => Some(*value as f64),
        Value::Real(value) => Some(*value),
        _ => None,
    }
}

fn checked_rgb(red: f64, green: f64, blue: f64, span: Span) -> Result<Rgb, RuntimeError> {
    if [red, green, blue]
        .into_iter()
        .any(|channel| !(0.0..=100.0).contains(&channel))
    {
        return Err(runtime_error(
            span,
            "RGB channels must be between 0 and 100",
        ));
    }
    Ok(Rgb { red, green, blue })
}

fn evaluate_builtin(name: &str, arguments: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if matches!(name, "باقي" | "تدوير") {
        if arguments.len() != 2 {
            return Err(runtime_error(
                span,
                format!("built-in `{name}` expects two arguments"),
            ));
        }
        let (Value::Integer(dividend), Value::Integer(divisor)) = (&arguments[0], &arguments[1])
        else {
            return Err(runtime_error(
                span,
                format!("`{name}` needs integer arguments"),
            ));
        };
        let quotient = dividend
            .checked_div(*divisor)
            .ok_or_else(|| runtime_error(span, "integer division by zero or overflow"))?;
        if name == "باقي" {
            let multiple = quotient
                .checked_mul(*divisor)
                .ok_or_else(|| runtime_error(span, "integer remainder overflow"))?;
            return dividend
                .checked_sub(multiple)
                .map(Value::Integer)
                .ok_or_else(|| runtime_error(span, "integer remainder overflow"));
        }
        let multiple = quotient
            .checked_mul(*divisor)
            .ok_or_else(|| runtime_error(span, "rounded multiple overflow"))?;
        if dividend == &multiple {
            Ok(Value::Integer(*dividend))
        } else {
            multiple
                .checked_add(*divisor)
                .map(Value::Integer)
                .ok_or_else(|| runtime_error(span, "rounded multiple overflow"))
        }
    } else {
        if arguments.len() != 1 {
            return Err(runtime_error(
                span,
                format!("built-in `{name}` expects one argument"),
            ));
        }
        match (name, &arguments[0]) {
            ("جذر", value) => {
                let number = numeric_to_f64(value)
                    .ok_or_else(|| runtime_error(span, "`جذر` needs a numeric argument"))?;
                let result = number.sqrt();
                result
                    .is_finite()
                    .then_some(Value::Real(result))
                    .ok_or_else(|| runtime_error(span, "`جذر` argument is outside its real domain"))
            }
            ("جب" | "تجب", value) => {
                let number = numeric_to_f64(value).ok_or_else(|| {
                    runtime_error(span, format!("`{name}` needs a numeric argument"))
                })?;
                let result = if name == "جب" {
                    number.sin()
                } else {
                    number.cos()
                };
                result
                    .is_finite()
                    .then_some(Value::Real(result))
                    .ok_or_else(|| runtime_error(span, format!("`{name}` result is not finite")))
            }
            ("ظل" | "تظل", value) => {
                let number = numeric_to_f64(value).ok_or_else(|| {
                    runtime_error(span, format!("`{name}` needs a numeric argument"))
                })?;
                let (numerator, denominator) = if name == "ظل" {
                    (number.sin(), number.cos())
                } else {
                    (number.cos(), number.sin())
                };
                let result = if denominator.abs() <= 0.000001 {
                    1.7e308
                } else {
                    numerator / denominator
                };
                result
                    .is_finite()
                    .then_some(Value::Real(result))
                    .ok_or_else(|| runtime_error(span, format!("`{name}` result is not finite")))
            }
            ("لغ", value) => {
                let number = numeric_to_f64(value)
                    .ok_or_else(|| runtime_error(span, "`لغ` needs a numeric argument"))?;
                let result = number.ln();
                result
                    .is_finite()
                    .then_some(Value::Real(result))
                    .ok_or_else(|| runtime_error(span, "`لغ` argument is outside its real domain"))
            }
            ("صحيح", Value::Integer(value)) => Ok(Value::Integer(*value)),
            ("صحيح", Value::Real(value)) if value.is_finite() => {
                let truncated = value.trunc();
                if truncated < i64::MIN as f64 || truncated >= -(i64::MIN as f64) {
                    Err(runtime_error(
                        span,
                        "`صحيح` result is outside the integer range",
                    ))
                } else {
                    Ok(Value::Integer(truncated as i64))
                }
            }
            ("أبجدي", Value::Character(value)) => {
                Ok(Value::Logical(is_jeem_alphabetic_letter(*value)))
            }
            ("عكس", Value::Logical(value)) => Ok(Value::Logical(!value)),
            ("صحيح" | "أبجدي" | "عكس", _) => Err(runtime_error(
                span,
                format!("invalid argument type for `{name}`"),
            )),
            _ => Err(runtime_error(
                span,
                format!("predefined function `{name}` has no executable contract in this profile"),
            )),
        }
    }
}

fn eval_constant_expression(
    expression: &Expr,
    constants: &HashMap<String, Value>,
) -> Result<Value, String> {
    match expression {
        Expr::Integer(value) => Ok(Value::Integer(*value)),
        Expr::Real(value) => Ok(Value::Real(*value)),
        Expr::Logical(value) => Ok(Value::Logical(*value)),
        Expr::Text(value) => Ok(Value::Text(value.clone())),
        Expr::Character(value) => Ok(Value::Character(*value)),
        Expr::Variable(name, _) => constants
            .get(name)
            .cloned()
            .ok_or_else(|| format!("constant expression cannot use variable `{name}`")),
        Expr::Index(_, _, _) => Err("constant expression cannot index an array".to_owned()),
        Expr::IndexedField(_, _, _, _) => {
            Err("constant expression cannot access an array-record field".to_owned())
        }
        Expr::Field(_, _, _) => Err("constant expression cannot access a record field".to_owned()),
        Expr::Call(name, arguments, span) => {
            if !is_builtin_name(name) {
                return Err("constant expression cannot call a user routine".to_owned());
            }
            let values = arguments
                .iter()
                .map(|argument| eval_constant_expression(argument, constants))
                .collect::<Result<Vec<_>, _>>()?;
            evaluate_builtin(name, &values, *span).map_err(|error| error.message)
        }
        Expr::Unary(operand, operator, span) => {
            let value = eval_constant_expression(operand, constants)?;
            match (operator, value) {
                (Unary::Not, Value::Logical(value)) => Ok(Value::Logical(!value)),
                (Unary::Negate, Value::Integer(value)) => {
                    value.checked_neg().map(Value::Integer).ok_or_else(|| {
                        format!("{}:{}: integer arithmetic overflow", span.line, span.column)
                    })
                }
                (Unary::Absolute, Value::Integer(value)) => {
                    value.checked_abs().map(Value::Integer).ok_or_else(|| {
                        format!("{}:{}: integer arithmetic overflow", span.line, span.column)
                    })
                }
                (Unary::Negate, Value::Real(value)) => Ok(Value::Real(-value)),
                (Unary::Absolute, Value::Real(value)) => Ok(Value::Real(value.abs())),
                (Unary::Not, _) => Err("`نفي` needs a logical constant".to_owned()),
                _ => Err("numeric unary operator needs a numeric constant".to_owned()),
            }
        }
        Expr::Binary(left, operator, right, span) => {
            let left = eval_constant_expression(left, constants)?;
            let right = eval_constant_expression(right, constants)?;
            eval_binary(left, *operator, right, *span).map_err(|error| error.message)
        }
    }
}

fn eval_binary(
    left: Value,
    operator: Arithmetic,
    right: Value,
    span: Span,
) -> Result<Value, RuntimeError> {
    if matches!(
        operator,
        Arithmetic::Greater
            | Arithmetic::GreaterOrEqual
            | Arithmetic::Less
            | Arithmetic::LessOrEqual
            | Arithmetic::Equal
            | Arithmetic::NotEqual
            | Arithmetic::And
            | Arithmetic::Or
    ) {
        let result = match operator {
            Arithmetic::Equal | Arithmetic::NotEqual => {
                let equal = match (&left, &right) {
                    (Value::Integer(_) | Value::Real(_), Value::Integer(_) | Value::Real(_)) => {
                        numeric_to_f64(&left) == numeric_to_f64(&right)
                    }
                    _ => left == right,
                };
                if operator == Arithmetic::Equal {
                    equal
                } else {
                    !equal
                }
            }
            Arithmetic::And => matches!(
                (&left, &right),
                (Value::Logical(true), Value::Logical(true))
            ),
            Arithmetic::Or => matches!(
                (&left, &right),
                (Value::Logical(false), Value::Logical(true))
                    | (Value::Logical(true), Value::Logical(false))
                    | (Value::Logical(true), Value::Logical(true))
            ),
            Arithmetic::Greater
            | Arithmetic::GreaterOrEqual
            | Arithmetic::Less
            | Arithmetic::LessOrEqual => {
                let left_number = numeric_to_f64(&left)
                    .ok_or_else(|| runtime_error(span, "comparison needs numeric values"))?;
                let right_number = numeric_to_f64(&right)
                    .ok_or_else(|| runtime_error(span, "comparison needs numeric values"))?;
                match operator {
                    Arithmetic::Greater => left_number > right_number,
                    Arithmetic::GreaterOrEqual => left_number >= right_number,
                    Arithmetic::Less => left_number < right_number,
                    Arithmetic::LessOrEqual => left_number <= right_number,
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        };
        return Ok(Value::Logical(result));
    }
    if operator == Arithmetic::Power {
        if let (Value::Integer(base), Value::Integer(exponent)) = (&left, &right) {
            if *exponent >= 0 {
                let exponent = u32::try_from(*exponent)
                    .map_err(|_| runtime_error(span, "integer exponent is too large"))?;
                return base
                    .checked_pow(exponent)
                    .map(Value::Integer)
                    .ok_or_else(|| runtime_error(span, "integer arithmetic overflow"));
            }
        }
        let base = numeric_to_f64(&left)
            .ok_or_else(|| runtime_error(span, "power needs numeric values"))?;
        let exponent = numeric_to_f64(&right)
            .ok_or_else(|| runtime_error(span, "power needs numeric values"))?;
        let value = base.powf(exponent);
        return value
            .is_finite()
            .then_some(Value::Real(value))
            .ok_or_else(|| runtime_error(span, "real arithmetic overflow"));
    }
    if operator == Arithmetic::IntegerDivide {
        let (Value::Integer(left), Value::Integer(right)) = (&left, &right) else {
            return Err(runtime_error(span, "integer quotient needs integer values"));
        };
        if *right == 0 {
            return Err(runtime_error(span, "division by zero"));
        }
        return left
            .checked_div(*right)
            .map(Value::Integer)
            .ok_or_else(|| runtime_error(span, "integer arithmetic overflow"));
    }
    if operator != Arithmetic::Divide
        && let (Value::Integer(left), Value::Integer(right)) = (&left, &right)
    {
        let value = match operator {
            Arithmetic::Add => left.checked_add(*right),
            Arithmetic::Subtract => left.checked_sub(*right),
            Arithmetic::Multiply => left.checked_mul(*right),
            Arithmetic::Divide | Arithmetic::IntegerDivide => unreachable!(),
            Arithmetic::Power
            | Arithmetic::Greater
            | Arithmetic::GreaterOrEqual
            | Arithmetic::Less
            | Arithmetic::LessOrEqual
            | Arithmetic::Equal
            | Arithmetic::NotEqual
            | Arithmetic::And
            | Arithmetic::Or => unreachable!(),
        };
        return value
            .map(Value::Integer)
            .ok_or_else(|| runtime_error(span, "integer arithmetic overflow"));
    }
    let left = numeric_to_f64(&left)
        .ok_or_else(|| runtime_error(span, "arithmetic operator needs numbers"))?;
    let right = numeric_to_f64(&right)
        .ok_or_else(|| runtime_error(span, "arithmetic operator needs numbers"))?;
    let value = match operator {
        Arithmetic::Add => left + right,
        Arithmetic::Subtract => left - right,
        Arithmetic::Multiply => left * right,
        Arithmetic::Divide if right == 0.0 => return Err(runtime_error(span, "division by zero")),
        Arithmetic::Divide => left / right,
        Arithmetic::IntegerDivide => {
            return Err(runtime_error(span, "integer quotient needs integer values"));
        }
        Arithmetic::Power
        | Arithmetic::Greater
        | Arithmetic::GreaterOrEqual
        | Arithmetic::Less
        | Arithmetic::LessOrEqual
        | Arithmetic::Equal
        | Arithmetic::NotEqual
        | Arithmetic::And
        | Arithmetic::Or => unreachable!(),
    };
    value
        .is_finite()
        .then_some(Value::Real(value))
        .ok_or_else(|| runtime_error(span, "real arithmetic overflow"))
}

fn runtime_error(span: Span, message: impl Into<String>) -> RuntimeError {
    RuntimeError {
        span,
        message: message.into(),
    }
}
