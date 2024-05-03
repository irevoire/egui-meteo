use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\n\f]+")] // Ignore this regex pattern between tokens
enum Token {
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

    #[regex(r"[a-zA-Z_]+[a-zA-Z0-9]*")]
    Ident,
    #[regex(r"[0-9]+(\.[0-9]+)*")]
    Number,
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

            assert_eq!(lex.next(), Some(Ok(Token::Ident)));
            assert!(ident.starts_with(lex.slice()));
        }
    }

    #[test]
    fn number() {
        for ident in ["0", "123456789", "123.123"] {
            let mut lex = Token::lexer(ident);

            assert_eq!(
                lex.next(),
                Some(Ok(Token::Number)),
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
