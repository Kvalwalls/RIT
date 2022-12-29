use crate::objects::{Commit, Hash, Object};
use std::collections::HashMap;
use std::fs;
use std::io::Error;
use std::path::PathBuf;
use std::str::FromStr;

/// Dereference a ref (not guaranteed value existence)
fn deref(repo_path: &PathBuf, ref_: &String) -> Result<String, Error> {
    let path = repo_path.join(ref_);

    // End of the ref
    if !path.exists() {
        return Ok(ref_.clone());
    }

    let mut content = fs::read_to_string(path)?;
    // Remove trailing newline
    content.pop();
    if content.starts_with("ref: ") {
        deref(repo_path, &content[5..].to_string())
    } else {
        Ok(ref_.clone())
    }
}

/// This function resolve a reference giving the hash of the object behind it.
pub fn resolve(repo_path: &PathBuf, ref_: &String) -> Result<Hash, Error> {
    let ref_ = deref(repo_path, ref_)?;
    let path = repo_path.join(ref_);
    let mut content = fs::read_to_string(path)?;
    content.pop();
    Ok(Hash::from_str(&content[..]).unwrap())
}

/// This function return the HEAD commit
pub fn get_head(repo_path: &PathBuf) -> Option<Commit> {
    match resolve(repo_path, &String::from("HEAD")) {
        Ok(hash) => Some(*Commit::load(repo_path, hash)),
        _ => None,
    }
}

/// This function get all references given a path and adding a prefix
fn branches_(
    branches: &mut HashMap<String, Hash>,
    repo_path: &PathBuf,
    path: &PathBuf,
    prefix: &String,
) {
    for file in fs::read_dir(path).expect("Can't read in heads directory") {
        let file = file.unwrap();
        let file_name = file.file_name().into_string().unwrap();
        if file.path().is_dir() {
            let prefix = format!("{}{}/", prefix, file_name);
            branches_(branches, repo_path, &path.join(file_name), &prefix)
        } else {
            let branch_name = format!("{}{}", prefix, file_name);
            let hash = resolve(
                repo_path,
                &format!("refs/heads/{}", branch_name).to_string(),
            );
            if let Ok(hash) = hash {
                branches.insert(branch_name, hash);
            }
        }
    }
}

/// This function return the list of branches and their associated commits hash
pub fn branches(repo_path: &PathBuf) -> HashMap<String, Hash> {
    let mut res = HashMap::new();
    branches_(
        &mut res,
        &repo_path,
        &repo_path.join("refs/heads"),
        &String::new(),
    );
    res
}

/// This function return the current branch name and his associated commits hash
/// Return None if no commit or detached HEAD
pub fn current_branch(repo_path: &PathBuf) -> Option<(String, Hash)> {
    let path = repo_path.join("HEAD");
    let head = match get_head(repo_path) {
        Some(commit) => commit.hash(),
        None => return None,
    };
    match fs::read_to_string(&path) {
        Ok(branch) => {
            if branch.starts_with("ref:") {
                let mut branch = branch[16..].to_string();
                branch.pop();
                Some((branch, head))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// This function removes a ref given its path
pub fn remove_ref(path: &PathBuf) -> Result<(), Box<Error>> {
    fs::remove_file(&path)?;
    let mut path = path.clone();

    // Remove parent directories if empty
    loop {
        path.pop();
        if let Err(_) = fs::remove_dir(&path) {
            return Ok(());
        }
    }
}

/// This function update/create the object stored in a ref safely.
/// If dereferenced is true the ref is dereferenced before updated.
pub fn update(
    repo_path: &PathBuf,
    ref_: &String,
    value: &String,
    dereferenced: bool,
) -> Result<(), Error> {
    let ref_ = if dereferenced {
        deref(repo_path, ref_)?
    } else {
        ref_.clone()
    };
    let path = repo_path.join(ref_);

    // Create potention subdirectories
    let mut dir = path.clone();
    dir.pop();
    fs::create_dir_all(dir)?;

    // Write the ref
    fs::write(path, format!("{}\n", value))?;
    Ok(())
}
