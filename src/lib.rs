use std::cmp::Ordering;
use std::ffi::{OsStr, OsString};
use std::{env,fs, path};
use std::fs::{File, OpenOptions, remove_file, remove_dir};
use std::error::Error;
use std::io::{Write, Read};
use std::path::{Path, PathBuf};
use crypto::{sha1::Sha1, digest::Digest};
use flate2::{Compression, read::ZlibEncoder};
use chrono::{DateTime, Utc};

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
            init(&config.argument[0])?;
        },
        "add" => {
            add(&config.argument)?;
            println!("Successed add file: {:?}",&config.argument);
        },
        "rm" => {
            rm(&config.argument)?;
            println!("Successed remove file: {:?}",&config.argument);
        },
        "commit" => {
            let author = env::var("USERNAME")?;
            commit(&author, &config.argument[0])?;
            println!("Successed commit repository with message: \"{}\"",&config.argument[0]);
        },
        "branch" => {
            branch_check()?;
            branch_delete(&"second_branch".to_string())?;
        },
        "checkout" => todo!(),
        "merge" => todo!(),
        _=> return Err("inviald operater string".into()),
    }
    Ok(())
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
    head.write_all("refs/heads/master".as_bytes())?;
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
    return Err("failed find minigit repository".into());
}



fn find_index(buf: &Vec<Vec<u8>>, path_str: &Vec<u8>)-> (usize, i32) {
    if buf.len() == 0 {
        return (0,-1);
    }
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
                if mid == 0 {
                    return (0, -1);
                }
                end = mid - 1;
            }
            Ordering::Equal=> { 
                break;
            }
        }
    }
    (start, end as i32)
}

/**
 * 从路径path开始通过index里面的记录而不是文件系统来更新index内容（即buf）
 * 
 */
fn updata_index(buf: &mut Vec<Vec<u8>>, path: &PathBuf, root_path: &Path)-> Result<(), Box<dyn Error>> {
    let path = match path.parent() {
        None => return Err("updata index file failed: path don't have enough ancestor to minigit path's parent".into()),
        Some(p)=> p.to_path_buf(),
    };
    if !path.starts_with(root_path) {
        return Ok(());
    }
    // 根据index里面的记录而不是实际文件系统来更新path_ancestor
    let path_str = path.as_os_str().as_encoded_bytes().to_vec();
    let (start, end) = find_index(buf, &path_str);
    let mut find_ptr = start;
    if start as i32 <= end {
        find_ptr = (start + end as usize) / 2 + 1;
    }
    let buf_len = buf.len();
    let mut value = b"tree\0".to_vec();
    while find_ptr < buf_len {
        let find_position = buf[find_ptr].iter().position(|&b| b == b' ').unwrap();
        let find_path = &buf[find_ptr][0..find_position];
        if !find_path.starts_with(&path_str) {
            break;
        }
        let last_separator_index = find_path.windows(path::MAIN_SEPARATOR_STR.len()).rposition(|str| str == path::MAIN_SEPARATOR_STR.as_bytes()).unwrap();
        if find_path[0..last_separator_index] != path_str {
            find_ptr = find_ptr + 1;
            continue;
        }
        let mut find_key = buf[find_ptr][(find_position + 1)..].to_vec();
        let mut file_name = find_path[(last_separator_index + 1)..].to_vec();
        value.append(&mut find_key);
        value.push(b' ');
        value.append(&mut file_name);
        value.append(&mut "\0".as_bytes().to_vec());
        find_ptr = find_ptr + 1;
    }
    let key = save_value(&root_path.join(".minigit"), &value)?;
    let mut add_information = format!(" tree {key}").into_bytes();
    let mut path_str = path.as_os_str().as_encoded_bytes().to_vec();
    path_str.append(&mut add_information);
    if start as i32 <= end {
        buf[(start + end as usize) / 2] = path_str;
    }
    else {
        buf.insert(start, path_str);
    }
    updata_index(buf, &path, root_path)
}

