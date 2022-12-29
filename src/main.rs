#[macro_use]
extern crate clap;
use clap::App;
use std::process::exit;
fn main() {
    //加载配置文件
    let yaml = load_yaml!("anounce.yaml");
    //创建命令行程序
    let mut app = App::from_yaml(yaml);
    //捕获程序错误
    if let Err(e) = rit::run(&mut app) {
        eprintln!("{}", e);
        exit(10086);
    }
}
