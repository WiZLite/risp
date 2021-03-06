use std::cell::RefCell;
use std::clone;
use std::cmp::Ordering;
use std::rc::Rc;

use crate::{env::Env, object::Object, parser::parse};

fn eval_obj(obj: &Object, env: &mut Rc<RefCell<Env>>) -> Result<Object, String> {
    let mut current_obj = Box::new(obj.clone());
    let mut current_env = env.clone();
    loop {
        match *current_obj {
            Object::List(list) => {
                let head = &list[0];
                match head {
                    Object::BinaryOp(_) => {
                        return eval_binary_op(&list, &mut current_env);
                    }
                    Object::Keyword(_) => {
                        return eval_keyword(&list, &mut current_env);
                    }
                    Object::If => {
                        if list.len() != 4 {
                            return Err(format!("Invalid number of arguments for if statement"));
                        }
                        let cond_obj = eval_obj(&list[1], &mut current_env)?;
                        let cond = match cond_obj {
                            Object::Bool(b) => b,
                            _ => return Err(format!("Condition must be a boolean")),
                        };
                        if cond {
                            current_obj = Box::new(list[2].clone());
                        } else {
                            current_obj = Box::new(list[3].clone())
                        }
                        continue;
                    }
                    Object::Symbol(s) => {
                        let lambda = current_env.borrow_mut().get(s);
                        if lambda.is_none() {
                            return Err(format!("Unbound function: {}", s));
                        }
                        let func = lambda.unwrap();
                        match func {
                            Object::Lambda(params, body, lambda_env) => {
                                let new_env =
                                    Rc::new(RefCell::new(Env::extend(lambda_env.clone())));
                                for (i, param) in params.iter().enumerate() {
                                    let val = eval_obj(&list[i + 1], &mut current_env)?;
                                    new_env.borrow_mut().set(param, val);
                                }
                                current_obj = Box::new(Object::List(body));
                                current_env = new_env.clone();
                                continue;
                            }
                            _ => return Err(format!("Not a lambda: {}", s)),
                        }
                    }
                    _ => {
                        let mut new_list = Vec::new();
                        for obj in list {
                            let result = eval_obj(&obj, &mut current_env)?;
                            match result {
                                Object::Void => {}
                                _ => new_list.push(result),
                            }
                        }
                        return Ok(Object::List(new_list));
                    }
                }
            }
            Object::Symbol(s) => {
                return eval_symbol(&s, &mut current_env);
            }
            Object::Void => return Ok(Object::Void),
            Object::Lambda(_, _, _) => return Ok(obj.clone()),
            Object::Bool(_) => return Ok(obj.clone()),
            Object::Integer(n) => return Ok(Object::Integer(n)),
            Object::Float(n) => return Ok(Object::Float(n)),
            Object::String(s) => return Ok(Object::String(s.to_string())),
            Object::ListData(l) => return Ok(Object::ListData(l.to_vec())),
            _ => return Err(format!("Invalid object: {:?},", obj)),
        }
    }
    /*
    match obj {
        Object::Symbol(s) => eval_symbol(s, env),
        Object::List(list) => eval_list(list, env),
        _ => Ok(obj.clone())
    }
     */
}

fn eval_symbol(s: &str, env: &mut Rc<RefCell<Env>>) -> Result<Object, String> {
    let val = match s {
        "true" => return Ok(Object::Bool(true)),
        "false" => return Ok(Object::Bool(false)),
        "nil" => return Ok(Object::Void),
        _ => env.borrow_mut().get(s),
    };
    if val.is_none() {
        return Err(format!("Unbound symbol: {}", s));
    } else {
        Ok(val.unwrap())
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
        _ => return Err(format!("Invalid define")),
    };
    let val = eval_obj(&list[2], env)?;
    env.borrow_mut().set(&sym, val);
    Ok(Object::Void)
}

fn eval_list_data(list: &Vec<Object>, env: &mut Rc<RefCell<Env>>) -> Result<Object, String> {
    let mut elms = Vec::new();
    for obj in list[1..].iter() {
        elms.push(eval_obj(obj, env)?);
    }
    Ok(Object::ListData(elms))
}