fn start_updata_index(minigit_path: &PathBuf, path: &PathBuf)-> Result<(), Box<dyn Error>> {
    let root_path = match minigit_path.parent() {
        None=> return Err("update index file failed: minigit path have no parent".into()),
        Some(p)=> p,
    };
    let index_path = minigit_path.join("index");
    let mut read = fs::read(&index_path)?;
    let mut buf = [[].to_vec();0].to_vec();
    if !read.is_empty() {
        read.pop();
        buf = read.split(|&x| x == b'\n').map(|bytes| bytes.to_vec()).collect::<Vec<Vec<u8>>>();
    }
    // 接下来应该更新此路径上全部的key
    updata_index(&mut buf, path, root_path)?;
    // 最后将index清空后将buf写入index文件
    let buf: Vec<u8> = buf.iter().flat_map(|v| {let mut w = v.clone(); w.push(b'\n'); w}).collect();
    let mut index = File::create(&index_path)?;
    index.write_all(&buf)?;
    Ok(())
}


fn insert_index(minigit_path: &PathBuf, path: &PathBuf,key: &String)-> Result<(),Box<dyn Error>> {
    let mut path_type = "tree";
    if path.is_file() {
        path_type = "blob";
    }
    let mut add_information = format!(" {path_type} {key}").into_bytes();
    let mut path_str = path.as_os_str().as_encoded_bytes().to_vec();
    let index_path = minigit_path.join("index");
    let mut index = File::open(&index_path)?;
    let mut read = Vec::new();
    index.read_to_end(&mut read)?;
    let mut buf;
    if !read.is_empty() {
        read.pop();
        buf = read.split(|&x| x == b'\n').map(|bytes| bytes.to_vec()).collect::<Vec<Vec<u8>>>();
        let (start, end) = find_index(&buf, &path_str);
        if start as i32 > end {
            path_str.append(&mut add_information);
            buf.insert(start, path_str);
        }
        else { // start <= end 说明此时buf[(start + end) / 2]的路径与path_str一样，该更新而不是插入
            path_str.append(&mut add_information);
            let mid = (start + end as usize) / 2;
            buf[mid] = path_str;
        }
    }
    else {
        path_str.append(&mut add_information);
        buf = [path_str;1].to_vec();
    }
    let buf: Vec<u8> = buf.iter().flat_map(|v| {let mut w = v.clone(); w.push(b'\n'); w}).collect();
    let mut index = File::create(&index_path)?;
    index.write_all(&buf)?;
    Ok(())
}





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
        let mut save_file = File::create(save_path)?;
        save_file.write_all(value)?;
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
    let key = save_value(minigit_path, &value)?;
    insert_index(minigit_path, path, &key)?;
    Ok(key)
}

