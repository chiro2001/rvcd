use std::env;
use std::error::Error;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("rvcd_descriptor.bin"))
        .compile(&["proto/rvcd.proto"], &["proto"])
        .unwrap();

    tonic_build::compile_protos("proto/scaleda.proto")?;

    let antlr_url = "https://github.com/rrevenantt/antlr4rust/releases/download/antlr4-4.8-2-Rust0.3.0-beta/antlr4-4.8-2-SNAPSHOT-complete.jar";
    let antlr_path_str = format!("target/{}", antlr_url.split("/").last().unwrap());
    let antlr_path = std::path::Path::new(antlr_path_str.as_str());
    let antlr_file_exists = || antlr_path.exists();
    if !antlr_file_exists() {
        let data = reqwest::get(antlr_url)
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        std::fs::File::create(antlr_path.as_os_str())
            .unwrap()
            .write_all(data.as_ref())
            .unwrap();
    }
    assert!(antlr_file_exists());
    let antlr_path_abs = std::fs::canonicalize(antlr_path).unwrap();
    let antlr_path_abs = antlr_path_abs.into_os_string();
    let antlr_path_abs = antlr_path_abs.to_str().unwrap();
    println!("cargo:rerun-if-changed={}", antlr_path_abs);
    let grammars = vec!["VerilogLexer", "VerilogParser"];
    let additional_args = vec![Some("-visitor"), Some("-visitor")];
    for (grammar, arg) in grammars.into_iter().zip(additional_args) {
        let grammar_path = format!("{}.g4", grammar);
        gen_for_grammar(grammar_path.as_str(), antlr_path_abs, arg)?;
        println!("cargo:rerun-if-changed={}", grammar_path);
    }
    Ok(())
}

fn gen_for_grammar(
    grammar_file_path: &str,
    antlr_path: &str,
    additional_arg: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let input = env::current_dir().unwrap().join("antlr");
    let mut child = Command::new("java")
        .current_dir(input)
        .arg("-cp")
        .arg(antlr_path)
        .arg("org.antlr.v4.Tool")
        .arg("-Dlanguage=Rust")
        .arg("-o")
        // .arg(env::var("OUT_DIR").unwrap())
        .arg("../src/verilog")
        .arg(grammar_file_path)
        .args(additional_arg)
        .spawn()
        .expect("antlr tool failed to start");
    assert!(child.wait().unwrap().success());
    Ok(())
}
