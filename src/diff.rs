use diffy::PatchFormatter;

pub fn create_patch(old: &str, new: &str, max_body: usize) -> String {
    let patch = diffy::create_patch(old, new);

    let lines: Vec<String> = PatchFormatter::new()
        .missing_newline_message(false)
        .fmt_patch(&patch)
        .to_string()
        .to_owned()
        .lines()
        .filter(|l| !l.starts_with("---") && !l.starts_with("+++"))
        .map(|l| l.to_owned())
        .collect();

    let mut body = String::new();
    let mut truncated = 0_usize;

    for (index, line) in lines.iter().enumerate() {
        if body.len() + line.len() + 1 > max_body {
            truncated = lines.len() - index;
            break;
        }
        body.push_str(line);
        body.push('\n');
    }
    if truncated > 0 {
        body.push_str(&format!("… {truncated} more diff lines\n"));
    }

    body
}
