use crate::filters::CommandKind;

/// A benchmark scenario with input, expected command kind, and required signals.
pub struct Scenario {
    pub name: String,
    pub kind: CommandKind,
    pub input: String,
    /// Strings that MUST appear in filtered output (false positive detection).
    pub required_signals: Vec<String>,
}

/// All benchmark scenarios using realistic terminal output.
#[must_use]
pub fn all_scenarios() -> Vec<Scenario> {
    vec![
        git_push_scenario(),
        git_clone_scenario(),
        npm_install_scenario(),
        npm_install_clean_scenario(),
        cargo_build_scenario(),
        cargo_test_scenario(),
        docker_build_scenario(),
        docker_pull_scenario(),
        kubectl_events_scenario(),
        mixed_ansi_scenario(),
        pure_signal_scenario(),
    ]
}

fn git_push_scenario() -> Scenario {
    Scenario {
        name: "git push (transfer stats)".into(),
        kind: CommandKind::Git,
        input: "\
Enumerating objects: 42, done.
Counting objects: 100% (42/42), done.
Delta compression using up to 10 threads
Compressing objects: 100% (28/28), done.
Writing objects: 100% (30/30), 12.45 KiB | 6.22 MiB/s, done.
Total 30 (delta 18), reused 0 (delta 0), pack-reused 0
remote: Resolving deltas: 100% (18/18), completed with 8 local objects.
remote:
remote: Create a pull request for 'feat/denoiser' on GitHub by visiting:
remote:   https://github.com/Orellius/cli-denoiser/pull/new/feat/denoiser
remote:
To https://github.com/Orellius/cli-denoiser.git
 * [new branch]      feat/denoiser -> feat/denoiser
branch 'feat/denoiser' set up to track 'origin/feat/denoiser'."
            .into(),
        required_signals: vec![
            "feat/denoiser -> feat/denoiser".into(),
            "set up to track".into(),
        ],
    }
}

fn git_clone_scenario() -> Scenario {
    Scenario {
        name: "git clone (verbose)".into(),
        kind: CommandKind::Git,
        input: "\
Cloning into 'cli-denoiser'...
remote: Enumerating objects: 156, done.
remote: Counting objects: 100% (156/156), done.
remote: Compressing objects: 100% (98/98), done.
remote: Total 156 (delta 52), reused 140 (delta 40), pack-reused 0
Receiving objects: 100% (156/156), 48.23 KiB | 1.20 MiB/s, done.
Resolving deltas: 100% (52/52), done."
            .into(),
        required_signals: vec!["Cloning into".into()],
    }
}

fn npm_install_scenario() -> Scenario {
    Scenario {
        name: "npm install (deprecation spam)".into(),
        kind: CommandKind::Npm,
        input: "\
npm warn deprecated inflight@1.0.6: This module is not supported
npm warn deprecated glob@7.2.3: Glob versions prior to v9 are no longer supported
npm warn deprecated rimraf@3.0.2: Rimraf versions prior to v4 are no longer supported
npm warn deprecated @humanwhocodes/config-array@0.11.14: Use @eslint/config-array
npm warn deprecated @humanwhocodes/object-schema@2.0.3: Use @eslint/object-schema
npm warn deprecated eslint@8.57.0: This version is no longer supported
npm warn deprecated glob@7.2.3: Glob versions prior to v9 are no longer supported
npm warn deprecated glob@7.2.3: Glob versions prior to v9 are no longer supported
npm warn deprecated glob@7.2.3: Glob versions prior to v9 are no longer supported
npm warn deprecated glob@7.2.3: Glob versions prior to v9 are no longer supported

added 487 packages, and audited 488 packages in 12s

82 packages are looking for funding
  run `npm fund` for details

3 moderate severity vulnerabilities

To address all issues, run:
  npm audit fix"
            .into(),
        required_signals: vec![
            "added 487 packages".into(),
            "3 moderate severity".into(),
            "npm audit fix".into(),
        ],
    }
}

