use std::{
    env,
    io::BufRead,
    path::{Path, PathBuf},
    process::Command,
};

pub fn build_target_dep(name: impl Into<String>, package_root: impl AsRef<Path>) -> TargetDep {
    TargetDep {
        name: name.into(),
        manifest_path: package_root.as_ref().join("Cargo.toml"),
        ..Default::default()
    }
}

#[derive(Default)]
#[must_use = "must call build()"]
pub struct TargetDep {
    name: String,
    manifest_path: PathBuf,
    dest_path: Option<PathBuf>,
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

        // e.g. target/target-deps/<name>/
        let target_dir = Path::new(&build_env_var("OUT_DIR"))
            .join("target-deps")
            .join(&self.name);
        cmd.arg("--target-dir");
        cmd.arg(&target_dir);

        if let Some(ref target) = self.target {
            cmd.arg("--target");
            cmd.arg(target);
        }

        let status = cmd.status().expect("failed to execute process");
        if !status.success() {
            panic!(
                "error building target dep {:?} at {:?}: {:?}",
                self.name, self.manifest_path, status
            );
        }

        // Read dependency file

        let mut out_dir = target_dir;
        if let Some(ref target) = self.target {
            out_dir = out_dir.join(target);
        }
        out_dir = out_dir.join(profile);

        let dest_path = self.dest_path.unwrap_or_else(|| {
            Path::new(build_env_var("CARGO_MANIFEST_DIR").as_str()).join("target-deps")
        });
        if !dest_path.exists() {
            std::fs::create_dir(&dest_path).unwrap();
        }

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
                .strip_suffix(":")
                .expect("output missing trailing :");

            // Move output to destination
            let dest_file = dest_path.join(Path::new(out_path).file_name().unwrap());
            std::fs::rename(out_path, dest_file).unwrap();

            // Emit dependencies
            for dep_path in dep_paths {
                println!("cargo:rerun-if-changed={}", dep_path);
            }
        }
    }
}

fn build_env_var(key: impl AsRef<str>) -> String {
    env::var(key.as_ref()).expect(
        format!(
            "missing required env var {:?}; cargo-target-dep is meant to be used from a build.rs",
            key.as_ref()
        )
        .as_ref(),
    )
}
