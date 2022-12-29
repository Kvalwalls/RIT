use crate::index::{EntryType, Index};
use crate::objects::{Blob, Object, Tree, TreeEntry};
use crate::refs;
use crate::utils;
use clap::ArgMatches;
use colored::Colorize;
use glob::glob;
use glob::Pattern;
use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

#[derive(Eq, PartialEq, PartialOrd, Ord)]
enum Status {
    New(String),
    ModifiedStaged(String),
    DeletedStaged(String),
    ModifiedNotStaged(String),
    DeletedNotStaged(String),
    Untracked(String),
}

impl Status {
    /// Create a new Status given a type and a path
    pub fn new(type_: &str, path: &PathBuf) -> Self {
        let mut path_str = utils::find_relative_path(&path)
            .to_str()
            .unwrap()
            .to_string();
        if path.is_dir() {
            path_str.push('/');
        }
        match &type_.to_ascii_lowercase()[..] {
            "new" => Status::New(path_str),
            "modifiedstaged" => Status::ModifiedStaged(path_str),
            "modifiednotstaged" => Status::ModifiedNotStaged(path_str),
            "deletenotstaged" => Status::DeletedNotStaged(path_str),
            "deletestaged" => Status::DeletedStaged(path_str),
            "untracked" => Status::Untracked(path_str),
            _ => panic!(format!("fatal: type '{}' unknown", type_)),
        }
    }
}

fn compute_untracked(
    status: &mut BTreeSet<Status>,
    path: &PathBuf,
    last_commit: &Tree,
    index: &Index,
    ignored: &Vec<Pattern>,
) -> Result<(), Box<Error>> {
    // TODO: Not that simple should check that it could be track (need gitignore feature)
    if utils::is_ignored(path, ignored)? {
        return Ok(());
    }
    if !index.contains(path)? && !last_commit.contains(path)? {
        status.insert(Status::new("untracked", &path));
        return Ok(());
    }

    // If directory call recursively
    if path.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            let file_name = entry.unwrap().file_name();
            let path = path.join(&file_name);
            compute_untracked(status, &path, last_commit, index, &ignored)?;
        }
    }
    Ok(())
}

fn compute_tracked(
    status: &mut BTreeSet<Status>,
    path: &PathBuf,
    last_commit: &Tree,
    index: &Index,
) -> Result<(), Box<Error>> {
    let root = utils::find_root()?;
    let path = fs::canonicalize(&path)?;
    for (entry_path, (entry_type, hash)) in index.entries.iter() {
        let entry_path = PathBuf::from(&entry_path);
        let full_path = root.join(&entry_path);

        // Check if the file is part of path
        if !full_path.starts_with(&path) {
            continue;
        }

        // Staged files (new/modified)
        if !last_commit.contains(&entry_path)? {
            status.insert(Status::new("new", &full_path));
        } else {
            let entry = last_commit.get_entry(&entry_path).unwrap();
            match (entry, entry_type) {
                (TreeEntry::File(c_hash), EntryType::File) if hash == c_hash => (),
                (TreeEntry::Executable(c_hash), EntryType::Executable) if hash == c_hash => (),
                (TreeEntry::Symlink(c_hash), EntryType::Symlink) if hash == c_hash => (),
                (TreeEntry::Directory(_), _) => {
                    status.insert(Status::new("new", &full_path));
                }
                _ => {
                    status.insert(Status::new("modifiedstaged", &full_path));
                }
            }
        }

        // Unstaged files (modified/deleted)
        let metadata = fs::symlink_metadata(&full_path);
        if !full_path.exists() || metadata.unwrap().is_dir() {
            status.insert(Status::new("deletenotstaged", &full_path));
        } else {
            // Compute blob
            let metadata = fs::symlink_metadata(&full_path).unwrap();
            let content = if metadata.file_type().is_symlink() {
                fs::read_link(&full_path)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .as_bytes()
                    .to_vec()
            } else {
                fs::read(&full_path)?
            };
            let blob = Blob::new(content);
            let file_type = Index::get_file_type(&full_path);
            if blob.hash() != *hash || file_type != *entry_type {
                status.insert(Status::new("modifiednotstaged", &full_path));
            }
        }
    }

    // Staged files (deleted)
    let mut stack = vec![(PathBuf::new(), last_commit)];
    while let Some((cur_path, tree)) = stack.pop() {
        for (file_name, entry) in tree.entries.iter() {
            let cur_path = cur_path.join(file_name);
            let full_path = root.join(&cur_path);
            // Check if the file is part of path
            if !full_path.starts_with(&path) {
                continue;
            }

            if let TreeEntry::Directory(tree) = entry {
                stack.push((cur_path, tree));
            } else if !index.contains(&cur_path)? {
                status.insert(Status::new("deletestaged", &full_path));
            }
        }
    }
    Ok(())
}
fn display(status: &BTreeSet<Status>) {
    // Clean working tree
    if status.is_empty() {
        return println!("nothing to commit, working tree clean");
    }

    // Staged files
    if status.iter().any(|s| match s {
        Status::New(_) | Status::ModifiedStaged(_) | Status::DeletedStaged(_) => true,
        _ => false,
    }) {
        println!("Changes to be committed:\n");
        for status in status.iter() {
            match status {
                Status::New(path) => println!("\tnew file:   {}", path.green()),
                Status::ModifiedStaged(path) => println!("\tmodified:   {}", path.green()),
                Status::DeletedStaged(path) => println!("\tdeleted:   {}", path.green()),
                _ => (),
            }
        }
        println!();
    }

    // Unstaged files
    if status.iter().any(|s| match s {
        Status::ModifiedNotStaged(_) | Status::DeletedNotStaged(_) => true,
        _ => false,
    }) {
        println!(
            "Changes not staged for commit:\n  \
             (use \"git add <file>...\" to update what will be committed)\n"
        );
        for status in status.iter() {
            match status {
                Status::ModifiedNotStaged(path) => println!("\tmodified:   {}", path.red()),
                Status::DeletedNotStaged(path) => println!("\tdeleted:   {}", path.red()),
                _ => (),
            }
        }
        println!();
    }

    // Untracked files or directories
    if status.iter().any(|s| match s {
        Status::Untracked(_) => true,
        _ => false,
    }) {
        println!(
            "Untracked files:\n  \
             (use \"git add <file>...\" to include in what will be comitted)\n"
        );
        for status in status.iter() {
            if let Status::Untracked(path) = status {
                println!("\t{}", path.red());
            }
        }
        println!();
    }
}

pub fn run(args: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let root = utils::find_root()?;
    let repo_path = utils::find_repo()?;

    let index = Index::load(&repo_path);

    let last_commit = match refs::get_head(&repo_path) {
        Some(commit) => *Tree::load(&repo_path, commit.tree),
        None => Tree::new(),
    };

    let mut status = BTreeSet::new();
    let ignored = utils::ignored(&root)?;

    if !args.is_present("PATHSPEC") {
        compute_untracked(&mut status, &root, &last_commit, &index, &ignored)?;
        compute_tracked(&mut status, &root, &last_commit, &index)?;
    } else {
        for spec in args.values_of("PATHSPEC").unwrap() {
            for entry in glob(spec)? {
                if let Ok(path) = entry {
                    compute_untracked(&mut status, &path, &last_commit, &index, &ignored)?;
                    compute_tracked(&mut status, &path, &last_commit, &index)?;
                }
            }
        }
    }
    display(&status);
    Ok(())
}