fn eval_map(list: &Vec<Object>, env: &mut Rc<RefCell<Env>>) -> Result<Object, String> {
    if list.len() != 3 {
        return Err(format!(
            "Invalid arity for map function. map accepts only two args."
        ));
    }
    let lambda = eval_obj(&list[1], env)?;
    let coll = eval_obj(&list[2], env)?;
    let (params, body, lambda_env) = match lambda {
        Object::Lambda(p, b, e) => {
            if p.len() != 1 {
                return Err(format!(
                    "Invalid number of parameters for map lambda function {:?}",
                    p
                ));
            }
            (p, b, e)
        }
        _ => return Err(format!("Not a lambda while evaluating map: {}", lambda)),
    };

    let items = match coll {
        Object::ListData(list) => list,
        _ => return Err(format!("Invalid map arguments: {:?}", list)),
    };

    let first_arg = &params[0];
    let mut result_list = Vec::new();
    for item in items.iter() {
        let val = eval_obj(&item, env)?;
        let mut new_env = Rc::new(RefCell::new(Env::extend(lambda_env.clone())));
        new_env.borrow_mut().set(&first_arg, val);
        let new_body = body.clone();
        let result = eval_obj(&Object::List(new_body), &mut new_env)?;
        result_list.push(result);
    }
    Ok(Object::ListData(result_list))
}

fn eval_filter(list: &Vec<Object>, env: &mut Rc<RefCell<Env>>) -> Result<Object, String> {
    if list.len() != 3 {
        return Err(format!(
            "Invalid arity for filter function. filter accepts only 2 args"
        ));
    }
    let lambda = eval_obj(&list[1], env)?;
    let coll = eval_obj(&list[2], env)?;
    let (params, body, lambda_env) = match lambda {
        Object::Lambda(p, b, e) => {
            if p.len() != 1 {
                return Err(format!(
                    "Invalid number of parameters for filter lambda function {:?}",
                    p
                ));
            }
            (p, b, e)
        }
        _ => return Err(format!("Not a lambda while evaluating filter {:?}", lambda)),
    };

    let items = match coll {
        Object::ListData(list) => list,
        _ => {
            return Err(format!(
                "Invalid filter arguments. Second argument must be a list but found : {:?}",
                list
            ))
        }
    };

    let first_arg = &params[0];
    let mut result_list = Vec::new();
    for item in items.iter() {
        let val = eval_obj(&item, env)?;
        let mut new_env = Rc::new(RefCell::new(Env::extend(lambda_env.clone())));
        new_env.borrow_mut().set(&first_arg, val.clone());
        let new_body = body.clone();
        match eval_obj(&Object::List(new_body), &mut new_env)? {
            Object::Bool(b) => {
                if b {
                    result_list.push(val);
                }
            }
            _ => continue,
        }
    }
    Ok(Object::ListData(result_list))
}

fn eval_reduce(list: &Vec<Object>, env: &mut Rc<RefCell<Env>>) -> Result<Object, String> {
    if list.len() != 4 {
        return Err(format!(
            "Invalid arity for reduce function. reduce accepts only 3 args"
        ));
    }
    let lambda = eval_obj(&list[1], env)?;
    let coll = eval_obj(&list[3], env)?;
    let (params, body, lambda_env) = match lambda {
        Object::Lambda(p, b, e) => {
            if p.len() != 2 {
                return Err(format!(
                    "Invalid number of parameters for reduce lambda function {:?}",
                    p
                ));
            }
            (p, b, e)
        }
        _ => return Err(format!("Not a lambda whle evaluating reduce {:?}", lambda)),
    };
    let items = match coll {
        Object::ListData(data) => data,
        _ => {
            return Err(format!(
                "Invalid reduce argumetns. Second arguments must be a list but found {:?}",
                coll
            ))
        }
    };
    let arg_a = &params[0];
    let arg_b = &params[1];
    let mut a = eval_obj(&list[2], env)?;
    for i in 0..items.len() {
        let b = eval_obj(&items[i], env)?;
        let new_env = Rc::new(RefCell::new(Env::extend(lambda_env.clone())));
        new_env.borrow_mut().set(arg_a, a);
        new_env.borrow_mut().set(arg_b, b);
        a = eval_obj(&Object::List(body.clone()), &mut new_env.clone())?;
    }
    Ok(a)
}

