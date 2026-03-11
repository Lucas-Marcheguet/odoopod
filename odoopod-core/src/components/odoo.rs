pub struct Odoo;

impl Odoo {

    pub async fn download_extract_source_code(version: String, _is_enterprise: bool, destination: std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        println!("Downloading Odoo source code for version {}...", version);
        let bytes = Odoo::download_source_code(version.clone(), _is_enterprise).await.unwrap();
        println!("Extracting Odoo source code for version {}...", version);
        Odoo::extract_tarball(bytes, destination.clone()).await?;
        println!("Odoo source code successfully extracted to the destination path {}", destination.display());
        Ok(())
    }

    pub async fn download_source_code(version: String, _enterprise: bool) -> Result<bytes::Bytes, Box<dyn std::error::Error>> {
        let url = format!("https://api.github.com/repos/odoo/odoo/tarball/{version}");
        let client = reqwest::Client::new();
        let bytes = client.get(&url)
            .header("User-Agent", "OdooPod")
            .send()
            .await?
            .bytes()
            .await?;
        Ok(bytes)
    }

    pub async fn extract_tarball(bytes: bytes::Bytes, destination: std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        // Extract the Odoo source code tarball to the destination path
        let tar = flate2::read::GzDecoder::new(&bytes[..]);
        let mut archive = tar::Archive::new(tar);
        //Unpack only the odoo source code from the archive to the destination path
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;
            // The source code is in a subdirectory named odoo-<hash>, so we need to strip the first component of the path and extract the rest to the destination path
            let mut components = path.components();
            components.next();
            let relative_path = components.as_path();
            let destination_path = destination.join(relative_path);
            if let Some(parent) = destination_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            entry.unpack(destination_path)?;
        }
        Ok(())
    }
}