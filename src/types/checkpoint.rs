use std::hash::{Hash, Hasher};

use pyo3::pyclass;

#[pyclass(get_all)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct JournalCheckpoint{
    pub log_i: usize,
    pub journal_i: usize,
}

impl Hash for JournalCheckpoint {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.log_i.hash(state);
        self.journal_i.hash(state);
    }
}
