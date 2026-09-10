use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref ENTRY_REGEX: Regex =
        Regex::new(r"^\s*(?<title>.+?)(?:\s+\(?(?<year>\d{4})\)?)?\s*$").unwrap();
    static ref COMMENT_REGEX: Regex = Regex::new(r"#[^#]*$").unwrap();
}

#[derive(Debug, Default)]
pub(super) struct EntryParts {
    pub name: String,
    pub year: Option<i64>,
    pub comment: Option<String>,
}

impl EntryParts {
    pub fn new(text: &str) -> Result<EntryParts, anyhow::Error> {
        let mut entry_parts = EntryParts::default();

        let mut haystack = text.to_string();

        if let Some(mat) = COMMENT_REGEX.find(text) {
            entry_parts.comment = Some(mat.as_str().to_owned());
            haystack = COMMENT_REGEX.replace(text, "").trim().to_string()
        }

        if let Some(caps) = ENTRY_REGEX.captures(&haystack) {
            if let Some(mat) = caps.name("title") {
                entry_parts.name = mat.as_str().to_owned();
            }
            if let Some(mat) = caps.name("year")
                && let Ok(year) = mat.as_str().trim_matches(|c| c == '(' || c == ')').parse::<i64>()
            {
                entry_parts.year = Some(year);
            }
        }

        match entry_parts.name.eq(&String::default()) {
            true => Err(anyhow::anyhow!("Entry missing title")),
            false => Ok(entry_parts),
        }
    }
}
