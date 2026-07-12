use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::State;

use crate::error::{AppError, Result};
use crate::minecraft::launch::AuthSession;
use crate::settings;
use crate::state::Launcher;

/// Autenticação Microsoft → Xbox Live → XSTS → Minecraft (device code flow).
///
/// Exige um "client ID" de um aplicativo registrado no portal Azure com a
/// API do Minecraft aprovada pela Mojang
/// (https://help.minecraft.net/hc/en-us/articles/16254801392141). O usuário
/// configura o client ID na tela de Configurações.
const DEVICE_CODE_URL: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const SCOPE: &str = "XboxLive.signin offline_access";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredAccount {
    pub uuid: String,
    pub name: String,
    pub refresh_token: String,
    pub mc_token: String,
    /// Época unix (segundos) em que o mc_token expira
    pub expires_at: i64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AccountsFile {
    pub accounts: Vec<StoredAccount>,
    pub active: Option<String>,
}

#[derive(Serialize)]
pub struct AccountPublic {
    pub uuid: String,
    pub name: String,
    pub active: bool,
}

fn load_accounts(launcher: &Launcher) -> AccountsFile {
    let path = launcher.root.join("accounts.json");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn store_accounts(launcher: &Launcher, accounts: &AccountsFile) -> Result<()> {
    std::fs::write(
        launcher.root.join("accounts.json"),
        serde_json::to_string_pretty(accounts)?,
    )?;
    Ok(())
}

fn client_id(launcher: &Launcher) -> Result<String> {
    settings::load_settings(launcher)?
        .msa_client_id
        .filter(|c| !c.trim().is_empty())
        .ok_or_else(|| {
            AppError::msg(
                "Configure o Client ID do Azure nas Configurações antes de entrar. \
                 É preciso registrar um app no portal Azure e ter aprovação da Mojang.",
            )
        })
}

#[derive(Serialize, Deserialize)]
pub struct DeviceCode {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_in: u64,
}

#[tauri::command]
pub async fn msa_begin(launcher: State<'_, Launcher>) -> Result<DeviceCode> {
    let client_id = client_id(&launcher)?;
    let res = launcher
        .http
        .post(DEVICE_CODE_URL)
        .form(&[("client_id", client_id.as_str()), ("scope", SCOPE)])
        .send()
        .await?
        .error_for_status()?
        .json::<DeviceCode>()
        .await?;
    Ok(res)
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PollResult {
    Pending,
    Success { account: AccountPublic },
}

#[derive(Deserialize)]
struct MsaTokens {
    access_token: String,
    refresh_token: String,
}

/// Faz UMA tentativa de trocar o device code por tokens. O frontend chama
/// em intervalo até dar certo ou expirar.
#[tauri::command]
pub async fn msa_poll(launcher: State<'_, Launcher>, device_code: String) -> Result<PollResult> {
    let client_id = client_id(&launcher)?;
    let res = launcher
        .http
        .post(TOKEN_URL)
        .form(&[
            ("client_id", client_id.as_str()),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("device_code", device_code.as_str()),
        ])
        .send()
        .await?;

    if !res.status().is_success() {
        let body: serde_json::Value = res.json().await.unwrap_or_default();
        let error = body["error"].as_str().unwrap_or("");
        return match error {
            "authorization_pending" | "slow_down" => Ok(PollResult::Pending),
            "authorization_declined" => Err(AppError::msg("Login recusado pelo usuário")),
            "expired_token" => Err(AppError::msg("O código expirou — tente novamente")),
            other => Err(AppError::msg(format!("Erro no login Microsoft: {other}"))),
        };
    }

    let tokens: MsaTokens = res.json().await?;
    let account = complete_login(&launcher, &tokens).await?;
    Ok(PollResult::Success { account })
}

/// MSA token → Xbox Live → XSTS → Minecraft token → perfil; salva a conta.
async fn complete_login(launcher: &Launcher, tokens: &MsaTokens) -> Result<AccountPublic> {
    // 1. Xbox Live
    let xbl: serde_json::Value = launcher
        .http
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={}", tokens.access_token),
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT",
        }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let xbl_token = xbl["Token"]
        .as_str()
        .ok_or_else(|| AppError::msg("Resposta do Xbox Live sem token"))?;
    let uhs = xbl["DisplayClaims"]["xui"][0]["uhs"]
        .as_str()
        .ok_or_else(|| AppError::msg("Resposta do Xbox Live sem user hash"))?;

    // 2. XSTS
    let xsts_res = launcher
        .http
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbl_token] },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT",
        }))
        .send()
        .await?;
    if xsts_res.status().as_u16() == 401 {
        let body: serde_json::Value = xsts_res.json().await.unwrap_or_default();
        let xerr = body["XErr"].as_u64().unwrap_or(0);
        let msg = match xerr {
            2148916233 => "Esta conta Microsoft não tem um perfil Xbox — crie um em xbox.com",
            2148916238 => "Conta de menor de idade: precisa entrar em uma família Microsoft",
            _ => "Xbox Live recusou o login",
        };
        return Err(AppError::msg(msg));
    }
    let xsts: serde_json::Value = xsts_res.error_for_status()?.json().await?;
    let xsts_token = xsts["Token"]
        .as_str()
        .ok_or_else(|| AppError::msg("Resposta XSTS sem token"))?;

    // 3. Minecraft services
    let mc: serde_json::Value = launcher
        .http
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&json!({ "identityToken": format!("XBL3.0 x={uhs};{xsts_token}") }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let mc_token = mc["access_token"]
        .as_str()
        .ok_or_else(|| AppError::msg("Minecraft services não retornou token"))?;
    let expires_in = mc["expires_in"].as_i64().unwrap_or(86400);

    // 4. Perfil (falha se a conta não tem Minecraft comprado)
    let profile_res = launcher
        .http
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(mc_token)
        .send()
        .await?;
    if profile_res.status().as_u16() == 404 {
        return Err(AppError::msg("Esta conta Microsoft não possui o Minecraft"));
    }
    let profile: serde_json::Value = profile_res.error_for_status()?.json().await?;
    let uuid = profile["id"]
        .as_str()
        .ok_or_else(|| AppError::msg("Perfil sem UUID"))?
        .to_string();
    let name = profile["name"].as_str().unwrap_or("Jogador").to_string();

    let mut accounts = load_accounts(launcher);
    accounts.accounts.retain(|a| a.uuid != uuid);
    accounts.accounts.push(StoredAccount {
        uuid: uuid.clone(),
        name: name.clone(),
        refresh_token: tokens.refresh_token.clone(),
        mc_token: mc_token.to_string(),
        expires_at: chrono::Utc::now().timestamp() + expires_in - 60,
    });
    accounts.active = Some(uuid.clone());
    store_accounts(launcher, &accounts)?;

    Ok(AccountPublic {
        uuid,
        name,
        active: true,
    })
}

