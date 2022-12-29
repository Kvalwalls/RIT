# RIT
**Git by Rust**

**WBC/LZH/YCY/ZZH**

## 实现

RIT

​	├──cmd（Git命令模块）

​				├──add.rs（add命令实现）

​				├──branch.rs（branch命令实现）

​				├──checkout.rs（checkout命令实现）

​				├──commit.rs（commit命令实现）

​				├──config.rs（config命令实现）

​				├──init.rs（init命令实现）

​				├──log.rs（log命令实现）

​				├──mod.rs（Git命令模块化）

​				├──rm.rs（rm命令实现）

​				├──status.rs（status命令实现）

​	├──objects（Git对象模块）

​				├──blob.rs（blob对象实现）

​				├──commit.rs（commit对象实现）

​				├──mod.rs（Git对象模块化）

​				├──tree.rs（tree对象实现）

​	├──anounce.yaml（App的yaml配置）

​	├──index.rs（暂存区实现）

​	├──lib.rs（库函数实现）

​	├──main.rs（主函数实现）

​	├──refs.rs（引用实现）

​	├──utils.rs（工具实现）

​	├──Cargo.lock（Cargo文件）

​	├──Cargo.toml（Cargo文件）

## 使用

### 1. 编译

```shell
cargo run --package rit --bin rit
```

### 2. 配置环境变量

```shell
export PATH=/home/ubuntu/rit/target/debug:$PATH
```

### 3. 运行命令

```shell
rit 1.0.0
WBC/LZH/YCY/ZZH
Git By Rust

USAGE:
    rit [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    add         将文件从工作区添加到暂存区
    branch      查看、创建、删除分支
    checkout    切换分支
    commit      将文件从暂存区添加到本地仓库
    config      设置配置信息
    help        查看帮助信息
    init        初始化本地仓库
    log         查看日志
    rm          删除暂存区文件
    status      查看当前版本的状态
```

初始化本地仓库命令

```shell
rit init <仓库名>
```

添加文件命令

```shell
rit add <文件名>
```

提交文件命令

```shell
rit commit --message "xxx"
```

创建分支命令

```shell
rit branch <分支名>
```

查看日志命令

```shell
rit log
```

查看版本状态命令

```shell
rit status
```

切换分支命令

```shell
rit checkout <分支名>
```

查看帮助信息命令

```shell
rit -h
```

```shell
rit --help
```

