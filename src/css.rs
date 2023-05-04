use cssparser::{Parser, ParseError, ParserInput, ToCss, Token};
use std::collections::HashSet;
use std::cell::RefCell;


enum ParseState {
    Basic,
    Nested,
    UrlFunction
}

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
        let s = match &kind {
            DeferralKind::Import => "Import",
            DeferralKind::Font => "Font",
            DeferralKind::Image => "Image",
        };

        self.deferrals.push(Deferral {
            kind: kind,
            i: i,
            data: data
        });

        format!("/*DEFER:{}:{}*/", s, i)
    }
}


lazy_static! {
    static ref CSS_PROPS_WITH_IMAGE_URLS: HashSet<&'static str> = {
        HashSet::from_iter([
            // Universal
            "background",
            "background-image",
            "border-image",
            "border-image-source",
            "content",
            "cursor",
            "list-style",
            "list-style-image",
            "mask",
            "mask-image",
            // Specific to @counter-style
            "additive-symbols",
            "negative",
            "pad",
            "prefix",
            "suffix",
            "symbols",
        ])
    };
}


pub fn is_image_url_prop(prop: &str) -> bool {
    let lower = prop.to_lowercase();
    CSS_PROPS_WITH_IMAGE_URLS.contains(lower.as_str())
}


fn _rewrite_css<'a>(state: &RefCell<State>, parser: &mut Parser)
    -> Result<(), ParseError<'a, String>>
{
    // https://github.com/Y2Z/monolith/blob/master/src/css.rs#L131
    loop {
        match parser.next_including_whitespace() {
            Ok(token) => {
                //println!("TOKEN {:?}", token);
                match token {
                    Token::WhiteSpace(_) => {
                        state.borrow_mut().pushstr(" ");
                    },
                    Token::UnquotedUrl(e) => {
                        let s = format!("url({})",
                            state.borrow_mut().defer(
                                DeferralKind::Image,
                                e.to_string()
                            )
                        );
                        state.borrow_mut().pushstr(s.as_str());
                    },
                    Token::Function(_) |
                    Token::ParenthesisBlock |
                    Token::SquareBracketBlock |
                    Token::CurlyBracketBlock => {
                        state.borrow_mut().push(&token);
                        let tc = token.clone(); // WHYYYYY

                        match parser.parse_nested_block(
                            |p| _rewrite_css(state, p)
                        ) {
                            Ok(s) => {},
                            Err(_) => break,
                        }
                        state.borrow_mut().pushstr(
                            match tc {
                                Token::Function(_) => ")",
                                Token::ParenthesisBlock => ")",
                                Token::SquareBracketBlock => "}",
                                _ => "}"
                            }
                        );
                    },
                    _ => state.borrow_mut().push(token)
                };
            },
            Err(e) => {
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

    match _rewrite_css(&state, &mut parser) {
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
