#[cfg(test)]
mod tests {
    use crate::*;
    use std::env;
    use std::{
        fs::File,
        io::{Read, Write},
    };

    #[tokio::test]
    async fn test_user_check() {
        let token = env::var("DROPBOX_TOKEN").unwrap();
        let client = client::AsyncDBXClient::new(&token);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let res = client.check_user("ping").await;
        assert!(res.is_ok())
    }

    #[tokio::test]
    async fn test_upload() {
        let token = env::var("DROPBOX_TOKEN").unwrap();
        let mut file = File::open("./profile.jpg").unwrap();
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).unwrap();
        let client = client::AsyncDBXClient::new(&token);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let option = UploadOption::new().disallow_auto_rename();
        let res = client.upload(buf, "/test/profile.jpg", &option).await;

        assert!(res.is_ok())
    }

    #[tokio::test]
    async fn test_move() {
        let token = env::var("DROPBOX_TOKEN").unwrap();
        let client = client::AsyncDBXClient::new(&token);
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        let option = MoveCopyOption::new()
            .allow_ownership_transfer()
            .allow_shared_folder()
            .allow_auto_rename();
        let res = client
            .move_file("/test/profile.jpg", "/profile.jpg", &option)
            .await;
        assert!(res.is_ok())
    }

    #[tokio::test]
    async fn test_copy() {
        let token = env::var("DROPBOX_TOKEN").unwrap();
        let client = client::AsyncDBXClient::new(&token);
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        let option = MoveCopyOption::new()
            .allow_ownership_transfer()
            .allow_shared_folder()
            .allow_auto_rename();
        let res = client
            .copy("/test/profile.jpg", "/profile.jpg", &option)
            .await;
        assert!(res.is_ok())
    }

    #[tokio::test]
    async fn test_download() {
        let token = env::var("DROPBOX_TOKEN").unwrap();
        let client = client::AsyncDBXClient::new(&token);
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        let res = client.download("/profile.jpg").await;
        let bytes = res.unwrap();
        let mut file = File::create("new_profile.jpg").unwrap();
        file.write_all(&bytes).unwrap();
    }

    #[test]
    fn test_blocking_user_check() {
        let token = env::var("DROPBOX_TOKEN").unwrap();
        let client = client::DBXClient::new(&token);
        let res = client.check_user("ping");
        assert!(res.is_ok())
    }

    #[test]
    fn test_blocking_upload() {
        let token = env::var("DROPBOX_TOKEN").unwrap();
        let mut file = File::open("./profile.jpg").unwrap();
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).unwrap();
        let client = client::DBXClient::new(&token);
        let option = UploadOption::new();
        let res = client.upload(buf, "/test/profile.jpg", &option);
        assert!(res.is_ok())
    }

    #[test]
    fn test_blocking_move() {
        let token = env::var("DROPBOX_TOKEN").unwrap();
        let client = client::DBXClient::new(&token);
        let option = MoveCopyOption::new()
            .allow_ownership_transfer()
            .allow_shared_folder()
            .allow_auto_rename();
        let res = client.move_file("/test/profile.jpg", "/profile.jpg", &option);
        assert!(res.is_ok())
    }

    #[test]
    fn test_blocking_copy() {
        let token = env::var("DROPBOX_TOKEN").unwrap();
        let client = client::DBXClient::new(&token);
        let option = MoveCopyOption::new()
            .allow_ownership_transfer()
            .allow_shared_folder()
            .allow_auto_rename();
        let res = client.copy("/test/profile.jpg", "/profile.jpg", &option);
        assert!(res.is_ok())
    }

    #[test]
    fn test_blocking_download() {
        let token = env::var("DROPBOX_TOKEN").unwrap();
        let client = client::DBXClient::new(&token);
        let res = client.download("/profile.jpg");
        let bytes = res.unwrap();
        let mut file = File::create("new_profile.jpg").unwrap();
        file.write_all(&bytes).unwrap();
    }
}
