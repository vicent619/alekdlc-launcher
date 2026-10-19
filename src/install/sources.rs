use serde::Deserialize;

use crate::install::download::Http;
use crate::net::error::ApiError;

const MOJANG_MANIFEST: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const RESOURCES: &str = "https://resources.download.minecraft.net";
const FABRIC_META: &str = "https://meta.fabricmc.net/v2/versions/loader";

#[derive(Clone, Debug)]
pub struct Artifact {
    pub path: String,
    pub url: String,
    pub sha1: String,
    pub size: u64,
}

#[derive(Clone, Debug)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

#[derive(Clone, Debug)]
pub struct GamePlan {
    pub client: Artifact,
    pub libraries: Vec<Artifact>,
    pub asset_index_id: String,
    pub asset_index: Artifact,
    pub assets: Vec<AssetObject>,
    pub main_class: String,
}

#[derive(Deserialize)]
struct VersionManifest {
    versions: Vec<VersionEntry>,
}

#[derive(Deserialize)]
struct VersionEntry {
    id: String,
    url: String,
}

#[derive(Deserialize)]
struct VersionDetail {
    downloads: Downloads,
    libraries: Vec<Library>,
    #[serde(rename = "assetIndex")]
    asset_index: AssetIndexRef,
    assets: String,
}

#[derive(Deserialize)]
struct Downloads {
    client: DownloadRef,
}

#[derive(Deserialize)]
struct DownloadRef {
    url: String,
    sha1: String,
    #[serde(default)]
    size: u64,
}

#[derive(Deserialize)]
struct AssetIndexRef {
    id: String,
    url: String,
    sha1: String,
    #[serde(default)]
    size: u64,
}

#[derive(Deserialize)]
struct Library {
    name: String,
    downloads: Option<LibraryDownloads>,
    rules: Option<Vec<Rule>>,
}

#[derive(Deserialize)]
struct LibraryDownloads {
    artifact: Option<ArtifactRef>,
}

#[derive(Deserialize)]
struct ArtifactRef {
    path: String,
    url: String,
    sha1: String,
    #[serde(default)]
    size: u64,
}

#[derive(Deserialize)]
struct Rule {
    action: String,
    os: Option<RuleOs>,
}

#[derive(Deserialize)]
struct RuleOs {
    name: Option<String>,
}

#[derive(Deserialize)]
struct AssetIndexDoc {
    objects: std::collections::HashMap<String, AssetEntry>,
}

#[derive(Deserialize)]
struct AssetEntry {
    hash: String,
    size: u64,
}

pub fn resolve_game(http: &Http, minecraft: &str) -> Result<GamePlan, ApiError> {
    let manifest: VersionManifest = http.json(MOJANG_MANIFEST)?;
    let entry = manifest
        .versions
        .iter()
        .find(|v| v.id == minecraft)
        .ok_or_else(|| ApiError::new("NO_VERSION", 0))?;
    let detail: VersionDetail = http.json(&entry.url)?;
    let client = Artifact {
        path: format!("versions/{minecraft}/{minecraft}.jar"),
        url: detail.downloads.client.url,
        sha1: detail.downloads.client.sha1,
        size: detail.downloads.client.size,
    };
    let libraries = detail
        .libraries
        .into_iter()
        .filter(|lib| allowed(&lib.rules))
        .filter_map(|lib| {
            lib.downloads
                .and_then(|d| d.artifact)
                .map(|a| Artifact {
                    path: format!("libraries/{}", a.path),
                    url: a.url,
                    sha1: a.sha1,
                    size: a.size,
                })
        })
        .collect();
    let asset_index = Artifact {
        path: format!("assets/indexes/{}.json", detail.asset_index.id),
        url: detail.asset_index.url.clone(),
        sha1: detail.asset_index.sha1,
        size: detail.asset_index.size,
    };
    let index_doc: AssetIndexDoc = http.json(&detail.asset_index.url)?;
    let assets = index_doc
        .objects
        .into_values()
        .map(|o| AssetObject { hash: o.hash, size: o.size })
        .collect();
    Ok(GamePlan {
        client,
        libraries,
        asset_index_id: detail.assets,
        asset_index,
        assets,
        main_class: "net.fabricmc.loader.impl.launch.knot.KnotClient".to_owned(),
    })
}

pub fn asset_url(hash: &str) -> String {
    format!("{RESOURCES}/{}/{}", &hash[..2], hash)
}

