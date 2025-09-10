use std::{
    fs::File,
    io::{self, BufReader, Read, Write},
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
        op: String,
        dest: String,
        #[serde(rename = "type")]
        constant_type: Type,
        value: Literal,
    },
    Value {
        // TODO: replace string op with ValueOp enums
        op: String,
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
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Copy)]
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
    Call,
    Ret,
    Print,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Type {
    Int,
    Bool,
    Float,
    Char,
    Ptr(Box<Self>),
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
    Float(f64),
    Char(char),
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
