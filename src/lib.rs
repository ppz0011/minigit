use std::fs;
use std::fs::File;
use std::error::Error;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Config{
    operate: String,
    argument: String,
}

impl Config{
    pub fn build(mut args: impl Iterator<Item = String>)-> Result<Config, &'static str>{
        args.next();
        let operate = match args.next(){
            Some(op) => op,
            None => return Err("Didn't get operate string in input")
        };
        let argument = match args.next(){
            Some(arg) => arg,
            None => return Err("Didn't get argument string in input")
        };
        Ok(
            Config {
                operate,
                argument,
            }
        )
    }
}

pub fn run(config:Config)-> Result<(), Box<dyn Error>>{
    match &config.operate as &str{
        "init" => {
            init(config.argument)
        },
        _=> Err("inviald operater string"),
    }
}

fn init(name: String)-> Result<(), Box<dyn Error>>{
    let mut path = PathBuf::from(".");
    path = path.join(name).join(".minigit");
    fs::create_dir_all(path)?;
    std::env::set_current_dir(path)?;
    fs::create_dir(path.join("refs"))?;
    fs::create_dir(path.join("objects"))?;
    Ok(())
}

fn add(){}

fn rm(){}

fn commit(){}

fn branch(){}

fn checkout(){}

fn merge(){}

#[cfg(test)]
mod test{
    use super::*;
    #[test]
    fn test_init(){}
}