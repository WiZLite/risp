use std::{rc::Rc, cell::{RefCell, Ref}};

use crate::{object::Object, env::Env, parser::parse};

fn eval_obj(obj: &Object, env: &mut Rc<RefCell<Env>>) -> Result<Object, String> {
    match obj {
        Object::Void => Ok(Object::Void),
        Object::Lambda(_params, _body) => Ok(Object::Void),
        Object::Bool(_) => Ok(obj.clone()),
        Object::Integer(n) => Ok(Object::Integer(*n)),
        Object::Symbol(s) => eval_symbol(s, env),
        Object::List(list) => eval_list(list, env)
    }
}

fn eval_symbol(s: &str, env: &mut Rc<RefCell<Env>>) -> Result<Object, String> {
    let val = env.borrow_mut().get(s);
    if val.is_none() {
        return Err(format!("Unbound symbol: {}", s));
    } else {
        Ok(val.unwrap().clone())
    }
}

fn eval_list(list: &Vec<Object>, env: &mut Rc<RefCell<Env>>) -> Result<Object, String> {
    let head = &list[0];
    match head {
        Object::Symbol(s) => match s.as_str() {
            "define" => eval_define(&list, env),
            "+" | "-" | "*" | "/" | "<" | ">" | "=" | "!=" => eval_binary_op(&list, env),
            "if" => eval_if(&list, env),
            "lambda" => eval_function_definition(&list),
            _ => eval_function_call(s, &list, env)
        },
        _ => {
            let mut new_list = Vec::new();
            for obj in list {
                let result = eval_obj(obj, env)?;
                match result {
                    Object::Void => {},
                    _ => new_list.push(result)
                }
            }
            Ok(Object::List(new_list))
        }
    }
}

pub fn eval(program: &str, env: &mut Rc<RefCell<Env>>) -> Result<Object, String> {
    let parsed_list = parse(program);
    if parsed_list.is_err() {
        return Err(format!("{}", parsed_list.err().unwrap()));
    }
    eval_obj(&parsed_list.unwrap(), env)
}

fn eval_define(list: &Vec<Object>, env: &mut Rc<RefCell<Env>>) -> Result<Object, String> {
    if list.len() != 3 {
        return Err(format!("Invalid number of arguments for define"));
    }
    let sym = match &list[1] {
        Object::Symbol(s) => s.clone(),
        _ => return Err(format!("Invalid define"))
    };
    let val = eval_obj(&list[2], env)?;
    env.borrow_mut().set(&sym, val);
    Ok(Object::Void)
}

fn eval_binary_op(list: &Vec<Object>, env: &mut Rc<RefCell<Env>>) -> Result<Object, String> {
    let operator = list[0].clone();
    let left = eval_obj(&list[1].clone(), env)?;
    let right = eval_obj(&list[2].clone(), env)?;
    let left_val = match left {
        Object::Integer(n) => n,
        _ => return Err(format!("Left operand must be an integer {:?}", left))
    };
    let right_val = match right {
        Object::Integer(n) => n,
        _ => return Err(format!("Right operand must be an integer {:?}", right))
    };
    match operator {
        Object::Symbol(s) => match s.as_str() {
            "+" => Ok(Object::Integer(left_val + right_val)),
            "-" => Ok(Object::Integer(left_val - right_val)),
            "*" => Ok(Object::Integer(left_val * right_val)),
            "/" => Ok(Object::Integer(left_val / right_val)),
            "<" => Ok(Object::Bool(left_val < right_val)),
            ">" => Ok(Object::Bool(left_val > right_val)),
            "=" => Ok(Object::Bool(left_val == right_val)),
            "!=" => Ok(Object::Bool(left_val != right_val)),
            _ => Err(format!("Invalid infix operator: {}", s))
        },
        _ => Err(format!("Operator must be a symbol")),
    }
}

fn eval_if(list: &Vec<Object>, env: &mut Rc<RefCell<Env>>) -> Result<Object, String> {
    let cond_obj = eval_obj(&list[1], env)?;
    let cond = match cond_obj {
        Object::Bool(b) => b,
        _ => return Err(format!("Condition must be a boolean"))
    };

    if cond == true {
        return eval_obj(&list[2], env);
    } else {
        return eval_obj(&list[3], env);
    }
}

fn eval_function_definition(list: &Vec<Object>) -> Result<Object, String> {
    let params = match &list[1] {
        Object::List(list) => {
            let mut params = Vec::new();
            for param in list {
                match param {
                    Object::Symbol(s) => params.push(s.clone()),
                    _ => return Err(format!("Invalid lambda parameter"))
                }
            }
            params
        },
        _ => return Err(format!("Invalid lambda"))
    };
    let body = match &list[2] {
        Object::List(list) => list.clone(),
        _ => return Err(format!("Invalid lambda"))
    };
    Ok(Object::Lambda(params, body))
}

fn eval_function_call(name: &str, list: &Vec<Object>, env: &mut Rc<RefCell<Env>>) -> Result<Object, String> {
    let lambda = env.borrow_mut().get(name);
    if lambda.is_none() {
        return Err(format!("Unbound symbol: {}", name));
    }
    let func = lambda.unwrap();
    if let Object::Lambda(params, body) = func {
        let mut new_env = Rc::new(RefCell::new(Env::extend(env.clone())));
        for (i, param) in params.iter().enumerate() {
            let val = eval_obj(&list[i + 1], env)?;
            new_env.borrow_mut().set(param, val);
        }
        return eval_obj(&Object::List(body), &mut new_env);
    } else {
        return Err(format!("Not a lambda: {}", name));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_add() {
        let mut env = Rc::new(RefCell::new(Env::new()));
        let result = eval("(+ 1 2)", &mut env).unwrap();
        assert_eq!(result, Object::Integer(3))
    }

    #[test]
    fn test_area_of_a_circle() {
        let mut env = Rc::new(RefCell::new(Env::new()));
        let program = "( (define r 10) (define pi 314) (* pi (* r r)) )";
        let result = eval(program, &mut env).unwrap();
        assert_eq!(result, Object::List(vec![Object::Integer((314 * 10 * 10) as i64)]))
    }
    
    #[test]
    fn test_sqr_function() {
        let mut env = Rc::new(RefCell::new(Env::new()));
        let program = "( (define sqr (lambda (r) (* r r))) (sqr 10) )";
        let result = eval(program, &mut env).unwrap();
        assert_eq!(result, Object::List(vec![Object::Integer(100)]));
    }

    #[test]
    fn test_factorial() {
        let mut env = Rc::new(RefCell::new(Env::new()));
        let program = "
            (
                (define fact (lambda (n) (if (< n 1) 1 (* n (fact (- n 1))))))
                (fact 5)
            )
        ";

        let result = eval(program, &mut env).unwrap();
        assert_eq!(result, Object::List(vec![Object::Integer((120) as i64)]));
    }
}