fn eval_binary_op(list: &Vec<Object>, env: &mut Rc<RefCell<Env>>) -> Result<Object, String> {
    let operator = list[0].clone();
    let left = &eval_obj(&list[1].clone(), env)?;
    let right = &eval_obj(&list[2].clone(), env)?;
    match operator {
        Object::BinaryOp(s) => match s.as_str() {
            "+" => match (left, right) {
                (Object::Integer(l), Object::Integer(r)) => Ok(Object::Integer(l + r)),
                (Object::Integer(l), Object::Float(r)) => Ok(Object::Float(*l as f64 + r)),
                (Object::Float(l), Object::Float(r)) => Ok(Object::Float(l + r)),
                (Object::Float(l), Object::Integer(r)) => Ok(Object::Float(l + *r as f64)),
                (Object::String(l), Object::String(r)) => Ok(Object::String(l.to_owned() + r)),
                _ => Err(format!("Invalid types for + operator {} {}", left, right)),
            },
            "-" => match (left, right) {
                (Object::Integer(l), Object::Integer(r)) => Ok(Object::Integer(l - r)),
                (Object::Float(l), Object::Float(r)) => Ok(Object::Float(l - r)),
                (Object::Integer(l), Object::Float(r)) => Ok(Object::Float(*l as f64 - r)),
                (Object::Float(l), Object::Integer(r)) => Ok(Object::Float(l - *r as f64)),
                _ => Err(format!("Invalid types for - operator {} {}", left, right)),
            },
            "*" => match (left, right) {
                (Object::Integer(l), Object::Integer(r)) => Ok(Object::Integer(l * r)),
                (Object::Float(l), Object::Float(r)) => Ok(Object::Float(l * r)),
                (Object::Integer(l), Object::Float(r)) => Ok(Object::Float(*l as f64 * r)),
                (Object::Float(l), Object::Integer(r)) => Ok(Object::Float(l * (*r) as f64)),
                _ => Err(format!("Invalid types for * operator {} {}", left, right)),
            },
            "/" => match (left, right) {
                (Object::Integer(l), Object::Integer(r)) => Ok(Object::Integer(l / r)),
                (Object::Float(l), Object::Float(r)) => Ok(Object::Float(l / r)),
                (Object::Integer(l), Object::Float(r)) => Ok(Object::Float(*l as f64 / r)),
                (Object::Float(l), Object::Integer(r)) => Ok(Object::Float(l / (*r) as f64)),
                _ => Err(format!("Invalid types for / operator {} {}", left, right)),
            },
            "%" => match (left, right) {
                (Object::Integer(l), Object::Integer(r)) => Ok(Object::Integer(l % r)),
                (Object::Float(l), Object::Float(r)) => Ok(Object::Float(l % r)),
                (Object::Integer(l), Object::Float(r)) => Ok(Object::Float(*l as f64 % r)),
                (Object::Float(l), Object::Integer(r)) => Ok(Object::Float(l % (*r) as f64)),
                _ => Err(format!("Invalid types for % operator {} {}", left, right)),
            },
            "<" => match (left, right) {
                (Object::Integer(l), Object::Integer(r)) => Ok(Object::Bool(l < r)),
                (Object::Float(l), Object::Float(r)) => Ok(Object::Bool(l < r)),
                (Object::Integer(l), Object::Float(r)) => Ok(Object::Bool((*l as f64) < *r)),
                (Object::Float(l), Object::Integer(r)) => Ok(Object::Bool(l < &(*r as f64))),
                (Object::String(l), Object::String(r)) => {
                    Ok(Object::Bool(l.cmp(&r) == Ordering::Less))
                }
                _ => Err(format!("Invalid types for < operator {} {}", left, right)),
            },
            ">" => match (left, right) {
                (Object::Integer(l), Object::Integer(r)) => Ok(Object::Bool(l > r)),
                (Object::Float(l), Object::Float(r)) => Ok(Object::Bool(l > r)),
                (Object::Integer(l), Object::Float(r)) => Ok(Object::Bool(*l as f64 > *r)),
                (Object::Float(l), Object::Integer(r)) => Ok(Object::Bool(l > &(*r as f64))),
                (Object::String(l), Object::String(r)) => {
                    Ok(Object::Bool(l.cmp(&r) == Ordering::Greater))
                }
                _ => Err(format!("Invalid types for > operator {} {}", left, right)),
            },
            "=" => match (left, right) {
                (Object::Integer(l), Object::Integer(r)) => Ok(Object::Bool(l == r)),
                (Object::String(l), Object::String(r)) => Ok(Object::Bool(l == r)),
                _ => Err(format!("Invalid types for == operator {} {}", left, right)),
            },
            "!=" => match (left, right) {
                (Object::Integer(l), Object::Integer(r)) => Ok(Object::Bool(l != r)),
                (Object::Float(l), Object::Float(r)) => Ok(Object::Bool(l != r)),
                (Object::Integer(l), Object::Float(r)) => Ok(Object::Bool(*l as f64 != *r)),
                (Object::Float(l), Object::Integer(r)) => Ok(Object::Bool(*l != (*r) as f64)),
                (Object::String(l), Object::String(r)) => {
                    Ok(Object::Bool(l.cmp(&r) != Ordering::Equal))
                }
                _ => Err(format!("Invalid types for != operator {} {}", left, right)),
            },
            "&" => match (left, right) {
                (Object::Bool(l), Object::Bool(r)) => Ok(Object::Bool(*l && *r)),
                _ => Err(format!("Invalid types for & operator {} {}", left, right)),
            },
            "|" => match (left, right) {
                (Object::Bool(l), Object::Bool(r)) => Ok(Object::Bool(*l || *r)),
                _ => Err(format!("Invalid types for | operator {} {}", left, right)),
            },
            _ => Err(format!("Invalid infix operator: {}", s)),
        },
        _ => Err(format!("Operator must be a symbol")),
    }
}

