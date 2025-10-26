use serde;
use serde_json;
use std::{
    fs::File,
    hash::Hasher,
    io::{self, BufReader, Read, Write},
    ops::{Add, BitAnd, BitOr, Div, Mul, Not, Sub},
    path::Path,
    process::{Command, Stdio},
};
use thiserror::Error;

// TODO (jq54): add support for imports

#[derive(Clone)]
pub struct RichProgram {
    pub original_text: Vec<String>,
    pub program: Program,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct Program {
    pub functions: Vec<Function>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct Function {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<Argument>>,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_type: Option<Type>,
    pub instrs: Vec<Code>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos: Option<Position>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct Argument {
    pub name: String,
    #[serde(rename = "type")]
    pub arg_type: Type,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos: Option<Position>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Hash, PartialEq, Eq)]
#[serde(untagged)]
pub enum Code {
    Label {
        label: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pos: Option<Position>,
    },
    Constant {
        op: ConstantOp,
        dest: String,
        #[serde(rename = "type")]
        constant_type: Type,
        value: Literal,
        #[serde(skip_serializing_if = "Option::is_none")]
        pos: Option<Position>,
    },
    Value {
        op: ValueOp,
        dest: String,
        #[serde(rename = "type")]
        value_type: Type,
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        funcs: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        labels: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pos: Option<Position>,
    },
    Effect {
        op: EffectOp,
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        funcs: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        labels: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pos: Option<Position>,
    },

    Memory {
        op: MemoryOp,
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        dest: Option<String>,
        #[serde(rename = "type")]
        ptr_type: Option<Type>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pos: Option<Position>,
    },
    Noop {
        op: Noop,
        #[serde(skip_serializing_if = "Option::is_none")]
        pos: Option<Position>,
    },
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Noop {
    Nop,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ConstantOp {
    Const,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum ValueOp {
    Add,
    Sub,
    Div,
    Mul,
    Eq,
    Lt,
    Gt,
    Le,
    Ge,
    Not,
    And,
    Or,
    Id,
    Fadd,
    Fsub,
    Fdiv,
    Fmul,
    Feq,
    Flt,
    Fgt,
    Fle,
    Fge,
    Ceq,
    Clt,
    Cle,
    Cgt,
    Cge,
    Char2int,
    Int2char,
    Float2bits,
    Bits2float,
    Call,
    Phi, // special op for bril SSA from
}
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum MemoryOp {
    Alloc,
    Free,
    Store,
    Load,
    PtrAdd,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum EffectOp {
    Jmp,
    Br,
    Ret,
    Call, // important, call can be both "effect" and "value op"
    Print,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Type {
    Int,
    Bool,
    Float,
    Char,
    Ptr(Box<Self>),
    None,
}

impl Type {
    pub fn is_ptr(&self) -> bool {
        matches!(self, Type::Ptr(_))
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub row: u64,
    pub col: u64,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Copy)]
#[serde(untagged)]
pub enum Literal {
    Int(i64),
    Bool(bool),
    Float(f64),
    Char(char),
}

impl Code {
    pub fn get_destination(&self) -> Option<&str> {
        match self {
            Code::Constant { dest, .. } => Some(dest),
            Code::Value { dest, .. } => Some(dest),
            Code::Memory { dest, .. } => dest.as_deref(),
            Code::Noop { .. } | Code::Label { .. } | Code::Effect { .. } => None,
        }
    }

    pub fn get_arguments(&self) -> Option<&Vec<String>> {
        match self {
            Code::Value { args, .. } => args.as_ref(),
            Code::Effect { args, .. } => args.as_ref(),
            Code::Memory { args, .. } => args.as_ref(),
            Code::Noop { .. } | Code::Label { .. } | Code::Constant { .. } => None,
        }
    }

    pub fn replace_destination(&mut self, new_dest: String) {
        if self.get_destination().is_none() {
            panic!("Attempted to replace destination on op with no destination");
        }

        match self {
            Code::Constant { dest, .. } => *dest = new_dest,
            Code::Value { dest, .. } => *dest = new_dest,
            Code::Memory { dest, .. } => {
                if let Some(d) = dest {
                    *d = new_dest;
                } else {
                    unreachable!();
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn replace_arguments(&mut self, new_args: Vec<String>) {
        if self.get_arguments().is_none() {
            panic!("Attempted to replace arguments on op with no arguments");
        }

        match self {
            Code::Value { args, .. } => *args = Some(new_args),
            Code::Effect { args, .. } => *args = Some(new_args),
            Code::Memory { args, .. } => *args = Some(new_args),
            _ => panic!("Attempted to replace arguments on non-arg op"),
        }
    }

    pub fn get_opcode_string(&self) -> String {
        match self {
            Code::Label { .. } => "label".to_string(),
            Code::Constant { op, .. } => format!("{:?}", op).to_lowercase(),
            Code::Value { op, .. } => format!("{:?}", op).to_lowercase(),
            Code::Effect { op, .. } => format!("{:?}", op).to_lowercase(),
            Code::Memory { op, .. } => format!("{:?}", op).to_lowercase(),
            Code::Noop { op, .. } => format!("{:?}", op).to_lowercase(),
        }
    }

    pub fn get_type(&self) -> Option<Type> {
        match self {
            Code::Constant { constant_type, .. } => Some(constant_type.clone()),
            Code::Value { value_type, .. } => Some(value_type.clone()),
            Code::Memory { ptr_type, .. } => ptr_type.clone(),
            _ => None,
        }
    }

    pub fn get_position(&self) -> Option<Position> {
        match self {
            Code::Label { pos, .. } => *pos,
            Code::Constant { pos, .. } => *pos,
            Code::Value { pos, .. } => *pos,
            Code::Effect { pos, .. } => *pos,
            Code::Memory { pos, .. } => *pos,
            Code::Noop { pos, .. } => *pos,
        }
    }

    pub fn get_labels(&self) -> Option<&Vec<String>> {
        match self {
            Code::Value { labels, .. } => labels.as_ref(),
            Code::Effect { labels, .. } => labels.as_ref(),
            _ => None,
        }
    }

    pub fn has_side_effects(&self) -> bool {
        match self {
            Code::Effect { .. } => true,
            Code::Memory { .. } => true,
            Code::Value {
                op: ValueOp::Call, ..
            } => true,
            _ => false,
        }
    }

    pub fn is_label(&self) -> bool {
        matches!(self, Code::Label { .. })
    }

    pub fn is_constant(&self) -> bool {
        matches!(self, Code::Constant { .. })
    }
}

impl std::fmt::Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let destination = self.get_destination();
        let arguments = self.get_arguments();

        if let Some(target) = destination {
            if let Some(sources) = arguments {
                write!(f, "{}={}{:?}", target, self.get_opcode_string(), sources)
            } else {
                write!(f, "{}={}[]", target, self.get_opcode_string())
            }
        } else {
            if let Some(sources) = arguments {
                write!(f, "{}{:?}", self.get_opcode_string(), sources)
            } else {
                write!(f, "{}[]", self.get_opcode_string())
            }
        }
    }
}

impl Literal {
    pub fn cast_to(&self, t: &Type) -> Literal {
        match t {
            Type::Int => match self {
                Literal::Int(x) => Literal::Int(*x),
                Literal::Bool(_) => panic!(),
                Literal::Float(x) => Literal::Int(*x as i64),
                Literal::Char(x) => Literal::Int(*x as i64),
            },
            Type::Bool => match self {
                Literal::Int(x) => Literal::Bool(*x != 0),
                Literal::Bool(_) => self.clone(),
                Literal::Float(x) => Literal::Bool(*x != 0.),
                Literal::Char(_) => panic!("no casts to bool from int"),
            },
            Type::Float => match self {
                Literal::Int(x) => Literal::Float(*x as f64),
                Literal::Bool(_) => panic!(),
                Literal::Float(x) => Literal::Float(*x),
                Literal::Char(_) => panic!(),
            },
            Type::Char => match self {
                Literal::Int(x) => Literal::Char((*x as u8) as char),
                _ => panic!(),
            },
            Type::Ptr(_) => panic!("cannot cast to ptr type"),
            Type::None => panic!("cannot cast to none type"),
        }
    }

    pub fn bitcast(&self, t: &Type) -> Literal {
        match t {
            Type::Int => match self {
                Literal::Float(x) => Literal::Int(x.to_bits() as i64),
                _ => panic!("invalid bitcast to int"),
            },
            Type::Float => match self {
                Literal::Int(x) => Literal::Float(f64::from_bits(*x as u64)),
                _ => panic!("invalid bitcast to float"),
            },
            _ => panic!("bitcast only supported between int and float"),
        }
    }
}

impl Add for Literal {
    type Output = Literal;
    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Literal::Int(a), Literal::Int(b)) => Literal::Int(a + b),
            (Literal::Float(a), Literal::Float(b)) => Literal::Float(a + b),
            _ => panic!("Invalid Add operands"),
        }
    }
}

impl Sub for Literal {
    type Output = Literal;
    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Literal::Int(a), Literal::Int(b)) => Literal::Int(a - b),
            (Literal::Float(a), Literal::Float(b)) => Literal::Float(a - b),
            _ => panic!("Invalid operands"),
        }
    }
}

impl Mul for Literal {
    type Output = Literal;
    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Literal::Int(a), Literal::Int(b)) => Literal::Int(a * b),
            (Literal::Float(a), Literal::Float(b)) => Literal::Float(a * b),
            _ => panic!("Invalid operands"),
        }
    }
}

