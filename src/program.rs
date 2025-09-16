use std::{
    fs::File,
    hash::{Hash, Hasher},
    io::{self, BufReader, Read, Write},
    mem,
    ops::{Add, BitAnd, BitOr, Div, Mul, Not, Sub},
};

use serde;
use serde_json;

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
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct Argument {
    pub name: String,
    #[serde(rename = "type")]
    pub arg_type: Type,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum Code {
    Label {
        label: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        position: Option<Position>,
    },
    Constant {
        // TODO: figure out if can fix this to be always "const"
        op: ConstantOp,
        dest: String,
        #[serde(rename = "type")]
        constant_type: Type,
        value: Literal,
    },
    Value {
        // TODO: replace string op with ValueOp enums
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
    },
    Effect {
        // TODO: replace string op with EffectOp?
        op: EffectOp,
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        funcs: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        labels: Option<Vec<String>>,
    },

    Memory {
        op: MemoryOp,
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        dest: Option<String>,
        #[serde(rename = "type")]
        ptr_type: Option<Type>,
    },
    Noop {
        op: Noop,
    },
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
}

impl Literal {
    pub fn cast_to(&self, t: &Type) -> Literal {
        match t {
            Type::Int => match self {
                Literal::Int(x) => Literal::Int(*x),
                Literal::Bool(_) => panic!(),
                Literal::Float(x) => Literal::Int(*x as i64),
                Literal::Char(_) => panic!(),
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
            Type::Char => panic!("no casts to char exist"),
            Type::Ptr(_) => panic!("cannot cast to ptr type"),
            Type::None => panic!("cannot cast to none type"),
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
        mem::discriminant(self) == mem::discriminant(other)
    }
}
impl Eq for ValueOp {}
impl Hash for ValueOp {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            ValueOp::Call => {
                std::ptr::addr_of!(self).hash(state);
            }
            other => std::mem::discriminant(other).hash(state),
        }
    }
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
impl Hash for EffectOp {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            EffectOp::Call => {
                std::ptr::addr_of!(self).hash(state);
            }
            other => std::mem::discriminant(other).hash(state),
        }
    }
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

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct Position {
    pub pos: RowCol,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos_end: Option<RowCol>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Copy)]
pub struct RowCol {
    pub row: u64,
    pub col: u64,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Copy)]
#[serde(untagged)]
pub enum Literal {
    Int(i64),
    Bool(bool),
    // force Eq for f64
    Float(f64),
    Char(char),
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

impl Program {
    /// Read a file with either .json or .bril extension and deserialize it into a Program. If the file extension is .bril
    /// then this function will spawn a child process to run the command bril2json and get the output and deserialize that.
    fn spawn_process_and_get_output(process: &str, file_name: &str) -> std::process::Output {
        let mut child = std::process::Command::new(process)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        child
            .stdin
            .as_mut()
            .expect("failed to open stdin")
            .write_all(
                std::fs::read(file_name)
                    .expect("could not read file")
                    .as_slice(),
            )
            .unwrap();

        child.wait_with_output().unwrap()
    }

    pub fn from_file(file_name: &str) -> Self {
        if file_name.ends_with(".bril") {
            let output = Self::spawn_process_and_get_output("bril2json", file_name);
            let program = serde_json::from_str(&String::from_utf8(output.stdout).unwrap()).unwrap();
            return program;
        }

        let file = File::open(file_name).unwrap();
        let reader = BufReader::new(file);
        let program = serde_json::from_reader(reader).unwrap();
        program
    }

    #[allow(dead_code)]
    pub fn from_str(program: &str) -> Self {
        serde_json::from_str(program).unwrap()
    }

    #[allow(dead_code)]
    pub fn from_stdin() -> Self {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf).unwrap();
        serde_json::from_str(&buf).unwrap()
    }

    #[allow(dead_code)]
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    #[allow(dead_code)]
    pub fn to_file(&self, file_name: &str) {
        // if the file extension ends in .bril, write to tmp file, convert to text, and then write to file
        if file_name.ends_with(".bril") {
            let tmp_file = tempfile::NamedTempFile::new().unwrap();
            let tmp_file_path = tmp_file.path().to_str().unwrap();
            std::fs::write(tmp_file_path, self.to_string()).unwrap();

            let output = Self::spawn_process_and_get_output("bril2txt", tmp_file_path);
            std::fs::write(file_name, output.stdout).unwrap();
            return;
        }

        let file = File::create(file_name).unwrap();
        serde_json::to_writer_pretty(file, self).unwrap();
    }
}
