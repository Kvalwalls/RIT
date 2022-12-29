use crate::index::{EntryType, Index};
use crate::objects::{Blob, Hash, Object};
use crate::utils;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::fs::Permissions;
use std::io;
use std::io::{BufRead, Write};
use std::os::unix;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::str::FromStr;

//文件类型
#[derive(Eq, PartialEq)]
pub enum TreeEntry {
    //普通文件
    File(Hash),
    //二进制文件
    Executable(Hash),
    //符号链接文件
    Symlink(Hash),
    //目录文件
    Directory(Tree),
}

//Tree类型
#[derive(Eq, PartialEq)]
pub struct Tree {
    pub entries: BTreeMap<String, TreeEntry>,
}
//Tree方法
impl Tree {
    //构造方法
    pub fn new() -> Tree {
        Tree {
            entries: BTreeMap::new(),
        }
    }
    //添加普通文件
    pub fn add_file(&mut self, path: String, hash: Hash) -> &mut Tree {
        self.entries.insert(path, TreeEntry::File(hash));
        self
    }
    //添加二进制文件
    pub fn add_executable(&mut self, path: String, hash: Hash) -> &mut Tree {
        self.entries.insert(path, TreeEntry::Executable(hash));
        self
    }
    //添加符号链接文件
    pub fn add_symlink(&mut self, path: String, hash: Hash) -> &mut Tree {
        self.entries.insert(path, TreeEntry::Symlink(hash));
        self
    }
    //添加目录文件
    pub fn add_directory(&mut self, path: String, tree: Tree) -> &mut Tree {
        self.entries.insert(path, TreeEntry::Directory(tree));
        self
    }
    //重写
    //保存Git对象方法
    pub fn save(&self, repo_path: &PathBuf) {
        // Save recursively all trees
        for (_, entry) in self.entries.iter() {
            if let TreeEntry::Directory(tree) = entry {
                tree.save(repo_path)
            }
        }

        let hash = self.hash().to_string();
        let repo_path = &repo_path.join("objects").join(&hash[..2]);
        if !repo_path.is_dir() {
            fs::create_dir(repo_path).expect("Fail creating object directory");
        }
        let repo_path = repo_path.join(&hash[2..]);
        if !repo_path.is_file() {
            // Compress the dump with zlib flate
            let file = File::create(repo_path).expect("Fail opening the object file");
            let mut data = ZlibEncoder::new(file, Compression::default());
            data.write_all(&self.dump())
                .expect("Error writing data to the object file");
        }
    }
    //生成Tree对象
    fn create_tree(&mut self, path: &PathBuf) {
        if let Some(root) = path.iter().next() {
            let root = root.to_str().unwrap().to_string();
            if let Some(entry) = self.entries.get_mut(&root) {
                let path: PathBuf = path.iter().skip(1).collect();
                if let TreeEntry::Directory(tree) = entry {
                    tree.create_tree(&path);
                } else {
                    panic!("Path invalid for the given index");
                }
            } else {
                self.add_directory(root, Tree::new());
                self.create_tree(&path);
            }
        }
    }
    //返回可修改的Tree对象
    fn get_mut_tree(&mut self, path: &PathBuf) -> &mut Self {
        if let Some(root) = path.iter().next() {
            let root = root.to_str().unwrap().to_string();
            let path: PathBuf = path.iter().skip(1).collect();
            if let Some(entry) = self.entries.get_mut(&root) {
                if let TreeEntry::Directory(tree) = entry {
                    return tree.get_mut_tree(&path);
                }
            }
            panic!("Path invalid for the given index");
        } else {
            self
        }
    }
    //更新Tree对象
    pub fn apply(
        &self,
        repo_path: &PathBuf,
        index: &mut Index,
        path: &PathBuf,
        new: &Self,
    ) -> Result<(), Box<dyn Error>> {
        for (filename, new_entry) in new.entries.iter() {
            let path = path.join(filename);
            // Update and create files of the new tree
            match (self.entries.get(filename), new_entry) {
                (Some(TreeEntry::Directory(cur_tree)), TreeEntry::Directory(new_tree)) => {
                    if cur_tree != new_tree {
                        cur_tree.apply(repo_path, index, &path, new_tree)?;
                        continue;
                    }
                }
                (Some(TreeEntry::Directory(_)), _) => {
                    fs::remove_dir_all(&path)?;
                    index.remove_entry(&path)?;
                }
                (Some(cur_entry), _) => {
                    if cur_entry == new_entry {
                        continue;
                    } else if let TreeEntry::Directory(_) = new_entry {
                        fs::remove_file(&path)?;
                        index.remove_entry(&path)?;
                    }
                }
                (None, _) => (),
            }

            // Apply new files / directories
            let blob = match new_entry {
                TreeEntry::Directory(new_tree) => {
                    fs::create_dir(&path)?;
                    Tree::new().apply(repo_path, index, &path, new_tree)?;
                    continue;
                }
                TreeEntry::File(hash) => {
                    let blob = Blob::load(repo_path, *hash);
                    fs::write(&path, &blob.data)?;
                    blob
                }
                TreeEntry::Executable(hash) => {
                    let blob = Blob::load(repo_path, *hash);
                    fs::write(&path, &blob.data)?;
                    fs::set_permissions(&path, Permissions::from_mode(0o755))?;
                    blob
                }
                TreeEntry::Symlink(hash) => {
                    let blob = Blob::load(repo_path, *hash);
                    unix::fs::symlink(std::str::from_utf8(&blob.data[..]).unwrap(), &path)?;
                    blob
                }
            };

            // Update the file to the index
            index.update_entry(&path, &blob)?;
        }

        // Remove files / directories from old tree
        for (filename, _) in self.entries.iter() {
            if !new.entries.contains_key(filename) {
                let path = path.join(filename);
                index.remove_entry(&path)?;
                if path.is_file() {
                    fs::remove_file(&path)?;
                } else if path.is_dir() {
                    fs::remove_dir_all(&path)?;
                }
            }
        }
        Ok(())
    }
    //通过Index对象生成Tree对象
    pub fn from(index: &Index) -> Self {
        let mut root = Tree::new();
        for (path, (entry_type, hash)) in index.entries.iter() {
            // Compute the tree
            let path = PathBuf::from(path);
            root.create_tree(&path.parent().unwrap().to_path_buf());
            let tree = root.get_mut_tree(&path.parent().unwrap().to_path_buf());

            // Add the file
            match entry_type {
                EntryType::File => {
                    tree.add_file(
                        path.file_name().unwrap().to_str().unwrap().to_string(),
                        *hash,
                    );
                }
                EntryType::Executable => {
                    tree.add_executable(
                        path.file_name().unwrap().to_str().unwrap().to_string(),
                        *hash,
                    );
                }
                EntryType::Symlink => {
                    tree.add_symlink(
                        path.file_name().unwrap().to_str().unwrap().to_string(),
                        *hash,
                    );
                }
            }
        }
        root
    }
    //判断Tree对象中是否包含文件
    pub fn contains(&self, path: &PathBuf) -> Result<bool, Box<Error>> {
        if let Some(root) = path.iter().next() {
            let root = root.to_str().unwrap().to_string();
            let mut path: PathBuf = path.iter().skip(1).collect();
            if let Some(entry) = self.entries.get(&root) {
                if let TreeEntry::Directory(tree) = entry {
                    tree.contains(&path)
                } else {
                    // Return wheter path is empty or not
                    Ok(!path.pop())
                }
            } else {
                Ok(false)
            }
        } else {
            Ok(true)
        }
    }
    //返回Tree对象中包含的相应文件
    pub fn get_entry(&self, path: &PathBuf) -> Result<&TreeEntry, Box<Error>> {
        if let Some(root) = path.iter().next() {
            let root = root.to_str().unwrap().to_string();
            let path: PathBuf = path.iter().skip(1).collect();
            if let Some(entry) = self.entries.get(&root) {
                if !path.clone().pop() {
                    // Return wheter path is empty or not
                    return Ok(entry);
                }
                if let TreeEntry::Directory(tree) = entry {
                    return tree.get_entry(&path);
                }
            }
        }
        Err(Box::new(io::Error::new(
            io::ErrorKind::NotFound,
            "fatal: path invalid",
        )))
    }
}

