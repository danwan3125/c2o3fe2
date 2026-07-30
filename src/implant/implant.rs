use std::{io::stderr, process::Command};
fn main() {
    let mut registered=false;
    let mut cmd=Command::new("echo")
    .arg("Hello, World")
    .output().expect("Something went wrong");
    let out=String::from_utf8_lossy(&cmd.stdout);
    let err=String::from_utf8_lossy(&cmd.stderr);
    println!("cmd output {}", out);
    println!("cmd err {}", err);
}
