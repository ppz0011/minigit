use std::cmp::Ordering;
use std::ffi::OsStr;
use std::{env,fs, path};
use std::fs::{File, OpenOptions};
use std::error::Error;
use std::io::{Write, Read};
use std::path::{Path, PathBuf};
use crypto::{sha1::Sha1, digest::Digest};
use flate2::{Compression, read::ZlibEncoder};

#[derive(Debug)]
pub struct Config{
    operate: String,
    argument: Vec<String>,
}

impl Config{
    /**
     *  'build'将minigit指令（通常是命令行输入）装配为Config（配置）
     * # 示例
     * '''
     * let v = vec!["init","test"];
        let it = v.iter().map(|x|{x.to_string()});
        let config = Config::build(it).unwrap_or_else(|err| {
            eprintln!("error at test build: {err}");
            std::process::exit(1);
        });
        assert_eq!((config.operate,config.argument),(String::from("init"),String::from("test")));
     * '''
     */
    pub fn build(mut args: impl Iterator<Item = String>)-> Result<Config, &'static str>{
        args.next();
        let operate = match args.next(){
            Some(op) => op,
            None => return Err("Didn't get operate string in input")
        };
        let mut argument = Vec::new();
        while let Some(arg) = args.next(){
            argument.push(arg);
        };
        Ok(
            Config {
                operate,
                argument,
            }
        )
    }
}


/*
mod object{
    use super::*;

    fn get_end_byte(path: PathBuf, path_head: &[u8;1], object_type: &[u8;2])->Vec<u8> {
        let path_value = path.as_os_str().as_encoded_bytes();
        let mut end_byte = Vec::new();
        end_byte.extend_from_slice(path_head);
        end_byte.extend_from_slice(path_value);
        end_byte.extend_from_slice(object_type);
        return end_byte;
    }

    pub struct Blob {
        value: Vec<u8>,
    }

    impl Blob {
        pub fn new(path: PathBuf)-> Result<Blob, Box<dyn Error>>{
        let file = File::open(&path)?;
        // 将path代表的文件的二进制内容使用zlib压缩并且存入字符动态数组value中
        let mut zlib = ZlibEncoder::new(file, Compression::fast());
        let mut value = Vec::new();
        zlib.read_to_end(&mut value)?;
        Ok(
            Blob{
                value,
            }
        )
        }
    }

    pub struct Tree {
        dir: Vec<Vec<u8>>,
    }

    impl Tree {
        pub fn new(path: PathBuf)-> Result<Tree, Box<dyn Error>>{
            if !path.is_dir() {
                return Err("Carete Tree Object falid, because path wasn't dir".into());
            }
            let mut dir = Vec::new();
            for entry in fs::read_dir(&path)? {
                let child_path = entry?.path();
                let temp = match child_path.file_name(){
                    None => unreachable!(),
                    Some(name) => name.as_encoded_bytes(),
                };
                dir.push(temp.to_vec());
            }
            Ok(
                Tree{
                    path,
                    dir,
                }
            )
        }
    }

    pub struct Commit{}

    impl Commit {}

    pub trait Save {
        fn save(&self, minigit_path: &PathBuf)-> Result<String, Box<dyn Error>>;   // 此函数会消耗Object

        fn save_value(minigit_path: &PathBuf,value: &Vec<u8>)-> Result<String, Box<dyn Error>> {
            if !minigit_path.is_dir() {
                return Err(".minigit doesn't exists".into());
            }
            // 将value中的数据使用SHA1算法加密成key
            let mut hasher = Sha1::new();
            hasher.input(value);
            let key: &str = &hasher.result_str();
            let save_path = minigit_path.join("objects").join(&key[0..2]);
            if !save_path.is_dir(){
                fs::create_dir(&save_path)?;
            }
            let save_path = save_path.join(&key[2..]);
            if !save_path.is_file(){ 
                // 此时文件需要保存并且更新index
                let mut save_file = File::create(save_path)?;
                save_file.write_all(value)?;
                return Ok(String::from(key));
            }
            Ok(String::from(key))
        }
    }
    }
    impl Save for Blob {
        fn save(&self, minigit_path: &PathBuf)-> Result<String, Box<dyn Error>> {   // save完成后会释放掉self
            return <Self as Save>::save_value(minigit_path, &self.value);
        }

    }
    }
    impl Save for Tree {
        fn save(self, minigit_path: &PathBuf)-> Result<String, Box<dyn Error>> {
            let mut child_ptr = Vec::new();
            for entry in self.path.read_dir()? {
                let key;
                let child_path = entry?.path();
                if child_path.is_file() {
                    let blob = Blob::new(child_path)?;
                    key = blob.save(minigit_path)?;
                }
                else if child_path.is_dir() {
                    let tree = Tree::new(child_path)?;
                    key = tree.save(minigit_path)?;
                }
                else {
                    return Err("invaild child_path".into());
                }
                if let Some(save_key) = key {
                    child_ptr.append(&mut save_key.as_bytes().to_vec());
                }
            }
            let mut end_byte = get_end_byte(self.path, b"?", b"01");
            let mut value = self.dir.into_iter()
                                         .map(|mut x| {
                                            x.push(b'?');
                                            x
                                         })
                                         .flatten()
                                         .collect::<Vec<u8>>();
            value.push(b'?');
            value.append(&mut child_ptr);
            value.append(&mut end_byte);
            let dir_key = <Self as Save>::save_value(minigit_path, &value)?;
            Ok(dir_key)
        }
    }
    /*impl Save for Commit {
        //fn save<'a>(&self, minigit_path: PathBuf)-> Result<&'a str, Box<dyn Error>>;
    }*/
}*/




