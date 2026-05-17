use anyhow::{Context, Result, bail};
use git2::{
    Cred, FetchOptions, IndexAddOption, PushOptions, RemoteCallbacks, Repository, StatusOptions,
};
use log::{error, info};
use std::path::PathBuf;
use chrono::Local;
use url::Url;

/// 克隆公开的仓库
///
/// 从服务器端 clone 一个 git 仓库到本地。
/// git_url 既可以包含 .git 后缀，也可以不包含。
/// into_dir 是 clone 到本地的目录。
pub fn clone_repo(git_url: &str, into_dir: &str) -> Result<()> {
    let git_url = git_url.trim();
    let into_dir = into_dir.trim();

    if git_url.is_empty() {
        return Err(anyhow::anyhow!("git_url 不能为空"));
    }
    if into_dir.is_empty() {
        return Err(anyhow::anyhow!("into_dir 不能为空"));
    }

    // 解析 git url
    let url =
        Url::parse(git_url).with_context(|| format!("{} 不是有效的git仓库地址。", git_url))?;

    // 提取项目名称
    let project_name = url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .map(|name| name.trim_end_matches(".git"))
        .ok_or_else(|| anyhow::anyhow!("无法从 git URL 中提取项目名称"))?;

    let mut project_dir = PathBuf::from(into_dir);
    project_dir.push(project_name);

    info!("开始从 {} 克隆仓库到 {}", git_url, project_dir.display());
    match Repository::clone(git_url, &project_dir) {
        Ok(_) => {
            info!("成功将仓库从 {} 克隆到 {}", git_url, project_dir.display());
            Ok(())
        }
        Err(e) => {
            error!(
                "克隆仓库时出错：从 {} 到 {}，错误信息：{}",
                git_url,
                project_dir.display(),
                e
            );
            Err(anyhow::anyhow!("克隆仓库失败：{}", e))
        }
    }
}

/// 克隆私有的仓库，需传入用户名和密码
pub fn clone_private_repo(
    git_url: &str,
    into_dir: &str,
    username: &str,
    password: &str,
) -> Result<()> {
    let git_url = git_url.trim();
    let into_dir = into_dir.trim();

    if git_url.is_empty() {
        return Err(anyhow::anyhow!("git_url 不能为空"));
    }
    if into_dir.is_empty() {
        return Err(anyhow::anyhow!("into_dir 不能为空"));
    }

    // 解析 git url
    let url =
        Url::parse(git_url).with_context(|| format!("{} 不是有效的git仓库地址。", git_url))?;

    // 提取项目名称
    let project_name = url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .map(|name| name.trim_end_matches(".git"))
        .ok_or_else(|| anyhow::anyhow!("无法从 git URL 中提取项目名称"))?;

    let mut project_dir = PathBuf::from(into_dir);
    project_dir.push(project_name);

    info!("开始从 {} 克隆仓库到 {}", git_url, project_dir.display());

    // 设置认证回调
    let mut callbacks = RemoteCallbacks::new();
    callbacks
        .credentials(|_url, _username, _password| Cred::userpass_plaintext(username, password));

    // 设置克隆选项
    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fetch_options);

    match builder.clone(git_url, &project_dir) {
        Ok(_) => {
            info!("成功将仓库从 {} 克隆到 {}", git_url, project_dir.display());
            Ok(())
        }
        Err(e) => {
            error!(
                "克隆仓库时出错：从 {} 到 {}，错误信息：{}",
                git_url,
                project_dir.display(),
                e
            );
            Err(anyhow::anyhow!("克隆仓库失败：{}", e))
        }
    }
}

/// create branch
pub fn new_branch(local_repo_dir: &str, new_branch_name: &str) -> Result<()> {
    // 打开本地仓库
    let repo = Repository::open(local_repo_dir)?;
    // 获取当前的 HEAD 提交对象
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    let new_branch = repo.branch(new_branch_name, &commit, false)?;
    if let Some(branch) = new_branch.get().shorthand() {
        info!("{} 分支创建完成", branch);
    }
    Ok(())
}

