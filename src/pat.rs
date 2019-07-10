use crate::symbol::{NonTerminal, Terminal};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MatchSegment {
    pub start: usize,
    pub len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Match {
    pub segments: Vec<MatchSegment>,
}

impl Match {
    pub fn matched(&self) -> bool {
        self.segments.len() > 0
    }

    pub fn unmatched(&self) -> bool {
        self.segments.len() == 0
    }

    pub fn add_offset(&mut self, offset: usize) {
        for seg in &mut self.segments {
            seg.start += offset;
        }
    }

    pub fn general_start(&self) -> usize {
        if self.matched() {
            self.segments[0].start
        } else {
            0
        }
    }

    pub fn general_len(&self) -> usize {
        self.general_end() - self.general_start()
    }

    pub fn general_end(&self) -> usize {
        if self.matched() {
            self.segments[self.segments.len() - 1].start
                + self.segments[self.segments.len() - 1].len
        } else {
            0
        }
    }

    pub fn append<F>(&mut self, right: F)
    where
        F: FnOnce(&Self) -> Match,
    {
        if self.matched() {
            let mut other = right(&self);
            if other.unmatched() {
                *self = other;
            } else if self.general_end() == other.general_start() {
                let first = other.segments.remove(0);
                let last = self.segments.len() - 1;
                self.segments[last].len += first.len;
                self.segments.append(&mut other.segments);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pattern {
    Terms(Vec<Terminal>),
    NonTerm(NonTerminal),
    And(Box<Pattern>, Box<Pattern>),
    Or(Box<Pattern>, Box<Pattern>),
}

impl Pattern {
    pub fn match_terms(&self, terms: &[Terminal]) -> Match {
        match_pattern(self, terms, 0)
    }
}

fn match_term_pat(
    pat: &[Terminal],
    terms: &[Terminal],
    offset: usize,
) -> Match {
    Match {
        segments: if terms[offset ..].starts_with(&*pat) {
            vec![MatchSegment { start: offset, len: pat.len() }]
        } else {
            vec![]
        },
    }
}

fn match_non_term_pat(
    non_term: &NonTerminal,
    terms: &[Terminal],
    offset: usize,
) -> Match {
    Match {
        segments: if terms.len() == offset + 1
            && non_term.contains(&terms[offset])
        {
            vec![MatchSegment { start: offset, len: 1 }]
        } else {
            vec![]
        },
    }
}

fn match_and_pat(
    left: &Pattern,
    right: &Pattern,
    terms: &[Terminal],
    offset: usize,
) -> Match {
    let mut lmatch = match_pattern(left, terms, offset);
    lmatch.append(|lmatch| match_pattern(right, terms, lmatch.general_end()));
    lmatch
}

fn match_or_pat(
    left: &Pattern,
    right: &Pattern,
    terms: &[Terminal],
    offset: usize,
) -> Match {
    let lmatch = match_pattern(left, terms, offset);
    if lmatch.matched() {
        lmatch
    } else {
        match_pattern(right, terms, offset)
    }
}

fn match_pattern(pat: &Pattern, terms: &[Terminal], offset: usize) -> Match {
    match pat {
        Pattern::Terms(test) => match_term_pat(test, terms, offset),

        Pattern::NonTerm(non_term) => {
            match_non_term_pat(non_term, terms, offset)
        },

        Pattern::And(left, right) => match_and_pat(left, right, terms, offset),

        Pattern::Or(left, right) => match_or_pat(left, right, terms, offset),
    }
}

#[cfg(test)]
mod test {
    use super::{Match, MatchSegment, NonTerminal, Pattern, Terminal};
    use crate::{
        find_syms,
        make_terms,
        symbol::{Symbol, Table},
    };

    fn terms_for_test() -> Table<Terminal> {
        make_terms!("a", "i", "u", "p", "c", "q")
    }

    fn non_terms_for_test(
        terms: &Table<Terminal>,
    ) -> Option<Table<NonTerminal>> {
        fn wrap(term: &Terminal) -> Symbol {
            Symbol::Term(term.clone())
        }

        Some(Table::new(vec![
            NonTerminal::new("V", find_syms!(terms, wrap, "a", "i", "u")?),
            NonTerminal::new("C", find_syms!(terms, wrap, "p", "c", "q")?),
        ]))
    }

    #[test]
    fn terms_pat() {
        let terms = terms_for_test();
        let string1 = find_syms!(&terms, Clone::clone, "a", "p", "u").unwrap();
        let string2 =
            find_syms!(&terms, Clone::clone, "q", "i", "u", "a").unwrap();

        let pat = Pattern::Terms(string1.clone());

        assert_eq!(
            pat.match_terms(&string1),
            Match { segments: vec![MatchSegment { start: 0, len: 3 }] }
        );
        assert_eq!(pat.match_terms(&string2), Match::default());
    }

    #[test]
    fn non_term_pat() {
        let terms = terms_for_test();
        let non_terms = non_terms_for_test(&terms).unwrap();
        let string1 = find_syms!(&terms, Clone::clone, "a").unwrap();
        let string2 = find_syms!(&terms, Clone::clone, "i").unwrap();
        let string3 = find_syms!(&terms, Clone::clone, "i", "a").unwrap();
        let string4 = find_syms!(&terms, Clone::clone, "p").unwrap();

        let pat = Pattern::NonTerm(non_terms.find("V").unwrap().clone());

        assert_eq!(
            pat.match_terms(&string1),
            Match { segments: vec![MatchSegment { start: 0, len: 1 }] }
        );
        assert_eq!(
            pat.match_terms(&string2),
            Match { segments: vec![MatchSegment { start: 0, len: 1 }] }
        );
        assert_eq!(pat.match_terms(&string3), Match::default());
        assert_eq!(pat.match_terms(&string4), Match::default());
    }

    #[test]
    fn and_pat() {
        let terms = terms_for_test();
        let non_terms = non_terms_for_test(&terms).unwrap();
        let string1 = find_syms!(&terms, Clone::clone, "c", "u").unwrap();
        let string2 = find_syms!(&terms, Clone::clone, "i").unwrap();
        let string3 = find_syms!(&terms, Clone::clone, "c", "u", "p").unwrap();
        let string4 = find_syms!(&terms, Clone::clone, "q").unwrap();

        let left = Pattern::Terms(string1.clone());
        let right = Pattern::NonTerm(non_terms.find("C").unwrap().clone());
        let pat = Pattern::And(Box::new(left), Box::new(right));

        assert_eq!(pat.match_terms(&string1), Match::default());
        assert_eq!(pat.match_terms(&string2), Match::default());
        assert_eq!(
            pat.match_terms(&string3),
            Match { segments: vec![MatchSegment { start: 0, len: 3 }] }
        );
        assert_eq!(pat.match_terms(&string4), Match::default());
    }

    #[test]
    fn or_pat() {
        let terms = terms_for_test();
        let non_terms = non_terms_for_test(&terms).unwrap();
        let string1 = find_syms!(&terms, Clone::clone, "c", "u").unwrap();
        let string2 = find_syms!(&terms, Clone::clone, "i").unwrap();
        let string3 = find_syms!(&terms, Clone::clone, "c", "u", "p").unwrap();
        let string4 = find_syms!(&terms, Clone::clone, "q").unwrap();

        let left = Pattern::Terms(string1.clone());
        let right = Pattern::NonTerm(non_terms.find("C").unwrap().clone());
        let pat = Pattern::Or(Box::new(left), Box::new(right));

        assert_eq!(
            pat.match_terms(&string1),
            Match { segments: vec![MatchSegment { start: 0, len: 2 }] }
        );
        assert_eq!(pat.match_terms(&string2), Match::default());
        assert_eq!(
            pat.match_terms(&string3),
            Match { segments: vec![MatchSegment { start: 0, len: 2 }] }
        );
        assert_eq!(
            pat.match_terms(&string4),
            Match { segments: vec![MatchSegment { start: 0, len: 1 }] }
        );
    }
}
