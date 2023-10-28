use chumsky::prelude::*;

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Eq,
    Comma,
    Key(String),
    Value(String),
    End,
}

fn lexer() -> impl Parser<char, Vec<Token>, Error = Simple<char>> {
    let eq = just('=').to(Token::Eq);
    let comma = just(',').to(Token::Comma);
    let key = text::ident().map(Token::Key);
    let value = none_of("\"")
        .repeated()
        .delimited_by(just('"'), just('"'))
        .map(|chars| {
            let string: String = chars.into_iter().collect();
            Token::Value(string)
        });
    let end = end().to(Token::End);
    let pad = choice((just(' ').ignored(),)).repeated();

    let token = choice((eq, comma, key, value)).padded_by(pad);
    token.repeated().then(end).map(|(mut tokens, end)| {
        tokens.push(end);
        tokens
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer() {
        let lexer = lexer();
        let src = r#"Bearer a="b", c="d" "#;
        let tokens = lexer.parse(src).unwrap();
        assert_eq!(
            tokens,
            [
                Token::Key("Bearer".into()),
                Token::Key("a".into()),
                Token::Eq,
                Token::Value("b".into()),
                Token::Comma,
                Token::Key("c".into()),
                Token::Eq,
                Token::Value("d".into()),
                Token::End,
            ]
        );
    }
}
