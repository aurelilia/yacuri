use alloc::{fmt, string::String, sync::Arc};
use core::{borrow::Borrow, cmp, cmp::Ordering, hash, ops::Deref};

/// This module is almost 1:1 from rust_analyzer: https://github.com/rust-analyzer/smol_str
/// Thank you to the rust_analyzer team!
/// Small changes were necessary to make it run inside a no_std environment,
/// maintaining a new crate for a single file seemed overkill so it's simply here.

/// A `SmolStr` is a string type that has the following properties:
///
/// * `size_of::<SmolStr>() == size_of::<String>()`
/// * `Clone` is `O(1)`
/// * Strings are stack-allocated if they are:
///     * Up to 22 bytes long
///     * Longer than 22 bytes, but substrings of `WS` (see below). Such strings consist
///     solely of consecutive newlines, followed by consecutive spaces
/// * If a string does not satisfy the aforementioned conditions, it is heap-allocated
///
/// Unlike `String`, however, `SmolStr` is immutable. The primary use case for
/// `SmolStr` is a good enough default storage for tokens of typical programming
/// languages. Strings consisting of a series of newlines, followed by a series of
/// whitespace are a typical pattern in computer programs because of indentation.
/// Note that a specialized interner might be a better solution for some use cases.
#[derive(Clone)]
pub struct SmolStr(Repr);

impl SmolStr {
    /// Constructs inline variant of `SmolStr`.
    ///
    /// Panics if `text.len() > 22`.
    #[inline]
    pub const fn new_inline(text: &str) -> SmolStr {
        let mut buf = [0; INLINE_CAP];
        let mut i = 0;
        while i < text.len() {
            buf[i] = text.as_bytes()[i];
            i += 1
        }
        SmolStr(Repr::Inline {
            len: text.len() as u8,
            buf,
        })
    }

    pub fn new<T>(text: T) -> SmolStr
    where
        T: AsRef<str>,
    {
        SmolStr(Repr::new(text))
    }

    #[inline(always)]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Default for SmolStr {
    fn default() -> SmolStr {
        SmolStr::new("")
    }
}

impl Deref for SmolStr {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<SmolStr> for SmolStr {
    fn eq(&self, other: &SmolStr) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for SmolStr {}

impl PartialEq<str> for SmolStr {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<SmolStr> for str {
    fn eq(&self, other: &SmolStr) -> bool {
        other == self
    }
}

impl<'a> PartialEq<&'a str> for SmolStr {
    fn eq(&self, other: &&'a str) -> bool {
        self == *other
    }
}

impl<'a> PartialEq<SmolStr> for &'a str {
    fn eq(&self, other: &SmolStr) -> bool {
        *self == other
    }
}

impl PartialEq<String> for SmolStr {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<SmolStr> for String {
    fn eq(&self, other: &SmolStr) -> bool {
        other == self
    }
}

impl<'a> PartialEq<&'a String> for SmolStr {
    fn eq(&self, other: &&'a String) -> bool {
        self == *other
    }
}

impl<'a> PartialEq<SmolStr> for &'a String {
    fn eq(&self, other: &SmolStr) -> bool {
        *self == other
    }
}

impl Ord for SmolStr {
    fn cmp(&self, other: &SmolStr) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl PartialOrd for SmolStr {
    fn partial_cmp(&self, other: &SmolStr) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl hash::Hash for SmolStr {
    fn hash<H: hash::Hasher>(&self, hasher: &mut H) {
        self.as_str().hash(hasher)
    }
}

impl fmt::Debug for SmolStr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for SmolStr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl<T> From<T> for SmolStr
where
    T: Into<String> + AsRef<str>,
{
    fn from(text: T) -> Self {
        Self::new(text)
    }
}

impl From<SmolStr> for String {
    fn from(text: SmolStr) -> Self {
        text.as_str().into()
    }
}

impl Borrow<str> for SmolStr {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

const INLINE_CAP: usize = 22;
const N_NEWLINES: usize = 32;
const N_SPACES: usize = 128;
const WS: &str =
    "\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n                                                                                                                                ";

#[derive(Clone, Debug)]
enum Repr {
    Heap(Arc<str>),
    Inline { len: u8, buf: [u8; INLINE_CAP] },
    Substring { newlines: usize, spaces: usize },
}

impl Repr {
    fn new<T>(text: T) -> Self
    where
        T: AsRef<str>,
    {
        {
            let text = text.as_ref();

            let len = text.len();
            if len <= INLINE_CAP {
                let mut buf = [0; INLINE_CAP];
                buf[..len].copy_from_slice(text.as_bytes());
                return Repr::Inline {
                    len: len as u8,
                    buf,
                };
            }

            if len <= N_NEWLINES + N_SPACES {
                let bytes = text.as_bytes();
                let possible_newline_count = cmp::min(len, N_NEWLINES);
                let newlines = bytes[..possible_newline_count]
                    .iter()
                    .take_while(|&&b| b == b'\n')
                    .count();
                let possible_space_count = len - newlines;
                if possible_space_count <= N_SPACES && bytes[newlines..].iter().all(|&b| b == b' ')
                {
                    let spaces = possible_space_count;
                    return Repr::Substring { newlines, spaces };
                }
            }
        }

        Repr::Heap(text.as_ref().into())
    }

    #[inline(always)]
    fn len(&self) -> usize {
        match self {
            Repr::Heap(data) => data.len(),
            Repr::Inline { len, .. } => *len as usize,
            Repr::Substring { newlines, spaces } => *newlines + *spaces,
        }
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        match self {
            Repr::Heap(data) => data.is_empty(),
            Repr::Inline { len, .. } => *len == 0,
            // A substring isn't created for an empty string.
            Repr::Substring { .. } => false,
        }
    }

    #[inline]
    fn as_str(&self) -> &str {
        match self {
            Repr::Heap(data) => &*data,
            Repr::Inline { len, buf } => {
                let len = *len as usize;
                let buf = &buf[..len];
                unsafe { ::core::str::from_utf8_unchecked(buf) }
            }
            Repr::Substring { newlines, spaces } => {
                let newlines = *newlines;
                let spaces = *spaces;
                assert!(newlines <= N_NEWLINES && spaces <= N_SPACES);
                &WS[N_NEWLINES - newlines..N_NEWLINES + spaces]
            }
        }
    }
}
