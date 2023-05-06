use std::collections::HashSet;
use std::cell::RefCell;

use lol_html::{element, Settings};
use lol_html::html_content::ContentType;

use crate::css::rewrite_css;
use crate::deferral::DeferralKind;
use crate::deferral::Deferral;

pub struct Output {
    pub html: String,
    pub text_content: String,
    pub page_links: Vec<String>,
    pub deferrals: Vec<Deferral>,

    pub st_doctype_removed: u32,
    pub st_comment_removed: u32,
    pub st_script_removed: u32,
    pub st_invalid_tag_removed: u32,
    pub st_invalid_attr_removed: u32,
    pub st_link_no_rel_removed: u32,
    pub st_link_non_stylesheet_removed: u32,
    pub st_link_no_href_removed: u32,
    pub st_link_non_http_removed: u32,
    pub st_anchors_rewritten: u32,
    pub st_inline_style_skipped: u32,
    pub st_style_attr_skipped: u32,
}


lazy_static! {
    static ref PERMITTED_HTML_TAGS: HashSet<&'static str> = {
        HashSet::from_iter([
            "link", "html", "head", "style", "body", "a", "abbr", "acronym",
            "address", "area", "b", "bdo", "big", "blockquote", "br",
            "button", "caption", "center", "cite", "code", "col", "colgroup",
            "dd", "del", "dfn", "dir", "div", "dl", "dt", "em", "fieldset",
            "font", "form", "h1", "h2", "h3", "h4", "h5", "h6", "hr", "i",
            "img", "input", "ins", "kbd", "label", "legend", "li", "map",
            "menu", "ol", "optgroup", "option", "p", "pre", "q", "s", "samp",
            "select", "small", "span", "strike", "strong", "sub", "sup",
            "table", "tbody", "td", "textarea", "tfoot", "th", "thead", "u",
            "tr", "tt", "u", "ul", "var",

            // https://www.emailonacid.com/blog/article/email-development/image-map-support-in-html-email/ ; used by some spam
            "area", "map",
        ])
    };

    static ref PERMITTED_HTML_ATTRS: HashSet<&'static str> = {
        HashSet::from_iter([
            "align", "alt", "aria-hidden", "aria-label", "bgcolor", "border",
            "cellpadding", "cellspacing", "class", "color", "colspan", "dir",
            "height", "hspace", "id", "lang", "rel", "href", "role", "src",
            "style", "type", "valign", "vspace", "width", "background",

            // for <area>
            "usemap", "name", "shape", "coords",
        ])
    };
}


