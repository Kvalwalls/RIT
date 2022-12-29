use clap::App;
use std::error::Error;
pub mod cmd;
pub mod objects;
pub mod index;
pub mod refs;
pub mod utils;
pub fn run(app: &mut App) -> Result<(), Box<dyn Error>> {
    //解析命令行参数
    let matches = app.clone().get_matches();
    //match表达式匹配Git命令
    match matches.subcommand() {
        ("add", Some(matches)) => cmd::add::run(matches),
        ("branch", Some(matches)) => cmd::branch::run(matches),
        ("commit", Some(matches)) => cmd::commit::run(matches),
        ("config", Some(matches)) => cmd::config::run(matches),
        ("init", Some(matches)) => cmd::init::run(matches),
        ("log", Some(matches)) => cmd::log::run(matches),
        ("status", Some(matches)) => cmd::status::run(matches),
        ("checkout", Some(matches)) => cmd::switch::run(matches),
        (_, None) => {
            app.print_help()?;
            println!();
            Ok(())
        }
        _ => panic!("不存在此命令！"),
    }
}