/// Sessão da conta ativa para lançar o jogo, renovando o token se expirou.
pub async fn active_session(launcher: &Launcher) -> Result<Option<AuthSession>> {
    let accounts = load_accounts(launcher);
    let Some(active) = &accounts.active else {
        return Ok(None);
    };
    let Some(account) = accounts.accounts.iter().find(|a| &a.uuid == active) else {
        return Ok(None);
    };

    let account = if account.expires_at <= chrono::Utc::now().timestamp() {
        // Token expirado: renova via refresh token
        let client_id = client_id(launcher)?;
        let tokens: MsaTokens = launcher
            .http
            .post(TOKEN_URL)
            .form(&[
                ("client_id", client_id.as_str()),
                ("grant_type", "refresh_token"),
                ("refresh_token", account.refresh_token.as_str()),
                ("scope", SCOPE),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        complete_login(launcher, &tokens).await?;
        let refreshed = load_accounts(launcher);
        refreshed
            .accounts
            .iter()
            .find(|a| &a.uuid == active)
            .cloned()
            .ok_or_else(|| AppError::msg("Conta sumiu após renovar token"))?
    } else {
        account.clone()
    };

    Ok(Some(AuthSession {
        username: account.name.clone(),
        uuid: account.uuid.clone(),
        access_token: account.mc_token.clone(),
        xuid: String::new(),
        user_type: "msa".into(),
    }))
}

// ------------------------- Skins -------------------------

#[derive(Serialize)]
pub struct SkinInfo {
    pub url: Option<String>,
    pub variant: Option<String>,
    pub capes: Vec<String>,
}

/// Perfil da conta ativa: skin atual (url + variante) e capas disponíveis.
#[tauri::command]
pub async fn get_skin(launcher: State<'_, Launcher>) -> Result<SkinInfo> {
    let session = active_session(&launcher)
        .await?
        .ok_or_else(|| AppError::msg("Nenhuma conta Microsoft ativa"))?;
    let profile: serde_json::Value = launcher
        .http
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&session.access_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let active_skin = profile["skins"]
        .as_array()
        .and_then(|skins| skins.iter().find(|s| s["state"] == "ACTIVE"));
    Ok(SkinInfo {
        url: active_skin.and_then(|s| s["url"].as_str().map(String::from)),
        variant: active_skin.and_then(|s| s["variant"].as_str().map(String::from)),
        capes: profile["capes"]
            .as_array()
            .map(|capes| {
                capes
                    .iter()
                    .filter_map(|c| c["alias"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
    })
}

/// Troca a skin da conta ativa. `png_base64` é o conteúdo do arquivo PNG;
/// `variant` é "classic" ou "slim".
#[tauri::command]
pub async fn upload_skin(
    launcher: State<'_, Launcher>,
    png_base64: String,
    variant: String,
) -> Result<()> {
    use base64::Engine;
    if variant != "classic" && variant != "slim" {
        return Err(AppError::msg("Variante deve ser 'classic' ou 'slim'"));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(png_base64.trim())
        .map_err(|_| AppError::msg("PNG inválido (base64)"))?;
    if bytes.len() > 512 * 1024 {
        return Err(AppError::msg("Arquivo muito grande — skins têm no máximo 24 KB"));
    }
    let session = active_session(&launcher)
        .await?
        .ok_or_else(|| AppError::msg("Nenhuma conta Microsoft ativa"))?;

    let form = reqwest::multipart::Form::new()
        .text("variant", variant)
        .part(
            "file",
            reqwest::multipart::Part::bytes(bytes)
                .file_name("skin.png")
                .mime_str("image/png")
                .map_err(|e| AppError::msg(e.to_string()))?,
        );
    launcher
        .http
        .post("https://api.minecraftservices.com/minecraft/profile/skins")
        .bearer_auth(&session.access_token)
        .multipart(form)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

// ------------------------- Comandos de conta -------------------------

#[tauri::command]
pub fn get_accounts(launcher: State<'_, Launcher>) -> Result<Vec<AccountPublic>> {
    let accounts = load_accounts(&launcher);
    Ok(accounts
        .accounts
        .iter()
        .map(|a| AccountPublic {
            uuid: a.uuid.clone(),
            name: a.name.clone(),
            active: accounts.active.as_deref() == Some(a.uuid.as_str()),
        })
        .collect())
}

#[tauri::command]
pub fn set_active_account(launcher: State<'_, Launcher>, uuid: Option<String>) -> Result<()> {
    let mut accounts = load_accounts(&launcher);
    accounts.active = uuid;
    store_accounts(&launcher, &accounts)
}

#[tauri::command]
pub fn remove_account(launcher: State<'_, Launcher>, uuid: String) -> Result<()> {
    let mut accounts = load_accounts(&launcher);
    accounts.accounts.retain(|a| a.uuid != uuid);
    if accounts.active.as_deref() == Some(uuid.as_str()) {
        accounts.active = None;
    }
    store_accounts(&launcher, &accounts)
}
