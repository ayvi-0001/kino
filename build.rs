use std::{process::Command, str};

use vergen_gitcl::{BuildBuilder, CargoBuilder, Emitter, GitclBuilder, RustcBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");

    Emitter::default()
        .add_instructions(&BuildBuilder::all_build()?)?
        .add_instructions(&CargoBuilder::all_cargo()?)?
        .add_instructions(&RustcBuilder::all_rustc()?)?
        .add_instructions(&GitclBuilder::all_git()?)?
        .add_instructions(&GitclBuilder::default().describe(true, false, None).build()?)?
        .emit()?;

    let (insertions, deletions) = get_git_shortstat();
    println!("cargo:rustc-env=GIT_INSERTIONS={}", insertions);
    println!("cargo:rustc-env=GIT_DELETIONS={}", deletions);

    Ok(())
}

fn get_git_shortstat() -> (u32, u32) {
    let output = Command::new("git")
        .args(["diff", "--shortstat", "HEAD"])
        .output()
        .expect("Failed to run git command");

    let stdout = str::from_utf8(&output.stdout).expect("Invalid UTF-8");
    let stats = stdout.trim();

    let parts: Vec<&str> = stats.split(',').collect();
    let mut insertions = 0;
    let mut deletions = 0;

    for part in parts {
        if part.contains("insertion") {
            insertions = part.split_whitespace().next().unwrap_or("0").parse().unwrap_or(0);
        } else if part.contains("deletion") {
            deletions = part.split_whitespace().next().unwrap_or("0").parse().unwrap_or(0);
        }
    }

    (insertions, deletions)
}
