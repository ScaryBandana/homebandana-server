// Copyright 2026 ScaryBandana
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::process::Command;

fn main() {
    // Rerun build script when HEAD changes.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");

    // Git commit hash
    let git_commit_hash = get_git_short_commit_hash();
    println!("cargo:rustc-env=GIT_COMMIT={git_commit_hash}");

    // Git branch name
    let git_branch_name = get_git_branch_name();
    println!("cargo:rustc-env=GIT_BRANCH={git_branch_name}");
}

// Hard-fails the build if no valid Git commit hash can be determined.
// This ensures that errors related to Git metadata get visible in CI, so that we don`t accidentally ship builds without Git metadata.
fn get_git_short_commit_hash() -> String {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .expect("Git is not installed or not available in PATH.");

    if !output.status.success() {
        let status = output.status;
        let stderr = String::from_utf8_lossy(&output.stderr);

        panic!("Git command 'git rev-parse --short HEAD' failed: {status}\n{stderr}",);
    }

    let git_commit_hash = String::from_utf8(output.stdout)
        .expect("Git commit hash is not valid UTF-8")
        .trim()
        .to_string();

    if git_commit_hash.is_empty() {
        panic!("Git commit hash is empty");
    }

    git_commit_hash
}

// Hard-fails the build if no valid Git branch name can be determined.
// This ensures that errors related to Git metadata get visible in CI, so that we don`t accidentally ship builds without Git metadata.
fn get_git_branch_name() -> String {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .expect("Git is not installed or not available in PATH.");

    if !output.status.success() {
        let status = output.status;
        let stderr = String::from_utf8_lossy(&output.stderr);

        panic!("Git command 'git rev-parse --abbrev-ref HEAD' failed: {status}\n{stderr}",);
    }

    let git_branch_name = String::from_utf8(output.stdout)
        .expect("Git branch name is not valid UTF-8")
        .trim()
        .to_string();

    if git_branch_name.is_empty() {
        panic!("Git branch name is empty");
    }

    if git_branch_name == "HEAD" {
        panic!("Detached HEAD detected. Cannot determine Git branch name.");
    }

    git_branch_name
}
