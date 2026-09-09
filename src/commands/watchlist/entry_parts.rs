use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    pub static ref ENTRY_REGEX: Regex =
        Regex::new(r"^\s*(?<title>.+?)(?:\s+\(?(?<year>\d{4})\)?)?\s*$").unwrap();
}

#[derive(Debug)]
pub(super) struct EntryParts(pub(super) String, pub(super) Option<i64>);

impl EntryParts {
    pub fn new(text: &str) -> Result<EntryParts, anyhow::Error> {
        let mut name: Option<String> = None;
        let mut year: Option<i64> = None;

        if let Some(caps) = ENTRY_REGEX.captures(text) {
            if let Some(name_match) = caps.name("title") {
                name = Some(name_match.as_str().to_owned());
            }
            if let Some(year_match) = caps.name("year") {
                let year_str = year_match.as_str();
                let year_trimmed = year_str
                    .strip_prefix("(")
                    .unwrap_or(year_str)
                    .strip_suffix(")")
                    .unwrap_or(year_str);

                if let Ok(year_parsed) = year_trimmed.parse::<i64>() {
                    year = Some(year_parsed);
                }
            }
        }

        if let Some(name) = name {
            Ok(EntryParts(name, year))
        } else {
            Err(anyhow::anyhow!("Entry missing title"))
        }
    }
}
