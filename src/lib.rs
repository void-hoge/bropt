pub mod brainfuck;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use brainfuck::{compile as bf_compile, run_to_bytes, Inst};

fn panic_to_pyerr(err: Box<dyn std::any::Any + Send>) -> PyErr {
    if let Some(s) = err.downcast_ref::<&str>() {
        PyRuntimeError::new_err(*s)
    } else if let Some(s) = err.downcast_ref::<String>() {
        PyRuntimeError::new_err(s.clone())
    } else {
        PyRuntimeError::new_err("panic occurred")
    }
}

#[pyclass]
pub struct Program {
    prog: Vec<Inst>,
}

#[pymethods]
impl Program {
    pub fn run(&self, py: Python<'_>, length: usize) -> PyResult<Py<PyBytes>> {
        let prog = self.prog.clone();
        match std::panic::catch_unwind(|| run_to_bytes(prog, length)) {
            Ok(out) => Ok(PyBytes::new(py, &out).into()),
            Err(err) => Err(panic_to_pyerr(err)),
        }
    }
}

#[pyfunction]
fn compile(code: &str) -> PyResult<Program> {
    match std::panic::catch_unwind(|| bf_compile(code)) {
        Ok(prog) => Ok(Program { prog }),
        Err(err) => Err(panic_to_pyerr(err)),
    }
}

#[pymodule]
fn bropt(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compile, m)?)?;
    m.add_class::<Program>()?;
    Ok(())
}
