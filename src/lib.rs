use pyo3::prelude::*;
use pyo3::types::*;
use owning_ref::OwningHandle;

use pyo3::exceptions::PyZeroDivisionError;
use pyo3::exceptions::PyException;
use pyo3::create_exception;

use mailparse;



create_exception!(mailpar, ParseError, PyException);


//fn slice_offset(parent: &[u8], child: &[u8]) -> usize {
    //(child.as_ptr() as usize) - (parent.as_ptr() as usize)
//}


#[pyclass]
struct PyParsedMail {
    ha: OwningHandle<
        Box<Vec<u8>>,
        Box<
            Result<
                mailparse::ParsedMail<'static>,
                mailparse::MailParseError
            >
        >
    >
}


#[pymethods]
impl PyParsedMail {

}

#[pyfunction]
fn parse_mail<'a>(py: Python<'a>, buf: &[u8]) -> PyResult<PyParsedMail>
{
    let pm = PyParsedMail {
        ha: OwningHandle::new_with_fn(
            Box::new(buf.to_vec()),
            unsafe {
                |x| Box::new(mailparse::parse_mail((*x).as_slice()))
            }
        )
    };

    match &*pm.ha {
        Ok(_) => Ok(pm),
        Err(error) => Err(ParseError::new_err(error.to_string()))
    }
}


#[pymodule]
fn mailpar(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse_mail, m)?)?;
    m.add_class::<PyParsedMail>()?;
    m.add("ParseError", py.get_type::<ParseError>())?;
    Ok(())
}
