use semver::Version;

/// 检查是否已经是最新版本
/// # 返回
/// - (是否是最新版本, 最新版本号)
pub async fn is_up_to_date() -> Result<(bool, String), Box<dyn std::error::Error>> {
    // 获取当前程序的版本
    let current_version = Version::parse(env!("CARGO_PKG_VERSION"))?;

    // 从GitHub API获取最新的release信息

    // 设置ua
    let client = reqwest::Client::builder()
        .user_agent("snowbreak_gacha_export")
        .build()?;
    let response = client
        .get("https://api.github.com/repos/enximi/snowbreak_gacha_export/releases/latest")
        .send()
        .await?;
    let latest_release = response.text().await?;
    let json = serde_json::from_str::<serde_json::Value>(&latest_release)?;
    let latest_release_tag_name = json["tag_name"].as_str().ok_or("tag_name not found")?;
    let latest_version_str = latest_release_tag_name.trim_start_matches('v');
    let latest_version = Version::parse(latest_version_str)?;

    if latest_version > current_version {
        Ok((false, latest_release_tag_name.to_string()))
    } else {
        Ok((true, latest_release_tag_name.to_string()))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_is_up_to_date() {
        let result = is_up_to_date().await;
        println!("{:?}", result)
    }
}
