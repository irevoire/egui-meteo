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
use miette::SourceSpan;

use crate::lexer::{LexingError, Token};

#[derive(Debug, Clone, thiserror::Error, miette::Diagnostic)]
#[error("{kind}")]
#[diagnostic(help("try doing this instead"))]
pub struct Error {
    #[source_code]
    src: String,

    #[diagnostic_source]
    kind: ErrorKind,
}

#[derive(Debug, Clone, thiserror::Error, miette::Diagnostic)]
pub enum ErrorKind {
    #[error(transparent)]
    Lexer(#[from] LexingError),

    #[error("Expected primary expression, found {0}")]
    ExpectedPrimary(Cow<'static, str>),

    #[error("Missing closing parenthesis")]
    MissingParens {
        #[label("Opening parenthesis")]
        left: SourceSpan,
        #[label("Missing parenthesis")]
        right: SourceSpan,
    },
}

type Result<T, E = Error> = std::result::Result<T, E>;

pub struct Parser<'a> {
    source: &'a str,
    lexer: Lexer<'a, Token<'a>>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let lexer = Token::lexer(input);
        Parser {
            source: input,
            lexer,
        }
    }

    pub fn primary_error(&self, s: impl Into<Cow<'static, str>>) -> Error {
        self.error(ErrorKind::ExpectedPrimary(s.into()))
    }

    pub fn error(&self, kind: ErrorKind) -> Error {
        Error {
            src: self.source.into(),
            kind,
        }
    }

    pub fn parse_expression(&mut self) -> Result<Expression> {
        self.parse_primary()
    }

    pub fn parse_primary(&mut self) -> Result<Expression> {
        match self
            .lexer
            .next()
            .ok_or_else(|| self.primary_error("EoF"))?
            .map_err(|err| self.primary_error(err.to_string()))?
        {
            Token::Ident(_) => Ok(Expression::Primary(Literal::String(self.lexer.span()))),
            Token::Number(n) => Ok(Expression::Primary(Literal::Number(self.lexer.span(), n))),
            Token::LeftParens => {
                let left = self.lexer.span();
                let expr = self.parse_expression()?;
                let next = self.lexer.next();

                let right = self.lexer.span();
                let error = self.error(ErrorKind::MissingParens {
                    left: SourceSpan::new(left.start.into(), left.end - left.start),
                    right: SourceSpan::new(right.start.into(), right.end - right.start),
                });

                let next = next.ok_or(error.clone())?.map_err(|_| error.clone())?;
                if next != Token::RightParens {
                    return Err(error);
                };

                Ok(Expression::Group {
                    opening_paren: left,
                    expression: Box::new(expr),
                    closing_paren: self.lexer.span(),
                })
            }
            other => Err(self.primary_error(format!("{:?}: `{}`", other, self.lexer.slice()))),
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
    use miette::IntoDiagnostic;

    use super::*;

    #[test]
    fn test_literal() {
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

    #[test]
    fn error_mismatch_parens() {
        miette::set_hook(Box::new(|_| {
            Box::new(
                miette::MietteHandlerOpts::new()
                    .context_lines(2)
                    .color(false)
                    .build(),
            )
        }))
        .unwrap();

        let input = "(1";
        let mut parser = Parser::new(input);
        let error = parser.parse_primary().into_diagnostic().unwrap_err();
        let error = format!("{error:?}");
        insta::assert_snapshot!(error, @r###"
          × Missing closing parenthesis
        "###);
    }
}
