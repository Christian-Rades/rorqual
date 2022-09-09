use chrono::format::{self, strftime::StrftimeItems, Parsed};
use chrono::{DateTime, Utc};
use git2::{Delta, DiffDelta, DiffOptions, Repository, Sort, Tree};
use petgraph::{graph::NodeIndex, Graph, Undirected};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use regex::RegexSet;
use std::error::Error;

use std::collections::{HashMap, HashSet};

pub struct GitFilter {
    pub start_date: Option<DateTime<Utc>>,
    pub max_commits: Option<u32>,
    pub path_filters: RegexSet,
}

//Idea to intern filepaths it's prob shit
struct InternedPath<'a> {
    parent: Option<&'a InternedPath<'a>>,
    element: String,
}

pub fn repo_to_changesets(
    path: std::path::PathBuf,
    filter: &GitFilter,
) -> Vec<git_graph::ChangeSet> {
    let mut graph = Graph::<HashMap<String, String>, i64, Undirected>::new_undirected();

    let repo = Repository::open(path).unwrap();
    let commit_trees = search_repo(&repo, filter).unwrap();
    let commit_trees: Vec<git2::Tree> = commit_trees.into_iter().collect();

    let mut options = DiffOptions::new();
    //no big impact
    options.skip_binary_check(true);

    let diffs = commit_trees.windows(2).flat_map(|window| match window {
        [old, new] => repo
            .diff_tree_to_tree(Some(old), Some(new), Some(&mut options))
            .ok(),
        _ => None,
    });

    let mut out: Vec<git_graph::ChangeSet> = Vec::new();
    for d in diffs {
        let change = d
            .deltas()
            .map(|delta| git_graph::GitFile {
                name: delta
                    .old_file()
                    .path()
                    .unwrap()
                    .to_string_lossy()
                    .to_string(),
                status: delta_status(&delta),
            })
            .filter(|file| {
                filter.path_filters.is_empty() || filter.path_filters.is_match(&file.name)
            })
            .collect();
        out.push(change);
    }
    out
}

fn delta_status(delta: &DiffDelta) -> git_graph::Status {
    match delta.status() {
        Delta::Added | Delta::Copied => git_graph::Status::Added,
        Delta::Deleted | Delta::Ignored => git_graph::Status::Deleted,
        _ => git_graph::Status::Modified,
    }
}

fn search_repo<'repo>(
    repo: &'repo Repository,
    filter: &GitFilter,
) -> Result<impl Iterator<Item = git2::Tree<'repo>> + 'repo, git2::Error> {
    let mut rev_walk = repo.revwalk()?;
    rev_walk.set_sorting(Sort::NONE);
    rev_walk.push_head()?;

    let dt = filter.start_date.unwrap().naive_utc().timestamp();

    let commit_trees = rev_walk
        .flat_map(move |commit_id| repo.find_commit(commit_id.unwrap()))
        .take_while(move |commit| commit.time().seconds() > dt)
        .filter(|commit| commit.parents().len() != 1)
        // .filter(|commit| commit.message().and_then(|msg: &str| Some(msg.contains("Merge pull request"))).unwrap_or(false))
        .flat_map(|commit| commit.tree());
    Ok(commit_trees)
}

use std::fs::{copy, create_dir, rename};
use std::path::Path;
use tempfile::TempDir;
use walkdir::WalkDir;

use super::git_graph;

static FIXTURES_PATH: &str = "./tests/fixtures";

fn load_fixture_repo(name: &str) -> Result<(TempDir, Repository), Box<dyn std::error::Error>> {
    let tmpdir = TempDir::new()?;
    let fixture_path = Path::new(FIXTURES_PATH).join(name);
    copy_recursively(&fixture_path, tmpdir.path())?;

    let gitted = tmpdir.path().join(".gitted");
    if gitted.exists() {
        rename(gitted, tmpdir.path().join(".git"))?;
    }
    let gitattributes = tmpdir.path().join("gitattributes");
    if gitattributes.exists() {
        rename(gitattributes, tmpdir.path().join(".gitattributes"))?;
    }
    let gitignore = tmpdir.path().join("gitignore");
    if gitignore.exists() {
        rename(gitignore, tmpdir.path().join(".gitignore"))?;
    }

    let repo = Repository::init(tmpdir.path())?;

    Ok((tmpdir, repo))
}

fn copy_recursively(source: &Path, target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for source_entry in WalkDir::new(source) {
        let entry = source_entry?;
        let root = entry.path().strip_prefix(source)?;
        let dest = target.join(root);

        if dest == target {
            continue;
        }

        if entry.file_type().is_dir() {
            create_dir(dest)?;
        } else {
            copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

#[test]
fn test_scan_repo() {
    let (_dir, repo) = load_fixture_repo("basic-repo").unwrap();
    let trees = search_repo(&repo).unwrap();
    assert_eq!(2, trees.count());
}
