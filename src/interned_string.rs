use std::collections::HashMap;
use std::sync::Mutex;

lazy_static! {
    static ref STRING_INTERN: Mutex<(Vec<String>, HashMap<String, InternedString>)> =
        Mutex::new((Vec::new(), HashMap::new()));
}

#[derive(Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Copy)]
pub struct InternedString {
    index: u8,
}

impl InternedString {
    fn max() -> usize {
        ::std::u8::MAX as usize
    }
    pub fn from_string(s: &str) -> Self {
        let mut intern = STRING_INTERN.lock().unwrap();
        if let Some(idx) = intern.1.get(s) {
            return *idx;
        }
        assert!(intern.0.len() < InternedString::max());
        intern.0.push(s.to_string());
        let out = Self {
            index: (intern.0.len() - 1) as u8,
        };
        intern.1.insert(s.to_string(), out);
        out
    }
    pub fn to_string(&self) -> String {
        STRING_INTERN.lock().unwrap().0[self.index as usize].clone()
    }
}
