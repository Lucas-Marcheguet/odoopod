pub struct UvInstaller;

impl UvInstaller {
    pub fn get_current_uv_version(uv_path: std::path::PathBuf) -> Result<String, Box<dyn std::error::Error>> {
        // Get the current version of uv installed, by running `uv --version` and parsing the output
        if uv_path.exists() {
            let output = std::process::Command::new(uv_path.join("uv"))
                .arg("--version")
                .output()
                .expect("Failed to execute uv --version");
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return Ok(version);
            }
        }
        Err("uv is not ready".into())
    }

    async fn download_extract_tarball(uv_path: std::path::PathBuf, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Download the uv tarball and extract it to self.uv_path
        let bytes = reqwest::get(url)
            .await?
            .bytes()
            .await?;
        if !uv_path.exists() {
            std::fs::create_dir_all(&uv_path)?;
        }
        println!("Downloaded uv tarball from {}, extracting...", url);
        let tar = flate2::read::GzDecoder::new(&bytes[..]);
        let mut archive = tar::Archive::new(tar);
        //Unpack only the uv and uvx binarys from the archive to self.uv_path
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;
            if path.file_name().unwrap() == "uv" || path.file_name().unwrap() == "uv.exe" {
                entry.unpack(uv_path.join(path.file_name().unwrap()))?;
            }
        }
        Ok(())
    }

    pub async fn install_uv(uv_path: std::path::PathBuf) {
        // Install the uv binary as a prebuilt binary for each platform, and set self.uv_path and self.current_version accordingly

        // Check on https://api.github.com/repos/astral-sh/uv/releases/latest for the latest version of uv, and download the appropriate binary for the platform, then extract it to self.uv_path
        let client = reqwest::Client::builder()
            .user_agent("OdooPod/1.0")
            .build()
            .unwrap();
        let response = client.get("https://api.github.com/repos/astral-sh/uv/releases/latest")
            .send()
            .await;
        if response.is_err() {
            // Handle the error, maybe by logging it or returning it to the caller
            println!("An error occurred while fetching the latest version of uv: {}", response.err().unwrap());
            return;
        }
        if let Ok(resp) = response.as_ref() {
            if !resp.status().is_success() {
                println!(
                    "Failed to fetch latest uv release metadata. HTTP status: {}",
                    resp.status()
                );
                return;
            }
        }
        let json_data: serde_json::Value = response.unwrap().json().await.unwrap();
        for asset in json_data["assets"].as_array().unwrap() {
            let asset_name = asset["name"].as_str().unwrap();
            let target = std::env::consts::OS;
            let arch = std::env::consts::ARCH;
            
            let matches: bool = match (target, arch) {
                ("windows", "x86_64") => asset_name.contains("x86_64-pc-windows-msvc.zip"),
                ("windows", "aarch64") => asset_name.contains("aarch64-pc-windows-msvc.zip"),
                ("windows", "x86") => asset_name.contains("i686-pc-windows-msvc.zip"),
                ("macos", "x86_64") => asset_name.contains("x86_64-apple-darwin.tar.gz"),
                ("macos", "aarch64") => asset_name.contains("aarch64-apple-darwin.tar.gz"),
                ("linux", "x86_64") => asset_name.contains("x86_64-unknown-linux-gnu.tar.gz"),
                ("linux", "aarch64") => asset_name.contains("aarch64-unknown-linux-gnu.tar.gz"),
                _ => false,
            };

            if matches && !asset_name.contains(".sha256") {
                let download_url = asset["browser_download_url"].as_str().unwrap();
                println!("Downloading uv from {}...", download_url);
                UvInstaller::download_extract_tarball(uv_path.clone(), download_url).await.unwrap();
                break;
            }
        }
    }
}

pub struct UvInstance {
    uv_path: std::path::PathBuf,
}

impl UvInstance {

    pub fn new(uv_path: std::path::PathBuf) -> Self {
        UvInstance { uv_path }
    }

    pub async fn install_python_version(&self, python_version: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Install the specified python version using uv
        let output = tokio::process::Command::new(self.uv_path.join("uv"))
            .arg("python")
            .arg("install")
            .arg(python_version)
            .output().await?;

        if !output.status.success() {
            return Err(format!("Failed to install python version {}: {}", python_version, String::from_utf8_lossy(&output.stderr)).into());
        }

        Ok(())
    }

    pub async fn create_venv(&self, python_version: &str, venv_path: std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        // Create a virtual environment using uv for the specified python version and venv path
        let output = tokio::process::Command::new(self.uv_path.join("uv"))
            .arg("venv")
            .arg("--python")
            .arg(python_version)
            .arg(venv_path)
            .output().await?;

        if !output.status.success() {
            return Err(format!("Failed to create virtual environment: {}", String::from_utf8_lossy(&output.stderr)).into());
        }

        Ok(())
    }

    pub async fn pip_install_requirements(&self, venv_path: std::path::PathBuf, requirements_path: std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        // Install the requirements using uv pip install for the specified virtual environment and requirements path
        let output = tokio::process::Command::new(self.uv_path.join("uv"))
            .arg("pip")
            .arg("install")
            .arg("--no-cache")
            .arg("-r")
            .arg(requirements_path)
            .env("VIRTUAL_ENV", venv_path)
            .output().await?;

        if !output.status.success() {
            return Err(format!("Failed to install requirements: {}", String::from_utf8_lossy(&output.stderr)).into());
        }

        Ok(())
    }

    pub async fn pip_install(&self, venv_path: std::path::PathBuf, package: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Install a package using uv pip install for the specified virtual environment and package name
        let output = tokio::process::Command::new(self.uv_path.join("uv"))
            .arg("pip")
            .arg("install")
            .arg(package)
            .arg("--only-binary=:all")
            .env("VIRTUAL_ENV", venv_path)
            .output().await?;

        if !output.status.success() {
            return Err(format!("Failed to install package {}: {}", package, String::from_utf8_lossy(&output.stderr)).into());
        }

        Ok(())
    }


}