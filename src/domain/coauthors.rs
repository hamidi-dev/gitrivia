use anyhow::Result;
use git2::Repository;
use std::collections::BTreeMap;

pub fn top_coauthors(repo: &Repository) -> Result<BTreeMap<String, usize>> {
    let mut file_authors: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut rw = repo.revwalk()?;
    rw.push_head()?;

    for oid in rw.flatten() {
        let commit = repo.find_commit(oid)?;
        let tree = commit.tree()?;

        if let Ok(parent) = commit.parent(0) {
            let parent_tree = parent.tree()?;
            let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), None)?;
            for delta in diff.deltas() {
                if let Some(path) = delta.new_file().path().and_then(|p| p.to_str()) {
                    let author = commit.author().email().unwrap_or("unknown").to_string();
                    let authors = file_authors.entry(path.to_string()).or_default();
                    if !authors.contains(&author) {
                        authors.push(author);
                    }
                }
            }
        }
    }

    let mut pairs: BTreeMap<String, usize> = BTreeMap::new();
    for authors in file_authors.values() {
        for i in 0..authors.len() {
            for j in i + 1..authors.len() {
                let mut pair = vec![authors[i].clone(), authors[j].clone()];
                pair.sort();
                let key = format!("{} + {}", pair[0], pair[1]);
                *pairs.entry(key).or_default() += 1;
            }
        }
    }
    Ok(pairs)
}
