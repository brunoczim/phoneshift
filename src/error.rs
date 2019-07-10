use super::{
    fmt_ext::SeqFmt,
    source::Span,
    token::{Token, TokenPattern},
};
use std::fmt;

#[derive(Debug, Clone)]
pub enum ErrorKind {
    BadChar(Span),
    UnclosedString(Span),
    Expected(String, Token),
}

impl ErrorKind {
    pub fn expected<P>(expected: P, found: Token) -> Self
    where
        P: TokenPattern,
    {
        let mut string = String::new();
        {
            let mut fmtr = SeqFmt::new(&mut string);
            expected
                .render(&mut fmtr)
                .and_then(|_| fmtr.finish())
                .expect("String as fmt::Writer cannot fail");
        }
        ErrorKind::Expected(string, found)
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorKind::BadChar(span) => write!(
                fmtr,
                "unsupported character {} {}",
                span.content(),
                span
            ),

            ErrorKind::Expected(expected, found) => {
                write!(fmtr, "expected {}, found {}", expected, found)
            },

            ErrorKind::UnclosedString(span) => {
                write!(fmtr, "unclosed string {}", span)
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct Error {
    pub kind: ErrorKind,
    pub warning: bool,
}

impl fmt::Display for Error {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        if self.warning {
            fmtr.write_str("\x1b[1;93mWarning\x1b[0m: ")?;
        } else {
            fmtr.write_str("\x1b[1;91mError\x1b[0m: ")?;
        }

        write!(fmtr, "{}", self.kind)
    }
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    errors: Vec<Error>,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        let mut errors = 0;

        for error in &self.errors {
            if !error.warning {
                errors += 1;
            }
            write!(fmtr, "\n{}\n\n{:=>80}\n", error, "")?;
        }

        if errors == 0 {
            fmtr.write_str("\n\x1b[1;92mSuccessful compilation!\x1b[0m")
        } else {
            write!(
                fmtr,
                "\n\x1b[1;91mFound {} error{}! Compilation failed!\x1b[0m",
                errors,
                if errors == 1 { "" } else { "s" },
            )
        }
    }
}

impl Diagnostic {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn raise(&mut self, kind: ErrorKind) {
        self.errors.push(Error { kind, warning: false });
    }

    pub fn warn(&mut self, kind: ErrorKind) {
        self.errors.push(Error { kind, warning: true });
    }

    pub fn errors(&self) -> &[Error] {
        &self.errors
    }

    pub fn take_errors(self) -> Vec<Error> {
        self.errors
    }
}
