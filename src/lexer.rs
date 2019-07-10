use super::{
    error::{Diagnostic, ErrorKind},
    ipa,
    source::Reader,
    token::{Token, TokenKind, TokenPattern},
};

#[derive(Debug, Clone)]
pub struct Lexer {
    toks: Vec<Result<Token, ()>>,
    pos: usize,
    reader: Reader,
}

impl Lexer {
    pub fn new(reader: Reader, errs: &mut Diagnostic) -> Self {
        let mut this = Self { toks: Vec::with_capacity(1), pos: 0, reader };
        let res = this.read(errs);
        this.toks.push(res);
        this
    }

    pub fn reader(&self) -> &Reader {
        &self.reader
    }

    pub fn reader_mut(&mut self) -> &mut Reader {
        &mut self.reader
    }

    pub fn is_eof(&self) -> bool {
        self.curr().ok().map_or(false, |tok| tok.kind == TokenKind::Eof)
    }

    pub fn curr(&self) -> Result<Token, ()> {
        self.toks[self.pos].clone()
    }

    pub fn next(&mut self, errs: &mut Diagnostic) -> bool {
        self.advance(1, errs) == 1
    }

    pub fn prev(&mut self) -> bool {
        self.rollback(1) == 1
    }

    pub fn advance(&mut self, count: usize, errs: &mut Diagnostic) -> usize {
        let mut advanced = count.min(self.toks.len() - self.pos - 1);
        self.pos += advanced;

        while self
            .curr()
            .ok()
            .filter(|tok| tok.kind != TokenKind::Eof && advanced < count)
            .is_some()
        {
            let tok = self.read(errs);
            self.toks.push(tok);
            self.pos += 1;
            advanced += 1;
        }
        advanced
    }

    pub fn rollback(&mut self, count: usize) -> usize {
        let rolled = count.min(self.pos);
        self.pos -= rolled;
        rolled
    }

    pub fn check<P>(&self, pat: P, errs: &mut Diagnostic) -> Result<Token, ()>
    where
        P: TokenPattern,
    {
        let tok = self.curr()?;

        if pat.test(&tok) {
            Ok(tok)
        } else {
            let err = ErrorKind::expected(pat, tok);
            Err(errs.raise(err))
        }
    }

    pub fn expect<P>(
        &mut self,
        pat: P,
        errs: &mut Diagnostic,
    ) -> Result<Token, ()>
    where
        P: TokenPattern,
    {
        let tok = self.check(pat, errs)?;
        self.next(errs);
        Ok(tok)
    }

    fn read(&mut self, errs: &mut Diagnostic) -> Result<Token, ()> {
        self.skip_discardable();

        if self.is_unquoted() {
            self.read_unquoted(errs)
        } else if self.is_quoted_start() {
            self.read_quoted(errs)
        } else if self.is_class_ident_start() {
            self.read_class_ident(errs)
        } else if self.is_equal_symbol() {
            self.read_equal_symbol(errs)
        } else if self.is_comma() {
            self.read_comma(errs)
        } else if self.is_pipe() {
            self.read_pipe(errs)
        } else if self.is_open_paren() {
            self.read_open_paren(errs)
        } else if self.is_close_paren() {
            self.read_close_paren(errs)
        } else {
            self.read_eof(errs)
        }
    }

    fn skip_discardable(&mut self) {
        self.skip_whitespace();
        while self.skip_comment() && self.skip_whitespace() {}
    }

    fn skip_whitespace(&mut self) -> bool {
        let mut skipped = false;

        while self.is_whitespace() {
            self.reader.next();
            skipped = true;
        }

        skipped
    }

    fn skip_comment(&mut self) -> bool {
        if self.skip_line_comment_start() {
            while self.reader.curr().filter(|&s| s != "\n").is_some() {
                self.reader.next();
            }

            true
        } else {
            false
        }
    }

    fn skip_line_comment_start(&mut self) -> bool {
        if self.reader.curr() == Some(";") {
            self.reader.next();
            true
        } else {
            false
        }
    }

    fn is_whitespace(&self) -> bool {
        self.reader.curr().map_or(false, |ch| ch.contains(char::is_whitespace))
    }

    fn is_quoted_start(&self) -> bool {
        self.reader.curr().map_or(false, |ch| ch == "'")
    }

