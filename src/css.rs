use cssparser::{Parser, ParseError, ParserInput, ToCss, Token};
use std::cell::RefCell;

use crate::deferral::DeferralKind;
use crate::deferral::Deferral;


enum ParseState {
    Basic,
    Nested,
    UrlFunction
}

pub struct Output {
    pub css: String,
    pub deferrals: Vec<Deferral>
}

struct State {
    state: ParseState,
    output: String,
    deferrals: Vec<Deferral>,
}

impl State {
    fn pushstr(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn push(&mut self, token: &Token) {
        self.pushstr(&token.to_css_string());
    }

    fn defer(&mut self, kind: DeferralKind, data: String) -> String {
        let i = self.deferrals.len();
        let s = kind.as_str();

        self.deferrals.push(Deferral {
            kind: kind,
            i: i,
            data: data
        });

        format!("/*DEFER:{}:{}*/", s, i)
    }
}


fn _do_block<'a>(in_url: bool,
                 token: Token,
                 state: &RefCell<State>,
                 parser: &mut Parser)
{
    state.borrow_mut().push(&token);

    parser.parse_nested_block(|p| _rewrite_css(in_url, state, p));

    state.borrow_mut().pushstr(
        match token {
            Token::Function(_) => ")",
            Token::ParenthesisBlock => ")",
            Token::SquareBracketBlock => "}",
            _ => "}"
        }
    );
}


fn _rewrite_css<'a>(in_url: bool, state: &RefCell<State>, parser: &mut Parser)
    -> Result<(), ParseError<'a, String>>
{
    // https://github.com/Y2Z/monolith/blob/master/src/css.rs#L131
    loop {
        match parser.next_including_whitespace() {
            Ok(token) => {
                //println!("TOKEN {:?}", token);
                match token {
                    Token::QuotedString(s) => {
                        if !in_url {
                            state.borrow_mut().push(token);
                        } else {
                            let s = format!("url({})",
                                state.borrow_mut().defer(
                                    DeferralKind::QuotedUrl,
                                    s.to_string()
                                )
                            );
                            state.borrow_mut().pushstr(s.as_str());
                        }
                    },
                    Token::UnquotedUrl(e) => {
                        let s = format!("url({})",
                            state.borrow_mut().defer(
                                DeferralKind::UnquotedUrl,
                                e.to_string()
                            )
                        );
                        state.borrow_mut().pushstr(s.as_str());
                    },
                    Token::Function(s) => {
                        let is_url = s.to_string().to_lowercase() == "url";
                        _do_block(is_url, token.clone(), state, parser);
                    },
                    Token::ParenthesisBlock |
                    Token::SquareBracketBlock |
                    Token::CurlyBracketBlock => {
                        _do_block(false, token.clone(), state, parser);
                    },
                    _ => state.borrow_mut().push(token)
                };
            },
            Err(_) => {
                break;
            }
        };
    }

    Ok(())
}


pub fn rewrite_css(css: &str)
    -> Result<Output, ParseError<String>>
{
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);
    let state = RefCell::new(
        State {
            state: ParseState::Basic,
            output: String::new(),
            deferrals: Vec::new(),
        }
    );

    match _rewrite_css(false, &state, &mut parser) {
        Ok(_) => {
            let state_ = state.into_inner();
            Ok(Output {
                css: state_.output,
                deferrals: state_.deferrals,
            })
        },
        Err(e) => Err(e)
    }
}
