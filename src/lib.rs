use std::{
    env,
    io::BufRead,
    path::{Path, PathBuf},
    process::Command,
};

pub fn build_target_dep(
    package_root: impl AsRef<Path>,
    output_path: impl Into<PathBuf>,
) -> TargetDep {
    TargetDep {
        manifest_path: package_root.as_ref().join("Cargo.toml"),
        output_path: output_path.into(),
        ..Default::default()
    }
}

#[derive(Default)]
#[must_use = "must call build()"]
pub struct TargetDep {
    manifest_path: PathBuf,
    output_path: PathBuf,
    profile: Option<String>,
    target: Option<String>,
}

impl TargetDep {
    pub fn release(mut self) -> Self {
        self.profile = Some("release".to_string());
        self
    }

    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    pub fn build(self) {
        // Run `cargo build`

        let cargo_bin = build_env_var("CARGO");
        let mut cmd = Command::new(cargo_bin);

        cmd.arg("build");

        cmd.arg("--manifest-path");
        cmd.arg(&self.manifest_path);

        let profile = self.profile.as_deref().unwrap_or("debug");
        cmd.arg("--profile");
        cmd.arg(profile);

        // e.g. target/target-deps/output__path/
        let target_dir = Path::new(&build_env_var("OUT_DIR"))
            .join("target-deps")
            .join(
                self.output_path
                    .to_string_lossy()
                    .replace(std::path::MAIN_SEPARATOR, "__"),
            );
        cmd.arg("--target-dir");
        cmd.arg(&target_dir);

        if let Some(ref target) = self.target {
            cmd.arg("--target");
            cmd.arg(target);
        }

        let status = cmd.status().expect("failed to execute process");
        if !status.success() {
            panic!(
                "error building target dep {:?}: {:?}",
                self.manifest_path, status
            );
        }

        // Read dependency file

        let mut out_dir = target_dir;
        if let Some(ref target) = self.target {
            out_dir = out_dir.join(target);
        }
        out_dir = out_dir.join(profile);

        // TODO(lann): Better error handling/reporting
        for dep_file in glob::glob(out_dir.join("*.d").to_str().unwrap()).unwrap() {
            let contents = std::fs::read(dep_file.unwrap()).unwrap();
            // TODO(lann): Handle multiple output  lines/files (?)
            let line = contents.lines().next().unwrap().unwrap();

            // Split on spaces, skipping escaped spaces
            // https://github.com/rust-lang/rust/blob/65f6d33b775eddfc0128c04083bbf3beea360114/compiler/rustc_interface/src/passes.rs#L596
            let mut paths: Vec<&str> = Vec::new();
            let mut last_match = 0;
            for (idx, _) in line.match_indices(' ') {
                if idx > 0 && &line[idx - 1..idx] != "\\" {
                    paths.push(&line[last_match..idx]);
                    last_match = idx + 1;
                }
            }
            paths.push(&line[last_match..]);

            let (out_path, dep_paths) = paths.split_first().unwrap();
            let out_path = out_path
                .strip_suffix(':')
                .expect("output missing trailing :");

            // Move output
            std::fs::rename(out_path, &self.output_path).unwrap_or_else(|err| {
                panic!(
                    "Failed to move output {:?} to {:?}: {}",
                    out_path, &self.output_path, err
                )
            });

            // Emit dependencies
            for dep_path in dep_paths {
                println!("cargo:rerun-if-changed={}", dep_path);
            }
        }
    }
}

fn build_env_var(key: impl AsRef<str>) -> String {
    env::var(key.as_ref()).unwrap_or_else(|_| {
        panic!(
            "missing required env var {:?}; cargo-target-dep is meant to be used from a build.rs",
            key.as_ref()
        )
    })
}
