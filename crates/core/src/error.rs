#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldError {
    Empty,
    TooLong { max: usize },
    Invalid { allowed: &'static str },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub field: &'static str,
    pub source: FieldError,
}

impl ValidationError {
    pub fn new(field: &'static str, source: FieldError) -> Self {
        Self { field, source }
    }
}
