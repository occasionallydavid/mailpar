use std::collections::HashSet;

use lol_html::html_content::Element;
use lol_html::{element, Settings};


const PERMITTED_HTML_TAGS: &str = "link html head style body a abbr acronym address area b bdo big blockquote br button caption center cite code col colgroup dd del dfn dir div dl dt em fieldset font form h1 h2 h3 h4 h5 h6 hr i img input ins kbd label legend li map menu ol optgroup option p pre q s samp select small span strike strong sub sup table tbody td textarea tfoot th thead u tr tt u ul var";


const PERMITTED_HTML_ATTRS: &str = "align alt aria-hidden aria-label bgcolor border cellpadding cellspacing class color colspan dir height hspace id lang rel href role src style type valign vspace width background";


#[derive(Debug)]
pub enum DeferralKind {
    StyleLink,
    StyleInline,
    Source,
    ImageLink,
}


pub struct Deferral {
    pub kind: DeferralKind,
    pub i: usize,
    pub data: String
}


pub struct Output {
    pub html: String,
    pub text_content: String,
    pub page_links: Vec<String>,
    pub deferrals: Vec<Deferral>
}


pub fn rewrite_html(s: &str) -> Result<Output, lol_html::errors::RewritingError> {
    let permitted_html_tags: HashSet<&str> = HashSet::from_iter(
        PERMITTED_HTML_TAGS.split(' ')
    );

    let permitted_html_attrs: HashSet<&str> = HashSet::from_iter(
        PERMITTED_HTML_ATTRS.split(' ')
    );

    let mut style_links = Vec::new();
    let mut sources = Vec::new();
    let mut backgrounds = Vec::new();
    let mut inline_styles = Vec::new();

    let mut inline_style = String::new();
    let mut text_content = String::new();
    let mut page_links = Vec::new();

    let defer = |d: &mut Vec<Deferral>, kind, data| {
        let i = d.len();
        let s = match &kind {
            DeferralKind::StyleLink => "StyleLink",
            DeferralKind::StyleInline => "StyleInline",
            DeferralKind::Source => "Source",
            DeferralKind::ImageLink => "ImageLink",
        };

        d.push(Deferral {
            kind: kind,
            i: i,
            data: data
        });

        format!("<!--DEFER:{}:{}-->", s, i)
    };

    let result = lol_html::rewrite_str(s, Settings {
        document_content_handlers: vec![
            // Remove DOCTYPE
            lol_html::doctype!(|dt| {
                dt.remove();
                Ok(())
            }),

            // Remove all comments
            lol_html::doc_comments!(|comment| {
                comment.remove();
                Ok(())
            }),
        ],

        element_content_handlers: vec![
            // Strip scripts
            element!("script", |elem| {
                elem.remove();
                Ok(())
            }),

            // Strip invalid elems
            element!("*", |elem| {
                if !permitted_html_tags.contains(elem.tag_name().as_str()) {
                    println!("REMOVE BAD TAG: {}", elem.tag_name());
                    elem.remove_and_keep_content();
                    return Ok(());
                }

                let mut v = Vec::new();
                for attr in elem.attributes() {
                    let name = attr.name();
                    if !permitted_html_attrs.contains(name.as_str()) {
                        v.push(name);
                    }
                }

                for name in v {
                    println!("REMOVE BAD ATTR: {}", name);
                    elem.remove_attribute(name.as_str());
                }

                Ok(())
            }),

            // transform_link()
            element!("link", |elem| {
                match elem.get_attribute("rel") {
                    None => {
                        println!("drop <link> with no rel");
                        elem.remove();
                        return Ok(());
                    },
                    Some(rel) => {
                        if !rel.eq_ignore_ascii_case("stylesheet") {
                            println!("drop non-style <link>: rel={}", rel);
                            elem.remove();
                            return Ok(());
                        }
                    }
                };

                let href = match elem.get_attribute("href") {
                    None => {
                        println!("drop <link> with no href");
                        elem.remove();
                        return Ok(());
                    },
                    Some(href) => {
                        if !href.starts_with("http") {
                            println!("drop non-http <link>: href={}", href);
                            elem.remove();
                            return Ok(());
                        }
                        href
                    }
                };

                elem.replace(
                    defer(&mut style_links,
                          DeferralKind::StyleLink, href).as_str(),
                    lol_html::html_content::ContentType::Html
                );

                Ok(())
            }),

            // inline styles
            lol_html::text!("style", |text| {
                if inline_style.len() == 0 {
                    inline_style += text.as_str();
                    text.replace(
                        defer(&mut inline_styles,
                              DeferralKind::StyleInline, "".to_string()).as_str(),
                        lol_html::html_content::ContentType::Html
                    );
                } else {
                    inline_style += text.as_str();
                    text.remove();
                    if text.last_in_text_node() {
                        inline_styles.last_mut().unwrap().data = inline_style.clone();
                        println!("WTF {}", inline_style);
                        inline_style.clear();
                    }
                }
                Ok(())
            }),

            lol_html::text!("*", |text| {
                if text.text_type() ==  lol_html::html_content::TextType::Data && !text.removed() {
                    if text.as_str().len() > 0 {
                        if text_content.len() != 0 {
                            text_content.push(' ');
                        }
                        text_content += text.as_str();
                    }
                }
                Ok(())
            }),

            element!("a[href]", |elem| {
                elem.set_attribute("target", "_blank")?;
                elem.set_attribute("rel", "noopener noreferrer")?;
                page_links.push(
                    html_escape::decode_html_entities(
                        elem.get_attribute("href").unwrap().as_str()
                    ).into_owned()
                );
                Ok(())
            }),

            element!("*[background]", |elem| {
                let bg = elem.get_attribute("background").unwrap();
                elem.set_attribute("background", defer(&mut backgrounds, DeferralKind::Source, bg).as_str());
                Ok(())
            }),

            element!("*[src]", |elem| {
                let src = elem.get_attribute("src").unwrap();
                elem.set_attribute("src", defer(&mut sources, DeferralKind::Source, src).as_str());
                Ok(())
            }),
        ],
        ..Settings::default()
    });

    style_links.append(&mut sources);
    style_links.append(&mut backgrounds);
    style_links.append(&mut inline_styles);

    match result {
        Ok(s) => {
            let mut text = String::new();
            html_escape::decode_html_entities_to_string(
                text_content.as_str(),
                &mut text
            );

            Ok(Output {
                html: s,
                text_content: text,
                page_links: page_links,
                deferrals: style_links,
            })
        },
        Err(e) => {
            Err(e)
        }
    }
}
