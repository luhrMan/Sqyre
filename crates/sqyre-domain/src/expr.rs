//! Math expression evaluation.
//!
//! Hand-rolled recursive-descent parser — avoids `meval` → ancient `nom`.

use crate::{Macro, ScalarValue};
use std::f64::consts::{E, PI};

type Result<T> = std::result::Result<T, String>;

/// Evaluate a math expression after `${var}` substitution.
/// Supports `+ - * / ^`, unary `+/-`, parentheses, functions (`sqrt`, `abs`,
/// `round`, `floor`, `ceil`, `trunc`, `sin`, `cos`, `tan`, `ln`), and `~pi` / `~e`.
pub fn evaluate_expression(expr: &str, macro_: &Macro) -> Result<f64> {
    let resolved = crate::expand_variable_refs(expr, macro_)?;
    evaluate_numeric(&resolved)
}

fn evaluate_numeric(expr: &str) -> Result<f64> {
    let mut expr = expr.trim().to_string();
    if expr.is_empty() {
        return Err("empty expression".into());
    }
    expr = expr.replace("~pi", &format!("{PI}"));
    expr = expr.replace("~e", &format!("{E}"));

    let mut p = Parser::new(&expr);
    let val = p.parse_expr()?;
    p.skip_ws();
    if p.pos < p.bytes.len() {
        return Err(format!(
            "failed to evaluate expression: unexpected input at {:?}",
            &expr[p.pos..]
        ));
    }
    Ok(val)
}