pub fn rewrite_html(s: &str) -> Result<Output, lol_html::errors::RewritingError> {
    let mut style_links = Vec::new();
    let mut style_attrs = Vec::new();
    let mut sources = Vec::new();
    let mut backgrounds = Vec::new();
    let mut inline_styles = Vec::new();

    let mut inline_style = String::new();
    let text_content = RefCell::new(String::new());
    let mut page_links = Vec::new();

    let mut st_doctype_removed = 0;
    let mut st_comment_removed = 0;
    let mut st_script_removed = 0;
    let mut st_invalid_tag_removed = 0;
    let mut st_invalid_attr_removed = 0;
    let mut st_link_no_rel_removed = 0;
    let mut st_link_non_stylesheet_removed = 0;
    let mut st_link_no_href_removed = 0;
    let mut st_link_non_http_removed = 0;
    let mut st_anchors_rewritten = 0;
    let mut st_inline_style_skipped = 0;
    let mut st_style_attr_skipped = 0;

    let defer = |d: &mut Vec<Deferral>, kind: DeferralKind, data| {
        let i = d.len();
        let s = kind.as_str();

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
                st_doctype_removed += 1;
                Ok(())
            }),

            // Remove all comments
            lol_html::doc_comments!(|comment| {
                comment.remove();
                st_comment_removed += 1;
                Ok(())
            }),
        ],

        element_content_handlers: vec![
            // Strip scripts
            element!("script", |elem| {
                elem.remove();
                st_script_removed += 1;
                Ok(())
            }),

            // Strip invalid elems
            element!("*", |elem| {
                if !PERMITTED_HTML_TAGS.contains(elem.tag_name().as_str()) {
                    //println!("REMOVE BAD TAG: {}", elem.tag_name());
                    elem.remove_and_keep_content();
                    st_invalid_tag_removed += 1;
                    return Ok(());
                }

                let mut v = Vec::new();
                for attr in elem.attributes() {
                    let name = attr.name();
                    if !PERMITTED_HTML_ATTRS.contains(name.as_str()) {
                        v.push(name);
                    }
                }

                for name in v {
                    //println!("REMOVE BAD ATTR: {}", name);
                    elem.remove_attribute(name.as_str());
                    st_invalid_attr_removed += 1;
                }

                Ok(())
            }),

            // transform_link()
            element!("link", |elem| {
                match elem.get_attribute("rel") {
                    None => {
                        //println!("drop <link> with no rel");
                        elem.remove();
                        st_link_no_rel_removed += 1;
                        return Ok(());
                    },
                    Some(rel) => {
                        if !rel.eq_ignore_ascii_case("stylesheet") {
                            //println!("drop non-style <link>: rel={}", rel);
                            elem.remove();
                            st_link_non_stylesheet_removed += 1;
                            return Ok(());
                        }
                    }
                };

                let href = match elem.get_attribute("href") {
                    None => {
                        //println!("drop <link> with no href");
                        elem.remove();
                        st_link_no_href_removed += 1;
                        return Ok(());
                    },
                    Some(href) => {
                        if !href.starts_with("http") {
                            //println!("drop non-http <link>: href={}", href);
                            elem.remove();
                            st_link_non_http_removed += 1;
                            return Ok(());
                        }
                        href
                    }
                };

                elem.replace(
                    defer(&mut style_links,
                          DeferralKind::StyleLink, href).as_str(),
                    ContentType::Html
                );

                Ok(())
            }),

            element!("[style]", |elem| {
                let data = html_escape::decode_html_entities(
                    elem.get_attribute("style").unwrap().as_str()
                ).into_owned();

                // TODO escaping
                let mut output = rewrite_css(data.as_str()).unwrap();
                elem.set_attribute("style", output.css.as_str());
                style_attrs.append(&mut output.deferrals);

                Ok(())
            }),

            element!("style", |el| {
                //println!("REMOVING STYLE");
                el.remove();
                Ok(())
            }),

            // inline styles
            lol_html::text!("style", |text| {
                inline_style += text.as_str();

                if !text.last_in_text_node() {
                    text.remove();
                    return Ok(());
                }

                let mut output = rewrite_css(inline_style.as_str()).unwrap();
                let mut x = String::new();
                x += "<style>";
                x += output.css.as_str();
                x += "</style>";
                text.replace(x.as_str(), ContentType::Html);
                inline_styles.append(&mut output.deferrals);

                inline_style.clear();
                Ok(())
            }),

            element!("br", |_elem| {
                text_content.borrow_mut().push('\n');
                Ok(())
            }),

            lol_html::text!("*", |text| {
                if text.text_type() == lol_html::html_content::TextType::Data && !text.removed() {
                    let s = text.as_str().trim();
                    if s.len() > 0 {
                        (*text_content.borrow_mut()) += s;
                        (*text_content.borrow_mut()) += " ";
                    }
                }
                Ok(())
            }),

            element!("area[href], a[href]", |elem| {
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
                let bg = html_escape::decode_html_entities(
                    elem.get_attribute("background").unwrap().as_str()
                ).into_owned();
                elem.set_attribute("background", defer(&mut backgrounds, DeferralKind::Source, bg).as_str());
                Ok(())
            }),

            element!("*[src]", |elem| {
                let src = html_escape::decode_html_entities(
                    elem.get_attribute("src").unwrap().as_str()
                ).into_owned();
                elem.set_attribute("src", defer(&mut sources, DeferralKind::Source, src).as_str());
                Ok(())
            }),
        ],
        ..Settings::default()
    });

    style_links.append(&mut sources);
    style_links.append(&mut backgrounds);
    style_links.append(&mut inline_styles);
    style_links.append(&mut style_attrs);

    match result {
        Ok(s) => {
            let mut text = String::new();
            html_escape::decode_html_entities_to_string(
                text_content.into_inner().as_str(),
                &mut text
            );

            Ok(Output {
                html: s,
                text_content: text,
                page_links: page_links,
                deferrals: style_links,

                st_doctype_removed: st_doctype_removed,
                st_comment_removed: st_comment_removed,
                st_script_removed: st_script_removed,
                st_invalid_tag_removed: st_invalid_tag_removed,
                st_invalid_attr_removed: st_invalid_attr_removed,
                st_link_no_rel_removed: st_link_no_rel_removed,
                st_link_non_stylesheet_removed: st_link_non_stylesheet_removed,
                st_link_no_href_removed: st_link_no_href_removed,
                st_link_non_http_removed: st_link_non_http_removed,
                st_anchors_rewritten: st_anchors_rewritten,
                st_inline_style_skipped: st_inline_style_skipped,
                st_style_attr_skipped: st_style_attr_skipped,
            })
        },
        Err(e) => {
            Err(e)
        }
    }
}