fn save_tree(path: &PathBuf, minigit_path: &PathBuf)-> Result<String, Box<dyn Error>> {
    let mut value = Vec::new();
    value.append(&mut "tree\0".as_bytes().to_vec());
    for entry in path.read_dir()? {
        let key: String;
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
    let dir_key = save_value(minigit_path, &value)?;
    insert_index(minigit_path,path,&dir_key)?;
    Ok(dir_key)
}


fn save_object(path: &PathBuf)-> Result<(), Box<dyn Error>> {
    let minigit_path = &find_minigit(&path)?;
    let ignore = OsStr::new(".minigit");
    if path.is_file() {
        save_blob(path, minigit_path)?;
    }
    else if path.is_dir() {
        match path.file_name() {
            None => return Err("save_object failed:path is dir but can't get file name".into()), 
            Some(name)=> {
                if name == ignore {
                    return Ok(());
                }
                save_tree(path, minigit_path)?;
            }
        }
    }
    else {
        return Err("save_object failed: Invaid path".into())
    }
    start_updata_index(minigit_path,path)
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




fn remove_index(minigit_path: &PathBuf, path: &PathBuf)-> Result<(), Box<dyn Error>> {
    let path_str = path.as_os_str().as_encoded_bytes().to_vec();
    let index_path = minigit_path.join("index");
    let mut index = File::open(&index_path)?;
    let mut buf = Vec::new();
    index.read_to_end(&mut buf)?;
    if buf.len() > 0 {
        buf.pop();
    }
    let mut buf = buf.split(|&x| x == b'\n').map(|bytes| bytes.to_vec()).collect::<Vec<Vec<u8>>>();
    let (start, end) = find_index(&buf, &path_str);
    if start as i32 <= end {
        buf.remove((start + end as usize) / 2);
    }
    // 接下来应该更新此路径上全部的key
    let root_path = match minigit_path.parent() {
        None=> return Err("update index file failed: minigit path have no parent".into()),
        Some(p)=> p,
    };
    updata_index(&mut buf, path, root_path)?;
    // 最后将index清空后将buf写入index文件
    let buf: Vec<u8> = buf.iter().flat_map(|v| {let mut w = v.clone(); w.push(b'\n'); w}).collect();
    let mut index = File::create(&index_path)?;
    index.write_all(&buf)?;
    Ok(())
}

fn remove_blob(minigit_path: &PathBuf, path: &PathBuf)-> Result<(), Box<dyn Error>> {
    remove_file(path)?;
    remove_index(minigit_path, path)?;
    Ok(())
}

fn remove_tree(minigit_path: &PathBuf, path: &PathBuf)-> Result<(), Box<dyn Error>> {
    for entry in path.read_dir()? {
        let child_path = &entry?.path();
        if child_path.is_file() {
            remove_blob(minigit_path, child_path)?;
        }
        else {
            remove_tree(minigit_path, child_path)?;
        }
    }
    remove_dir(path)?;
    remove_index(minigit_path, path)?;
    Ok(())
}


fn remove_object(path: &PathBuf)-> Result<(), Box<dyn Error>> {
    let minigit_path = &find_minigit(path)?;
    let ignore = OsStr::new(".minigit");
    if path.is_file() {
        remove_blob(minigit_path, path)?;
    }
    else if path.is_dir() {
        match path.file_name() {
            None => return Err("save_object failed:path is dir but can't get file name".into()), 
            Some(name)=> {
                if name == ignore {
                    return Ok(());
                }
                remove_tree(minigit_path, path)?;
            }
        }
    }
    else {
        return Err("save_object failed: Invaid path".into())
    }
    start_updata_index(minigit_path,path)
}


fn rm(paths: &Vec<String>)->Result<(), Box<dyn Error>> {
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
                remove_object(&entry?.path())?;
            }
        }
        else {
            remove_object(&path)?;
        }
    }
    Ok(())
}




fn create_tree_from_index(minigit_path: &PathBuf)-> Result<Vec<u8>, Box<dyn Error>> {
    let index_path = minigit_path.join("index");
    if !index_path.is_file() {
        return Err("commit failed: no such index file".into());
    }
    let mut index = File::open(index_path)?;
    let mut buf = [0;256];
    let n = index.read(&mut buf)?;
    if n == 0 {
        return Ok("\0".as_bytes().to_vec());
    }
    let start = buf.iter().position(|&b| b == b' ').unwrap() + 6;
    let end = buf.iter().position(|&b| b == b'\n').unwrap();
    let key = buf[start..end].to_vec();
    Ok(key)
}


fn commit(author: &String, message: &String)-> Result<(), Box<dyn Error>> {
    let minigit_path = &find_minigit(& env::current_dir()?)?;
    let mut tree_key = create_tree_from_index(minigit_path)?;
    let mut head = File::open(minigit_path.join("HEAD"))?;
    let mut current_commit = String::new();
    head.read_to_string(&mut current_commit)?;
    let current_commit = minigit_path.join(&current_commit);
    let mut parent_commit = String::new();
    if !current_commit.is_file() {
        parent_commit = "\0".to_string();
    }
    else {
        let mut head = File::open(&current_commit)?;
        head.read_to_string(&mut parent_commit)?;
    }
    let now: DateTime<Utc> = Utc::now();
    let mut commit_value = format!("commit\0parent {parent_commit}\nauthor {author}\ndatetime {now}\n note {message}\n tree ").into_bytes();
    commit_value.append(&mut tree_key);
    let key = save_value(minigit_path, &commit_value)?;
    let mut head = File::create(&current_commit)?;
    head.write_all(&key.into_bytes())?;
    Ok(())
}