/** 'run' 通过输入配置，通过运行对应函数来实现对应的minigit指令 \\
 * # 示例
 * '''
    let config: Config = Config{operate:"init".to_string(), argument:"test".to_string()};
    run(config);
    let path =  std::env::current_dir().unwrap_or_else(|err|{
        eprintln!("test_run failed at get current dir: {err}");
        std::process::exit(1);
    });
    assert!(path.join(config.argument).exists());      
 * '''
 */
pub fn run(config: &Config)-> Result<(), Box<dyn Error>>{
    match &config.operate as &str{
        "init" => {
            init(&config.argument[0])
        },
        "add" => {
            add(&config.argument)
        },
        "rm" => todo!(),
        "commit" => todo!(),
        "branch" => todo!(),
        "checkout" => todo!(),
        "merge" => todo!(),
        _=> Err("inviald operater string".into()),
    }
}

/**
 * 'init'根据输入的仓库名字创建minigit仓库，如果该仓库已经存在会将配置初始化，仓库内容不变
 * # 示例
 * '''
 *      let name = "test".to_string();
        let _unused = init(name).unwrap_or_else(|err|{
            eprintln!("error at test_init: {err}");
        });
        let path = env::current_dir().unwrap();
        assert!(path.join("test").is_dir());
 * '''
 */
fn init(name: &String)-> Result<(), Box<dyn Error>>{
    let path = env::current_dir()?.join(name).join(".minigit");
    let mut is_first = true;
    if path.is_dir() {
        fs::remove_dir_all(&path)?;
        is_first = false;
    }
    fs::create_dir_all(&path)?;
    fs::create_dir_all(path.join("refs/heads"))?;
    fs::create_dir(path.join("objects"))?;
    File::create(path.join("index"))?;
    let mut head = File::create(path.join("HEAD"))?;
    head.write_all("ref: refs/heads/master".as_bytes())?;
    if is_first { 
        println!("Initialized empty Git repository in {}",path.to_str().unwrap());
    }
    else{
        println!("Reinitialized empty Git repository in {}",path.to_str().unwrap());
    }
    Ok(())
}

/**
 * 'find_minigit'从此路径开始寻找'.minigit'文件夹（也就是minigit库配置文件存放的地方）
 */
fn find_minigit(path: &PathBuf)-> Result<PathBuf, Box<dyn Error>>{
    if !path.exists() {
        return Err("find minigit faild: invaild path".into());
    }
    let mut start_path = path.clone();
    if start_path.is_file() {
        if !start_path.pop() {
            return Err("Can't find minigit because path have no parent".into());
        }
    }
    let target = OsStr::new(".minigit");
    loop{
        for entry in start_path.read_dir()? {
            let child_path = entry?.path();
            if Some(target) == child_path.file_name() {
                return Ok(child_path);
            }
        }
        if !start_path.pop() {
            break;
        }
    }
    return Err("failed find minigit".into());
}