fn npm_install_clean_scenario() -> Scenario {
    Scenario {
        name: "npm install (clean, no warnings)".into(),
        kind: CommandKind::Npm,
        input: "\
added 150 packages, and audited 151 packages in 4s

found 0 vulnerabilities"
            .into(),
        required_signals: vec!["added 150 packages".into(), "0 vulnerabilities".into()],
    }
}

fn cargo_build_scenario() -> Scenario {
    Scenario {
        name: "cargo build (69 crates)".into(),
        kind: CommandKind::Cargo,
        input: "\
    Updating crates.io index
     Locking 94 packages to latest Rust 1.93.1 compatible versions
      Adding rusqlite v0.32.1 (available: v0.39.0)
 Downloading crates ...
  Downloaded tokio-macros v2.7.0
  Downloaded cc v1.2.59
  Downloaded tokio v1.51.0
   Compiling proc-macro2 v1.0.106
   Compiling unicode-ident v1.0.24
   Compiling quote v1.0.45
   Compiling libc v0.2.184
   Compiling serde_core v1.0.228
   Compiling cfg-if v1.0.4
   Compiling memchr v2.8.0
   Compiling once_cell v1.21.4
   Compiling regex-syntax v0.8.10
   Compiling aho-corasick v1.1.4
   Compiling regex-automata v0.4.14
   Compiling regex v1.12.3
   Compiling syn v2.0.117
   Compiling serde v1.0.228
   Compiling serde_json v1.0.149
   Compiling clap v4.6.0
   Compiling chrono v0.4.44
   Compiling rusqlite v0.32.1
   Compiling cli-denoiser v0.1.0 (/home/user/cli-denoiser)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 23.32s"
            .into(),
        required_signals: vec!["Finished".into(), "23.32s".into()],
    }
}

fn cargo_test_scenario() -> Scenario {
    Scenario {
        name: "cargo test (with failures)".into(),
        kind: CommandKind::Cargo,
        input: "\
   Compiling cli-denoiser v0.1.0
    Finished `test` profile in 0.40s
     Running unittests src/lib.rs

running 37 tests
test filters::ansi::tests::strips_color_codes ... ok
test filters::ansi::tests::preserves_clean_text ... ok
test filters::git::tests::block_collapses_transfer ... FAILED
test filters::npm::tests::keeps_errors ... ok
test pipeline::tests::pipeline_drops_empty_lines ... ok

failures:

---- filters::git::tests::block_collapses_transfer stdout ----
thread panicked at src/filters/git.rs:189:9:
assertion `left == right` failed
  left: 4
 right: 2

failures:
    filters::git::tests::block_collapses_transfer

test result: FAILED. 35 passed; 1 failed; 0 ignored"
            .into(),
        required_signals: vec![
            "FAILED".into(),
            "block_collapses_transfer".into(),
            "left: 4".into(),
            "35 passed; 1 failed".into(),
        ],
    }
}

