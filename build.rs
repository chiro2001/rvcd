use std::env;
use std::error::Error;
use std::io::Write;
use std::process::Command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    tonic_build::compile_protos("proto/rvcd.proto")?;

    // auto download jar file
    // let antlr_version = "4.12.0";
    // let antlr_version = "4.8.0";
    // let antlr_version = "4.8-2-SNAPSHOT";
    // let antlr_url = format!(
    //     // "https://www.antlr.org/download/antlr-{}-complete.jar",
    //     "https://repo1.maven.org/maven2/org/antlr/antlr4/{}/antlr4-{}.jar",
    //     antlr_version, antlr_version
    // );
    let antlr_url = "https://github.com/rrevenantt/antlr4rust/releases/download/antlr4-4.8-2-Rust-0.2/antlr4-4.8-2-SNAPSHOT-complete.jar";
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
    println!(
        "cargo:rerun-if-changed={}",
        std::fs::canonicalize(antlr_path).unwrap().to_str().unwrap()
    );
    let grammars = vec!["verilog/VerilogLexer", "verilog/VerilogParser"];
    let additional_args = vec![Some("-visitor"), Some("-visitor")];
    for (grammar, arg) in grammars.into_iter().zip(additional_args) {
        let grammar_path = format!("antlr/{}.g4", grammar);
        gen_for_grammar(grammar_path.as_str(), antlr_path_str.as_str(), arg)?;
        println!("cargo:rerun-if-changed={}", grammar_path);
    }
    Ok(())
}

fn gen_for_grammar(
    grammar_file_path: &str,
    antlr_path: &str,
    additional_arg: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let input = env::current_dir().unwrap();
    let mut child = Command::new("java")
        .current_dir(input)
        .arg("-cp")
        .arg(antlr_path)
        .arg("org.antlr.v4.Tool")
        .arg("-Dlanguage=Rust")
        .arg("-o")
        .arg("target/antlr_gen")
        .arg(grammar_file_path)
        .args(additional_arg)
        .spawn()
        .expect("antlr tool failed to start");
    assert!(child.wait().unwrap().success());
    Ok(())
}
