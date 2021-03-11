# rust-dropbox

[![crate.io](https://img.shields.io/crates/v/rust-dropbox)](https://crates.io/crates/rust-dropbox)

A convenient tool binding to the Dropbox APIv2,
Now it can operate `user_check`,`upload(file size below 150 MB)`, `move` and `download`.
It will handle error messages from Dropbox api.
And there is a async api can be activate by feature `non-blocking`.

For use, you need a Dropbox [access token](https://www.dropbox.com/developers/apps/)

# Installation

- Find in [crates.io](https://crates.io/crates/rust-dropbox)
- Use [cargo-edit](https://crates.io/crates/cargo-edit)
```sh
cargo add rust-dropbox
```

# Usage
### blocking api
- user_check
```rust
use rust_dropbox::*;
use std::env;
let token = env::var("DROPBOX_TOKEN").unwrap();
let client = client::DBXClient::new(&token);
let res = client.check_user("ping");
assert!(res.is_ok())
```

- upload
```rust
use rust_dropbox::*
use std::env;
use std::{
    fs::File,
    io::Read,
};
let token = env::var("DROPBOX_TOKEN").unwrap();
let mut file = File::open("./profile.jpg").unwrap();
let mut buf: Vec<u8> = Vec::new();
file.read_to_end(&mut buf).unwrap();
let client = client::DBXClient::new(&token);
let res = client.upload(buf, "/test/profile.jpg", UploadMode::Overwrite);
assert!(res.is_ok())   
```

- move
```rust
use rust_dropbox::*
use std::env;

let token = env::var("DROPBOX_TOKEN").unwrap();
let client = client::DBXClient::new(&token);
let move_option = MoveOption::new()
    .allow_ownership_transfer()
    .allow_shared_folder()
    .allow_auto_rename();
let res = client.move_file("/test/profile.jpg", "/profile.jpg", move_option);
assert!(res.is_ok())
```

- download
```rust
use rust_dropbox::*
use std::env;
use std::{
    fs::File,
    io::Write,
};

let token = env::var("DROPBOX_TOKEN").unwrap();
let client = client::DBXClient::new(&token);
let res = client.download("/profile.jpg");
let bytes = res.unwrap();
let mut file = File::create("new_profile.jpg").unwrap();
file.write_all(&bytes).unwrap();
```

### To use non-blocking api
```toml
rust-dropbox={version=*,default-features=false,features=["non-blocking"]}
```