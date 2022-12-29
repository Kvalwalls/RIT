use clap::ArgMatches;
use std::env;
use std::error::Error;
use std::fs;
use std::fs::{DirBuilder, File};

pub fn run(args: &ArgMatches) -> Result<(), Box<dyn Error>> {
    //获取工作区目录
    let repo_path = match args.value_of("directory") {
        Some(path) => {
            let mut p = std::path::PathBuf::new();
            p.push(path);
            p
        }
        None => env::current_dir()?,
    };
    let git_path = repo_path.join(".rit");
    let reinitialized = git_path.exists();
    //创建目录
    let mut dir_builder = DirBuilder::new();
    dir_builder
        .recursive(true)
        .create(git_path.join("objects/info"))?;
    dir_builder.create(git_path.join("refs/heads"))?;
    dir_builder.create(git_path.join("refs/tags"))?;
    //创建文件
    if !git_path.join("HEAD").is_file() {
        //设置文件内容为"ref: refs/heads/master"
        fs::write(git_path.join("HEAD"), "ref: refs/heads/master\n")?;
    }
    if !git_path.join("index").is_file() {
        File::create(git_path.join("index"))?;
    }
    //判断本地仓库是否创建成功
    if !args.is_present("quiet") {
        match reinitialized {
            true => println!(
                "重复创建Rit仓库{}",
                git_path.to_str().unwrap()
            ),
            false => println!(
                "成功创建Rit仓库{}",
                git_path.to_str().unwrap()
            ),
        }
    }
    Ok(())
}