/// Store calculate results preferring integers when the value is whole.
pub fn numeric_to_scalar(f: f64) -> ScalarValue {
    if f.fract() == 0.0 && f.is_finite() && f.abs() <= i64::MAX as f64 {
        ScalarValue::Int(f as i64)
    } else {
        ScalarValue::Float(f)
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            bytes: s.as_bytes(),
            pos: 0,
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.bytes.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        self.skip_ws();
        let c = self.bytes.get(self.pos).copied()?;
        self.pos += 1;
        Some(c)
    }

    fn expect(&mut self, want: u8) -> Result<()> {
        match self.bump() {
            Some(c) if c == want => Ok(()),
            other => Err(format!(
                "failed to evaluate expression: expected '{}', got {:?}",
                want as char,
                other.map(|c| c as char)
            )),
        }
    }

    /// expr := term (('+' | '-') term)*
    fn parse_expr(&mut self) -> Result<f64> {
        let mut left = self.parse_term()?;
        loop {
            match self.peek() {
                Some(b'+') => {
                    self.bump();
                    left += self.parse_term()?;
                }
                Some(b'-') => {
                    self.bump();
                    left -= self.parse_term()?;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    /// term := power (('*' | '/') power)*
    fn parse_term(&mut self) -> Result<f64> {
        let mut left = self.parse_power()?;
        loop {
            match self.peek() {
                Some(b'*') => {
                    self.bump();
                    left *= self.parse_power()?;
                }
                Some(b'/') => {
                    self.bump();
                    let right = self.parse_power()?;
                    left /= right;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    /// power := unary ('^' power)?  — right-associative
    fn parse_power(&mut self) -> Result<f64> {
        let base = self.parse_unary()?;
        if self.peek() == Some(b'^') {
            self.bump();
            let exp = self.parse_power()?;
            Ok(base.powf(exp))
        } else {
            Ok(base)
        }
    }

    /// unary := ('+' | '-') unary | primary
    fn parse_unary(&mut self) -> Result<f64> {
        match self.peek() {
            Some(b'+') => {
                self.bump();
                self.parse_unary()
            }
            Some(b'-') => {
                self.bump();
                Ok(-self.parse_unary()?)
            }
            _ => self.parse_primary(),
        }
    }

    /// primary := number | ident '(' expr ')' | '(' expr ')'
    fn parse_primary(&mut self) -> Result<f64> {
        match self.peek() {
            Some(b'(') => {
                self.bump();
                let v = self.parse_expr()?;
                self.expect(b')')?;
                Ok(v)
            }
            Some(b'0'..=b'9') | Some(b'.') => self.parse_number(),
            Some(b'a'..=b'z') | Some(b'A'..=b'Z') | Some(b'_') => self.parse_call_or_ident(),
            other => Err(format!(
                "failed to evaluate expression: unexpected {:?}",
                other.map(|c| c as char)
            )),
        }
    }

    fn parse_number(&mut self) -> Result<f64> {
        self.skip_ws();
        let start = self.pos;
        while self.pos < self.bytes.len()
            && (self.bytes[self.pos].is_ascii_digit() || self.bytes[self.pos] == b'.')
        {
            self.pos += 1;
        }
        // scientific notation
        if self.pos < self.bytes.len()
            && (self.bytes[self.pos] == b'e' || self.bytes[self.pos] == b'E')
        {
            self.pos += 1;
            if self.pos < self.bytes.len()
                && (self.bytes[self.pos] == b'+' || self.bytes[self.pos] == b'-')
            {
                self.pos += 1;
            }
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }
        let s = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| "failed to evaluate expression: invalid number".to_string())?;
        s.parse::<f64>()
            .map_err(|_| format!("failed to evaluate expression: invalid number {s:?}"))
    }

    fn parse_call_or_ident(&mut self) -> Result<f64> {
        self.skip_ws();
        let start = self.pos;
        while self.pos < self.bytes.len()
            && (self.bytes[self.pos].is_ascii_alphanumeric() || self.bytes[self.pos] == b'_')
        {
            self.pos += 1;
        }
        let name = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| "failed to evaluate expression: bad ident".to_string())?
            .to_ascii_lowercase();

        if self.peek() != Some(b'(') {
            return Err(format!(
                "failed to evaluate expression: unknown identifier {name:?}"
            ));
        }
        self.bump(); // '('
        let arg = self.parse_expr()?;
        self.expect(b')')?;
        apply_fn(&name, arg)
    }
}

fn apply_fn(name: &str, x: f64) -> Result<f64> {
    let v = match name {
        "sqrt" => x.sqrt(),
        "abs" => x.abs(),
        "round" => x.round(),
        "floor" => x.floor(),
        "ceil" => x.ceil(),
        "trunc" => x.trunc(),
        "sin" => x.sin(),
        "cos" => x.cos(),
        "tan" => x.tan(),
        "ln" => x.ln(),
        _ => {
            return Err(format!(
                "failed to evaluate expression: unknown function {name:?}"
            ));
        }
    };
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_basic_and_functions() {
        let macro_ = Macro::new("t", 0, vec![]);
        assert_eq!(evaluate_expression("1+2*3", &macro_).unwrap(), 7.0);
        assert_eq!(evaluate_expression("abs(-4)", &macro_).unwrap(), 4.0);
        assert_eq!(evaluate_expression("floor(3.9)", &macro_).unwrap(), 3.0);
        assert_eq!(evaluate_expression("ceil(3.1)", &macro_).unwrap(), 4.0);
        assert_eq!(evaluate_expression("trunc(3.9)", &macro_).unwrap(), 3.0);
        assert_eq!(evaluate_expression("round(2.5)", &macro_).unwrap(), 3.0);
        assert!((evaluate_expression("sqrt(4)", &macro_).unwrap() - 2.0).abs() < 1e-9);
        assert!((evaluate_expression("2^3", &macro_).unwrap() - 8.0).abs() < 1e-9);
        assert!((evaluate_expression("(1+2)*3", &macro_).unwrap() - 9.0).abs() < 1e-9);
    }

    #[test]
    fn substitutes_vars_and_constants() {
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.variables.set("x", ScalarValue::Int(-5));
        assert_eq!(evaluate_expression("${x}+3", &macro_).unwrap(), -2.0);
        assert_eq!(evaluate_expression("${x}-250", &macro_).unwrap(), -255.0);
        assert_eq!(evaluate_expression("-30", &macro_).unwrap(), -30.0);
        let pi = evaluate_expression("~pi", &macro_).unwrap();
        assert!((pi - PI).abs() < 1e-6);
        let e = evaluate_expression("~e", &macro_).unwrap();
        assert!((e - E).abs() < 1e-6);
    }

    #[test]
    fn numeric_to_scalar_prefers_int() {
        assert_eq!(numeric_to_scalar(3.0), ScalarValue::Int(3));
        assert_eq!(numeric_to_scalar(3.5), ScalarValue::Float(3.5));
    }
}
