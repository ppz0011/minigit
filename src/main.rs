use std::{env, fs};
use minigit::Config;
use std::process;

fn main() {
    let config = Config::build(env::args()).unwrap_or_else(|err| {
        eprintln!("Error at input argument: {err}");
        process::exit(1);
    });
    if let Err(err) = minigit::run(&config){
        eprintln!("Error at make operator: {err}");
        process::exit(1);
    }
}

/*
use std::path::{Path, PathBuf};
fn main()-> std::io::Result<()> {
    let mut path = fs::metadata("C:\\Users\\Public\\rust\\minigit\\1.txt")?;
    let b1 = path.is_file();
    let b2 = path.is_dir();
    println!("b1 = {},b2 = {}",b1,b2);
    Ok(())
}*/