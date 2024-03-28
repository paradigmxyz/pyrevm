use pyo3::types::PyDict;
use pyo3::Python;
use std::io::Write;

pub(crate) struct PySysStdout;

// alloow us to capture stdout from python
// based on https://github.com/PyO3/pyo3/discussions/1960#discussioncomment-8414724
impl Write for PySysStdout {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let s = std::str::from_utf8(buf).unwrap();
        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            locals.set_item("s", s).unwrap();
            py.run("print(s, end='')", None, Some(locals)).unwrap();
        });
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Python::with_gil(|py| {
            py.run("import sys;sys.stdout.flush()", None, None).unwrap();
        });
        Ok(())
    }
}
