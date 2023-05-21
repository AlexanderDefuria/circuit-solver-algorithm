use std::error::Error;
use std::fmt::{write, Debug, Display, Formatter};

/// Possible Ok Statuses
///
/// Valid: Container is valid
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Status {
    Valid,
    Simplified,
    // TODO: Add more variants?
}

/// Possible Issues
///
/// Valid: Container is valid
#[derive(Debug, Clone)]
pub enum StatusError {
    Unknown,
    KnownIssue(String),
    MultipleIssues(Vec<StatusError>),
    // TODO: Add more variants?
}

pub type ValidationResult = Result<Status, StatusError>;

pub trait Validation {
    fn validate(&self) -> ValidationResult;
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match *self {
            Status::Valid => write!(f, "Valid"),
            Status::Simplified => write!(f, "Simplified"),
        }
    }
}

impl Display for StatusError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StatusError::Unknown => write!(f, "Unknown Issue"),
            StatusError::KnownIssue(str) => write!(f, "Known Issue: {}", str),
            StatusError::MultipleIssues(error_list) => {
                write!(f, "Multiple Issues: {:?}", error_list)
            }
        }
    }
}

impl Error for StatusError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            _ => None,
        }
    }
}

pub(crate) fn get_all_internal_status_errors<T: Validation>(list: &Vec<T>) -> Vec<StatusError> {
    list.iter()
        .enumerate()
        .filter_map(|(i, x)| match x.validate() {
            Err(e) => Some(e),
            _ => None,
        })
        .collect()
}

/// Check for duplicates in a list
///
/// Returns a Vec of StatusError::KnownIssue. If the vec is empty, there are no duplicates.
pub(crate) fn check_duplicates<T: Validation + PartialEq + Display>(
    list: &Vec<T>,
) -> Vec<StatusError> {
    let mut errors: Vec<StatusError> = Vec::new();
    let mut seen: Vec<&T> = Vec::new();
    for x in list {
        if seen.contains(&x) {
            errors.push(StatusError::KnownIssue(format!("Duplicate: {}", x)));
        }
        seen.push(x);
    }
    errors
}
