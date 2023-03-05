use std::{path::PathBuf, process::Command};

fn main() {
    generate_ona_bindings();
    compile_and_link_ona();
}

fn generate_ona_bindings() {
    let cwd = std::env::current_dir().unwrap();
    let include_path = cwd.join("ona/src");
    let bindings = bindgen::builder()
        .header("ona/src/NAR.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .clang_args(["-I", include_path.to_str().unwrap()])
        .allowlist_file(cwd.join("ona/src/.*\\.h").to_str().unwrap())
        .generate()
        .expect("Unable to generate ONA bindings.");
    bindings
        .write_to_file(PathBuf::from("src").join("sys.rs"))
        .expect("Unable to write bindings to file");
}

fn compile_and_link_ona() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let root_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    std::env::set_current_dir(&out_dir).unwrap();

    let main_c_path = root_dir.join("ona/src/main.c");
    let nar_first_stage_exe_path = out_dir.join("NAR_first_stage");
    let rule_table_path = root_dir.join("ona/src/RuleTable.c");

    // Source list
    let sources = std::fs::read_dir(root_dir.join("ona/src"))
        .unwrap()
        .chain(std::fs::read_dir(root_dir.join("ona/src/NetworkNAR")).unwrap())
        .filter_map(|entry| {
            let entry = entry.unwrap();

            if entry.file_type().unwrap().is_file()
                && entry.file_name().to_str().unwrap().ends_with(".c")
                && entry.file_name().to_str().unwrap() != "main.c"
            {
                Some(entry.path())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // for source in sources
    //     .iter()
    //     .chain([&main_c_path, &rule_table_path].into_iter())
    // {
    //     println!("cargo:rerun-if-changed={}", source.to_str().unwrap());
    // }

    // Common arguments
    let base_args = [
        // Base args
        "-D_POSIX_C_SOURCE=199506L",
        "-pedantic",
        "-std=c99",
        "-pthread",
        "-lpthread",
        "-lm",
        // Ignore warnings
        "-Wno-unknown-pragmas",
        "-Wno-tautological-compare",
        "-Wno-dollar-in-identifier-extension",
        "-Wno-unused-parameter",
        "-Wno-unused-variable",
    ];

    //
    // Stage 1
    //

    // TODO: detect if the SSE build fails, and then build without SSE.
    Command::new("cc")
        .args([
            "-mfpmath=sse",
            "-msse2",
            "-DSTAGE=1",
            "-Wall",
            "-Wextra",
            "-Wformat-security",
        ])
        .args(&sources)
        .arg(root_dir.join("ona/src/main.c"))
        .args(base_args)
        .arg(format!("-o{}", nar_first_stage_exe_path.to_str().unwrap()))
        .run();

    // Generate rule table
    let rule_table = Command::new(nar_first_stage_exe_path)
        .arg("NAL_GenerateRuleTable")
        .output()
        .unwrap()
        .stdout;
    std::fs::write(rule_table_path, rule_table).unwrap();

    //
    // Stage 2
    //

    Command::new("cc")
        .args(["-mfpmath=sse", "-msse2", "-c", "-DSTAGE=2"])
        .args(&sources)
        .arg(root_dir.join("ona/src/RuleTable.c"))
        .args(base_args)
        .run();
    let objects = std::fs::read_dir(&out_dir)
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.unwrap();
            if entry.file_name().to_str().unwrap().ends_with(".o") {
                Some(entry.path())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    Command::new("ar")
        .args(["rcs", "libONA.a"])
        .args(objects)
        .run();

    println!("cargo:rustc-link-search={}", out_dir.to_str().unwrap());
    println!("cargo:rustc-link-lib=static=ONA");
}

trait CommandExt {
    fn run(&mut self);
}

impl CommandExt for Command {
    fn run(&mut self) {
        let cmd = format!("{self:?}");
        if !self.spawn().unwrap().wait().unwrap().success() {
            panic!("Command failed: {cmd}");
        }
    }
}
