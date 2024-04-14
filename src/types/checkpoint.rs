use pyo3::pyclass;
use std::hash::Hash;

#[pyclass(get_all)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct JournalCheckpoint {
    pub log_i: usize,
    pub journal_i: usize,
}
