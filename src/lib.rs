use std::collections::BTreeMap;
use std::rc::Rc;

#[macro_use]
extern crate lazy_static;

use mailparse::body::Body::Base64;
use mailparse::body::Body::Binary;
use mailparse::body::Body::EightBit;
use mailparse::body::Body::QuotedPrintable;
use mailparse::body::Body::SevenBit;
use mailparse::DispositionType::Attachment;
use mailparse::DispositionType::Extension;
use mailparse::DispositionType::FormData;
use mailparse::DispositionType::Inline;
use mailparse::MailHeaderMap;
use owning_ref::OwningHandle;
use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::exceptions::PyIndexError;
use pyo3::prelude::*;
use pyo3::types::*;

use mailparse;

pub mod html;
pub mod css;
pub mod deferral;


create_exception!(mailpar, ParseError, PyException);


fn slice_offset(parent: &[u8], child: &[u8]) -> usize {
    (child.as_ptr() as usize) - (parent.as_ptr() as usize)
}


struct MailStorage {
    handle: OwningHandle<
        Box<Vec<u8>>,
        Box<
            Result<
                mailparse::ParsedMail<'static>,
                mailparse::MailParseError
            >
        >
    >,
    from_line: Box<Vec<u8>>
}


#[pyclass(unsendable)]
struct PyParsedMail {
    storage: Rc<MailStorage>,
    path: Vec<usize>
}


#[pyclass(unsendable)]
struct PyHeaders {
    storage: Rc<MailStorage>,
    path: Vec<usize>
}


#[pymethods]
impl PyHeaders {
    fn offset(&self) -> [usize; 2] {
        let handle = &(self.storage.handle);
        let sl = self.raw_bytes();
        return [slice_offset(handle.as_owner().as_slice(), sl), sl.len()];
    }

    fn raw_bytes(&self) -> &[u8] {
        let headers = _hpart(self).get_headers();
        headers.get_raw_bytes()
    }

    fn fromline(&self) -> Option<&[u8]> {
        let pm = _hpart(self);
        if pm.headers.len() == 0 {
            return None;
        }

        let key = pm.headers[0].get_key_raw();
        if key.starts_with(b"From ") {
            return Some(key);
        }

        None
    }

    fn first(&self, key: &str) -> Option<String> {
        let headers = _hpart(self).get_headers();
        headers.get_first_value(key)
    }

    //fn first_address(&self, key: &str) -> Option<String> {
    fn first_address(&self, py: Python, key: &str) -> PyResult<PyObject> {
        let headers = _hpart(self).get_headers();
        let addrs = match mailparse::addrparse_header(
            headers.get_first_header(key).unwrap()
        ) {
            Err(e) => return Err(ParseError::new_err(e.to_string())),
            Ok(addrs) => addrs
        };

        let lst = pyo3::types::PyList::empty(py);
        for addr in &addrs.into_inner() {
            match &addr {
                mailparse::MailAddr::Single(info) => {
                    let name = match &info.display_name {
                        None => "",
                        Some(s) => s.as_str(),
                    };

                    lst.append((name, info.addr.as_str())).expect("append to list");
                },
                //None => {},
                _ => panic!()
            }
        }

        Ok(lst.into())
    }

    fn all(&self, key: &str) -> Vec<String> {
        let headers = _hpart(self).get_headers();
        headers.get_all_values(key)
    }

    fn names(&self, py: Python) -> PyResult<PyObject> {
        let lst = pyo3::types::PyList::empty(py);

        let headers = _hpart(self).get_headers();
        for header in headers {
            lst.append(header.get_key());
        }

        Ok(lst.into())
    }
}


fn _hpart<'a>(parsed: &'a PyHeaders) -> &'a mailparse::ParsedMail<'a>
{
    let handle = &(parsed.storage.handle);
    let result = &*handle;

    let mut part = result.as_ref().unwrap();
    for i in &parsed.path {
        //println!("EEK {}", i);
        part = &(part.subparts[*i]);
    }

    part
}