/// push
pub fn push_to_remote(
    local_repo_dir: &str,
    target_remote_url: &str,
    branch_name: &str,
    username: &str,
    password: &str,
) -> Result<()> {
    // 打开本地仓库
    let repo = Repository::open(local_repo_dir)?;
    let mut origin = repo.find_remote("origin")?;
    if target_remote_url != origin.url().unwrap_or("") {
        repo.remote_rename("origin", "old-origin")?;
        origin = repo.remote("origin", target_remote_url)?;
    }

    let mut callbacks = RemoteCallbacks::new();
    callbacks
        .credentials(|_url, _username, _password| Cred::userpass_plaintext(username, password));

    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(callbacks);

    origin.push(
        &[format!("refs/heads/{branch_name}:refs/heads/{branch_name}")],
        Some(&mut push_options),
    )?;

    Ok(())
}

/// push an existing Git repository。
///
/// 包含以下命令：
/// ```bash
/// git remote rename origin old-origin
/// git remote add origin http://ip:port/test-group/docs.git
/// git push --set-upstream origin --all
/// git push --set-upstream origin --tags
/// ```
fn push_existing_repo(
    local_repo_dir: &str,
    target_remote_url: &str,
    username: &str,
    password: &str,
) -> Result<()> {
    // 打开本地仓库
    let repo = Repository::open(local_repo_dir)?;
    let mut origin = repo.find_remote("origin")?;

    println!("origin = '{}'", origin.url().unwrap_or(""));

    if target_remote_url != origin.url().unwrap_or("") {
        let timestamp = Local::now().format("%Y%m%d%H%M%S%3f").to_string();
        println!("timestamp = {}", timestamp);
        repo.remote_rename("origin", &format!("old-origin-{timestamp}"))?;
        origin = repo.remote("origin", target_remote_url)?;
    }

    // 注意，github 不支持密码授权 remote: Invalid username or token. Password authentication is not supported for Git operations.
    let mut callbacks = RemoteCallbacks::new();
    callbacks
        .credentials(|_url, _username_from_url, _allowed_types| {
            println!("url = '{}', username = '{}', password = '{:?}'", _url, username, password);
            Cred::userpass_plaintext(username, password)
        });

    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(callbacks);

    // 推送所有分支
    let local_branches = repo.branches(Some(git2::BranchType::Local))?;
    for branch in local_branches {
        let (mut branch, _) = branch?;
        let branch_name = branch.name()?.unwrap_or_default().to_string();

        println!("branch_name = '{}'", branch_name);

        origin.push(
            &[&format!(
                "refs/heads/{}:refs/heads/{}",
                branch_name,
                branch_name
            )],
            Some(&mut push_options),
        )?;

        branch.set_upstream(Some(&format!("origin/{}", branch_name)))?;
    }

    // 推送所有标签
    // 遍历所有本地标签并逐个推送
    for tag_name in repo
        .tag_names(None)?
        .iter()
        .flatten()
        .filter(|&name| !name.is_empty())
    {
        origin.push(
            &[&format!("refs/tags/{}:refs/tags/{}", tag_name, tag_name)],
            Some(&mut push_options),
        )?;
    }

    Ok(())
}

/// github 只支持通过 access token push 仓库
/// gitlab 既支持通过 access token push，也支持通过 password push
pub fn push_existing_repo_with_access_token(
    local_repo_dir: &str,
    target_remote_url: &str,
    access_token: &str,
) -> Result<()> {
    push_existing_repo(local_repo_dir, target_remote_url, "token", access_token)
}

/// gitee 只支持通过 password push
pub fn push_existing_repo_with_password(
    local_repo_dir: &str,
    target_remote_url: &str,
    username: &str,
    password: &str,
) -> Result<()> {
    push_existing_repo(local_repo_dir, target_remote_url, username, password)
}

/// commit
/// ```sh
/// git add .
/// git commit -m "Initial commit"
/// ```
pub fn commit_all_changes(local_repo_dir: &str, commit_message: &str) -> Result<()> {
    // 打开本地仓库
    let repo = Repository::open(local_repo_dir)?;

    // 配置状态检查选项
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .include_ignored(false)
        .recurse_untracked_dirs(true);

    // 获取仓库状态
    let statuses = repo.statuses(Some(&mut opts))?;
    // 无变更时直接退出
    if statuses.is_empty() {
        info!("无文件变化，跳过提交");
        return Ok(());
    }

    // git add .
    let mut index = repo.index()?;
    index.add_all(["."].iter(), IndexAddOption::DEFAULT, None)?;
    index.write()?;

    // git commit -m "commit message"
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let sig = repo.signature()?;
    let parent = repo.head()?.peel_to_commit()?;
    repo.commit(Some("HEAD"), &sig, &sig, commit_message, &tree, &[&parent])?;
    Ok(())
}


