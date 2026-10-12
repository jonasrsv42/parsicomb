/// Define an error enum whose variants each hold the error of one part of a
/// combinator, displayed through the furthest underlying error
///
/// Every type parameter must be an `ErrorNode` over the same element type as
/// the first one. Generates the enum with `#[derive(Debug)]`, its
/// `Display`, `Error` and `ErrorNode` impls, and a conversion into
/// `ParsicombError` when every part converts into it.
macro_rules! composite_error {
    (
        $(#[$meta:meta])*
        pub enum $name:ident<$first:ident $(, $rest:ident)* $(,)?> {
            $( $(#[$variant_meta:meta])* $variant:ident($field:ident) ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug)]
        pub enum $name<$first $(, $rest)*> {
            $( $(#[$variant_meta])* $variant($field) ),+
        }

        /// Displays the error that got furthest into the input
        impl<$first $(, $rest)*> std::fmt::Display for $name<$first $(, $rest)*>
        where
            $first: $crate::error::ErrorNode,
            $( $rest: $crate::error::ErrorNode<Element = <$first as $crate::error::ErrorNode>::Element>, )*
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", $crate::error::ErrorNode::likely_error(self))
            }
        }

        impl<$first $(, $rest)*> std::error::Error for $name<$first $(, $rest)*>
        where
            $first: $crate::error::ErrorNode,
            $( $rest: $crate::error::ErrorNode<Element = <$first as $crate::error::ErrorNode>::Element>, )*
        {
        }

        impl<$first $(, $rest)*> $crate::error::ErrorNode for $name<$first $(, $rest)*>
        where
            $first: $crate::error::ErrorNode,
            $( $rest: $crate::error::ErrorNode<Element = <$first as $crate::error::ErrorNode>::Element>, )*
        {
            type Element = <$first as $crate::error::ErrorNode>::Element;

            fn likely_error(&self) -> &dyn $crate::error::ErrorLeaf<Element = Self::Element> {
                match self {
                    $( $name::$variant(error) => error.likely_error(), )+
                }
            }

            fn info(&self) -> $crate::error::FailureInfo {
                match self {
                    $( $name::$variant(error) => error.info(), )+
                }
            }
        }

        /// Converts to the error of whichever part failed, without allocating
        impl<'code, $first $(, $rest)*> From<$name<$first $(, $rest)*>>
            for $crate::error::ParsicombError<'code, <$first as $crate::error::ErrorNode>::Element>
        where
            $first: $crate::error::ErrorNode
                + Into<$crate::error::ParsicombError<'code, <$first as $crate::error::ErrorNode>::Element>>,
            $( $rest: Into<$crate::error::ParsicombError<'code, <$first as $crate::error::ErrorNode>::Element>>, )*
        {
            fn from(error: $name<$first $(, $rest)*>) -> Self {
                match error {
                    $( $name::$variant(error) => error.into(), )+
                }
            }
        }
    };
}

pub(crate) use composite_error;
