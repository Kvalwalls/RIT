use glob::{Pattern, PatternError};
use std::env;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::io::{Error, ErrorKind};
use std::path::PathBuf;

/// This function return the path to the repository. If not in a my-git repository then return an
/// error.
pub fn find_repo() -> Result<PathBuf, Error> {
    let path = find_root()?;

    Ok(path.join(".rit"))
}

/// This function return the path to root of the project repository. If not in an
/// my-git repository then return an error.
pub fn find_root() -> Result<PathBuf, Error> {
    let mut path = env::current_dir()?;

    while !path.join(".rit").exists() {
        path = match path.parent() {
            Some(path) => path.to_path_buf(),
            None => {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    "fatal: not a git repository (or any of the parent directories): .rit",
                ))
            }
        }
    }

    Ok(path)
}

/// This function return relative the path to `dest`.
pub fn find_relative_path(dest: &PathBuf) -> PathBuf {
    let mut path = env::current_dir().unwrap();
    let dest = if !dest.is_absolute() {
        fs::canonicalize(dest).unwrap()
    } else {
        dest.clone()
    };
    let mut res = PathBuf::new();

    // Case same directory
    if dest == path {
        res.push(".");
        return res;
    }

    // Ascending directories
    while !dest.starts_with(&path) {
        res.push("..");
        path.pop();
    }

    // Descending tree
    while dest != path {
        let filename = dest
            .iter()
            .skip(path.iter().count())
            .next()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        res.push(&filename);
        path.push(&filename);
    }

    res
}

/// Return the list of ignored pattern. Contains at least .rit folder
pub fn ignored(root: &PathBuf) -> Result<Vec<Pattern>, PatternError> {
    let mut ignored = vec![Pattern::new(".rit")?];
    let ignore_file = root.join(".ritignore");
    if ignore_file.is_file() {
        let mut reader = BufReader::new(File::open(ignore_file).unwrap());
        let mut buffer = String::new();
        while reader.read_line(&mut buffer).unwrap() != 0 {
            // Remove trailing \n
            buffer.pop();
            // Remove trailing / (for directories)
            if buffer.ends_with("/") {
                buffer.pop();
            }
            if !buffer.is_empty() {
                ignored.push(Pattern::new(&buffer)?);
                buffer.clear()
            }
        }
    }
    Ok(ignored)
}

/// Check if an existing path is ignored or not
pub fn is_ignored(path: &PathBuf, ignored: &Vec<Pattern>) -> Result<bool, Error> {
    let path = fs::canonicalize(path)?;
    let mut path: PathBuf = path.iter().skip(find_root()?.iter().count()).collect();
    loop {
        if ignored.iter().any(|pattern| pattern.matches_path(&path)) {
            return Ok(true);
        }
        if !path.pop() {
            break;
        }
    }
    Ok(false)
}