fn _part<'a>(parsed: &'a PyParsedMail) -> &'a mailparse::ParsedMail<'a>
{
    let handle = &(parsed.storage.handle);
    let result = &*handle;

    let mut part = result.as_ref().unwrap();
    for i in &parsed.path {
        //println!("EEK {}", i);
        part = &(part.subparts[*i]);
    }

    part
}

#[pymethods]
impl PyParsedMail {
    fn offset(&self) -> [usize; 2] {
        let handle = &(self.storage.handle);
        let sl = _part(self).raw_bytes;
        return [slice_offset(handle.as_owner().as_slice(), sl), sl.len()];
    }

    fn from_line(&self, py: Python) -> PyObject {
        PyBytes::new(py, (*self.storage.from_line).as_slice()).into()
    }

    fn raw_bytes(&self, py: Python) -> PyObject {
        PyBytes::new(py, _part(self).raw_bytes).into()
    }

    fn body_offset(&self) -> [usize; 2] {
        let handle = &(self.storage.handle);
        let sl = self.body_encoded();
        return [slice_offset(handle.as_owner().as_slice(), sl), sl.len()];
    }

    fn subpart_count(&self) -> usize {
        _part(self).subparts.len()
    }

    fn mime_type(&self) -> &String {
        &(_part(self).ctype.mimetype)
    }

    fn charset(&self) -> &String {
        &(_part(self).ctype.charset)
    }

    fn params(&self) -> BTreeMap<String, String> {
        _part(self).ctype.params.clone()
    }

    fn param(&self, k: &str) -> Option<&String> {
        _part(self).ctype.params.get(k)
    }

    fn content_disposition(&self) -> String {
        match _part(self).get_content_disposition().disposition {
            Inline => "inline".to_string(),
            Attachment => "attachment".to_string(),
            FormData => "formdata".to_string(),
            Extension(s) => s,
        }
    }

    fn get_filename(&self) -> Option<String> {
        match _part(self).get_content_disposition().params.get("filename") {
            Some(s) => Some(s.clone()),
            None => match _part(self).ctype.params.get("name") {
                Some(s) => Some(s.clone()),
                None => None
            }
        }
    }

    fn path(&self) -> Vec<usize> {
        self.path.clone()
    }

    fn headers(&self) -> PyHeaders {
        PyHeaders {
            storage: self.storage.clone(),
            path: self.path.clone()
        }
    }

    fn subpart(&self, i: usize) -> PyResult<PyParsedMail> {
        let part = _part(self);
        if i >= part.subparts.len() {
            return Err(PyIndexError::new_err(i));
        }

        Ok(PyParsedMail {
            storage: self.storage.clone(),
            path: _subpath(&self.path, i)
        })
    }

    fn body(&self) -> PyResult<String> {
        match _part(self).get_body() {
            Ok(s) => Ok(s),
            Err(e) => Err(ParseError::new_err(e.to_string()))
        }
    }

    fn body_raw(&self, py: Python) -> PyResult<PyObject> {
        match _part(self).get_body_raw() {
            Ok(s) => Ok(PyBytes::new(py, s.as_slice()).into()),
            Err(e) => Err(ParseError::new_err(e.to_string()))
        }
    }

    fn body_encoding(&self) -> &str {
        match _part(self).get_body_encoded() {
            Base64(_) => "base64",
            QuotedPrintable(_) => "quotedprintable",
            SevenBit(_) => "7bit",
            EightBit(_) => "8bit",
            Binary(_) => "binary",
        }
    }

    fn body_encoded(&self) -> &[u8] {
        match _part(self).get_body_encoded() {
            Base64(eb) => eb.get_raw(),
            QuotedPrintable(eb) => eb.get_raw(),
            SevenBit(tb) => tb.get_raw(),
            EightBit(tb) => tb.get_raw(),
            Binary(bb) => bb.get_raw(),
        }
    }
}


