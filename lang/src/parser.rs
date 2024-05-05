/*
data
  |> filter (fun year_data -> yer_data.date.month == "Dec" || yer_data.date.month < "Mar")
  |> split (fun point -> point.date.month == "Aug")
  |> foreach (fun data -> data
    |> map (fun point -> point.temperature)
    |> draw
  )
*/

use std::borrow::Cow;

use logos::{Lexer, Logos, Span};

use crate::lexer::{LexingError, Token};

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum Error {
    #[error(transparent)]
    Lexer(#[from] LexingError),

    #[error("Expected primary expression, found {0}")]
    ExpectedPrimary(Cow<'static, str>),

    #[error("Missing closing parenthesis")]
    MissingParens {
        #[label("Opening parenthesis")]
        left: Span,
        #[label("Missing parenthesis")]
        right: Span,
    },
}

impl Error {
    pub fn primary(s: impl Into<Cow<'static, str>>) -> Self {
        Self::ExpectedPrimary(s.into())
    }
}

type Result<T, E = Error> = std::result::Result<T, E>;

pub struct Parser<'a> {
    lexer: Lexer<'a, Token<'a>>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let lexer = Token::lexer(input);
        Parser { lexer }
    }

    pub fn parse_expression(&mut self) -> Result<Expression> {
        todo!()
    }

    pub fn parse_primary(&mut self) -> Result<Expression> {
        match self.lexer.next().ok_or(Error::primary("EoF"))?? {
            Token::Ident(_) => Ok(Expression::Primary(Literal::String(self.lexer.span()))),
            Token::Number(n) => Ok(Expression::Primary(Literal::Number(self.lexer.span(), n))),
            Token::LeftParens => {
                let left = self.lexer.span();
                let expr = self.parse_expression()?;
                let next = self.lexer.next().ok_or(Error::primary("EoF"))??;
                if next != Token::RightParens {
                    return Err(Error::MissingParens {
                        left,
                        right: self.lexer.span(),
                    });
                };

                Ok(Expression::Group {
                    opening_paren: left,
                    expression: Box::new(expr),
                    closing_paren: self.lexer.span(),
                })
            }
            other => Err(Error::primary(format!(
                "{:?}: `{}`",
                other,
                self.lexer.slice()
            ))),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Expression {
    Function(Function),
    BinaryOp(BinOp),
    Primary(Literal),
    Group {
        opening_paren: Span,
        expression: Box<Expression>,
        closing_paren: Span,
    },
}

impl Expression {
    pub fn short_name(&self) -> &'static str {
        match self {
            Expression::Function(_) => "function",
            Expression::BinaryOp(binop) => binop.short_name(),
            Expression::Primary(primary) => primary.short_name(),
            Expression::Group {
                opening_paren,
                expression,
                closing_paren,
            } => "parens",
        }
    }

    pub fn unwrap_literal(self) -> Literal {
        match self {
            Expression::Primary(lit) => lit,
            expr => panic!("Unwraped literal, but {} was found", expr.short_name()),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Literal {
    String(Span),
    Bool(Span, bool),
    Number(Span, f64),
}

impl Literal {
    pub fn short_name(&self) -> &'static str {
        match self {
            Literal::String(_) => "string",
            Literal::Bool(_, _) => "boolean",
            Literal::Number(_, _) => "number",
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum BinOp {
    Reverse,
    Equal,
    InferiorOrEqual,
    SuperiorOrEqual,
    StrictInferior,
    StrictSuperior,
    LogicalOr,
    LogicalAnd,
}

impl BinOp {
    pub fn short_name(&self) -> &'static str {
        match self {
            BinOp::Reverse => "|>",
            BinOp::Equal => "==",
            BinOp::InferiorOrEqual => "<=",
            BinOp::SuperiorOrEqual => ">=",
            BinOp::StrictInferior => "<",
            BinOp::StrictSuperior => ">",
            BinOp::LogicalOr => "||",
            BinOp::LogicalAnd => "&&",
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Function {
    fun: Span,
    name: Option<Span>,
    ret: Span,
    body: Box<Expression>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn unit() {
        for (input, ret) in [
            ("data", Literal::String(0.."data".len())),
            ("Date(", Literal::String(0.."Date".len())),
            ("foreach", Literal::String(0.."foreach".len())),
            ("map ", Literal::String(0.."map".len())),
            ("_hello", Literal::String(0.."_hello".len())),
            ("he_llo", Literal::String(0.."he_llo".len())),
            ("tamo_du_30", Literal::String(0.."tamo_du_30".len())),
            ("30tamo", Literal::Number(0.."30".len(), 30.0)),
            ("0tam", Literal::Number(0.."0".len(), 0.0)),
        ] {
            let mut parser = Parser::new(input);
            let lit = parser.parse_primary().unwrap().unwrap_literal();
            assert_eq!(lit, ret);
        }
    }
}