fn branch_create(name: &String)-> Result<(), Box<dyn Error>> {
    let minigit_path = find_minigit(&env::current_dir()?)?;
    let branch_path = minigit_path.join("refs").join("heads").join(name);
    if branch_path.is_file() {
        return Err("create branch failed: branch {name} is existing, you can't create a existing branch".into());
    }
    let now_branch_name = fs::read_to_string(minigit_path.join("HEAD"))?;
    let last_commit_key = fs::read(minigit_path.join(&now_branch_name))?;
    fs::write(branch_path, last_commit_key)?;
    Ok(())
}

fn branch_check()-> Result<(), Box<dyn Error>> {
    let minigit_path = find_minigit(&env::current_dir()?)?;
    let branchs_path = minigit_path.join("refs").join("heads");
    let now_branch_name = fs::read_to_string(minigit_path.join("HEAD"))?;
    let now_branch_name = now_branch_name[11..].to_string();
    for entry in branchs_path.read_dir()? {
        let branch_name = entry?.file_name().into_string().unwrap();
        if branch_name == now_branch_name {
            println!("* {}",branch_name);
        }
        else{
            println!(" {}",branch_name);
        }
    }
    Ok(())
}

fn branch_delete(name: &String)-> Result<(), Box<dyn Error>> {
    let minigit_path = find_minigit(&env::current_dir()?)?;
    let branchs_path = minigit_path.join("refs").join("heads");
    let now_branch_name = fs::read_to_string(minigit_path.join("HEAD"))?;
    let now_branch_name = now_branch_name[11..].to_string();
    if &now_branch_name == name {
        return Err("delete branch failed: can't delete now branch".into());
    }
    for entry in branchs_path.read_dir()? {
        let entry = entry?;
        let branch_name = entry.file_name().into_string().unwrap();
        if &branch_name == name {
            fs::remove_file(entry.path())?;
            return Ok(());
        }
    }
    return Err("delete branch failed: no such branch".into());
}

fn checkout(){}

fn merge(){}




#[cfg(test)]
mod test{
    use chrono::Timelike;

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
        env::set_current_dir(&path)?;
        let mut file1 = File::create(path.join("1.txt")).unwrap();
        file1.write_all(b"Hello First World!")?;
        path = path.join("test_dir");
        fs::create_dir_all(&path)?;
        let mut file2 = File::create(path.join("2.txt")).unwrap();
        file2.write_all(b"Hello Second World!")?;
        add(&vec!["*".to_string()]).unwrap_or_else(|err|{
            eprintln!("error at test_add: {err}");
        });
        Ok(())
    }

    #[test]
    fn test_rm()-> std::io::Result<()>{
        test_add()?;
        rm(&vec!["test_dir\\2.txt".to_string()]).unwrap_or_else(|err| {
            eprintln!("error at test_rm: {err}");
        });
        Ok(())
    }

    #[test]
    fn test_commit()-> std::io::Result<()> {
        test_add()?;
        rm(&vec!["test_dir\\2.txt".to_string()]).unwrap_or_else(|err|{
            eprintln!("error at test_rm: {err}");
        });
        let author_name = "master".to_string();
        let message = "test first commit".to_string();
        commit(&author_name, &message).unwrap_or_else(|err|{
            eprintln!("error at test_commit: {err}");
        });
        Ok(())
    }

    #[test]
    fn test_branch()-> Result<(), Box<dyn Error>> {
        test_commit()?;
        println!("before create");
        branch_check()?;
        branch_create(&"second_branch".to_string())?;
        println!("after create");
        branch_check()?;
        branch_delete(&"second_branch".to_string())?;
        println!("after delete");
        branch_check()?;
        Ok(())
    }

    #[test]
    fn test() {
        for (key, value) in env::vars() {
            println!("{}: {}",key,value);
        }
    }
}
