use std::fs;
use std::path::Path;

pub fn git_branch(directory: &Path) -> Option<String> {
    let dot_git = directory
        .ancestors()
        .map(|dir| dir.join(".git"))
        .find(|candidate| candidate.exists())?;
    let git_dir = match fs::read_to_string(&dot_git) {
        Ok(file) => dot_git
            .parent()?
            .join(file.trim().strip_prefix("gitdir:")?.trim()),
        Err(_) => dot_git,
    };
    let head = fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim();
    Some(match head.strip_prefix("ref: refs/heads/") {
        Some(branch) => branch.to_string(),
        None => head.chars().take(7).collect(),
    })
}

pub fn host_name() -> Option<String> {
    let name = fs::read_to_string("/proc/sys/kernel/hostname").ok()?;
    Some(name.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_branch_comes_from_head_in_a_parent_or_a_gitdir_file() {
        let root = std::env::temp_dir().join(format!("curseterm-git-{}", std::process::id()));
        let nested = root.join("repo/src/deep");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir_all(root.join("repo/.git")).unwrap();
        fs::write(root.join("repo/.git/HEAD"), "ref: refs/heads/feature/x\n").unwrap();
        assert_eq!(git_branch(&nested), Some("feature/x".to_string()));
        let worktree = root.join("worktree");
        fs::create_dir_all(root.join("store")).unwrap();
        fs::create_dir_all(&worktree).unwrap();
        fs::write(worktree.join(".git"), "gitdir: ../store\n").unwrap();
        fs::write(root.join("store/HEAD"), "0123456789abcdef\n").unwrap();
        assert_eq!(git_branch(&worktree), Some("0123456".to_string()));
        fs::remove_dir_all(&root).unwrap();
    }
}
