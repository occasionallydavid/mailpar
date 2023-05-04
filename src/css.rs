use cssparser::{CowRcStr, Parser, ParseError, ParserInput, ToCss, Token};
use std::cell::RefCell;


pub enum DeferralKind {
    Import,
    Font,
    Image,
}

pub struct Deferral {
    pub kind: DeferralKind,
    pub i: usize,
    pub data: String
}

pub struct Output {
    pub css: String,
    pub deferrals: Vec<Deferral>
}


fn _rewrite_css<'a>(start: Option<&Token>,
                    deferrals: &RefCell<Vec<Deferral>>,
                    parser: &mut Parser)
    -> Result<String, ParseError<'a, String>>
{
    let output = RefCell::new(String::new());

    let pushstr = |s: &str| output.borrow_mut().push_str(s);
    let push = |tok: &Token| pushstr(tok.to_css_string().as_str());

    let defer = |kind, data| {
        let mut d = deferrals.borrow_mut();
        let i = d.len();
        let s = match &kind {
            DeferralKind::Import => "Import",
            DeferralKind::Font => "Font",
            DeferralKind::Image => "Image",
        };

        d.push(Deferral {kind: kind, i: i, data: data});
        format!("/*DEFER:{}:{}*/", s, i)
    };

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
                        push(
                            &(Token::UnquotedUrl(CowRcStr::from(
                                defer(DeferralKind::Image, e.to_string())
                                .as_str()
                            )))
                        );
                        continue;
                    },
                    Token::Function(_) |
                    Token::ParenthesisBlock |
                    Token::SquareBracketBlock |
                    Token::CurlyBracketBlock => {
                        let tc = token.clone();
                        push(&tc); // WHYYYYY
                        match parser.parse_nested_block(
                            |p| _rewrite_css(Some(&tc), deferrals, p)
                        ) {
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
            Err(_) => {
                //println!("DOINK {:?}", e);
                break;
            }
        };
    }

    Ok(output.into_inner())
}


pub fn rewrite_css(css: &str)
    -> Result<Output, ParseError<String>>
{
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);
    let mut deferrals = RefCell::new(Vec::new());

    match _rewrite_css(None, &deferrals, &mut parser) {
        Ok(css) => return Ok(Output {
            css: css,
            deferrals: deferrals.into_inner(),
        }),
        Err(e) => return Err(e),
    }
}
