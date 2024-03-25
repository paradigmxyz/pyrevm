use std::hash::{Hash};

use pyo3::pyclass;

#[pyclass(get_all)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct JournalCheckpoint{
    pub log_i: usize,
    pub journal_i: usize,
}
