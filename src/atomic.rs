/// Trait for atomic elements that can be used in parsing
/// This enables generic error formatting and position calculation
pub trait Atomic: Copy + Clone + PartialEq + std::fmt::Debug + std::fmt::Display {
    /// The newline character/element for this atomic type
    const NEWLINE: Self;

    /// Convert a slice of elements to a displayable string for error reporting
    fn slice_to_string(slice: &[Self]) -> String;
}

impl Atomic for u8 {
    const NEWLINE: Self = b'\n';

    fn slice_to_string(slice: &[Self]) -> String {
        String::from_utf8_lossy(slice).to_string()
    }
}
