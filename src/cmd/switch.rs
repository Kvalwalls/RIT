use crate::cmd::branch;
use crate::index::Index;
use crate::objects::{Commit, Hash, Object, Tree};
use crate::{refs, utils};
use clap::ArgMatches;
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;

pub fn switch_branch(
    repo_path: &PathBuf,
    branch: &String,
    branches: &HashMap<String, Hash>,
) -> Result<(), Box<dyn Error>> {
    // Find branch commit
    let commit = match branches.iter().find(|(b, _)| *b == branch) {
        Some((_, commit)) => Commit::load(repo_path, *commit),
        _ => return Err(Box::new(branch::ErrorBranch::NoBranchFound(branch.clone()))),
    };

    // Check if nothing has to be done
    if let Some((current_branch, _)) = refs::current_branch(&repo_path) {
        if current_branch == *branch {
            println!("Already on '{}'", branch);
            return Ok(());
        }
    }

    // Apply the commit to the fs
    let commit_tree = Tree::load(repo_path, commit.tree);
    let root = utils::find_root()?;
    let head_tree = Tree::load(repo_path, refs::get_head(repo_path).unwrap().tree);
    let mut index = Index::load(repo_path);
    head_tree.apply(repo_path, &mut index, &root, &commit_tree)?;

    // Save the new index
    index.save(repo_path);

    // Update HEAD
    refs::update(
        repo_path,
        &String::from("HEAD"),
        &format!("ref: refs/heads/{}", branch),
        false,
    )?;
    Ok(())
}

pub fn run(args: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let repo_path = utils::find_repo()?;
    let branch = args.value_of("BRANCH").unwrap().to_string();
    let mut branches = refs::branches(&repo_path);

    // Create branch
    if args.is_present("create") || args.is_present("force-create") {
        branch::create_branch(
            &repo_path,
            &branch,
            args.is_present("force-create"),
            &mut branches,
        )?;
    }

    switch_branch(&repo_path, &branch, &branches)?;

    Ok(())
}
