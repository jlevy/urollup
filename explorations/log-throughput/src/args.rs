//! Minimal `--name value` and `--flag` argument parsing.

use crate::Result;

pub struct Args {
    pairs: Vec<(String, Option<String>)>,
}

impl Args {
    pub fn parse(raw: &[String]) -> Result<Args> {
        let mut pairs = Vec::new();
        let mut i = 0;
        while i < raw.len() {
            let Some(name) = raw[i].strip_prefix("--") else {
                return Err(format!("unexpected argument #{}", i + 1).into());
            };
            let value = raw.get(i + 1).filter(|v| !v.starts_with("--")).cloned();
            i += if value.is_some() { 2 } else { 1 };
            pairs.push((name.to_string(), value));
        }
        Ok(Args { pairs })
    }

    pub fn value(&self, name: &str) -> Option<&str> {
        self.pairs.iter().find(|(n, _)| n == name).and_then(|(_, v)| v.as_deref())
    }

    pub fn required(&self, name: &str) -> Result<&str> {
        self.value(name).ok_or_else(|| format!("missing --{name}").into())
    }

    pub fn flag(&self, name: &str) -> bool {
        self.pairs.iter().any(|(n, _)| n == name)
    }
}
