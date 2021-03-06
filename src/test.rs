#[cfg(test)]
mod tests {
    use crate::*;
    use std::env;
    use std::{
        fs::File,
        io::{Read, Write},
    };

    #[test]
    fn test_user_check() {
        let token = env::var("DROPBOX_TOKEN").unwrap();
        let client = OAuth2Client::new(&token);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let res = rt.block_on(async move {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            client.check_user("ping").await
        });
        assert_eq!((), res.unwrap())
    }
    #[test]
    fn test_upload() {
        let token = env::var("DROPBOX_TOKEN").unwrap();
        let mut file = File::open("./profile.jpg").unwrap();
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).unwrap();
        let client = OAuth2Client::new(&token);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let res = rt.block_on(async move {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            client
                .upload(buf, "/test/profile.jpg", UploadMode::Overwrite)
                .await
        });

        assert_eq!((), res.unwrap())
    }

    #[test]
    fn test_move() {
        let token = env::var("DROPBOX_TOKEN").unwrap();
        let client = OAuth2Client::new(&token);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let res = rt.block_on(async move {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            let move_option = MoveOption::new()
                .allow_ownership_transfer()
                .allow_shared_folder()
                .auto_rename();
            client
                .move_file("/test/profile.jpg", "/profile.jpg", move_option)
                .await
        });
        assert_eq!((), res.unwrap())
    }

    #[test]
    fn test_download() {
        let token = env::var("DROPBOX_TOKEN").unwrap();
        let client = OAuth2Client::new(&token);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let res = rt.block_on(async move {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            client.download("/profile.jpg").await
        });
        let bytes = res.unwrap();
        let mut file = File::create("new_profile.jpg").unwrap();
        file.write_all(&bytes).unwrap();
    }
}