fn eval_function_definition(
    list: &Vec<Object>,
    env: &mut Rc<RefCell<Env>>,
) -> Result<Object, String> {
    let params = match &list[1] {
        Object::List(list) => {
            let mut params = Vec::new();
            for param in list {
                match param {
                    Object::Symbol(s) => params.push(s.clone()),
                    _ => return Err(format!("Invalid lambda parameter")),
                }
            }
            params
        }
        _ => return Err(format!("Invalid lambda")),
    };
    let body = match &list[2] {
        Object::List(list) => list.clone(),
        _ => return Err(format!("Invalid lambda")),
    };
    Ok(Object::Lambda(params, body, env.clone()))
}

fn eval_function_call(
    name: &str,
    list: &Vec<Object>,
    env: &mut Rc<RefCell<Env>>,
) -> Result<Object, String> {
    let lambda = env.borrow_mut().get(name);
    if lambda.is_none() {
        return Err(format!("Unbound symbol: {}", name));
    }
    let func = lambda.unwrap();
    return if let Object::Lambda(params, body, closure_env) = func {
        let mut new_env = Rc::new(RefCell::new(Env::extend(closure_env.clone())));
        for (i, param) in params.iter().enumerate() {
            let val = eval_obj(&list[i + 1], env)?;
            new_env.borrow_mut().set(param, val);
        }
        eval_obj(&Object::List(body), &mut new_env)
    } else {
        Err(format!("Not a lambda: {}", name))
    };
}

fn eval_keyword(list: &Vec<Object>, env: &mut Rc<RefCell<Env>>) -> Result<Object, String> {
    let head = &list[0];
    match head {
        Object::Keyword(s) => match s.as_str() {
            "define" => eval_define(&list, env),
            "list" => eval_list_data(&list, env),
            "print" => print_list(&list, env),
            "lambda" => eval_function_definition(&list, env),
            "map" => eval_map(&list, env),
            "filter" => eval_filter(&list, env),
            "reduce" => eval_reduce(&list, env),
            _ => Err(format!("Invalid keyword: {}", head)),
        },
        _ => Err(format!("Invalid keyword: {}", head)),
    }
}

fn print_list(list: &Vec<Object>, env: &mut Rc<RefCell<Env>>) -> Result<Object, String> {
    let mut new_list = Vec::new();

    for obj in list[1..].iter() {
        new_list.push(eval_obj(obj, env)?);
    }
    for obj in new_list.iter() {
        print!("{} ", obj);
    }
    println!("");
    Ok(Object::Void)
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
        assert_eq!(
            result,
            Object::List(vec![Object::Integer((314 * 10 * 10) as i64)])
        )
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

    #[test]
    fn test_map() {
        let mut env = Rc::new(RefCell::new(Env::new()));
        let program = "\
        (
            (define sqr (lambda (r) (* r r)))
            (define coll (list 1 2 3 4 5))
            (map sqr coll)
        )
        ";
        let result = eval(program, &mut env).unwrap();
        assert_eq!(
            result,
            Object::List(vec![Object::ListData(vec![
                Object::Integer(1),
                Object::Integer(4),
                Object::Integer(9),
                Object::Integer(16),
                Object::Integer(25),
            ])])
        )
    }

    #[test]
    fn test_reduce() {
        let mut env = Rc::new(RefCell::new(Env::new()));
        let program = "(
            (define add (lambda (a b) (+ a b)))
            (define coll (list 1 2 3 4 5))
            (reduce add 0 coll)
        )";
        let result = eval(program, &mut env).unwrap();
        assert_eq!(result, Object::List(vec![Object::Integer(15)]))
    }

    #[test]
    fn test_sum_n() {
        let mut env = Rc::new(RefCell::new(Env::new()));
        let program = "(
            (define sum-n  
              (lambda (n a)
                (if (= n 0) 
                  a
                  (sum-n (- n 1) (+ n a))
                )))
            (sum-n 100 0)
        )";
        let result = eval(program, &mut env).unwrap();
        assert_eq!(result, Object::List(vec![Object::Integer(5050)]))
    }

    #[test]
    fn test_closure() {
        let mut env = Rc::new(RefCell::new(Env::new()));
        let program = "(
            (define add  
              (lambda (a)
                (lambda (b) (+ a b))))
            (add 10 20)
        )";
        let result = eval(program, &mut env).unwrap();
        assert_eq!(result, Object::List(vec![Object::Integer(30)]))
    }
}
