// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::process::Command;

fn run_cmd(args: &[&str]) -> Result<String, Box<dyn Error>> {
    let cmd = Command::new(args[0]).args(&args[1..]).output()?;
    if cmd.status.success() {
        Ok(String::from_utf8_lossy(&cmd.stdout).trim().to_string())
    } else {
        Err(From::from("failed"))
    }
}

fn read_file(filename: &str) -> Result<String, Box<dyn Error>> {
    let mut file = File::open(filename)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    contents = contents.trim().to_string();

    Ok(contents)
}

fn print_version(include_sha: bool, include_local: bool) -> Result<(), Box<dyn Error>> {
    let mut version = read_file("../../CURRENT_VERSION")?;
    let sha = run_cmd(&["git", "rev-parse", "HEAD"])?;

    if include_sha {
        version.push('-');
        version.push_str(&sha);

        // if we're a non-release build, check to see if git has
        // unstaged changes
        if include_local && run_cmd(&["git", "diff", "--quiet"]).is_err() {
            version.push('.');
            version.push_str("localchanges");
        }
    }

    println!("cargo:rustc-env=GIT_VERSION={sha}");
    println!("cargo:rustc-env=ONEFUZZ_VERSION={version}");

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    // If we're built off of a tag, we accept CURRENT_VERSION as is.  Otherwise
    // modify it to indicate local build
    let (include_sha, include_local_changes) = if let Ok(val) = env::var("GITHUB_REF") {
        (!val.starts_with("refs/tags/"), false)
    } else {
        (true, true)
    };
    print_version(include_sha, include_local_changes)
}
