use std::process::Command;

fn main() {
    // Rerun this script if .git directory changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");

    // Get commit count
    let output = Command::new("git")
        .args(["rev-list", "--count", "HEAD"])
        .output();

    let commit_count: u64 = match output {
        Ok(o) if o.status.success() => String::from_utf8(o.stdout)
            .unwrap()
            .trim()
            .parse()
            .unwrap_or(0),
        _ => 0,
    };

    // Calculate version based on commits
    // x.y.z
    // z = count % 100
    // y = (count / 100) % 10
    // x = 1 + count / 1000

    let major = 1 + commit_count / 1000;
    let minor = (commit_count % 1000) / 100;
    let patch = commit_count % 100;

    let version = format!("{}.{}.{}", major, minor, patch);

    println!("cargo:rustc-env=GIT_VERSION={}", version);
}
