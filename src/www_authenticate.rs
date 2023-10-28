use std::{collections::HashMap, str::FromStr};

use chumsky::prelude::*;
use getset::Getters;
use once_cell::sync::Lazy;

#[allow(clippy::type_complexity)]
static LEXER: Lazy<Box<dyn Parser<char, Vec<Token>, Error = Simple<char>> + Sync + Send>> =
    Lazy::new(|| Box::new(lexer()));
static PARSER: Lazy<Box<dyn Parser<Token, WwwAuthenticate, Error = Simple<Token>> + Sync + Send>> =
    Lazy::new(|| Box::new(parser()));

#[derive(Debug, Getters)]
pub struct WwwAuthenticate {
    #[getset(get = "pub")]
    scheme: String,
    #[getset(get = "pub")]
    pairs: HashMap<String, String>,
}

impl FromStr for WwwAuthenticate {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let tokens = LEXER.parse(s).map_err(|_| ())?;
        PARSER.parse(tokens).map_err(|_| ())
    }
}

fn parser() -> impl Parser<Token, WwwAuthenticate, Error = Simple<Token>> {
    // scheme <- key
    let scheme = filter(|token| matches!(token, Token::Key(_))).map(|token| {
        if let Token::Key(key) = token {
            return key;
        }
        panic!()
    });

    // pair <- key eq value
    let pair = filter(|token| matches!(token, Token::Key(_)))
        .map(|token| {
            if let Token::Key(key) = token {
                return key;
            }
            panic!()
        })
        .then(just(Token::Eq).ignored())
        .then(
            filter(|token| matches!(token, Token::Value(_))).map(|token| {
                if let Token::Value(value) = token {
                    return value;
                }
                panic!()
            }),
        )
        .map(|((key, _), value)| (key, value));

    // s <- scheme ( pair )comma* $
    scheme
        .then(pair.separated_by(just(Token::Comma)))
        .then_ignore(just(Token::End))
        .map(|(scheme, pairs)| {
            let mut map = HashMap::new();
            pairs.into_iter().for_each(|(key, value)| {
                map.insert(key, value);
            });
            WwwAuthenticate { scheme, pairs: map }
        })
}

#[derive(Debug, Clone, PartialEq, Eq, std::hash::Hash)]
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

    #[test]
    fn test_parser() {
        let parser = parser();
        let lexer = lexer();
        let src = r#"Bearer a="b", c="d" "#;
        let tokens = lexer.parse(src).unwrap();
        let ast = parser.parse(tokens).unwrap();
        assert_eq!(ast.scheme, "Bearer");
        assert_eq!(ast.pairs.get("a").unwrap(), "b");
        assert_eq!(ast.pairs.get("c").unwrap(), "d");
    }
}
