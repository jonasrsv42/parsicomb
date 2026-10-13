//! An error with a progress of its own

use super::{ErrorLeaf, ErrorNode, ParsicombError, Progress};
use crate::atomic::Atomic;
use std::fmt;

/// An error reported as its inner error, with a progress set when it was
/// built instead of the inner error's
///
/// For wrappers that adjust how far a failure counts, such as
/// [`attempt`](crate::attempt()); build the progress from the inner error's, so that a
/// committed failure inside stays committed.
#[derive(Debug)]
pub struct WithProgress<E> {
    inner: E,
    progress: Progress,
}

impl<E> WithProgress<E> {
    /// Report `inner` with `progress`
    pub fn new(inner: E, progress: Progress) -> Self {
        WithProgress { inner, progress }
    }

    /// The inner error
    pub fn into_inner(self) -> E {
        self.inner
    }
}

impl<E: ErrorNode> fmt::Display for WithProgress<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner.likely_error())
    }
}

impl<E: ErrorNode> std::error::Error for WithProgress<E> {}

impl<E: ErrorNode> ErrorNode for WithProgress<E> {
    type Element = E::Element;

    fn likely_error(&self) -> &dyn ErrorLeaf<Element = Self::Element> {
        self.inner.likely_error()
    }

    fn progress(&self) -> Progress {
        self.progress
    }
}

/// Converts by wrapping (one allocation), which keeps the progress
impl<'code, E, T> From<WithProgress<E>> for ParsicombError<'code, T>
where
    E: ErrorNode<Element = T> + Send + Sync + 'code,
    T: Atomic,
{
    fn from(error: WithProgress<E>) -> Self {
        ParsicombError::wrap(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utf8::is_string;
    use crate::{ByteCursor, Cursor, Parser};

    #[test]
    fn test_reports_inner_error_with_its_own_progress() {
        let inner = is_string("pub").parse(ByteCursor::new(b"pux")).unwrap_err();
        let message = inner.to_string();
        let error = WithProgress::new(inner, Progress::at(0).committed());
        assert_eq!(error.progress(), Progress::Committed { at: 0 });
        assert_eq!(error.to_string(), message);

        let converted: ParsicombError = error.into();
        assert_eq!(converted.progress(), Progress::Committed { at: 0 });
        assert_eq!(converted.to_string(), message);
    }

    #[test]
    fn test_into_inner() {
        let inner = ByteCursor::new(b"").value().unwrap_err();
        let error = WithProgress::new(inner, Progress::at(0));
        assert!(matches!(error.into_inner(), ParsicombError::EndOfInput(_)));
    }
}