impl Div for Literal {
    type Output = Literal;
    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Literal::Int(a), Literal::Int(b)) => Literal::Int(a / b),
            (Literal::Float(a), Literal::Float(b)) => Literal::Float(a / b),
            _ => panic!("Invalid operands"),
        }
    }
}

impl BitAnd for Literal {
    type Output = Literal;
    fn bitand(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Literal::Bool(a), Literal::Bool(b)) => Literal::Bool(a && b),
            _ => panic!("Invalid operands"),
        }
    }
}

impl BitOr for Literal {
    type Output = Literal;
    fn bitor(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Literal::Bool(a), Literal::Bool(b)) => Literal::Bool(a || b),
            _ => panic!("Invalid operands"),
        }
    }
}

impl Not for Literal {
    type Output = Literal;
    fn not(self) -> Self::Output {
        match self {
            Literal::Bool(a) => Literal::Bool(!a),
            _ => panic!("Invalid operands"),
        }
    }
}

impl PartialOrd for Literal {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Literal::Int(a), Literal::Int(b)) => a.partial_cmp(b),
            (Literal::Float(a), Literal::Float(b)) => a.partial_cmp(b),
            (Literal::Char(a), Literal::Char(b)) => a.partial_cmp(b),
            _ => None, // no ordering for Bool or cross-type
        }
    }
}