pub fn commit_files(local_repo_dir: &str, commit_message: &str, files: Vec<&String>) -> Result<String> {
    // 打开本地仓库
    let repo = Repository::open(local_repo_dir)?;

    // 配置状态检查选项
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .include_ignored(false)
        .recurse_untracked_dirs(true);

    // 获取仓库状态
    let statuses = repo.statuses(Some(&mut opts))?;
    // 无变更时直接退出
    if statuses.is_empty() {
        info!("无文件变化，跳过提交");
        bail!("无文件变化，跳过提交");
    }

    // git add .
    let mut index = repo.index()?;
    // 将指定的文件添加到暂存区（仅限已跟踪或被明确添加的文件，不会添加被忽略的文件）
    index.add_all(files.iter(), IndexAddOption::DEFAULT, None)?;
    index.write()?;

    // git commit -m "commit message"
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let sig = repo.signature()?;
    let parent = repo.head()?.peel_to_commit()?;
    let oid = repo.commit(Some("HEAD"), &sig, &sig, commit_message, &tree, &[&parent])?;
    Ok(oid.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_repo_with_git_suffix() {
        let git_url = "https://gitee.com/markdown-lang/template-ruoyi-vue-ui";
        let into_dir = "D:\\sources\\markdown-lang\\demos";
        // 删除 demos/template-ruoyi-vue-ui 目录
        std::fs::remove_dir_all(format!("{into_dir}/template-ruoyi-vue-ui")).unwrap_or_default();
        assert!(clone_repo(git_url, into_dir).is_ok());
    }

    #[test]
    fn push_existing_repo_ruoyi() {
        let local_repo_dir = "D:\\sources\\y_project\\RuoYi-Vue-test";
        let target_remote_url = "http://106.14.217.112:8888/test-group/docs.git";
        let username = "jinzw";
        let password = "peanut!@#456";
        let result = push_existing_repo(local_repo_dir, target_remote_url, username, password);
        if result.is_err() {
            println!("aaa=== {:?}", result.err().unwrap());
        } else {
            assert!(result.is_ok());
        }
    }

    #[test]
    fn push_existing_repo_to_gitlab_use_token() {
        let local_repo_dir = "D:\\sources\\temp\\template-business";
        let target_remote_url = "http://106.14.217.112:8888/project38/docs.git";
        let username = "token";
        let password = "glpat-yxbNRPihnxU4qamAggys";
        let result = push_existing_repo(local_repo_dir, target_remote_url, username, password);
        if result.is_err() {
            println!("aaa=== {:?}", result.err().unwrap());
        } else {
            assert!(result.is_ok());
        }
    }

    #[test]
    fn push_existing_repo_to_gitee_use_token() {
        let local_repo_dir = "D:\\sources\\temp\\template-business";
        let target_remote_url = "https://gitee.com/wechat-rs/docs.git";
        let username = "xiaohulu";
        let password = "sowhAt!@#456";
        let result = push_existing_repo(local_repo_dir, target_remote_url, username, password);
        if result.is_err() {
            println!("aaa=== {:?}", result.err().unwrap());
        } else {
            assert!(result.is_ok());
        }
    }

    #[test]
    fn push_existing_repo_to_github() {
        let local_repo_dir = "D:\\sources\\single-spark-projects\\business-docs";
        let target_remote_url = "https://github.com/single-spark-projects/business-docs.git";
        let username = "token";
        // 注意，此处是 token，不是密码
        let password = "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";
        let result = push_existing_repo(local_repo_dir, target_remote_url, username, password);
        if result.is_err() {
            println!("aaa=== {:?}", result.err().unwrap());
        } else {
            assert!(result.is_ok());
        }
    }

    #[test]
    fn commit_all_changes_ruoyi() {
        let local_repo_dir = "D:\\sources\\y_project\\RuoYi-Vue-test";
        let result = commit_all_changes(local_repo_dir, "first commit");
        if result.is_err() {
            println!("aaa=== {:?}", result.err().unwrap());
        } else {
            assert!(result.is_ok());
        }
    }

    #[test]
    fn now() {
        let timestamp = Local::now().format("%Y%m%d%H%M%S%3f").to_string();
        println!("{}", timestamp);
    }
}