    fn is_unquoted(&self) -> bool {
        self.reader.curr().map_or(false, |ch| {
            ch == "_"
                || ch.len() == 1 && ch >= "a" && ch <= "z"
                || ch.len() == 1 && ch >= "A" && ch <= "Z"
                || ch.len() == 1 && ch >= "0" && ch <= "9"
                || ipa::SYMBOLS.binary_search(&ch).is_ok()
        })
    }

    fn is_class_ident_start(&self) -> bool {
        self.reader.curr().map_or(false, |ch| ch == "\\")
    }

    fn is_equal_symbol(&self) -> bool {
        self.reader.curr().map_or(false, |ch| ch == "=")
    }

    fn is_comma(&self) -> bool {
        self.reader.curr().map_or(false, |ch| ch == ",")
    }

    fn is_pipe(&self) -> bool {
        self.reader.curr().map_or(false, |ch| ch == "|")
    }

    fn is_open_paren(&self) -> bool {
        self.reader.curr().map_or(false, |ch| ch == "(")
    }

    fn is_close_paren(&self) -> bool {
        self.reader.curr().map_or(false, |ch| ch == ")")
    }

    fn read_unquoted(&mut self, _errs: &mut Diagnostic) -> Result<Token, ()> {
        self.reader.mark();
        while self.is_unquoted() {
            self.reader.next();
        }

        let span = self.reader.span();
        let kind = match &*span.content() {
            "alphabet" => TokenKind::Alphabet,
            "class" => TokenKind::Class,
            _ => TokenKind::String(span.content().to_string()),
        };

        Ok(Token { kind, span })
    }

    fn read_class_ident(
        &mut self,
        _errs: &mut Diagnostic,
    ) -> Result<Token, ()> {
        self.reader.mark();
        self.reader.next();
        let mut string = String::new();
        while self.is_unquoted() {
            if let Some(ch) = self.reader.curr() {
                string.push_str(ch);
                self.reader.next();
            }
        }

        let span = self.reader.span();
        let kind = TokenKind::ClassIdent(string);

        Ok(Token { kind, span })
    }

    fn read_quoted(&mut self, errs: &mut Diagnostic) -> Result<Token, ()> {
        self.reader.mark();
        let mut string = String::new();

        loop {
            self.reader.next();
            let ch = self.reader.curr().ok_or_else(|| {
                let err = ErrorKind::UnclosedString(self.reader.span());
                errs.raise(err);
            })?;

            match ch {
                "\\" => string.push_str(self.read_escaped(errs)?),
                "'" => break,
                _ => string.push_str(ch),
            }
        }

        self.reader.next();

        let span = self.reader.span();
        let kind = TokenKind::String(string);

        Ok(Token { kind, span })
    }

    fn read_escaped(&mut self, errs: &mut Diagnostic) -> Result<&str, ()> {
        self.reader.next();
        self.reader.curr().ok_or_else(|| {
            let err = ErrorKind::UnclosedString(self.reader.span());
            errs.raise(err);
        })
    }

    fn read_equal_symbol(
        &mut self,
        _errs: &mut Diagnostic,
    ) -> Result<Token, ()> {
        self.reader.mark();
        self.reader.next();
        Ok(Token { kind: TokenKind::Eq, span: self.reader.span() })
    }

    fn read_comma(&mut self, _errs: &mut Diagnostic) -> Result<Token, ()> {
        self.reader.mark();
        self.reader.next();
        Ok(Token { kind: TokenKind::Comma, span: self.reader.span() })
    }

    fn read_pipe(&mut self, _errs: &mut Diagnostic) -> Result<Token, ()> {
        self.reader.mark();
        self.reader.next();
        Ok(Token { kind: TokenKind::Pipe, span: self.reader.span() })
    }

    fn read_open_paren(&mut self, _errs: &mut Diagnostic) -> Result<Token, ()> {
        self.reader.mark();
        self.reader.next();
        Ok(Token { kind: TokenKind::OpenParen, span: self.reader.span() })
    }

    fn read_close_paren(
        &mut self,
        _errs: &mut Diagnostic,
    ) -> Result<Token, ()> {
        self.reader.mark();
        self.reader.next();
        Ok(Token { kind: TokenKind::CloseParen, span: self.reader.span() })
    }

