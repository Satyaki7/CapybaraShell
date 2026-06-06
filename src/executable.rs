use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub fn is_executable(cmd: &str) -> Option<String>{
    let path_var = env::var("PATH").unwrap_or_default();

    for dir in path_var.split(':') {
        let full_path = Path::new(dir).join(cmd);
        if full_path.exists() {
            if let Ok(metadata) = fs::metadata(&full_path) {
                let perms = metadata.permissions();

                //0o111 checks for execution permissions for user, group, and others --learned this.

                if perms.mode() & 0o111 != 0 {
                    return Some(full_path.to_string_lossy().to_string());
                }
            }
        }
    }
    None
}