impl Object for Tree {
    //Git对象转换文件数据方法
    fn dump(&self) -> Vec<u8> {
        // Compute data for each entry
        let mut data = vec![];
        for (path, entry) in self.entries.iter() {
            let (mode, hash) = match entry {
                TreeEntry::File(hash) => ("100644", *hash),
                TreeEntry::Executable(hash) => ("100755", *hash),
                TreeEntry::Symlink(hash) => ("120000", *hash),
                TreeEntry::Directory(tree) => ("40000", tree.hash()),
            };
            let entry = format!("{} {}\0", mode, path);
            data.append(&mut entry.into_bytes());
            data.extend_from_slice(&hash.bytes());
        }

        // Add header
        let header = format!("tree {}\0", data.len());
        let mut res = vec![];
        res.reserve(data.len() + header.len());
        res.append(&mut header.into_bytes());
        res.append(&mut data);
        res
    }

    //文件句柄转换Git对象方法
    fn from<R: BufRead>(mut reader: R) -> Box<Tree> {
        let mut buff = vec![];
        reader.read_until(0, &mut buff).unwrap();
        assert!(std::str::from_utf8(&buff).unwrap().starts_with("tree "));
        let mut res = Tree::new();
        loop {
            buff.clear();
            if reader.read_until(0, &mut buff).unwrap() == 0 {
                break;
            }
            buff.pop();
            let desc: Vec<&str> = std::str::from_utf8(&buff).unwrap().split(' ').collect();
            let mut hash = [0; 20];
            reader.read_exact(&mut hash).unwrap();
            let hash = hash
                .iter()
                .fold(String::new(), |res, e| res + &format!("{:02x}", e));
            let hash = Hash::from_str(hash.as_str()).unwrap();
            match desc[0] {
                "100644" => {
                    res.add_file(desc[1].to_string(), hash);
                }
                "100755" => {
                    res.add_executable(desc[1].to_string(), hash);
                }
                "120000" => {
                    res.add_symlink(desc[1].to_string(), hash);
                }
                "40000" => {
                    let tree = Tree::load(&utils::find_repo().unwrap(), hash);
                    res.add_directory(desc[1].to_string(), *tree);
                }
                _ => panic!("Unexpected file description in a tree"),
            }
        }
        Box::new(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn tree_dump() {
        let mut tree = Tree::new();
        tree.add_file(
            String::from("lol"),
            Hash::from_str("63cd04a52f5c8cb95686081b000223e968ed74f4").unwrap(),
        );
        let dump = tree.dump();
        assert_eq!(dump.len(), 39);
        assert_eq!(std::str::from_utf8(&dump[0..8]).unwrap(), "tree 31\0");
        assert_eq!(dump[7], 0);
        assert_eq!(dump[38], 0xf4);
    }

    #[test]
    fn tree_dump_multiple() {
        let mut tree = Tree::new();
        let mut sub_tree = Tree::new();
        sub_tree.add_file(
            String::from("lol"),
            Hash::from_str("9daeafb9864cf43055ae93beb0afd6c7d144bfa4").unwrap(),
        );
        tree.add_directory(String::from("dir"), sub_tree)
            .add_file(
                String::from("lol"),
                Hash::from_str("63cd04a52f5c8cb95686081b000223e968ed74f4").unwrap(),
            )
            .add_executable(
                String::from("run.sh"),
                Hash::from_str("5198cfd733f87f38ddfb400964c38c8ea238ea17").unwrap(),
            );
        let dump = tree.dump();
        assert_eq!(dump.len(), 103);
        assert_eq!(std::str::from_utf8(&dump[0..8]).unwrap(), "tree 95\0");
    }
}