/*
fn save_value(minigit_path: PathBuf,value: &[u8])-> Result<(), Box<dyn Error>>{
    if !minigit_path.is_dir() {
        return Err(".minigit doesn't exists".into());
    }
    // 将value中的数据使用SHA1算法加密成key
    let mut hasher = Sha1::new();
    hasher.input(value);
    let key: &str = &hasher.result_str();
    let mut save_path = minigit_path.join(&key[0..2]);
    if !save_path.is_dir(){
        fs::create_dir(save_path)?;
    }
    save_path = save_path.join(&key[2..]);
    if !save_path.is_file(){ 
        // 此时文件需要保存并且更新index
        let mut save_file = File::create(save_path)?;
        save_file.write_all(value);
        let mut index = OpenOptions::new().append(true).open(minigit_path.join("index"))?;
        index.write_all(key.as_bytes())?;
    }
    Ok(())
}


fn save(path: PathBuf)-> Result<(), Box<dyn Error>>{
    let minigit_path = find_minigit(path.clone())?;
    let mut value = Vec::new();
    if path.is_file() {
        let file = File::open(path)?;
        // 将path代表的文件的二进制内容使用zlib压缩并且存入字符动态数组value中
        let mut zlib = ZlibEncoder::new(file, Compression::fast());
        zlib.read_to_end(&mut value)?; 
    }
    else if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let child_path = entry?.path();
        }
    }
    else {
        return Err("Can't save file or dir with error path".into());
    }
    save_value(minigit_path,&value)?;
    Ok(())
}*/
fn save_value(path: &PathBuf, minigit_path: &PathBuf,value: &Vec<u8>)-> Result<String, Box<dyn Error>> {
    if !minigit_path.is_dir() {
        return Err(".minigit doesn't exists".into());
    }
    // 将value中的数据使用SHA1算法加密成key
    let mut hasher = Sha1::new();
    hasher.input(value);
    let key: &str = &hasher.result_str();
    let save_path = minigit_path.join("objects").join(&key[0..2]);
    if !save_path.is_dir(){
        fs::create_dir(&save_path)?;
    }
    let save_path = save_path.join(&key[2..]);
    if !save_path.is_file(){ 
        let mut save_file = File::create(save_path)?;
        save_file.write_all(value)?;
        insert_index(minigit_path,path,&String::from(key))?;
    }
    Ok(String::from(key))
}

fn save_blob(path: &PathBuf, minigit_path: &PathBuf)-> Result<String, Box<dyn Error>> {
    let file = File::open(path)?;
    // 将path代表的文件的二进制内容使用zlib压缩并且存入字符动态数组value中
    let mut zlib = ZlibEncoder::new(file, Compression::fast());
    let mut value = Vec::new();
    value.append(&mut "blob\0".as_bytes().to_vec());
    zlib.read_to_end(&mut value)?;
    let key = save_value(path, minigit_path, &value)?;
    Ok(key)
}

fn save_tree(path: &PathBuf, minigit_path: &PathBuf)-> Result<String, Box<dyn Error>> {
    let mut value = Vec::new();
    value.append(&mut "tree\0".as_bytes().to_vec());
    for entry in path.read_dir()? {
        let key;
        let child_type;
        let child_path = entry?.path();
        let child_name = child_path.file_name();
        let child_name = match child_name {
            None=> return Err(r"save_tree failed: child_path can't end with \..".into()),
            Some(name)=> name,
        };
        if child_path.is_file() {
            child_type = "blob";
            key = save_blob(&child_path, minigit_path)?;
        }
        else {
            child_type = "tree";
            key = save_tree(&child_path, minigit_path)?;
        }
        value.append(&mut format!("{child_type} {key} ").into_bytes());
        value.append(&mut child_name.as_encoded_bytes().to_vec());
        value.append(&mut "\0".as_bytes().to_vec());
    }
    let dir_key = save_value(path, minigit_path, &value)?;
    Ok(dir_key)
}

fn find_index(buf: &Vec<Vec<u8>>, path_str: &Vec<u8>)-> (usize, usize) {
    todo!();
}

fn insert_index(minigit_path: &PathBuf, path: &PathBuf,key: &String)-> Result<(),Box<dyn Error>> {
    let mut add_information = format!(" {key}").into_bytes();
    let mut path_str = path.as_os_str().as_encoded_bytes().to_vec();
    let index_path = minigit_path.join("index");
    let mut index = File::open(&index_path)?;
    let mut buf = Vec::new();
    index.read_to_end(&mut buf)?;
    let mut index = File::create(&index_path)?;
    if buf.is_empty() {
        println!("index is empty!");
        path_str.append(&mut add_information);
        path_str.push(b'\n');
        index.write_all(&path_str)?;
        return Ok(());
    }
    let mut buf = buf.split(|&x| x == b'\n').map(|bytes| bytes.to_vec()).collect::<Vec<Vec<u8>>>();
    buf.pop();
    let mut start: usize = 0;
    let mut end: usize = buf.len() - 1;
    while start <= end {
        let mid: usize = (end + start) / 2;
        let mid_path = buf[mid].clone();
        let (mid_path,_) = mid_path.split_at(mid_path.iter().position(|&x| x == b' ').unwrap());
        let mid_path = mid_path.to_vec();
        match mid_path.cmp(&path_str) {
            Ordering::Less=> {
                start = mid + 1;
            }
            Ordering::Greater=> {
                end = mid - 1;
            }
            Ordering::Equal=> { // 更新
                path_str.append(&mut add_information);
                buf[mid] = path_str.clone();
                break;
            }
        }
    }
    if start > end {
        path_str.append(&mut add_information);
        buf.insert(start, path_str);
    }
    let buf: Vec<u8> = buf.iter().flat_map(|v| {let mut w = v.clone(); w.push(b'\n'); w}).collect();
    index.write_all(&buf)?;
    Ok(())
}

