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
    let bizuro = outcome.java_dir.join(crate::secret!("bizuro_v_drm.exe"));
    let javaw = outcome.java_dir.join(crate::secret!("javaw.exe"));
    let separator = crate::secret!(";").to_owned();
    let classpath = outcome
        .classpath
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(&separator);

    let mut args = Vec::new();
    args.push(format!("{}{}G", crate::secret!("-Xmx"), params.memory_gb));
    args.push(format!("{}{}G", crate::secret!("-Xms"), params.memory_gb.min(2)));
    args.push(crate::secret!("-Dminecraft.launcher.brand=alekdlc").to_owned());
    args.push(crate::secret!("-cp").to_owned());
    args.push(classpath);
    args.push(outcome.main_class.clone());

    let game = [
        (crate::secret!("--username").to_owned(), params.username.to_owned()),
        (crate::secret!("--version").to_owned(), crate::secret!("fabric-loader").to_owned()),
        (crate::secret!("--gameDir").to_owned(), path_str(&outcome.game_dir)),
        (crate::secret!("--assetsDir").to_owned(), path_str(&outcome.game_dir.join(crate::secret!("assets")))),
        (crate::secret!("--assetIndex").to_owned(), outcome.asset_index.clone()),
        (crate::secret!("--uuid").to_owned(), params.uuid.to_owned()),
        (crate::secret!("--accessToken").to_owned(), params.access_token.to_owned()),
        (crate::secret!("--userType").to_owned(), crate::secret!("msa").to_owned()),
        (crate::secret!("--versionType").to_owned(), crate::secret!("release").to_owned()),
        (crate::secret!("--alekdlcToken").to_owned(), params.client_token.clone()),
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
