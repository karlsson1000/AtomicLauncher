use sha1::{Digest, Sha1};

pub async fn download_file_verified(
    url: &str,
    destination: &std::path::Path,
    expected_sha1: Option<&str>,
) -> Result<(), String> {
    let client = crate::utils::http::get_client();
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    let temp_path = {
        let file_name = destination
            .file_name()
            .ok_or_else(|| "Invalid destination path".to_string())?;
        destination
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(format!(
                "{}.part",
                file_name.to_string_lossy()
            ))
    };

    let write_result = write_streaming(response, &temp_path).await;

    match write_result {
        Ok(digest) => {
            if let Some(expected) = expected_sha1 {
                if digest != expected {
                    let _ = tokio::fs::remove_file(&temp_path).await;
                    return Err("sha1 verification failed".to_string());
                }
            }
            if let Err(e) = tokio::fs::rename(&temp_path, destination).await {
                let _ = tokio::fs::remove_file(&temp_path).await;
                return Err(format!("Failed to finalize download: {}", e));
            }
            Ok(())
        }
        Err(e) => {
            let _ = tokio::fs::remove_file(&temp_path).await;
            Err(e)
        }
    }
}

async fn write_streaming(
    mut response: reqwest::Response,
    temp_path: &std::path::Path,
) -> Result<String, String> {
    use tokio::io::AsyncWriteExt;

    let mut hasher = Sha1::new();
    let mut file = tokio::fs::File::create(temp_path)
        .await
        .map_err(|e| format!("Failed to create temporary file: {}", e))?;

    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("Download interrupted: {}", e))?
    {
        hasher.update(&chunk);
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Failed to write download: {}", e))?;
    }

    file.flush()
        .await
        .map_err(|e| format!("Failed to flush download: {}", e))?;
    drop(file);

    Ok(format!("{:x}", hasher.finalize()))
}