fn save_object(path: &PathBuf)-> Result<(), Box<dyn Error>> {
    let minigit_path = find_minigit(&path)?;
    let ignore = OsStr::new(".minigit");
    if path.is_file() {
        save_blob(path, &minigit_path)?;
    }
    else if path.is_dir() {
        match path.file_name() {
            None => return Err("save_object failed:path is dir but can't get file name".into()), 
            Some(name)=> {
                if name == ignore {
                    return Ok(());
                }
                save_tree(path, &minigit_path)?;
            }
        }
    }
    else {
        return Err("save_object failed: Invaid path".into())
    }
    Ok(())
}


/**
 * add 函数负责将一系列文件或者文件夹保存到索引，如果已经保存则检查是否有改变，如果有改变则保存改变后的新文件到索引
 */
fn add(paths: &Vec<String>)-> Result<(), Box<dyn Error>> {
    let current_path = env::current_dir()?;
    let paths = paths.iter().map(|str| {
                                    let mut cstr = str.clone();
                                    if  cstr.ends_with('.') {
                                        cstr.pop();
                                        cstr.push('*');
                                    }
                                    current_path.join(cstr)})
                                    .collect::<Vec<PathBuf>>();
    let tag = OsStr::new("*");
    for path in paths {
        let file_name = match path.file_name() {
            None=> return Err(r"add failed: path can't end with \..".into()),
            Some(name)=> name,
        };
        if file_name == tag{
            let mut save_path = path.clone();
            if !save_path.pop() {
                return Err(r"add failed: path have no parent and end with \. or \*".into());
            }
            for entry in save_path.read_dir()? {
                save_object(&entry?.path())?;
            }
        }
        else {
            save_object(&path)?;
        }
    }
    Ok(())
}

fn rm(){}

fn commit(){}

fn branch(){}

fn checkout(){}

fn merge(){}

#[cfg(test)]
mod test{
    use super::*;
    #[test]
    fn test_build(){
        let v = vec!["init","test"];
        let it = v.iter().map(|x|{x.to_string()});
        let config = Config::build(it).unwrap_or_else(|err| {
            eprintln!("error at test build: {err}");
            std::process::exit(1);
        });
        assert_eq!((config.operate,config.argument),(String::from("init"),vec![String::from("test")]));
        //dbg!(config);
    }

    #[test]
    fn test_run(){
        let config: Config = Config{operate:"init".to_string(), argument:vec!["test".to_string()]};
        run(&config).unwrap_or_else(|err|{
            eprintln!("error at test_init: {err}");
        });
        let path =  std::env::current_dir().unwrap_or_else(|err|{
            eprintln!("test_run failed at get current dir: {err}");
            std::process::exit(1);
        });
        assert!(path.join(&config.argument[0]).exists());
    }

    #[test]
    fn test_init(){
        let name = "test".to_string();
        let _unused = init(&name).unwrap_or_else(|err|{
            eprintln!("error at test_init: {err}");
        });
        let path = env::current_dir().unwrap();
        assert!(path.join(name).is_dir());
    }

    #[test]
    fn test_add()-> std::io::Result<()>{
        let name = "test".to_string();
        init(&name).unwrap_or_else(|err|{
            eprintln!("error at test_add: {err}");
        });
        let mut path = env::current_dir().unwrap().join(name);
        let mut file1 = File::create(path.join("1.txt")).unwrap();
        file1.write_all(b"Hello First World!")?;
        path = path.join("test_dir");
        fs::create_dir_all(&path)?;
        let mut file2 = File::create(path.join("2.txt")).unwrap();
        file2.write_all(b"Hello Second World!")?;
        env::set_current_dir(path)?;
        add(&vec![".".to_string()]).unwrap_or_else(|err|{
            eprintln!("error at test_add: {err}");
        });
        Ok(())
    }

    #[test]
    fn test_rm() {
        let path = PathBuf::from("C:\\Users\\Public\\rust\\minigit\\test\\.minigit\\index");
        let mut index = OpenOptions::new().write(true).read(true).open(path).unwrap();
        let buf = "Hello World!".as_bytes();
        index.write_all(buf);
    }
}
