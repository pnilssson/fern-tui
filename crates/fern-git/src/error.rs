#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("git is not installed")]
    Missing,
    #[error("{0}")]
    Git(String),
}
