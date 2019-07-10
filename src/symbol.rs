use std::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    rc::Rc,
};

pub trait DescKey {
    fn desc(&self) -> &str;

    fn cmp_desc(&self, other: &Self) -> Ordering {
        self.desc().cmp(&other.desc())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Symbol {
    Term(Terminal),
    NonTerm(NonTerminal),
}

impl DescKey for Symbol {
    fn desc(&self) -> &str {
        match self {
            Symbol::Term(term) => term.desc(),
            Symbol::NonTerm(nonterm) => nonterm.desc(),
        }
    }
}

#[derive(Debug, Clone, Eq)]
pub struct Terminal {
    desc: Rc<str>,
}

impl PartialEq for Terminal {
    fn eq(&self, other: &Self) -> bool {
        self.ptr() == other.ptr()
    }
}

impl PartialOrd for Terminal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.ptr().partial_cmp(&other.ptr())
    }
}

impl Ord for Terminal {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ptr().cmp(&other.ptr())
    }
}

impl Hash for Terminal {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.ptr().hash(state)
    }
}

impl DescKey for Terminal {
    fn desc(&self) -> &str {
        &self.desc
    }
}

impl Terminal {
    pub fn new(desc: &str) -> Self {
        Self { desc: Rc::from(desc) }
    }

    fn ptr(&self) -> *const u8 {
        self.desc.as_bytes().as_ptr()
    }
}

impl fmt::Display for Terminal {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(self.desc())
    }
}

#[derive(Debug)]
struct NonTerminalInner {
    members: Box<[Symbol]>,
    desc: Box<str>,
}

#[derive(Debug, Clone)]
pub struct NonTerminal {
    inner: Rc<NonTerminalInner>,
}

impl PartialEq for NonTerminal {
    fn eq(&self, other: &Self) -> bool {
        self.ptr() == other.ptr()
    }
}

impl Eq for NonTerminal {}

impl PartialOrd for NonTerminal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.ptr().partial_cmp(&other.ptr())
    }
}

impl Ord for NonTerminal {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ptr().cmp(&other.ptr())
    }
}

impl Hash for NonTerminal {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.ptr().hash(state)
    }
}

impl DescKey for NonTerminal {
    fn desc(&self) -> &str {
        &self.inner.desc
    }
}

impl NonTerminal {
    pub fn new<S, V>(desc: S, members: V) -> Self
    where
        S: Into<Box<str>>,
        V: Into<Box<[Symbol]>>,
    {
        Self {
            inner: Rc::new(NonTerminalInner {
                members: members.into(),
                desc: desc.into(),
            }),
        }
    }

    pub fn contains(&self, term: &Terminal) -> bool {
        for member in self.members() {
            match member {
                Symbol::Term(other) if other == term => return true,
                Symbol::NonTerm(other) if other.contains(term) => return true,
                _ => (),
            }
        }

        false
    }

    pub fn members(&self) -> &[Symbol] {
        &self.inner.members
    }

    fn ptr(&self) -> *const NonTerminalInner {
        &*self.inner as *const _
    }
}

impl fmt::Display for NonTerminal {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(self.desc())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table<S>
where
    S: DescKey,
{
    elems: Box<[S]>,
}

impl<S> Table<S>
where
    S: DescKey,
{
    pub fn new<I>(iterable: I) -> Self
    where
        I: IntoIterator<Item = S>,
    {
        let mut elems = iterable.into_iter().collect::<Vec<_>>();
        elems.sort_by(DescKey::cmp_desc);

        let mut i = 1;
        while i < elems.len() {
            if elems[i - 1].desc() == elems[i].desc() {
                elems.remove(i - 1);
            } else {
                i += 1;
            }
        }

        Self { elems: elems.into() }
    }

    pub fn as_slice(&self) -> &[S] {
        &self.elems
    }

    pub fn find(&self, desc: &str) -> Option<&S> {
        self.elems
            .binary_search_by(|symbol| symbol.desc().cmp(desc))
            .ok()
            .map(|index| &self.elems[index])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    pub symbols: Vec<Terminal>,
}

#[macro_export]
macro_rules! make_terms {
    ($($desc:expr),*) => {
        $crate::symbol::Table::new(
            vec![$($crate::symbol::Terminal::new($desc)),*]
        )
    };
}

#[macro_export]
macro_rules! find_syms {
    ($table:expr, $map:expr, $($desc:expr),*) => {{
        let table = $table;
        let builder = || Some(vec![$({
            let symbol = table.find($desc)?;
            $map(symbol)
        }),*]);
        builder()
    }};
}
