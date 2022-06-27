use std::{cell::RefCell, rc::Rc};

use env::Env;
use linefeed::{Interface, ReadResult};
use object::Object;

mod lexer;
mod object;
mod parser;
mod eval;
mod env;

const PROMPT: &str = "lisp-rs> ";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reader = Interface::new(PROMPT).unwrap();
    let mut env = Rc::new(RefCell::new(Env::new()));
    let mut current_source = "".to_string();
    let mut unclosed_lparen: i32 = 0;
    while let ReadResult::Input(input) = reader.read_line().unwrap() {
        if input.eq("exit") {
            break;
        }

        unclosed_lparen = unclosed_lparen + input.chars().fold(0,|a,b|  {if b == '(' { a + 1 } else if b == ')' { a - 1 } else { a }});
        current_source = current_source + " " + &input;
        if unclosed_lparen > 0 {
            continue;
        }
        
        let val = eval::eval(current_source.as_ref(), &mut env)?;
        println!("val: {}", val);
        match val {
            Object::Void => {},
            Object::Integer(n) => println!("{}", n),
            Object::Bool(b) => println!("{}", b),
            Object::Symbol(s) => println!("{}", s),
            Object::Lambda(params, body) => {
                println!("(lambda (");
                for (i, param) in params.iter().enumerate() {
                    if i == param.len() - 1 {
                        println!("{}", param);
                    } else {
                        println!("{} ", param);
                    }
                }
                println!(")");
                for expr in body {
                    println!(" {}", expr);
                }
            }
            _ => println!("{}", val)
        }
        current_source = String::new();
    }

    println!("Good bye");
    Ok(())
}