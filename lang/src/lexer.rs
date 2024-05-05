use logos::Logos;
use std::num::ParseFloatError;

#[derive(Clone, Debug, Default, thiserror::Error, PartialEq, Eq)]
pub enum LexingError {
    #[default]
    #[error("Other")]
    Other,

    #[error(transparent)]
    NumberError(#[from] ParseFloatError),
}

#[derive(Logos, Debug, PartialEq)]
#[logos(error = LexingError)]
#[logos(skip r"[ \t\n\f]+")] // Ignore this regex pattern between tokens
pub enum Token<'a> {
    // First class operators
    #[token("|>")]
    RightTriangle,
    #[token("->")]
    RightArrow,

    // Conditions
    #[token("==")]
    Equal,
    #[token("<=")]
    InferiorOrEqual,
    #[token("<")]
    StrictInferior,
    #[token(">=")]
    SuperiorOrEqual,
    #[token(">")]
    StrictSuperior,
    #[token("||")]
    LogicalOr,
    #[token("&&")]
    LogicalAnd,

    // Unary/Binary operators
    #[token("-")]
    Minus,
    #[token("+")]
    Plus,
    #[token("/")]
    Slash,
    #[token("*")]
    Star,
    #[token("!")]
    Bang,

    // Parens
    #[token("(")]
    LeftParens,
    #[token(")")]
    RightParens,

    // Misc
    #[token(".")]
    Dot,
    #[token("\"")]
    DoubleQuote,

    #[regex(r"[a-zA-Z_]+[a-zA-Z0-9]*", |lex| lex.slice())]
    Ident(&'a str),
    // We parse too many `.123` on purpose to return the right error message on float with three or more `.`
    #[regex("-?[0-9]+(\\.[0-9]+)*", |lex| lex.slice().parse())]
    Number(f64),
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn identifier() {
        for ident in [
            "data",
            "Date",
            "Date(",
            "foreach",
            "split",
            "map ",
            "_hello",
            "he_llo",
            "tamo_du_30",
        ] {
            let mut lex = Token::lexer(ident);

            assert_eq!(lex.next(), Some(Ok(Token::Ident(lex.slice()))));
            assert!(ident.starts_with(lex.slice()));
        }
    }

    #[test]
    fn number() {
        for (ident, ret) in [("0", 0.0), ("123456789", 123456789.0), ("123.123", 123.123)] {
            let mut lex = Token::lexer(ident);

            assert!(
                matches!(lex.next(), Some(Ok(Token::Number(n))) if n == ret),
                "error on {}",
                lex.slice()
            );
            assert!(ident.starts_with(lex.slice()));
        }
    }

    #[test]
    fn operators() {
        for (ident, ret) in [
            ("|>", Token::RightTriangle),
            ("->", Token::RightArrow),
            ("==", Token::Equal),
            ("<=", Token::InferiorOrEqual),
            ("<", Token::StrictInferior),
            (">=", Token::SuperiorOrEqual),
            (">", Token::StrictSuperior),
            ("||", Token::LogicalOr),
            ("&&", Token::LogicalAnd),
        ] {
            let mut lex = Token::lexer(ident);

            assert_eq!(lex.next(), Some(Ok(ret)));
            assert!(ident.starts_with(lex.slice()));
        }
    }

    #[test]
    fn plot_data_of_february() {
        let input = r###"
data
  |> filter (fun year_data -> yer_data.date.month == "Feb")
  |> split (fun point -> point.date.month == "Aug")
  |> foreach (fun data -> data
    |> map (fun point -> point.temperature)
    |> draw
  )
        "###;
        let lex = Token::lexer(input);

        for token in lex {
            assert!(token.is_ok());
        }
    }
}
