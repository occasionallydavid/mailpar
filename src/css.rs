use cssparser::{CowRcStr, Parser, ParseError, ParserInput, ToCss, Token};
use std::cell::RefCell;


pub struct Deferral {
    pub data: String
}


fn _rewrite_css<'a>(parser: &mut Parser)
    -> Result<String, ParseError<'a, String>>
{
    let output = RefCell::new(String::new());
    let mut deferrals = Vec::new();

    let pushstr = |s: &str| output.borrow_mut().push_str(s);
    let push = |tok: &Token| pushstr(tok.to_css_string().as_str());

    let ws = Token::WhiteSpace(" ");

    loop {
        let t = parser.next_including_whitespace();
        match t {
            Ok(token) => {
                //let t2 = token.clone();
                match token {
                    Token::WhiteSpace(_) => {
                        push(&ws);
                        continue;
                    },
                    Token::UnquotedUrl(e) => {
                        deferrals.push(Deferral { data: e.to_string() });
                        push(&(Token::UnquotedUrl(CowRcStr::from("FEEP"))));
                        continue;
                    },
                    Token::Function(_) |
                    Token::ParenthesisBlock |
                    Token::SquareBracketBlock |
                    Token::CurlyBracketBlock => {
                        let tc = token.clone();
                        push(&tc); // WHYYYYY
                        match parser.parse_nested_block(_rewrite_css) {
                            Ok(s) => pushstr(s.as_str()),
                            Err(_) => break,
                        }
                        pushstr(match tc {
                            Token::Function(_) => ")",
                            Token::ParenthesisBlock => ")",
                            Token::SquareBracketBlock => "}",
                            _ => "}"
                        });
                        continue;
                    },
                    _ => {},
                };
                //println!("EEK {:?}", token);
                push(token);
            },
            Err(e) => {
                println!("DOINK {:?}", e);
                break;
            }
        };
    }

    Ok(output.into_inner())
}


pub fn rewrite_css(css: &str) -> String {
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);

    match _rewrite_css(&mut parser) {
        Ok(s) => return s,
        Err(_) => return String::new(),
    }
}
