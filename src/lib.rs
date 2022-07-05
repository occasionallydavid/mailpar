use pyo3::prelude::*;
use pyo3::types::*;
use owning_ref::OwningHandle;
use std::rc::Rc;
use std::collections::BTreeMap;
use mailparse::body::Body::Binary;
use mailparse::body::Body::Base64;
use mailparse::body::Body::QuotedPrintable;
use mailparse::body::Body::SevenBit;
use mailparse::body::Body::EightBit;
use mailparse::DispositionType::Extension;
use mailparse::DispositionType::FormData;
use mailparse::DispositionType::Attachment;
use mailparse::DispositionType::Inline;

use pyo3::exceptions::PyException;
use pyo3::exceptions::PyIndexError;
use pyo3::create_exception;

use mailparse;



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
    >
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
    fn raw_byte_offset(&self) -> usize {
        let handle = &(self.storage.handle);
        slice_offset(handle.as_owner().as_slice(), _part(self).raw_bytes)
    }

    fn raw_bytes(&self, py: Python) -> PyObject {
        PyBytes::new(py, _part(self).raw_bytes).into()
    }

    fn raw_body_offset(&self, py: Python) -> usize {
        let handle = &(self.storage.handle);
        slice_offset(handle.as_owner().as_slice(), self._body_encoded())
    }

    fn raw_body_length(&self, py: Python) -> usize {
        self._body_encoded().len()
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

    fn get_body(&self) -> PyResult<String> {
        match _part(self).get_body() {
            Ok(s) => Ok(s),
            Err(e) => Err(ParseError::new_err(e.to_string()))
        }
    }

    fn get_body_raw(&self, py: Python) -> PyResult<PyObject> {
        match _part(self).get_body_raw() {
            Ok(s) => Ok(PyBytes::new(py, s.as_slice()).into()),
            Err(e) => Err(ParseError::new_err(e.to_string()))
        }
    }

    fn _body_encoded(&self) -> &[u8] {
        match _part(self).get_body_encoded() {
            Base64(eb) => eb.get_raw(),
            QuotedPrintable(eb) => eb.get_raw(),
            SevenBit(tb) => tb.get_raw(),
            EightBit(tb) => tb.get_raw(),
            Binary(bb) => bb.get_raw(),
        }
    }

    fn get_body_encoded(&self, py: Python) -> PyResult<PyObject> {
        let s = self._body_encoded();
        Ok(PyBytes::new(py, s).into())
    }
}


fn _subpath(path: &Vec<usize>, i: usize) -> Vec<usize> {
    let mut new = path.clone();
    new.push(i);
    new
}


#[pyfunction]
fn parse_mail<'a>(_py: Python<'a>, buf: &[u8]) -> PyResult<PyParsedMail>
{
    let handle = OwningHandle::new_with_fn(
        Box::new(buf.to_vec()),
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
                    }
                ),
                path: vec![]
            }
        ),
        Err(error) => Err(ParseError::new_err(error.to_string()))
    }
}


#[pymodule]
fn mailpar(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse_mail, m)?)?;
    m.add_class::<PyParsedMail>()?;
    m.add_class::<PyHeaders>()?;
    m.add("ParseError", py.get_type::<ParseError>())?;
    Ok(())
}
