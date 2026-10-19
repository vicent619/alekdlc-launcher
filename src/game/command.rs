use std::path::{Path, PathBuf};

use crate::install::installer::Outcome;

pub struct LaunchSpec {
    pub bizuro: PathBuf,
    pub javaw: PathBuf,
    pub args: Vec<String>,
    pub client_token: String,
    pub bo_token: String,
    pub working_dir: PathBuf,
}

pub struct LaunchParams<'a> {
    pub outcome: &'a Outcome,
    pub username: &'a str,
    pub uuid: &'a str,
    pub access_token: &'a str,
    pub client_token: String,
    pub bo_token: String,
    pub memory_gb: u32,
    pub launcher_version: &'a str,
}

pub fn build(params: LaunchParams) -> LaunchSpec {
    let outcome = params.outcome;
    let bizuro = outcome.java_dir.join("bizuro_v.exe");
    let javaw = outcome.java_dir.join("javaw.exe");
    let separator = ";";
    let classpath = outcome
        .classpath
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(separator);

    let mut args = Vec::new();
    args.push(format!("-Xmx{}G", params.memory_gb));
    args.push(format!("-Xms{}G", params.memory_gb.min(2)));
    args.push("-Dminecraft.launcher.brand=alekdlc".to_owned());
    args.push("-cp".to_owned());
    args.push(classpath);
    args.push(outcome.main_class.clone());

    let game = [
        ("--username", params.username.to_owned()),
        ("--version", "fabric-loader".to_owned()),
        ("--gameDir", path_str(&outcome.game_dir)),
        ("--assetsDir", path_str(&outcome.game_dir.join("assets"))),
        ("--assetIndex", outcome.asset_index.clone()),
        ("--uuid", params.uuid.to_owned()),
        ("--accessToken", params.access_token.to_owned()),
        ("--userType", "msa".to_owned()),
        ("--versionType", "release".to_owned()),
        ("--alekdlcToken", params.client_token.clone()),
    ];
    for (key, value) in game {
        args.push(key.to_owned());
        args.push(value);
    }

    LaunchSpec {
        bizuro,
        javaw,
        args,
        client_token: params.client_token,
        bo_token: params.bo_token,
        working_dir: outcome.game_dir.clone(),
    }
}

fn path_str(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
