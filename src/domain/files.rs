use anyhow::Result;
use git2::Repository;
use std::collections::BTreeMap;

pub fn file_contributions(repo: &Repository) -> Result<BTreeMap<String, BTreeMap<String, usize>>> {
    let mut file_authors: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut rw = repo.revwalk()?;
    rw.push_head()?;

    for oid in rw.flatten() {
        let commit = repo.find_commit(oid)?;
        let tree = commit.tree()?;

        if let Ok(parent) = commit.parent(0) {
            let parent_tree = parent.tree()?;
            let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), None)?;
            diff.deltas().for_each(|delta| {
                if let Some(path) = delta.new_file().path().and_then(|p| p.to_str()) {
                    let email = commit.author().email().unwrap_or("unknown").to_string();
                    *file_authors
                        .entry(path.to_string())
                        .or_default()
                        .entry(email)
                        .or_default() += 1;
                }
            });
        }
    }
    Ok(file_authors)
}