    fn read_eof(&mut self, errs: &mut Diagnostic) -> Result<Token, ()> {
        self.reader.mark();
        if self.reader.next() {
            Err(errs.raise(ErrorKind::BadChar(self.reader.span())))
        } else {
            Ok(Token { kind: TokenKind::Eof, span: self.reader.span() })
        }
    }
}

#[cfg(test)]
mod test {
    use super::Lexer;
    use crate::{error::Diagnostic, source::Src, token::TokenKind};

    #[test]
    fn parens_and_unquoted() {
        let src = Src::new("foo.psh", "((x)())");
        let mut errs = Diagnostic::new();

        let mut lexer = Lexer::new(src.reader(), &mut errs);

        assert_eq!(lexer.curr().unwrap().kind, TokenKind::OpenParen);
        assert!(lexer.next(&mut errs));
        assert_eq!(lexer.curr().unwrap().kind, TokenKind::OpenParen);
        assert!(lexer.next(&mut errs));
        assert_eq!(
            lexer.curr().unwrap().kind,
            TokenKind::String("x".to_owned())
        );
        assert!(lexer.next(&mut errs));
        assert_eq!(lexer.curr().unwrap().kind, TokenKind::CloseParen);
        assert!(lexer.next(&mut errs));
        assert_eq!(lexer.curr().unwrap().kind, TokenKind::OpenParen);
        assert!(lexer.next(&mut errs));
        assert_eq!(lexer.curr().unwrap().kind, TokenKind::CloseParen);
        assert!(lexer.next(&mut errs));
        assert_eq!(lexer.curr().unwrap().kind, TokenKind::CloseParen);
        assert!(lexer.next(&mut errs));
        assert_eq!(lexer.curr().unwrap().kind, TokenKind::Eof);

        assert!(!lexer.next(&mut errs));
        assert_eq!(lexer.curr().unwrap().kind, TokenKind::Eof);

        assert_eq!(errs.as_slice().len(), 0);
    }

    #[test]
    fn keywords_and_commas() {
        let src = Src::new("foo.psh", "class, alphabet,");
        let mut errs = Diagnostic::new();

        let mut lexer = Lexer::new(src.reader(), &mut errs);

        assert_eq!(lexer.curr().unwrap().kind, TokenKind::Class);
        assert!(lexer.next(&mut errs));
        assert_eq!(lexer.curr().unwrap().kind, TokenKind::Comma);
        assert!(lexer.next(&mut errs));
        assert_eq!(lexer.curr().unwrap().kind, TokenKind::Alphabet);
        assert!(lexer.next(&mut errs));
        assert_eq!(lexer.curr().unwrap().kind, TokenKind::Comma);
        assert!(lexer.next(&mut errs));
        assert_eq!(lexer.curr().unwrap().kind, TokenKind::Eof);

        assert!(!lexer.next(&mut errs));
        assert_eq!(lexer.curr().unwrap().kind, TokenKind::Eof);

        assert_eq!(errs.as_slice().len(), 0);
    }

    #[test]
    fn class_and_quoted_with_escape() {
        let src = Src::new("foo.psh", r"'ah\\he\'end' \V");
        let mut errs = Diagnostic::new();

        let mut lexer = Lexer::new(src.reader(), &mut errs);

        assert_eq!(
            lexer.curr().unwrap().kind,
            TokenKind::String(r"ah\he'end".to_owned())
        );
        assert!(lexer.next(&mut errs));

        assert_eq!(
            lexer.curr().unwrap().kind,
            TokenKind::ClassIdent("V".to_owned())
        );
        assert!(lexer.next(&mut errs));
        assert_eq!(lexer.curr().unwrap().kind, TokenKind::Eof);

        assert!(!lexer.next(&mut errs));
        assert_eq!(lexer.curr().unwrap().kind, TokenKind::Eof);

        assert_eq!(errs.as_slice().len(), 0);
    }

    #[test]
    fn error_unclosed() {
        let src = Src::new("foo.psh", r"'ah");

        let mut errs = Diagnostic::new();

        let lexer = Lexer::new(src.reader(), &mut errs);

        assert!(lexer.curr().is_err());
        assert_eq!(errs.as_slice().len(), 1);
    }
}