impl Ord for Literal {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).expect("Invalid Ord comparison")
    }
}

impl PartialEq for ValueOp {
    fn eq(&self, other: &Self) -> bool {
        if matches!(self, ValueOp::Call) || matches!(other, ValueOp::Call) {
            return false;
        }
        // Compare discriminants (variant identity)
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}
impl Eq for ValueOp {}
impl std::hash::Hash for ValueOp {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            ValueOp::Call => {
                std::ptr::addr_of!(self).hash(state);
            }
            other => std::mem::discriminant(other).hash(state),
        }
    }
}

impl PartialEq for EffectOp {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (EffectOp::Call, _) => false,
            (_, EffectOp::Call) => false,
            (EffectOp::Jmp, EffectOp::Jmp) => true,
            (EffectOp::Br, EffectOp::Br) => true,
            (EffectOp::Ret, EffectOp::Ret) => true,
            (EffectOp::Print, EffectOp::Print) => true,
            _ => false,
        }
    }
}
impl Eq for EffectOp {}
impl std::hash::Hash for EffectOp {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            EffectOp::Call => {
                std::ptr::addr_of!(self).hash(state);
            }
            other => std::mem::discriminant(other).hash(state),
        }
    }
}

impl PartialEq for Literal {
    fn eq(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (Self::Int(lhs), Self::Int(rhs)) => lhs == rhs,
            (Self::Bool(lhs), Self::Bool(rhs)) => lhs == rhs,
            (Self::Float(lhs), Self::Float(rhs)) => lhs.to_le_bytes() == rhs.to_le_bytes(),
            (Self::Char(lhs), Self::Char(rhs)) => lhs == rhs,
            _ => false,
        }
    }
}
impl Eq for Literal {}
impl std::hash::Hash for Literal {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}