fn docker_build_scenario() -> Scenario {
    Scenario {
        name: "docker build (cached layers)".into(),
        kind: CommandKind::Docker,
        input: "\
Step 1/8 : FROM node:20-alpine
 ---> a1b2c3d4e5f6
Step 2/8 : WORKDIR /app
 ---> Using cache
 ---> b2c3d4e5f6a1
Step 3/8 : COPY package*.json ./
 ---> Using cache
 ---> c3d4e5f6a1b2
Step 4/8 : RUN npm ci --production
 ---> Using cache
 ---> d4e5f6a1b2c3
Step 5/8 : COPY . .
 ---> e5f6a1b2c3d4
Step 6/8 : RUN npm run build
 ---> Running in f6a1b2c3d4e5
> next build
Build complete
Removing intermediate container f6a1b2c3d4e5
 ---> a1b2c3d4e5f7
Step 7/8 : EXPOSE 3000
 ---> Running in b2c3d4e5f6a2
Removing intermediate container b2c3d4e5f6a2
 ---> c3d4e5f6a1b3
Step 8/8 : CMD [\"node\", \"server.js\"]
 ---> Running in d4e5f6a1b2c4
Removing intermediate container d4e5f6a1b2c4
 ---> e5f6a1b2c3d5
Successfully built e5f6a1b2c3d5
Successfully tagged myapp:latest"
            .into(),
        required_signals: vec![
            "Step 6/8".into(),
            "npm run build".into(),
            "Build complete".into(),
            "Successfully built".into(),
            "Successfully tagged".into(),
        ],
    }
}

fn docker_pull_scenario() -> Scenario {
    Scenario {
        name: "docker pull (layer progress)".into(),
        kind: CommandKind::Docker,
        input: "\
Using default tag: latest
latest: Pulling from library/node
a1b2c3d4e5f6: Pulling fs layer
b2c3d4e5f6a1: Pulling fs layer
c3d4e5f6a1b2: Pulling fs layer
a1b2c3d4e5f6: Downloading  25.6MB/52.1MB
b2c3d4e5f6a1: Downloading  5.2MB/10.0MB
a1b2c3d4e5f6: Pull complete
b2c3d4e5f6a1: Pull complete
c3d4e5f6a1b2: Downloading  120MB/245MB
c3d4e5f6a1b2: Pull complete
Digest: sha256:abc123def456
Status: Downloaded newer image for node:latest
docker.io/library/node:latest"
            .into(),
        required_signals: vec!["docker.io/library/node:latest".into()],
    }
}

fn kubectl_events_scenario() -> Scenario {
    Scenario {
        name: "kubectl events (scheduling noise)".into(),
        kind: CommandKind::Kubectl,
        input: "\
LAST SEEN   TYPE     REASON           OBJECT              MESSAGE
2m          Normal   Scheduled        pod/web-abc         Successfully assigned
2m          Normal   Pulling          pod/web-abc         Pulling image nginx:1.25
1m          Normal   Pulled           pod/web-abc         Successfully pulled
1m          Normal   Created          pod/web-abc         Created container nginx
1m          Normal   Started          pod/web-abc         Started container nginx
30s         Warning  BackOff          pod/worker-xyz      Back-off restarting failed container
15s         Warning  FailedMount      pod/api-def         MountVolume.SetUp failed"
            .into(),
        required_signals: vec!["BackOff".into(), "FailedMount".into(), "Back-off".into()],
    }
}

fn mixed_ansi_scenario() -> Scenario {
    Scenario {
        name: "mixed ANSI + progress".into(),
        kind: CommandKind::Unknown,
        input: "\
\x1b[32m✓\x1b[0m Loading configuration
\x1b[32m✓\x1b[0m Connecting to database
\x1b[33m⠋\x1b[0m Processing records...
\x1b[33m⠙\x1b[0m Processing records... 25%
\x1b[33m⠹\x1b[0m Processing records... 50%
\x1b[33m⠸\x1b[0m Processing records... 75%
\x1b[33m⠼\x1b[0m Processing records... 100%
\x1b[32m✓\x1b[0m Processed 1,234 records
\x1b[31m✗\x1b[0m Failed to send notification: SMTP timeout
\x1b[1m\x1b[4mSummary:\x1b[0m 1,233 succeeded, 1 failed"
            .into(),
        required_signals: vec![
            "Loading configuration".into(),
            "Processed 1,234 records".into(),
            "SMTP timeout".into(),
            "1,233 succeeded, 1 failed".into(),
        ],
    }
}

fn pure_signal_scenario() -> Scenario {
    Scenario {
        name: "pure signal (no noise)".into(),
        kind: CommandKind::Unknown,
        input: "\
error[E0308]: mismatched types
 --> src/main.rs:10:5
  |
8 | fn foo() -> i32 {
  |             --- expected `i32` because of return type
9 |     let x = \"hello\";
10|     x
  |     ^ expected `i32`, found `&str`

For more information about this error, try `rustc --explain E0308`."
            .into(),
        required_signals: vec![
            "error[E0308]".into(),
            "mismatched types".into(),
            "expected `i32`, found `&str`".into(),
            "rustc --explain".into(),
        ],
    }
}
