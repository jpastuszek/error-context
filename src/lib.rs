trait ErrorContext<C> {
    fn with_context(self, context: C) -> Self;
}

trait ResultErrorWhile<C> {
    fn error_while(self, context: C) -> Self;
}

impl<O, E, C> ResultErrorWhile<C> for Result<O, E> where E: ErrorContext<C> {
    fn error_while(self, context: C) -> Self {
        self.map_err(|e| e.with_context(context))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::*;

    #[derive(Debug)]
    enum FooError {
        Foo { context: Option<String> },
        Bar { num: i32, ctx: Option<String> },
    }

    impl ErrorContext<String> for FooError  {
        fn with_context(self, context: String) -> Self {
            match self {
                FooError::Foo { .. } => FooError::Foo { context: Some(context) },
                FooError::Bar { num, .. } => FooError::Bar { num, ctx: Some(context) },
            }
        }
    }

    #[test]
    fn it_works() {
        // use std::io::{Error, ErrorKind};
        // let custom_error = Error::new(ErrorKind::Other, "oh no!");

        let err: Result<(), FooError> = Err(FooError::Foo { context: None });
        assert_matches!(err.error_while("doing stuff".to_string()), Err(FooError::Foo { context: Some(c) }) => assert_eq!(c, "doing stuff".to_string()));

        let err: Result<(), FooError> = Err(FooError::Bar { num: 1, ctx: None });
        assert_matches!(err.error_while("doing stuff".to_string()), Err(FooError::Bar { num: 1, ctx: Some(c) }) => assert_eq!(c, "doing stuff".to_string()));
    }
}