#[derive(Error, Debug)]
pub enum ProgramError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("JSON parsing error: {error}\nErroneous JSON section around line {line}, column {column}:\n{json_snippet}")]
    JsonWithContent {
        error: serde_json::Error,
        line: usize,
        column: usize,
        json_snippet: String,
    },
    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("Process execution failed: {process} exited with code {code}")]
    ProcessFailed { process: String, code: i32 },
    #[error("Process '{process}' not found or failed to start")]
    ProcessNotFound { process: String },
    #[error("Unsupported file extension: {ext}")]
    UnsupportedExtension { ext: String },
}

impl RichProgram {
    /// Extract a snippet of JSON around the error location with context lines.
    fn extract_json_error_context(
        json_content: &str,
        error: &serde_json::Error,
    ) -> (usize, usize, String) {
        // serde_json::Error provides line() and column() methods that return usize directly
        let line = error.line();
        let column = error.column();

        if line == 0 {
            return (
                0,
                0,
                "Unable to determine error location in JSON".to_string(),
            );
        }

        let lines: Vec<&str> = json_content.lines().collect();
        let context_lines = 10; // Show 10 lines before and after the error

        let start_line = line.saturating_sub(context_lines + 1); // -1 because line numbers are 1-based
        let end_line = (line + context_lines).min(lines.len());

        let mut snippet = String::new();
        for (i, line_content) in lines[start_line..end_line].iter().enumerate() {
            let line_num = start_line + i + 1;
            let marker = if line_num == line { ">>> " } else { "    " };
            snippet.push_str(&format!("{}{:3}: {}\n", marker, line_num, line_content));
        }

        // Add column pointer for the error line
        if column > 0 && line <= lines.len() {
            let pointer = format!(">>>     {}^\n", " ".repeat(column));
            snippet.push_str(&pointer);
        }

        (line, column, snippet.trim_end().to_string())
    }

    /// Converts a Bril source file to JSON format using the `bril2json` command.
    ///
    /// This function reads the specified Bril file, spawns a `bril2json` process,
    /// pipes the file contents to its stdin, and returns the JSON output as bytes.
    ///
    /// # Arguments
    /// * `file_path` - Path to the `.bril` file to convert
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - The JSON output as bytes from `bril2json`
    /// * `Err(ProgramError)` - If the file cannot be read, process fails to spawn,
    ///   or `bril2json` exits with a non-zero status code
    ///
    /// # Errors
    /// * `ProgramError::Io` - File I/O errors
    /// * `ProgramError::ProcessNotFound` - `bril2json` command not found
    /// * `ProgramError::ProcessFailed` - `bril2json` exited with error code
    fn run_bril2json(file_path: &Path) -> Result<Vec<u8>, ProgramError> {
        let file_contents = std::fs::read(file_path)?;
        let mut child = Command::new("bril2json")
            .args(["-p"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| ProgramError::ProcessNotFound {
                process: "bril2json".into(),
            })?;

        child.stdin.as_mut().unwrap().write_all(&file_contents)?;
        let output = child.wait_with_output()?;

        if !output.status.success() {
            return Err(ProgramError::ProcessFailed {
                process: "bril2json".into(),
                code: output.status.code().unwrap_or(-1),
            });
        }
        Ok(output.stdout)
    }

    fn run_bril2txt(file_path: &Path) -> Result<Vec<u8>, ProgramError> {
        let file_contents = std::fs::read(file_path)?;
        let mut child = Command::new("bril2txt")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| ProgramError::ProcessNotFound {
                process: "bril2txt".into(),
            })?;

        child.stdin.as_mut().unwrap().write_all(&file_contents)?;
        let output = child.wait_with_output()?;

        if !output.status.success() {
            return Err(ProgramError::ProcessFailed {
                process: "bril2txt".into(),
                code: output.status.code().unwrap_or(-1),
            });
        }
        Ok(output.stdout)
    }

