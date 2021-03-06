use std::{fmt, rc::Rc, cell::RefCell};

use crate::env::Env;

#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    Void,
    Integer(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Symbol(String),
    Keyword(String),
    If,
    BinaryOp(String),
    Lambda(Vec<String>, Vec<Object>, Rc<RefCell<Env>>),
    List(Vec<Object>),
    ListData(Vec<Object>),
}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Object::Void => write!(f, "Void"),
            Object::Integer(n) => write!(f, "{}", n),
            Object::Float(f_) => write!(f, "{}", f_),
            Object::Bool(b) => write!(f, "{}", b),
            Object::String(s) => write!(f, "{}", s),
            Object::Symbol(s) => write!(f, "{}", s),
            Object::Keyword(s) => write!(f, "{}", s),
            Object::If => write!(f, "if"),
            Object::BinaryOp(s) => write!(f, "{}", s),
            Object::Lambda(params, body, _) => {
                write!(f, "Lambda(")?;
                for param in params {
                    write!(f, "{} ", param)?;
                }
                write!(f, ")")?;
                for expr in body {
                    write!(f, " {}", expr)?;
                }
                Ok(())
            }
            Object::List(list) => {
                write!(f, "(")?;
                for (i, obj) in list.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", obj)?;
                }
                write!(f, ")")
            }
            Object::ListData(list) => {
                write!(f, "(")?;
                for (i, obj) in list.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", obj)?;
                }
                write!(f, ")")
            }
        }
    }
}