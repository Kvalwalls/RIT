use crate::objects::{Commit, Object};
use crate::{refs, utils};
use clap::ArgMatches;
use std::collections::{HashSet, VecDeque};
use std::error::Error;
use std::io;

pub fn run(_: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let repo_path = utils::find_repo()?;
    // TODO: Handle piping to the $PAGER (eg: less)

    if let Some(head) = refs::get_head(&repo_path) {
        // This is used for the commits queue
        let mut commits = VecDeque::new();
        // This set allow duplicate handling
        let mut used = HashSet::new();

        used.insert(head.hash());
        commits.push_front(head);

        while let Some(commit) = commits.pop_front() {
            println!("{}", commit);
            for parent in commit.parents.iter() {
                if !used.contains(parent) {
                    commits.push_back(*Commit::load(&repo_path, *parent));
                    used.insert(*parent);
                }
            }
        }
        Ok(())
    } else {
        // No commit found
        Err(Box::new(io::Error::new(
            io::ErrorKind::NotFound,
            "fatal: your current branch does not have any commits yet",
        )))
    }
}