    /// Creates a Program from a file with either `.json` or `.bril` extension.
    ///
    /// For `.bril` files, this function automatically converts them to JSON using
    /// the `bril2json` command before parsing. For `.json` files, it directly
    /// deserializes the content.
    ///
    /// # Arguments
    /// * `filename` - Path to the program file (`.json` or `.bril`)
    ///
    /// # Returns
    /// * `Some(Program)` - Successfully parsed program
    /// * `None` - If file cannot be read, parsed, or converted
    ///
    /// # Panics
    /// This function will panic if:
    /// * File I/O operations fail
    /// * JSON deserialization fails
    /// * UTF-8 conversion fails (for `.bril` files)
    /// * The `bril2json` process fails (for `.bril` files)
    ///
    /// # Examples
    /// ```rust
    /// // Load a JSON program file
    /// let program = Program::from_file("examples/test.json").unwrap();
    ///
    /// // Load and convert a Bril source file
    /// let program = Program::from_file("examples/test.bril").unwrap();
    /// ```
    ///
    /// # Note
    /// This function uses `unwrap()` extensively and will panic on errors.
    /// Consider using a Result-returning version for production code.
    pub fn from_file(filename: &Path) -> Result<Self, ProgramError> {
        match filename.extension().and_then(|ext| ext.to_str()) {
            Some("bril") => {
                let raw_text = std::fs::read_to_string(filename)?
                    .lines()
                    .map(|s| s.to_string())
                    .collect();
                let json_output = Self::run_bril2json(filename)?;
                let json_string = String::from_utf8(json_output)?;
                let program = serde_json::from_str::<Program>(&json_string).map_err(|error| {
                    let (line, column, json_snippet) =
                        Self::extract_json_error_context(&json_string, &error);
                    ProgramError::JsonWithContent {
                        error,
                        line,
                        column,
                        json_snippet,
                    }
                })?;

                Ok(RichProgram {
                    original_text: raw_text,
                    program,
                })
            }
            Some("json") => {
                let file = File::open(filename)?;
                let mut reader = BufReader::new(file);
                let mut json_content = String::new();
                reader.read_to_string(&mut json_content)?;

                let program = serde_json::from_str::<Program>(&json_content).map_err(|error| {
                    let (line, column, json_snippet) =
                        Self::extract_json_error_context(&json_content, &error);
                    ProgramError::JsonWithContent {
                        error,
                        line,
                        column,
                        json_snippet,
                    }
                })?;
                Ok(RichProgram {
                    original_text: vec![],
                    program,
                })
            }
            Some(ext) => Err(ProgramError::UnsupportedExtension {
                ext: ext.to_string(),
            }),
            None => Err(ProgramError::UnsupportedExtension {
                ext: "none".to_string(),
            }),
        }
    }

    #[allow(dead_code)]
    pub fn to_string(self) -> String {
        serde_json::to_string(&self.program).unwrap()
    }

    #[allow(dead_code)]
    pub fn to_file(self, file_name: &Path) -> Result<(), ProgramError> {
        // if the file extension ends in .bril, write to tmp file, convert to text, and then write to file
        if file_name.to_str().unwrap().ends_with(".bril") {
            let tmp_file = tempfile::NamedTempFile::new().unwrap();
            let tmp_file_path = tmp_file.path();
            std::fs::write(tmp_file_path, self.to_string()).unwrap();

            let output = Self::run_bril2txt(tmp_file_path)?;
            std::fs::write(file_name, output).unwrap();
            println!("Wrote to {}", file_name.display());
            return Ok(());
        }

        let file = File::create(file_name).unwrap();
        serde_json::to_writer_pretty(file, &self.program).unwrap();
        Ok(())
    }
}