fn allowed(rules: &Option<Vec<Rule>>) -> bool {
    let Some(rules) = rules else {
        return true;
    };
    let mut allowed = false;
    for rule in rules {
        let applies = rule
            .os
            .as_ref()
            .and_then(|os| os.name.as_deref())
            .map(|name| name == "windows")
            .unwrap_or(true);
        if applies {
            allowed = rule.action == "allow";
        }
    }
    allowed
}

#[derive(Clone, Debug)]
pub struct FabricPlan {
    pub libraries: Vec<Artifact>,
    pub main_class: String,
}

#[derive(Deserialize)]
struct FabricLoaderDoc {
    #[serde(rename = "launcherMeta")]
    launcher_meta: FabricLauncherMeta,
    loader: FabricRef,
    intermediary: FabricRef,
}

#[derive(Deserialize)]
struct FabricRef {
    maven: String,
}

#[derive(Deserialize)]
struct FabricLauncherMeta {
    #[serde(rename = "mainClass")]
    main_class: MainClass,
    libraries: FabricLibraries,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum MainClass {
    Plain(String),
    Sided { client: String },
}

#[derive(Deserialize)]
struct FabricLibraries {
    #[serde(default)]
    common: Vec<FabricLibrary>,
    #[serde(default)]
    client: Vec<FabricLibrary>,
}

#[derive(Deserialize)]
struct FabricLibrary {
    name: String,
    url: Option<String>,
}

pub fn resolve_fabric(http: &Http, minecraft: &str, loader: &str) -> Result<FabricPlan, ApiError> {
    let url = format!("{FABRIC_META}/{minecraft}/{loader}");
    let doc: FabricLoaderDoc = http.json(&url)?;
    let main_class = match doc.launcher_meta.main_class {
        MainClass::Plain(value) => value,
        MainClass::Sided { client } => client,
    };
    let mut libraries = Vec::new();
    libraries.push(maven_artifact(http, &doc.loader.maven, "https://maven.fabricmc.net/")?);
    libraries.push(maven_artifact(http, &doc.intermediary.maven, "https://maven.fabricmc.net/")?);
    for lib in doc.launcher_meta.libraries.common.into_iter().chain(doc.launcher_meta.libraries.client) {
        let base = lib.url.unwrap_or_else(|| "https://maven.fabricmc.net/".to_owned());
        libraries.push(maven_artifact(http, &lib.name, &base)?);
    }
    Ok(FabricPlan { libraries, main_class })
}

fn maven_path(coords: &str) -> String {
    let parts: Vec<&str> = coords.splitn(4, ':').collect();
    let group = parts[0].replace('.', "/");
    let artifact = parts.get(1).copied().unwrap_or("");
    let version = parts.get(2).copied().unwrap_or("");
    let classifier = parts.get(3).map(|c| format!("-{c}")).unwrap_or_default();
    format!("{group}/{artifact}/{version}/{artifact}-{version}{classifier}.jar")
}

#[derive(Clone, Deserialize)]
struct ModrinthVersion {
    files: Vec<ModrinthFile>,
}

#[derive(Clone, Deserialize)]
struct ModrinthFile {
    url: String,
    filename: String,
    hashes: ModrinthHashes,
    #[serde(default)]
    primary: bool,
    #[serde(default)]
    size: u64,
}

#[derive(Clone, Deserialize)]
struct ModrinthHashes {
    sha1: String,
}

pub fn resolve_fabric_api(http: &Http, minecraft: &str) -> Result<Artifact, ApiError> {
    let url = format!(
        "https://api.modrinth.com/v2/project/fabric-api/version?loaders=%5B%22fabric%22%5D&game_versions=%5B%22{minecraft}%22%5D"
    );
    let versions: Vec<ModrinthVersion> = http.json(&url)?;
    let files = versions.into_iter().next().map(|v| v.files).unwrap_or_default();
    let file = files
        .iter()
        .find(|f| f.primary)
        .cloned()
        .or_else(|| files.into_iter().next())
        .ok_or_else(|| ApiError::new("NO_FABRIC_API", 0))?;
    Ok(Artifact {
        path: format!("mods/{}", file.filename),
        url: file.url,
        sha1: file.hashes.sha1,
        size: file.size,
    })
}

fn maven_artifact(http: &Http, coords: &str, base: &str) -> Result<Artifact, ApiError> {
    let path = maven_path(coords);
    let base = if base.ends_with('/') { base.to_owned() } else { format!("{base}/") };
    let url = format!("{base}{path}");
    let sha1 = http.text(&format!("{url}.sha1")).ok().map(|s| s.trim().split_whitespace().next().unwrap_or("").to_owned()).unwrap_or_default();
    Ok(Artifact {
        path: format!("libraries/{path}"),
        url,
        sha1,
        size: 0,
    })
}