fn _subpath(path: &Vec<usize>, i: usize) -> Vec<usize> {
    let mut new = path.clone();
    new.push(i);
    new
}


fn read_off_from_line(buf: &[u8]) -> (&[u8], &[u8])
{
    if !buf.starts_with("From ".as_bytes()) {
        return (&[], buf);
    }

    match buf.iter().position(|&b| b == b'\n') {
        Some(i) => (&buf[..i+1], &buf[i+1..]),
        None => (&[], buf)
    }
}


#[pyfunction]
fn from_bytes<'a>(_py: Python<'a>, buf: &[u8]) -> PyResult<PyParsedMail>
{
    let (header_buf, body_buf) = read_off_from_line(buf);

    let handle = OwningHandle::new_with_fn(
        Box::new(body_buf.to_vec()),
        unsafe {
            |x| Box::new(mailparse::parse_mail((*x).as_slice()))
        }
    );

    match &*handle {
        Ok(_) => Ok(
            PyParsedMail {
                storage: Rc::new(
                    MailStorage {
                        handle: handle,
                        from_line: Box::new(header_buf.to_vec())
                    }
                ),
                path: vec![]
            }
        ),
        Err(error) => Err(ParseError::new_err(error.to_string()))
    }
}


#[pyfunction]
pub fn rewrite_html(py: Python, s: &str) -> PyResult<PyObject>
{
    let lst = pyo3::types::PyList::empty(py);

    match crate::html::rewrite_html(s) {
        Ok(output) => {
            for deferral in output.deferrals {
                //lst.append((name, info.addr.as_str())).expect("append to list");
                lst.append((deferral.i, deferral.kind as i32, deferral.data));
            }

            let dct = pyo3::types::PyDict::new(py);
            dct.set_item("html", output.html);
            dct.set_item("page_links", output.page_links);
            dct.set_item("text_content", output.text_content);
            dct.set_item("deferrals", lst);

            dct.set_item("doctype_removed", output.st_doctype_removed);
            dct.set_item("comment_removed", output.st_comment_removed);
            dct.set_item("script_removed", output.st_script_removed);
            dct.set_item("invalid_tag_removed", output.st_invalid_tag_removed);
            dct.set_item("invalid_attr_removed", output.st_invalid_attr_removed);
            dct.set_item("link_no_rel_removed", output.st_link_no_rel_removed);
            dct.set_item("link_non_stylesheet_removed", output.st_link_non_stylesheet_removed);
            dct.set_item("link_no_href_removed", output.st_link_no_href_removed);
            dct.set_item("link_non_http_removed", output.st_link_non_http_removed);
            dct.set_item("anchors_rewritten", output.st_anchors_rewritten);
            dct.set_item("inline_style_skipped", output.st_inline_style_skipped);
            dct.set_item("style_attr_skipped", output.st_style_attr_skipped);

            Ok(dct.into())
            //Ok((output.html, lst).into(py))
        },
        Err(_) => panic!(),
    }
}


#[pyfunction]
pub fn rewrite_css(py: Python, s: &str) -> PyResult<PyObject>
{
    let lst = pyo3::types::PyList::empty(py);

    match crate::css::rewrite_css(s, 0) {
        Ok(output) => {
            for deferral in output.deferrals {
                lst.append((deferral.i, deferral.kind as i32, deferral.data));
            }

            let dct = pyo3::types::PyDict::new(py);
            dct.set_item("css", output.css);
            dct.set_item("deferrals", lst);
            Ok(dct.into())
        },
        Err(_) => panic!(),
    }
}


#[pymodule]
fn mailpar(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(from_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(rewrite_html, m)?)?;
    m.add_function(wrap_pyfunction!(rewrite_css, m)?)?;
    m.add_class::<PyParsedMail>()?;
    m.add_class::<PyHeaders>()?;
    m.add("ParseError", py.get_type::<ParseError>())?;
    Ok(())
}
